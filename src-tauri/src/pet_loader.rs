use crate::asar::{AsarReader, list_files};
use crate::types::PetInfo;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PetJson {
    id: String,
    display_name: String,
    description: String,
    spritesheet_path: String,
}

pub fn pets_dir() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("CODEX_PETS_DIR") {
        if !custom.is_empty() {
            return Some(PathBuf::from(custom));
        }
    }
    let home = dirs::home_dir()?;
    Some(home.join(".codex/pets"))
}

pub fn load_pets() -> Vec<PetInfo> {
    let mut pets = load_user_pets();
    pets.extend(load_codex_builtin_pets());
    pets
}

fn load_user_pets() -> Vec<PetInfo> {
    let Some(pets_dir) = pets_dir() else {
        return vec![];
    };
    let Ok(canonical_root) = pets_dir.canonicalize() else {
        return vec![];
    };

    let mut pets = Vec::new();
    let entries = match std::fs::read_dir(&canonical_root) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for entry in entries.flatten() {
        // Reject symlinks at the top level outright; they can point anywhere.
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            continue;
        }
        let pet_dir = entry.path();
        let Ok(canonical_pet_dir) = pet_dir.canonicalize() else {
            continue;
        };
        // Defense in depth: even after canonicalize, the dir must still live
        // under the pet root.
        if !canonical_pet_dir.starts_with(&canonical_root) {
            continue;
        }

        let json_path = canonical_pet_dir.join("pet.json");
        if !json_path.exists() {
            continue;
        }

        let raw = match std::fs::read_to_string(&json_path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let json: PetJson = match serde_json::from_str(&raw) {
            Ok(j) => j,
            Err(_) => continue,
        };
        if !is_safe_pet_id(&json.id) {
            continue;
        }

        // Honor the manifest's spritesheetPath (documented in the README) but
        // refuse any value that's absolute, contains a `..` segment, or
        // canonicalizes outside the pet directory. read_pet_image
        // independently rechecks against allowed roots, but stopping a bad
        // value here means it never even shows up in the menu.
        let manifest_sprite = std::path::Path::new(&json.spritesheet_path);
        if manifest_sprite.is_absolute()
            || manifest_sprite
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            continue;
        }
        let sprite_path = canonical_pet_dir.join(manifest_sprite);

        // Reject symlinked or out-of-tree spritesheets so read_pet_image can't
        // be tricked into reading e.g. ~/.ssh/id_rsa via a link inside the
        // pet folder.
        let Ok(canonical_sprite) = sprite_path.canonicalize() else {
            continue;
        };
        if !canonical_sprite.starts_with(&canonical_pet_dir) {
            continue;
        }
        if let Ok(meta) = std::fs::symlink_metadata(&canonical_sprite) {
            if meta.file_type().is_symlink() {
                continue;
            }
        }

        pets.push(PetInfo {
            id: json.id,
            display_name: json.display_name,
            description: json.description,
            spritesheet_path: json.spritesheet_path,
            directory: canonical_pet_dir.to_string_lossy().into_owned(),
            spritesheet_abs_path: canonical_sprite.to_string_lossy().into_owned(),
        });
    }

    pets
}

/// Pet IDs flow into filenames in the cache dir and into IPC; restrict them to
/// a safe character set so they can never contain `..`, path separators, or
/// other shell-special characters.
fn is_safe_pet_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn codex_asar_path() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("CODEX_ASAR_PATH") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Some(p);
        }
    }

    // Codex.app is shipped as a macOS Electron build; on Linux/Windows the
    // user has to point CODEX_ASAR_PATH at the right location themselves.
    #[cfg(target_os = "macos")]
    {
        let candidates: &[&str] = &["/Applications/Codex.app/Contents/Resources/app.asar"];
        for candidate in candidates {
            let p = PathBuf::from(candidate);
            if p.exists() {
                return Some(p);
            }
        }
        if let Some(home) = dirs::home_dir() {
            let user_app = home.join("Applications/Codex.app/Contents/Resources/app.asar");
            if user_app.exists() {
                return Some(user_app);
            }
        }
    }

    None
}

fn load_codex_builtin_pets() -> Vec<PetInfo> {
    let Some(asar_path) = codex_asar_path() else {
        return vec![];
    };

    let cache_dir = match builtin_cache_dir() {
        Some(d) => d,
        None => return vec![],
    };
    if std::fs::create_dir_all(&cache_dir).is_err() {
        return vec![];
    }

    let mut reader = match AsarReader::open(&asar_path) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    // Snapshot file paths so the borrow on `reader.header` ends before we
    // mutably borrow `reader` for read_file calls.
    let entries = list_files(&reader.header);
    let mut pets = Vec::new();
    let canonical_cache = match cache_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => return vec![],
    };
    for path in entries {
        let Some(file_name) = path.rsplit('/').next() else {
            continue;
        };
        // Match files like "codex-spritesheet-v4-Bl6P89d_.webp"
        if !file_name.ends_with(".webp") {
            continue;
        }
        let id = match extract_pet_id(file_name) {
            Some(id) => id,
            None => continue,
        };
        // The asar header is untrusted input — a malicious archive could try
        // `../../LaunchAgents/x` as the prefix. is_safe_pet_id forbids dots
        // and separators so the id can only become a flat filename, but we
        // still verify the canonical destination stays inside the cache dir
        // before writing anything.
        let dest = canonical_cache.join(format!("{}.webp", id));
        if !dest.parent().is_some_and(|p| p == canonical_cache) {
            continue;
        }

        // TOCTOU defense: a local attacker (or a resurrected stale entry)
        // could pre-create dest as a symlink before we run, and we'd
        // happily skip writing and hand the symlink path to read_pet_image.
        // Always remove the file if its symlink_metadata says it's a link;
        // any rewrite below proceeds via O_EXCL semantics on platforms that
        // support it via std::fs.
        if let Ok(meta) = std::fs::symlink_metadata(&dest) {
            if meta.file_type().is_symlink() {
                let _ = std::fs::remove_file(&dest);
            }
        }

        if !dest.exists() {
            let bytes = match reader.read_file(&path) {
                Ok(b) => b,
                Err(_) => continue,
            };
            // Use OpenOptions::create_new to refuse if the path materialized
            // between the remove and the write (symlink race), then write
            // bytes into the actual regular file.
            use std::fs::OpenOptions;
            use std::io::Write;
            let mut file = match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&dest)
            {
                Ok(f) => f,
                Err(_) => continue,
            };
            if file.write_all(&bytes).is_err() {
                let _ = std::fs::remove_file(&dest);
                continue;
            }
        }

        // Final canonicalization: the resolved sprite path must live inside
        // canonical_cache. This rejects any symlink that snuck in despite
        // the prior checks.
        let canonical_dest = match dest.canonicalize() {
            Ok(p) => p,
            Err(_) => continue,
        };
        if !canonical_dest.starts_with(&canonical_cache) {
            continue;
        }

        pets.push(PetInfo {
            id: id.clone(),
            display_name: pretty_name(&id),
            description: format!("Built-in Codex pet ({})", id),
            spritesheet_path: format!("{}.webp", id),
            directory: canonical_cache.to_string_lossy().into_owned(),
            spritesheet_abs_path: canonical_dest.to_string_lossy().into_owned(),
        });
    }
    pets
}

/// Approved roots that `read_pet_image` is allowed to serve bytes from.
/// Any spritesheet path passed through IPC must canonicalize into one of
/// these directories.
pub fn allowed_pet_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(d) = pets_dir().and_then(|p| p.canonicalize().ok()) {
        roots.push(d);
    }
    if let Some(d) = builtin_cache_dir().and_then(|p| p.canonicalize().ok()) {
        roots.push(d);
    }
    roots
}

fn builtin_cache_dir() -> Option<PathBuf> {
    // dirs::cache_dir() is platform-aware:
    //   macOS:   ~/Library/Caches
    //   Linux:   $XDG_CACHE_HOME or ~/.cache
    //   Windows: %LOCALAPPDATA%
    Some(dirs::cache_dir()?.join("codex-pet-desktop/builtin-pets"))
}

/// Codex bundles spritesheets under stable names with a hash suffix:
///   "codex-spritesheet-v4-Bl6P89d_.webp" → "codex"
///   "null-signal-spritesheet-v4-CCoTR-8t.webp" → "null-signal"
///
/// The asar header is untrusted input. Reject anything that isn't a flat
/// `[A-Za-z0-9_-]+` token so the id can't carry `..`, `/`, or other
/// path-traversal payloads into the cache filename.
fn extract_pet_id(file_name: &str) -> Option<String> {
    const NEEDLE: &str = "-spritesheet-v4-";
    let idx = file_name.find(NEEDLE)?;
    let candidate = &file_name[..idx];
    if !is_safe_pet_id(candidate) {
        return None;
    }
    Some(candidate.to_string())
}

fn pretty_name(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    let mut capitalize = true;
    for ch in id.chars() {
        if ch == '-' || ch == '_' {
            out.push(' ');
            capitalize = true;
        } else if capitalize {
            for c in ch.to_uppercase() {
                out.push(c);
            }
            capitalize = false;
        } else {
            out.push(ch);
        }
    }
    out
}

#[allow(dead_code)]
fn _unused(_: &Path) {}

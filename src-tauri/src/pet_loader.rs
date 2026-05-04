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
    if !pets_dir.exists() {
        return vec![];
    }

    let mut pets = Vec::new();
    let entries = match std::fs::read_dir(&pets_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let json_path = path.join("pet.json");
        let sprite_path = path.join("spritesheet.webp");
        if !json_path.exists() || !sprite_path.exists() {
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
        pets.push(PetInfo {
            id: json.id,
            display_name: json.display_name,
            description: json.description,
            spritesheet_path: json.spritesheet_path,
            directory: path.to_string_lossy().into_owned(),
            spritesheet_abs_path: sprite_path.to_string_lossy().into_owned(),
        });
    }

    pets
}

const CODEX_APP_PATHS: &[&str] = &[
    "/Applications/Codex.app/Contents/Resources/app.asar",
    // user-installed location
];

fn codex_asar_path() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("CODEX_ASAR_PATH") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Some(p);
        }
    }
    for candidate in CODEX_APP_PATHS {
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
        let dest = cache_dir.join(format!("{}.webp", id));

        // Extract once, then keep using the cached file.
        if !dest.exists() {
            let bytes = match reader.read_file(&path) {
                Ok(b) => b,
                Err(_) => continue,
            };
            if std::fs::write(&dest, &bytes).is_err() {
                continue;
            }
        }

        pets.push(PetInfo {
            id: id.clone(),
            display_name: pretty_name(&id),
            description: format!("Built-in Codex pet ({})", id),
            spritesheet_path: format!("{}.webp", id),
            directory: cache_dir.to_string_lossy().into_owned(),
            spritesheet_abs_path: dest.to_string_lossy().into_owned(),
        });
    }
    pets
}

fn builtin_cache_dir() -> Option<PathBuf> {
    if let Some(home) = dirs::home_dir() {
        return Some(home.join("Library/Caches/codex-pet-desktop/builtin-pets"));
    }
    None
}

/// Codex bundles spritesheets under stable names with a hash suffix:
///   "codex-spritesheet-v4-Bl6P89d_.webp" → "codex"
///   "null-signal-spritesheet-v4-CCoTR-8t.webp" → "null-signal"
fn extract_pet_id(file_name: &str) -> Option<String> {
    const NEEDLE: &str = "-spritesheet-v4-";
    let idx = file_name.find(NEEDLE)?;
    Some(file_name[..idx].to_string())
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

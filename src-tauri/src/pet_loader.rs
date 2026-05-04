use crate::types::PetInfo;
use serde::Deserialize;
use std::path::PathBuf;

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

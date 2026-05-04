use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub selected_pet_id: Option<String>,
    pub position: Position,
    pub scale: f64,
    pub always_on_top: bool,
    pub poll_interval_ms: u64,
    pub text_size: String,
    pub enabled_agents: HashMap<String, bool>,
    pub animation_speeds: HashMap<String, f64>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut enabled_agents = HashMap::new();
        enabled_agents.insert("opencode".into(), true);
        enabled_agents.insert("claude-code".into(), true);
        enabled_agents.insert("codex".into(), true);

        Self {
            selected_pet_id: None,
            position: Position { x: 0.0, y: 0.0 },
            scale: 1.0,
            always_on_top: true,
            poll_interval_ms: 30_000,
            text_size: "medium".into(),
            enabled_agents,
            animation_speeds: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThreadStatus {
    Busy,
    Idle,
    Waiting,
    Error,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveThread {
    pub tool: String,
    pub status: ThreadStatus,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetInfo {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub spritesheet_path: String,
    pub directory: String,
    pub spritesheet_abs_path: String,
}

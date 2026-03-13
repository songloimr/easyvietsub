use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilities {
    pub os: String,
    pub ffmpeg_available: bool,
    pub ffprobe_available: bool,
    pub local_ffmpeg_installed: bool,
    pub local_ffprobe_installed: bool,
    pub hardware_acceleration_available: bool,
    pub detected_accelerators: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub api_key: String,
    pub output_directory: String,
    pub last_opened_project_path: String,
    pub saved_translation_instruction: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            output_directory: String::new(),
            last_opened_project_path: String::new(),
            saved_translation_instruction: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshot {
    pub version: u32,
    pub exported_at: String,
    pub job: Value,
}

pub fn migrate_project_snapshot(snapshot: ProjectSnapshot) -> Result<ProjectSnapshot, String> {
    match snapshot.version {
        1 => Ok(snapshot),
        _ => Err("Project schema version hiện chưa được hỗ trợ để migrate.".into()),
    }
}

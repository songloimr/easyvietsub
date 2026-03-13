use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioTrackInfo {
    pub index: i64,
    pub codec: String,
    pub channels: u64,
    pub language: Option<String>,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MediaInspection {
    pub path: String,
    pub kind: String,
    pub duration_seconds: f64,
    pub file_size_bytes: u64,
    pub audio_tracks: Vec<AudioTrackInfo>,
    pub sample_rate: Option<u64>,
}

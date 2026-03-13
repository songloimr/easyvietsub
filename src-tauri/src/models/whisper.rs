use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WhisperModelOption {
    pub id: String,
    pub label: String,
    pub filename: String,
    pub size_bytes: u64,
    pub description: String,
    pub downloaded: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperRequest {
    pub audio_path: String,
    pub track_index: u32,
    pub source_language: String,
    pub model_id: String,
    pub compute_mode: String,
}

pub fn whisper_model_catalog() -> Vec<(String, String, String, u64, String, String)> {
    vec![
        (
            "small".into(),
            "small".into(),
            "ggml-small.bin".into(),
            488 * 1024 * 1024,
            "Độ chính xác cao hơn với chi phí inference lớn hơn.".into(),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin".into(),
        ),
        (
            "medium".into(),
            "medium".into(),
            "ggml-medium.bin".into(),
            1_530 * 1024 * 1024,
            "Phù hợp media dài hoặc âm thanh khó.".into(),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin".into(),
        ),
        (
            "large".into(),
            "large".into(),
            "ggml-large-v3.bin".into(),
            3_090 * 1024 * 1024,
            "Chất lượng tốt nhất nhưng đòi hỏi tài nguyên cao.".into(),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin".into(),
        ),
    ]
}

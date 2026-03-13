use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PipelineProgressEvent {
    pub job_id: String,
    pub phase: String,
    pub progress: f64,
    pub message: String,
    pub eta_seconds: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgressEvent {
    pub model_id: String,
    pub progress: u64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDownloadProgressEvent {
    pub stage: String,
    pub progress: u64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub message: String,
}

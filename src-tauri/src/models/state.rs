use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri_plugin_shell::process::CommandChild;
use whisper_rs::WhisperContext;

pub struct RunningProcesses {
    pub jobs: Mutex<HashMap<String, CommandChild>>,
    /// Cancel tokens for Whisper transcription jobs (checked during inference and segment reading).
    pub whisper_cancel_tokens: Mutex<HashMap<String, Arc<AtomicBool>>>,
    /// Cancel tokens for Gemini translation jobs (checked between chunks).
    pub gemini_cancel_tokens: Mutex<HashMap<String, Arc<AtomicBool>>>,
    /// Cached WhisperContext to avoid reloading the model on every transcription.
    /// Tuple: (model_path, use_gpu, context).
    pub whisper_ctx_cache: Mutex<Option<(String, bool, Arc<WhisperContext>)>>,
    /// Shared HTTP client for Gemini API calls (reuses connections/TLS sessions).
    pub http_client: reqwest::Client,
}

impl Default for RunningProcesses {
    fn default() -> Self {
        Self {
            jobs: Mutex::default(),
            whisper_cancel_tokens: Mutex::default(),
            gemini_cancel_tokens: Mutex::default(),
            whisper_ctx_cache: Mutex::new(None),
            http_client: reqwest::Client::new(),
        }
    }
}

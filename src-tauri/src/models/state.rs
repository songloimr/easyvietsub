use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri_plugin_shell::process::CommandChild;

pub struct RunningProcesses {
    pub jobs: Mutex<HashMap<String, CommandChild>>,
    /// Cancel tokens for Whisper transcription jobs (checked during inference and segment reading).
    pub whisper_cancel_tokens: Mutex<HashMap<String, Arc<AtomicBool>>>,
    /// Cancel tokens for Gemini translation jobs (checked between chunks).
    pub gemini_cancel_tokens: Mutex<HashMap<String, Arc<AtomicBool>>>,
    /// Shared HTTP client for Gemini API calls (reuses connections/TLS sessions).
    pub http_client: reqwest::Client,
}

impl Default for RunningProcesses {
    fn default() -> Self {
        Self {
            jobs: Mutex::default(),
            whisper_cancel_tokens: Mutex::default(),
            gemini_cancel_tokens: Mutex::default(),
            http_client: reqwest::Client::new(),
        }
    }
}

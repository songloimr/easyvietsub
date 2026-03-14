use super::subtitle::SubtitleSegment;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiModelOption {
    pub id: String,
    pub label: String,
    pub description: String,
    pub experimental: Option<bool>,
}

/// Token usage statistics from Gemini API calls.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

impl TokenUsage {
    /// Accumulate tokens from another TokenUsage.
    pub fn accumulate(&mut self, other: &TokenUsage) {
        self.prompt_tokens += other.prompt_tokens;
        self.completion_tokens += other.completion_tokens;
        self.total_tokens += other.total_tokens;
    }
}

/// Translation result containing segments and token usage.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationResult {
    pub segments: Vec<SubtitleSegment>,
    pub token_usage: TokenUsage,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiTranslateRequest {
    pub job_id: String,
    pub api_key: String,
    pub model_id: String,
    pub translation_instruction: String,
    pub segments: Vec<SubtitleSegment>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiDirectRequest {
    pub job_id: String,
    pub api_key: String,
    pub model_id: String,
    pub audio_path: String,
    pub track_index: u32,
    pub duration_seconds: f64,
    pub source_language: String,
    pub translation_instruction: String,
}

pub const GEMINI_MODELS_ALLOWLIST: &[&str] = &[
    "gemini-3.1-flash-lite-preview",
    "gemini-2.5-flash-lite",
    "gemini-2.5-flash",
    "gemini-3-flash-preview",
];

/// Number of subtitle segments per translation chunk.
pub const TRANSLATION_CHUNK_SIZE: usize = 300;
/// Minimum chunk size for adaptive chunking.
pub const MIN_CHUNK_SIZE: usize = 50;
/// Maximum chunk size for adaptive chunking.
pub const MAX_CHUNK_SIZE: usize = 500;
/// Number of previously-translated segments sent as context for continuity.
pub const TRANSLATION_CONTEXT_SIZE: usize = 20;
/// Maximum retry depth when splitting an oversized chunk in half.
pub const MAX_CHUNK_SPLIT_DEPTH: usize = 2;
/// Audio chunk duration for Gemini direct mode (seconds).
pub const GEMINI_AUDIO_CHUNK_DURATION_SECS: f64 = 20.0;
/// Maximum number of retries for Gemini API calls (total attempts = MAX_GEMINI_RETRIES + 1)
pub const MAX_GEMINI_RETRIES: u32 = 2;
/// Initial backoff duration in milliseconds for Gemini retries
pub const GEMINI_INITIAL_BACKOFF_MS: u64 = 1000;

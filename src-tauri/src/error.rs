use serde::{Deserialize, Serialize};
use std::fmt;

/// Serializable representation of AppError for frontend consumption.
/// Uses tagged enum format: { "category": "fileSystem", "message": "...", ... }
#[derive(Serialize)]
#[serde(tag = "category", rename_all = "camelCase")]
enum AppErrorKind {
    FileSystem {
        message: String,
        path: Option<String>,
    },
    Network {
        message: String,
        url: Option<String>,
        status_code: Option<u16>,
    },
    MediaProcessing {
        message: String,
        context: Option<String>,
    },
    Model {
        message: String,
        model_name: Option<String>,
    },
    Configuration {
        message: String,
        key: Option<String>,
    },
    Cancelled {
        message: String,
        phase: Option<String>,
    },
    Validation {
        message: String,
        field: Option<String>,
    },
    Unknown {
        message: String,
    },
}

/// Error categories for the application
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "category", rename_all = "camelCase")]
pub enum AppError {
    /// File system related errors (file not found, permission denied, etc.)
    FileSystem {
        message: String,
        path: Option<String>,
    },
    /// Network/HTTP errors (API calls, downloads, etc.)
    Network {
        message: String,
        url: Option<String>,
        status_code: Option<u16>,
    },
    /// Media processing errors (FFmpeg, Whisper, audio decoding, etc.)
    MediaProcessing {
        message: String,
        context: Option<String>,
    },
    /// AI/ML model errors (Whisper, Gemini, model loading, etc.)
    Model {
        message: String,
        model_name: Option<String>,
    },
    /// Configuration/settings errors
    Configuration {
        message: String,
        key: Option<String>,
    },
    /// User cancellation
    Cancelled {
        message: String,
        phase: Option<String>,
    },
    /// Validation errors (invalid input, bad format, etc.)
    Validation {
        message: String,
        field: Option<String>,
    },
    /// Unexpected/unknown errors
    Unknown { message: String },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::FileSystem { message, path, .. } => {
                if let Some(p) = path {
                    write!(f, "File system error ({}): {}", p, message)
                } else {
                    write!(f, "File system error: {}", message)
                }
            }
            AppError::Network {
                message,
                url,
                status_code,
                ..
            } => {
                let mut s = format!("Network error: {}", message);
                if let Some(code) = status_code {
                    s.push_str(&format!(" (HTTP {})", code));
                }
                if let Some(u) = url {
                    s.push_str(&format!(" [{}]", u));
                }
                write!(f, "{}", s)
            }
            AppError::MediaProcessing {
                message, context, ..
            } => {
                if let Some(ctx) = context {
                    write!(f, "Media processing error ({}): {}", ctx, message)
                } else {
                    write!(f, "Media processing error: {}", message)
                }
            }
            AppError::Model {
                message,
                model_name,
                ..
            } => {
                if let Some(name) = model_name {
                    write!(f, "Model error ({}): {}", name, message)
                } else {
                    write!(f, "Model error: {}", message)
                }
            }
            AppError::Configuration { message, key, .. } => {
                if let Some(k) = key {
                    write!(f, "Configuration error ({}): {}", k, message)
                } else {
                    write!(f, "Configuration error: {}", message)
                }
            }
            AppError::Cancelled { message, phase } => {
                if let Some(p) = phase {
                    write!(f, "Cancelled ({}): {}", p, message)
                } else {
                    write!(f, "Cancelled: {}", message)
                }
            }
            AppError::Validation { message, field } => {
                if let Some(fld) = field {
                    write!(f, "Validation error ({}): {}", fld, message)
                } else {
                    write!(f, "Validation error: {}", message)
                }
            }
            AppError::Unknown { message, .. } => {
                write!(f, "Unknown error: {}", message)
            }
        }
    }
}

impl std::error::Error for AppError {}

// Manual Serialize impl: converts AppError to AppErrorKind (which has #[derive(Serialize)])
// This sends structured JSON to the Tauri frontend instead of a plain string.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let kind = match self {
            AppError::FileSystem { message, path, .. } => AppErrorKind::FileSystem {
                message: message.clone(),
                path: path.clone(),
            },
            AppError::Network {
                message,
                url,
                status_code,
                ..
            } => AppErrorKind::Network {
                message: message.clone(),
                url: url.clone(),
                status_code: *status_code,
            },
            AppError::MediaProcessing {
                message, context, ..
            } => AppErrorKind::MediaProcessing {
                message: message.clone(),
                context: context.clone(),
            },
            AppError::Model {
                message,
                model_name,
                ..
            } => AppErrorKind::Model {
                message: message.clone(),
                model_name: model_name.clone(),
            },
            AppError::Configuration { message, key, .. } => AppErrorKind::Configuration {
                message: message.clone(),
                key: key.clone(),
            },
            AppError::Cancelled { message, phase } => AppErrorKind::Cancelled {
                message: message.clone(),
                phase: phase.clone(),
            },
            AppError::Validation { message, field } => AppErrorKind::Validation {
                message: message.clone(),
                field: field.clone(),
            },
            AppError::Unknown { message, .. } => AppErrorKind::Unknown {
                message: message.clone(),
            },
        };
        kind.serialize(serializer)
    }
}

// Conversion helpers
impl AppError {
    pub fn file_system(message: impl Into<String>, path: Option<String>) -> Self {
        AppError::FileSystem {
            message: message.into(),
            path,
        }
    }

    pub fn network(
        message: impl Into<String>,
        url: Option<String>,
        status_code: Option<u16>,
    ) -> Self {
        AppError::Network {
            message: message.into(),
            url,
            status_code,
        }
    }

    pub fn media_processing(message: impl Into<String>, context: Option<String>) -> Self {
        AppError::MediaProcessing {
            message: message.into(),
            context,
        }
    }

    pub fn model(message: impl Into<String>, model_name: Option<String>) -> Self {
        AppError::Model {
            message: message.into(),
            model_name,
        }
    }

    pub fn configuration(message: impl Into<String>, key: Option<String>) -> Self {
        AppError::Configuration {
            message: message.into(),
            key,
        }
    }

    pub fn cancelled(message: impl Into<String>, phase: Option<String>) -> Self {
        AppError::Cancelled {
            message: message.into(),
            phase,
        }
    }

    pub fn validation(message: impl Into<String>, field: Option<String>) -> Self {
        AppError::Validation {
            message: message.into(),
            field,
        }
    }

    pub fn unknown(message: impl Into<String>) -> Self {
        AppError::Unknown {
            message: message.into(),
        }
    }
}

// From std::io::Error
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => {
                AppError::file_system("File or directory not found", None)
            }
            std::io::ErrorKind::PermissionDenied => {
                AppError::file_system("Permission denied", None)
            }
            _ => AppError::file_system(format!("I/O error: {}", err), None),
        }
    }
}

// From reqwest::Error
impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        let url = err.url().map(|u| u.to_string());
        let status_code = err.status().map(|s| s.as_u16());

        if err.is_timeout() {
            AppError::network("Request timeout", url, status_code)
        } else if err.is_connect() {
            AppError::network("Connection failed", url, status_code)
        } else {
            AppError::network(format!("HTTP request failed: {}", err), url, status_code)
        }
    }
}

// From serde_json::Error
impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::configuration(format!("JSON parsing error: {}", err), None)
    }
}

// From String (allows gradual migration from Result<T, String>)
impl From<String> for AppError {
    fn from(msg: String) -> Self {
        AppError::unknown(msg)
    }
}

// From &str
impl From<&str> for AppError {
    fn from(msg: &str) -> Self {
        AppError::unknown(msg)
    }
}

// From which::Error
impl From<which::Error> for AppError {
    fn from(err: which::Error) -> Self {
        AppError::configuration(format!("Binary not found: {}", err), None)
    }
}

// From zip::result::ZipError
impl From<zip::result::ZipError> for AppError {
    fn from(err: zip::result::ZipError) -> Self {
        AppError::file_system(format!("ZIP error: {}", err), None)
    }
}

// From std::num::ParseIntError
impl From<std::num::ParseIntError> for AppError {
    fn from(err: std::num::ParseIntError) -> Self {
        AppError::validation(format!("Invalid number: {}", err), None)
    }
}

// From tauri::Error
impl From<tauri::Error> for AppError {
    fn from(err: tauri::Error) -> Self {
        AppError::configuration(format!("Tauri error: {}", err), None)
    }
}

// From Mutex PoisonError (generic)
impl<T> From<std::sync::PoisonError<T>> for AppError {
    fn from(_err: std::sync::PoisonError<T>) -> Self {
        AppError::unknown("Internal lock poisoned")
    }
}

// From tauri_plugin_shell::Error
impl From<tauri_plugin_shell::Error> for AppError {
    fn from(err: tauri_plugin_shell::Error) -> Self {
        AppError::media_processing(format!("Shell error: {}", err), None)
    }
}

// Result type alias for convenience
pub type AppResult<T> = Result<T, AppError>;

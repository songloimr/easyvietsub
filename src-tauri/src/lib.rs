use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_shell::{
    process::{CommandChild, CommandEvent},
    ShellExt,
};

use tokio::io::AsyncWriteExt;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const GEMINI_MODELS_ALLOWLIST: &[&str] = &[
    "gemini-3.1-flash-lite-preview",
    "gemini-2.5-flash-lite",
    "gemini-2.5-flash",
    "gemini-3-flash-preview",
];

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AudioTrackInfo {
    index: i64,
    codec: String,
    channels: u64,
    language: Option<String>,
    title: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MediaInspection {
    path: String,
    kind: String,
    duration_seconds: f64,
    file_size_bytes: u64,
    audio_tracks: Vec<AudioTrackInfo>,
    sample_rate: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WhisperModelOption {
    id: String,
    label: String,
    filename: String,
    size_bytes: u64,
    description: String,
    downloaded: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiModelOption {
    id: String,
    label: String,
    description: String,
    experimental: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SubtitleSegment {
    id: String,
    start_ms: u64,
    end_ms: u64,
    source_text: String,
    translated_text: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeCapabilities {
    os: String,
    ffmpeg_available: bool,
    ffprobe_available: bool,
    local_ffmpeg_installed: bool,
    local_ffprobe_installed: bool,
    hardware_acceleration_available: bool,
    detected_accelerators: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AppSettings {
    api_key: String,
    output_directory: String,
    last_opened_project_path: String,
    saved_translation_instruction: String,
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
struct ProjectSnapshot {
    version: u32,
    exported_at: String,
    job: Value,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PipelineProgressEvent {
    job_id: String,
    phase: String,
    progress: f64,
    message: String,
    eta_seconds: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ModelDownloadProgressEvent {
    model_id: String,
    progress: u64,
    downloaded_bytes: u64,
    total_bytes: u64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RuntimeDownloadProgressEvent {
    stage: String,
    progress: u64,
    downloaded_bytes: u64,
    total_bytes: u64,
    message: String,
}

#[derive(Default)]
struct RunningProcesses {
    jobs: Mutex<HashMap<String, CommandChild>>,
    whisper_cancel_tokens: Mutex<HashMap<String, Arc<std::sync::atomic::AtomicBool>>>,
}

fn migrate_project_snapshot(snapshot: ProjectSnapshot) -> Result<ProjectSnapshot, String> {
    match snapshot.version {
        1 => Ok(snapshot),
        _ => Err("Project schema version hiện chưa được hỗ trợ để migrate.".into()),
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WhisperRequest {
    audio_path: String,
    source_language: String,
    model_id: String,
    compute_mode: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiTranslateRequest {
    api_key: String,
    model_id: String,
    translation_instruction: String,
    segments: Vec<SubtitleSegment>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiDirectRequest {
    api_key: String,
    model_id: String,
    audio_path: String,
    source_language: String,
    translation_instruction: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenderStyle {
    font_family: String,
    font_size: u64,
    text_color: String,
    outline_color: String,
    outline_width: u64,
    background_color: String,
    line_spacing: u64,
    margin_x: u64,
    margin_y: u64,
    bold: bool,
    italic: bool,
    position: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenderRequest {
    input_path: String,
    output_path: String,
    subtitle_content: String,
    style: RenderStyle,
}

fn whisper_model_catalog() -> Vec<(String, String, String, u64, String, String)> {
    vec![
        (
            "tiny".into(),
            "tiny".into(),
            "ggml-tiny.bin".into(),
            78 * 1024 * 1024,
            "Nhanh nhất, phù hợp preview hoặc máy yếu.".into(),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin".into(),
        ),
        (
            "base".into(),
            "base".into(),
            "ggml-base.bin".into(),
            148 * 1024 * 1024,
            "Cân bằng tốt cho media ngắn và vừa.".into(),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin".into(),
        ),
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

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn app_cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("models");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn tools_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("tools");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn fonts_dir(app: &AppHandle) -> Result<PathBuf, String> {
    // Priority 1: Try bundled resources directory (production)
    let resources_dir = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?;
    
    if resources_dir.join("Arial.ttf").exists() {
        return Ok(resources_dir);
    }
    
    // Priority 2: Development mode - walk up from exe dir to find fonts/ folder.
    // macOS dev:   target/debug/bundle/macos/App.app/Contents/MacOS/exe (many levels)
    // Windows dev: target/debug/exe.exe (fewer levels)
    // Walk up to a reasonable depth (max 8 ancestors) to stay robust across OSes.
    let exe_path = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut ancestor = exe_path.as_path();
    for _ in 0..8 {
        if let Some(parent) = ancestor.parent() {
            let candidate = parent.join("fonts");
            if candidate.join("Arial.ttf").exists() {
                return Ok(candidate);
            }
            ancestor = parent;
        } else {
            break;
        }
    }
    
    // Fallback: app data dir with fonts subdirectory
    let dir = app_data_dir(app)?.join("fonts");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("settings.json"))
}

fn command_filename(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn managed_binary_path(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    Ok(tools_dir(app)?.join(command_filename(name)))
}

fn bundled_sidecar_path(name: &str) -> Result<PathBuf, String> {
    let exe_path = std::env::current_exe().map_err(|error| error.to_string())?;
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| "Không xác định được thư mục executable.".to_string())?;
    let base_dir = if exe_dir.ends_with("deps") {
        exe_dir.parent().unwrap_or(exe_dir)
    } else {
        exe_dir
    };

    Ok(base_dir.join(command_filename(name)))
}

fn resolve_runtime_binary_path(
    app: &AppHandle,
    managed_name: &str,
    sidecar_name: &str,
    system_name: &str,
) -> Result<PathBuf, String> {
    let managed = managed_binary_path(app, managed_name)?;
    if managed.exists() {
        return Ok(managed);
    }

    let sidecar = bundled_sidecar_path(sidecar_name)?;
    if sidecar.exists() {
        return Ok(sidecar);
    }

    which::which(system_name).map_err(|error| error.to_string())
}

fn resolve_ffmpeg_path(app: &AppHandle) -> Result<PathBuf, String> {
    resolve_runtime_binary_path(app, "ffmpeg", "ffmpeg-sidecar", "ffmpeg")
}

fn resolve_ffprobe_path(app: &AppHandle) -> Result<PathBuf, String> {
    resolve_runtime_binary_path(app, "ffprobe", "ffprobe-sidecar", "ffprobe")
}

async fn download_to_path_with_progress(
    app: &AppHandle,
    url: &str,
    destination: &Path,
    stage: &str,
    start_progress: u64,
    end_progress: u64,
    message: &str,
) -> Result<(), String> {
    let mut response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!(
            "Tải runtime thất bại với status {}",
            response.status()
        ));
    }

    let total_bytes = response.content_length().unwrap_or_default();
    let mut downloaded_bytes = 0_u64;
    let mut file = tokio::fs::File::create(destination)
        .await
        .map_err(|error| error.to_string())?;

    emit_runtime_download_progress(
        app,
        stage,
        start_progress,
        downloaded_bytes,
        total_bytes,
        message.to_string(),
    );

    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        file.write_all(&chunk)
            .await
            .map_err(|error| error.to_string())?;
        downloaded_bytes += chunk.len() as u64;

        let progress = if total_bytes == 0 {
            start_progress
        } else {
            let ratio = downloaded_bytes as f64 / total_bytes as f64;
            (start_progress as f64 + ratio * (end_progress - start_progress) as f64)
                .round()
                .clamp(start_progress as f64, end_progress as f64) as u64
        };

        emit_runtime_download_progress(
            app,
            stage,
            progress,
            downloaded_bytes,
            total_bytes,
            format!("{message} ({progress}%)"),
        );
    }

    file.flush().await.map_err(|error| error.to_string())?;

    emit_runtime_download_progress(
        app,
        stage,
        end_progress,
        downloaded_bytes,
        total_bytes,
        format!("{message} ({end_progress}%)"),
    );

    Ok(())
}

fn extract_first_file_from_zip(zip_path: &Path, destination: &Path) -> Result<(), String> {
    let source = fs::File::open(zip_path).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(source).map_err(|error| error.to_string())?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        if !entry.is_file() {
            continue;
        }

        let mut output = fs::File::create(destination).map_err(|error| error.to_string())?;
        io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = output
                .metadata()
                .map_err(|error| error.to_string())?
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(destination, permissions).map_err(|error| error.to_string())?;
        }

        return Ok(());
    }

    Err("Archive runtime không chứa binary hợp lệ.".into())
}

/// Extract a named file from a 7z archive to destination path.
/// Searches for file_name in any subfolder (e.g., "ffmpeg.exe" matches "*/bin/ffmpeg.exe").
fn extract_named_file_from_7z(
    archive_path: &Path,
    file_name: &str,
    destination: &Path,
) -> Result<(), String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    // Create temp directory to extract full archive
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let temp_dir = std::env::temp_dir().join(format!("easyvietsub_7z_{}", timestamp));
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    // Extract full archive to temp dir
    sevenz_rust::decompress_file(archive_path, &temp_dir)
        .map_err(|e| format!("Không thể giải nén 7z archive: {}", e))?;

    // Find the target file recursively
    let mut found_path: Option<PathBuf> = None;
    fn walk_dir(dir: &Path, file_name: &str, found: &mut Option<PathBuf>) -> io::Result<()> {
        if found.is_some() {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk_dir(&path, file_name, found)?;
            } else if let Some(name) = path.file_name() {
                if name == file_name {
                    *found = Some(path.clone());
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    walk_dir(&temp_dir, file_name, &mut found_path).map_err(|e| e.to_string())?;

    if let Some(source) = found_path {
        // Copy to destination
        fs::copy(&source, destination).map_err(|e| e.to_string())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(destination)
                .map_err(|e| e.to_string())?
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(destination, permissions).map_err(|e| e.to_string())?;
        }

        // Cleanup temp dir
        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    } else {
        let _ = fs::remove_dir_all(&temp_dir);
        Err(format!(
            "Không tìm thấy file '{}' trong 7z archive.",
            file_name
        ))
    }
}

fn emit_progress(
    app: &AppHandle,
    job_id: &str,
    phase: &str,
    progress: f64,
    message: impl Into<String>,
    eta_seconds: Option<u64>,
) {
    let _ = app.emit(
        "pipeline-progress",
        PipelineProgressEvent {
            job_id: job_id.to_string(),
            phase: phase.to_string(),
            progress,
            message: message.into(),
            eta_seconds,
        },
    );
}

fn emit_runtime_download_progress(
    app: &AppHandle,
    stage: &str,
    progress: u64,
    downloaded_bytes: u64,
    total_bytes: u64,
    message: String,
) {
    let _ = app.emit(
        "runtime-download-progress",
        RuntimeDownloadProgressEvent {
            stage: stage.to_string(),
            progress,
            downloaded_bytes,
            total_bytes,
            message,
        },
    );
}

fn register_running_process(
    state: &State<RunningProcesses>,
    job_id: &str,
    child: CommandChild,
) -> Result<(), String> {
    let mut jobs = state
        .jobs
        .lock()
        .map_err(|_| "Không khóa được process registry.".to_string())?;
    if let Some(existing) = jobs.remove(job_id) {
        let _ = existing.kill();
    }
    jobs.insert(job_id.to_string(), child);
    Ok(())
}

fn take_running_process(
    state: &State<RunningProcesses>,
    job_id: &str,
) -> Result<Option<CommandChild>, String> {
    let mut jobs = state
        .jobs
        .lock()
        .map_err(|_| "Không khóa được process registry.".to_string())?;
    Ok(jobs.remove(job_id))
}

fn parse_ffmpeg_progress_ms(line: &str) -> Option<u64> {
    let trimmed = line.trim();
    if let Some(value) = trimmed.strip_prefix("out_time_ms=") {
        return value.parse::<u64>().ok();
    }

    if let Some(value) = trimmed.strip_prefix("out_time_us=") {
        return value.parse::<u64>().ok().map(|micro| micro / 1000);
    }

    None
}

fn guess_accelerators() -> Vec<String> {
    match std::env::consts::OS {
        "macos" => vec!["videotoolbox".into(), "metal".into()],
        "windows" => vec!["d3d12va".into(), "dxva2".into()],
        _ => Vec::new(),
    }
}

fn hardware_acceleration_available() -> bool {
    !guess_accelerators().is_empty()
}

fn parse_hex_color(input: &str) -> (u8, u8, u8, u8) {
    let clean = input.trim().trim_start_matches('#');
    match clean.len() {
        6 => {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(255);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(255);
            (255, r, g, b)
        }
        8 => {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(255);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(255);
            let a = u8::from_str_radix(&clean[6..8], 16).unwrap_or(255);
            (a, r, g, b)
        }
        _ => (255, 255, 255, 255),
    }
}

fn ass_color(input: &str) -> String {
    let (alpha, red, green, blue) = parse_hex_color(input);
    let inverse_alpha = 255u8.saturating_sub(alpha);
    format!(
        "&H{:02X}{:02X}{:02X}{:02X}",
        inverse_alpha, blue, green, red
    )
}

fn ass_alignment(position: &str) -> u8 {
    match position {
        "top" => 8,
        "center" => 5,
        _ => 2,
    }
}

fn srt_timestamp_to_ms(input: &str) -> Result<u64, String> {
    let parts: Vec<&str> = input.split([':', ',']).collect();
    if parts.len() != 4 {
        return Err(format!("Timestamp không hợp lệ: {input}"));
    }

    let hours = parts[0].parse::<u64>().map_err(|error| error.to_string())?;
    let minutes = parts[1].parse::<u64>().map_err(|error| error.to_string())?;
    let seconds = parts[2].parse::<u64>().map_err(|error| error.to_string())?;
    let millis = parts[3].parse::<u64>().map_err(|error| error.to_string())?;
    Ok(hours * 3_600_000 + minutes * 60_000 + seconds * 1_000 + millis)
}

fn ms_to_ass_timestamp(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let centiseconds = (ms % 1_000) / 10;
    format!("{hours}:{minutes:02}:{seconds:02}.{centiseconds:02}")
}

fn parse_srt(input: &str) -> Result<Vec<SubtitleSegment>, String> {
    let normalized = input.replace("\r\n", "\n");
    let mut segments = Vec::new();

    for (index, block) in normalized.split("\n\n").enumerate() {
        let lines: Vec<&str> = block
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        if lines.len() < 3 {
            continue;
        }

        let time_line = lines[1];
        let timestamps: Vec<&str> = time_line.split(" --> ").collect();
        if timestamps.len() != 2 {
            return Err(format!("Block SRT không hợp lệ ở index {index}"));
        }

        let start_ms = srt_timestamp_to_ms(timestamps[0])?;
        let end_ms = srt_timestamp_to_ms(timestamps[1])?;
        let text = lines[2..].join("\n");

        segments.push(SubtitleSegment {
            id: format!("segment-{}", index + 1),
            start_ms,
            end_ms,
            source_text: text.clone(),
            translated_text: text,
        });
    }

    Ok(segments)
}

fn write_ass_file(
    path: &Path,
    segments: &[SubtitleSegment],
    style: &RenderStyle,
) -> Result<(), String> {
    let primary = ass_color(&style.text_color);
    let outline = ass_color(&style.outline_color);
    let back = ass_color(&style.background_color);
    let alignment = ass_alignment(&style.position);
    let bold = if style.bold { -1 } else { 0 };
    let italic = if style.italic { -1 } else { 0 };

    let mut content = String::from(
        "[Script Info]\nScriptType: v4.00+\nWrapStyle: 0\nScaledBorderAndShadow: yes\nYCbCr Matrix: TV.709\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n",
    );

    content.push_str(&format!(
        "Style: Default,{},{},{},&H000000FF,{},{},{},{},0,0,100,100,0,0,1,{},0,{},{},{},{},1\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        style.font_family,
        style.font_size,
        primary,
        outline,
        back,
        bold,
        italic,
        style.outline_width,
        alignment,
        style.margin_x,
        style.margin_x,
        style.margin_y
    ));

    for segment in segments {
        let text = segment
            .translated_text
            .replace('\n', "\\N")
            .replace(',', "\\,")
            .replace('{', "\\{")
            .replace('}', "\\}");
        content.push_str(&format!(
            "Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
            ms_to_ass_timestamp(segment.start_ms),
            ms_to_ass_timestamp(segment.end_ms),
            text
        ));
    }

    fs::write(path, content).map_err(|error| error.to_string())
}

#[tauri::command]
fn export_ass_subtitle(
    path: String,
    segments: Vec<SubtitleSegment>,
    style: RenderStyle,
) -> Result<(), String> {
    let path = Path::new(&path);
    write_ass_file(path, &segments, &style)
}

#[tauri::command]
fn detect_runtime_capabilities(app: AppHandle) -> RuntimeCapabilities {
    let accelerators = guess_accelerators();
    let local_ffmpeg_installed = managed_binary_path(&app, "ffmpeg")
        .map(|path| path.exists())
        .unwrap_or(false);
    let local_ffprobe_installed = managed_binary_path(&app, "ffprobe")
        .map(|path| path.exists())
        .unwrap_or(false);

    RuntimeCapabilities {
        os: std::env::consts::OS.to_string(),
        ffmpeg_available: resolve_ffmpeg_path(&app).is_ok(),
        ffprobe_available: resolve_ffprobe_path(&app).is_ok(),
        local_ffmpeg_installed,
        local_ffprobe_installed,
        hardware_acceleration_available: !accelerators.is_empty(),
        detected_accelerators: accelerators,
    }
}

#[tauri::command]
async fn install_local_ffmpeg_runtime(app: AppHandle) -> Result<RuntimeCapabilities, String> {
    let tools = tools_dir(&app)?;
    let ffmpeg_path = managed_binary_path(&app, "ffmpeg")?;
    let ffprobe_path = managed_binary_path(&app, "ffprobe")?;

    match std::env::consts::OS {
        "macos" => {
            // macOS: Download 2 separate zip files from evermeet.cx
            let ffmpeg_zip = tools.join("ffmpeg-runtime.zip");
            let ffprobe_zip = tools.join("ffprobe-runtime.zip");

            download_to_path_with_progress(
                &app,
                "https://evermeet.cx/ffmpeg/get/zip",
                &ffmpeg_zip,
                "ffmpeg",
                0,
                48,
                "Đang tải FFmpeg local",
            )
            .await?;
            extract_first_file_from_zip(&ffmpeg_zip, &ffmpeg_path)?;
            emit_runtime_download_progress(
                &app,
                "ffmpeg",
                50,
                0,
                0,
                "Đã giải nén FFmpeg local (50%)".to_string(),
            );

            download_to_path_with_progress(
                &app,
                "https://evermeet.cx/ffmpeg/get/ffprobe/zip",
                &ffprobe_zip,
                "ffprobe",
                50,
                98,
                "Đang tải FFprobe local",
            )
            .await?;
            extract_first_file_from_zip(&ffprobe_zip, &ffprobe_path)?;
            emit_runtime_download_progress(
                &app,
                "ffprobe",
                100,
                0,
                0,
                "FFmpeg local đã sẵn sàng (100%)".to_string(),
            );

            let _ = fs::remove_file(&ffmpeg_zip);
            let _ = fs::remove_file(&ffprobe_zip);
        }

        "windows" => {
            // Windows: Download single 7z archive from gyan.dev (~100MB)
            let archive_path = tools.join("ffmpeg-release-full.7z");

            download_to_path_with_progress(
                &app,
                "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-full.7z",
                &archive_path,
                "ffmpeg",
                0,
                88,
                "Đang tải FFmpeg local",
            )
            .await?;

            emit_runtime_download_progress(
                &app,
                "ffmpeg",
                90,
                0,
                0,
                "Đang giải nén FFmpeg (90%)".to_string(),
            );
            extract_named_file_from_7z(&archive_path, "ffmpeg.exe", &ffmpeg_path)?;

            emit_runtime_download_progress(
                &app,
                "ffmpeg",
                95,
                0,
                0,
                "Đang giải nén FFprobe (95%)".to_string(),
            );
            extract_named_file_from_7z(&archive_path, "ffprobe.exe", &ffprobe_path)?;

            emit_runtime_download_progress(
                &app,
                "ffmpeg",
                100,
                0,
                0,
                "FFmpeg local đã sẵn sàng (100%)".to_string(),
            );

            let _ = fs::remove_file(&archive_path);
        }

        other => {
            return Err(format!(
                "Hệ điều hành '{}' chưa được hỗ trợ tải FFmpeg tự động. Vui lòng cài FFmpeg thủ công.",
                other
            ));
        }
    }

    Ok(detect_runtime_capabilities(app))
}

#[tauri::command]
fn remove_local_ffmpeg_runtime(app: AppHandle) -> Result<RuntimeCapabilities, String> {
    let ffmpeg_path = managed_binary_path(&app, "ffmpeg")?;
    let ffprobe_path = managed_binary_path(&app, "ffprobe")?;

    if ffmpeg_path.exists() {
        fs::remove_file(&ffmpeg_path).map_err(|error| error.to_string())?;
    }

    if ffprobe_path.exists() {
        fs::remove_file(&ffprobe_path).map_err(|error| error.to_string())?;
    }

    Ok(detect_runtime_capabilities(app))
}

#[tauri::command]
fn list_whisper_models(app: AppHandle) -> Result<Vec<WhisperModelOption>, String> {
    let dir = models_dir(&app)?;
    Ok(whisper_model_catalog()
        .into_iter()
        .map(
            |(id, label, filename, size_bytes, description, _url)| WhisperModelOption {
                downloaded: dir.join(&filename).exists(),
                id,
                label,
                filename,
                size_bytes,
                description,
            },
        )
        .collect())
}

#[tauri::command]
async fn download_whisper_model(
    app: AppHandle,
    model_id: String,
) -> Result<WhisperModelOption, String> {
    let dir = models_dir(&app)?;
    let (_, label, filename, size_bytes, description, url) = whisper_model_catalog()
        .into_iter()
        .find(|(id, _, _, _, _, _)| id == &model_id)
        .ok_or_else(|| "Whisper model không hợp lệ.".to_string())?;

    let mut response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!(
            "Tải model thất bại với status {}",
            response.status()
        ));
    }

    let path = dir.join(&filename);
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|error| error.to_string())?;
    let total_bytes = response.content_length().unwrap_or(size_bytes);
    let mut downloaded_bytes = 0_u64;

    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        file.write_all(&chunk)
            .await
            .map_err(|error| error.to_string())?;
        downloaded_bytes += chunk.len() as u64;

        let progress = if total_bytes == 0 {
            0
        } else {
            ((downloaded_bytes as f64 / total_bytes as f64) * 100.0)
                .round()
                .clamp(0.0, 100.0) as u64
        };

        app.emit(
            "model-download-progress",
            ModelDownloadProgressEvent {
                model_id: model_id.clone(),
                progress,
                downloaded_bytes,
                total_bytes,
            },
        )
        .map_err(|error| error.to_string())?;
    }

    file.flush().await.map_err(|error| error.to_string())?;

    Ok(WhisperModelOption {
        id: model_id,
        label,
        filename,
        size_bytes,
        description,
        downloaded: true,
    })
}

#[tauri::command]
fn remove_whisper_model(app: AppHandle, model_id: String) -> Result<WhisperModelOption, String> {
    let dir = models_dir(&app)?;
    let (_, label, filename, size_bytes, description, _) = whisper_model_catalog()
        .into_iter()
        .find(|(id, _, _, _, _, _)| id == &model_id)
        .ok_or_else(|| "Whisper model không hợp lệ.".to_string())?;

    let path = dir.join(&filename);
    if path.exists() {
        fs::remove_file(&path).map_err(|error| error.to_string())?;
    }

    Ok(WhisperModelOption {
        id: model_id,
        label,
        filename,
        size_bytes,
        description,
        downloaded: false,
    })
}

#[tauri::command]
async fn inspect_media(app: AppHandle, file_path: String) -> Result<MediaInspection, String> {
    let ffprobe = resolve_ffprobe_path(&app)?;
    let output = app
        .shell()
        .command(ffprobe)
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            &file_path,
        ])
        .output()
        .await
        .map_err(|error| error.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let raw = String::from_utf8_lossy(&output.stdout).to_string();

    let value: Value = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    let streams = value["streams"]
        .as_array()
        .ok_or_else(|| "Không đọc được streams từ ffprobe.".to_string())?;
    let format = value["format"]
        .as_object()
        .ok_or_else(|| "Không đọc được format metadata.".to_string())?;

    let audio_tracks = streams
        .iter()
        .filter(|stream| stream["codec_type"] == "audio")
        .map(|stream| AudioTrackInfo {
            index: stream["index"].as_i64().unwrap_or_default(),
            codec: stream["codec_name"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            channels: stream["channels"].as_u64().unwrap_or_default(),
            language: stream["tags"]["language"]
                .as_str()
                .map(|value| value.to_string()),
            title: stream["tags"]["title"]
                .as_str()
                .unwrap_or("Audio track")
                .to_string(),
        })
        .collect::<Vec<_>>();

    let kind = if streams.iter().any(|stream| stream["codec_type"] == "video") {
        "video"
    } else {
        "audio"
    };

    let duration_seconds = format
        .get("duration")
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or_default();
    let file_size_bytes = format
        .get("size")
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_default();
    let sample_rate = streams
        .iter()
        .find(|stream| stream["codec_type"] == "audio")
        .and_then(|stream| stream["sample_rate"].as_str())
        .and_then(|value| value.parse::<u64>().ok());

    Ok(MediaInspection {
        path: file_path,
        kind: kind.to_string(),
        duration_seconds,
        file_size_bytes,
        audio_tracks,
        sample_rate,
    })
}

#[tauri::command]
async fn normalize_audio(
    app: AppHandle,
    state: State<'_, RunningProcesses>,
    job_id: String,
    file_path: String,
    track_index: i64,
    output_directory: String,
    duration_seconds: f64,
) -> Result<String, String> {
    let output_root = if output_directory.trim().is_empty() {
        app_cache_dir(&app)?.join("normalized")
    } else {
        PathBuf::from(output_directory)
    };
    fs::create_dir_all(&output_root).map_err(|error| error.to_string())?;

    let stem = Path::new(&file_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("audio");
    let output_path = output_root.join(format!("{}-normalized.mp3", stem));

    let (mut rx, child) = app
        .shell()
        .command(resolve_ffmpeg_path(&app)?)
        .args([
            "-y",
            "-i",
            &file_path,
            "-map",
            &format!("0:{track_index}"),
            "-vn",
            "-acodec",
            "libmp3lame",
            "-progress",
            "pipe:1",
            "-nostats",
            output_path.to_string_lossy().as_ref(),
        ])
        .spawn()
        .map_err(|error| error.to_string())?;

    register_running_process(&state, &job_id, child)?;
    emit_progress(
        &app,
        &job_id,
        "preprocess",
        10.0,
        "FFmpeg đang normalize audio.",
        None,
    );

    let mut stderr_lines: Vec<String> = Vec::new();

    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(line) => {
                let text = String::from_utf8_lossy(&line).trim().to_string();
                if let Some(current_ms) = parse_ffmpeg_progress_ms(&text) {
                    let total_ms = (duration_seconds.max(1.0) * 1000.0) as u64;
                    let ratio = (current_ms as f64 / total_ms as f64).clamp(0.0, 1.0);
                    let progress = 10.0 + ratio * 12.0;
                    let eta = if current_ms > 0 {
                        let remaining_ms = total_ms.saturating_sub(current_ms);
                        Some(remaining_ms / 1000)
                    } else {
                        None
                    };
                    emit_progress(
                        &app,
                        &job_id,
                        "preprocess",
                        progress,
                        "Đang normalize audio sang mp3.",
                        eta,
                    );
                }
            }
            CommandEvent::Stderr(line) => {
                let text = String::from_utf8_lossy(&line).trim().to_string();
                if !text.is_empty() {
                    stderr_lines.push(text.clone());
                    emit_progress(&app, &job_id, "preprocess", 10.0, text, None);
                }
            }
            CommandEvent::Error(message) => {
                let _ = take_running_process(&state, &job_id)?;
                return Err(message);
            }
            CommandEvent::Terminated(payload) => {
                let removed = take_running_process(&state, &job_id)?;
                if payload.code == Some(0) {
                    emit_progress(
                        &app,
                        &job_id,
                        "preprocess",
                        22.0,
                        "Audio đã sẵn sàng.",
                        Some(0),
                    );
                    if removed.is_some() {
                        break;
                    }
                } else if removed.is_none() {
                    return Err("Job đã bị hủy bởi người dùng.".into());
                } else {
                    let ffmpeg_log = stderr_lines.join("\n");
                    return Err(format!(
                        "FFmpeg normalize thất bại với code {:?}\n\n--- FFmpeg log ---\n{}",
                        payload.code, ffmpeg_log
                    ));
                }
            }
            _ => {}
        }
    }

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
async fn transcribe_with_whisper(
    app: AppHandle,
    state: State<'_, RunningProcesses>,
    job_id: String,
    payload: WhisperRequest,
) -> Result<Vec<SubtitleSegment>, String> {
    let model = whisper_model_catalog()
        .into_iter()
        .find(|(id, _, _, _, _, _)| id == &payload.model_id)
        .ok_or_else(|| "Model Whisper không hợp lệ.".to_string())?;
    let model_path = models_dir(&app)?.join(model.2);
    if !model_path.exists() {
        return Err(
            "Whisper model chưa được tải. Hãy tải model trong phần setup trước khi chạy.".into(),
        );
    }

    let normalized_compute_mode = payload.compute_mode.to_lowercase();
    let hardware_available = hardware_acceleration_available();
    let use_gpu = match normalized_compute_mode.as_str() {
        "cpu" => false,
        "hardware" => hardware_available,
        _ => hardware_available,
    };

    if normalized_compute_mode == "hardware" && !hardware_available {
        emit_progress(
            &app,
            &job_id,
            "transcribe",
            43.0,
            "Thiết bị không có hardware acceleration khả dụng. Whisper sẽ fallback sang CPU.",
            None,
        );
    } else if normalized_compute_mode == "auto" && !hardware_available {
        emit_progress(
            &app,
            &job_id,
            "transcribe",
            43.0,
            "Không phát hiện accelerator phù hợp. Whisper sẽ chạy ở CPU mode.",
            None,
        );
    } else if normalized_compute_mode == "cpu" {
        emit_progress(
            &app,
            &job_id,
            "transcribe",
            43.0,
            "Whisper đang chạy ở CPU-only mode.",
            None,
        );
    }

    emit_progress(
        &app,
        &job_id,
        "transcribe",
        45.0,
        "Whisper đang khởi tạo model.",
        None,
    );

    // Register a cancel token for this whisper job.
    let cancel_token = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut tokens = state
            .whisper_cancel_tokens
            .lock()
            .map_err(|_| "Không khóa được cancel tokens.".to_string())?;
        tokens.insert(job_id.clone(), Arc::clone(&cancel_token));
    }

    let app_clone = app.clone();
    let job_id_clone = job_id.clone();
    let model_path_str = model_path.to_string_lossy().to_string();
    let audio_path = payload.audio_path.clone();
    let source_language = payload.source_language.clone();

    // Run blocking whisper-rs inference on a dedicated thread.
    let result = tokio::task::spawn_blocking(move || {
        let mut ctx_params = WhisperContextParameters::default();
        ctx_params.use_gpu(use_gpu);
        let ctx = WhisperContext::new_with_params(&model_path_str, ctx_params)
            .map_err(|error| format!("Không tải được Whisper model: {error}"))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_temperature(0.1);
        params.set_temperature_inc(0.0);
        params.set_n_max_text_ctx(64);
        params.set_entropy_thold(3.0);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        if source_language != "auto" {
            params.set_language(Some(source_language.as_str()));
        }
        // Emit progress at 48% to show model is loaded.
        emit_progress(
            &app_clone,
            &job_id_clone,
            "transcribe",
            48.0,
            "Model đã tải xong. Đang transcribe...",
            None,
        );

        // Load and decode audio to 16kHz mono f32 samples using hound or direct WAV reading.
        // whisper-rs expects 16kHz mono f32 PCM.
        let samples = load_audio_as_f32(&app_clone, &audio_path)?;

        emit_progress(
            &app_clone,
            &job_id_clone,
            "transcribe",
            50.0,
            "Đang phân tích audio.",
            None,
        );

        let mut state_whisper = ctx
            .create_state()
            .map_err(|error| format!("Không khởi tạo được Whisper state: {error}"))?;

        state_whisper
            .full(params, &samples)
            .map_err(|error| format!("Whisper inference thất bại: {error}"))?;

        emit_progress(
            &app_clone,
            &job_id_clone,
            "transcribe",
            63.0,
            "Đang đọc kết quả transcription.",
            None,
        );

        let num_segments = state_whisper
            .full_n_segments()
            .map_err(|error| format!("Không đọc được số segments: {error}"))?;

        let mut subtitle_segments = Vec::new();
        for i in 0..num_segments {
            if cancel_token.load(std::sync::atomic::Ordering::Relaxed) {
                return Err("Job đã bị hủy bởi người dùng.".to_string());
            }

            let text = state_whisper
                .full_get_segment_text(i)
                .map_err(|error| format!("Không đọc được segment text {i}: {error}"))?;
            let start_ms = (state_whisper
                .full_get_segment_t0(i)
                .map_err(|error| format!("Không đọc được t0 segment {i}: {error}"))?
                * 10) as u64;
            let end_ms = (state_whisper
                .full_get_segment_t1(i)
                .map_err(|error| format!("Không đọc được t1 segment {i}: {error}"))?
                * 10) as u64;

            let text = text.trim().to_string();
            if text.is_empty() {
                continue;
            }

            subtitle_segments.push(SubtitleSegment {
                id: format!("segment-{}", i + 1),
                start_ms,
                end_ms,
                source_text: text.clone(),
                translated_text: text,
            });
        }

        Ok(subtitle_segments)
    })
    .await
    .map_err(|error| format!("Whisper thread panic: {error}"))?;

    // Clean up cancel token.
    {
        let mut tokens = state
            .whisper_cancel_tokens
            .lock()
            .map_err(|_| "Không khóa được cancel tokens.".to_string())?;
        tokens.remove(&job_id);
    }

    match result {
        Ok(segments) => {
            emit_progress(
                &app,
                &job_id,
                "transcribe",
                65.0,
                "Whisper transcription hoàn tất.",
                Some(0),
            );
            Ok(segments)
        }
        Err(error) => Err(error),
    }
}

/// Load audio file to 16kHz mono f32 PCM samples using ffmpeg (system or managed).
/// This is a blocking function, to be called inside spawn_blocking.
fn load_audio_as_f32(app: &AppHandle, audio_path: &str) -> Result<Vec<f32>, String> {
    // Use resolved ffmpeg path (managed → sidecar → system) instead of bare "ffmpeg".
    let ffmpeg = resolve_ffmpeg_path(app)?;
    let output = Command::new(&ffmpeg)
        .args([
            "-y",
            "-i",
            audio_path,
            "-f",
            "f32le",
            "-acodec",
            "pcm_f32le",
            "-ac",
            "1",
            "-ar",
            "16000",
            "pipe:1",
        ])
        .output()
        .map_err(|error| format!("Không thể chạy ffmpeg để decode audio: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg decode audio thất bại: {stderr}"));
    }

    let bytes = output.stdout;
    if bytes.len() % 4 != 0 {
        return Err("Dữ liệu audio PCM không hợp lệ (không chia hết cho 4).".into());
    }

    let samples: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    Ok(samples)
}

fn extract_text_part(response: &Value) -> Result<String, String> {
    response["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|value| value.to_string())
        .ok_or_else(|| "Gemini response không chứa text hợp lệ.".to_string())
}

async fn call_gemini(api_key: &str, model_id: &str, body: Value) -> Result<Value, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_id, api_key
    );
    let response = reqwest::Client::new()
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!(
            "Gemini request thất bại với status {}",
            response.status()
        ));
    }

    response
        .json::<Value>()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn translate_segments_with_gemini(
    payload: GeminiTranslateRequest,
) -> Result<Vec<SubtitleSegment>, String> {
    let simplified_segments = payload
        .segments
        .iter()
        .map(|segment| {
            json!({
                "id": segment.id,
                "source_text": segment.source_text
            })
        })
        .collect::<Vec<_>>();

    let prompt = format!(
        "User instruction: {}\n\n\
        Return ONLY a JSON object: {{\"segments\":[{{\"id\":\"...\",\"translated_text\":\"...\"}}]}}",
        payload.translation_instruction
    );

    let body = json!({
        "contents": [
            {
                "role": "user",
                "parts": [
                    { "text": prompt },
                    { "text": serde_json::to_string(&simplified_segments).map_err(|error| error.to_string())? }
                ]
            }
        ],
        "generationConfig": {
            "responseMimeType": "application/json"
        }
    });

    let response = call_gemini(&payload.api_key, &payload.model_id, body).await?;
    let text = extract_text_part(&response)?;
    let parsed: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let translated = parsed["segments"]
        .as_array()
        .ok_or_else(|| "Gemini không trả về segments hợp lệ.".to_string())?;

    let translated_map = translated
        .iter()
        .filter_map(|segment| {
            let id = segment["id"].as_str()?;
            let translated_text = segment["translated_text"].as_str()?;
            Some((id.to_string(), translated_text.to_string()))
        })
        .collect::<std::collections::HashMap<_, _>>();

    Ok(payload
        .segments
        .into_iter()
        .map(|mut segment| {
            segment.translated_text = translated_map
                .get(&segment.id)
                .cloned()
                .unwrap_or_else(|| segment.source_text.clone());
            segment
        })
        .collect())
}

fn chunk_audio(app: &AppHandle, audio_path: &str, duration: f64) -> Result<Vec<(PathBuf, u64, u64)>, String> {
    let ffmpeg = resolve_ffmpeg_path(app)?;
    let chunks_dir = app_cache_dir(app)?.join("gemini-direct");
    fs::create_dir_all(&chunks_dir).map_err(|error| error.to_string())?;

    let mut chunks = Vec::new();
    let chunk_seconds = 20u64;
    let total_seconds = duration.ceil() as u64;
    let mut offset = 0u64;

    while offset < total_seconds {
        let output_path =
            chunks_dir.join(format!("chunk-{}-{}.mp3", offset, offset + chunk_seconds));
        let status = Command::new(&ffmpeg)
            .arg("-y")
            .arg("-ss")
            .arg(offset.to_string())
            .arg("-t")
            .arg(chunk_seconds.to_string())
            .arg("-i")
            .arg(audio_path)
            .arg("-vn")
            .arg("-acodec")
            .arg("libmp3lame")
            .arg("-b:a")
            .arg("160k")
            .arg(&output_path)
            .status()
            .map_err(|error| error.to_string())?;

        if !status.success() {
            return Err("Không thể chunk audio cho Gemini direct mode.".into());
        }

        chunks.push((
            output_path,
            offset * 1000,
            (offset + chunk_seconds).min(total_seconds) * 1000,
        ));
        offset += chunk_seconds;
    }

    Ok(chunks)
}

#[tauri::command]
async fn transcribe_direct_with_gemini(
    app: AppHandle,
    payload: GeminiDirectRequest,
) -> Result<Vec<SubtitleSegment>, String> {
    let inspection = inspect_media(app.clone(), payload.audio_path.clone()).await?;
    let chunks = chunk_audio(&app, &payload.audio_path, inspection.duration_seconds)?;
    let mut segments = Vec::new();

    for (index, (chunk_path, start_ms, end_ms)) in chunks.into_iter().enumerate() {
        let bytes = fs::read(&chunk_path).map_err(|error| error.to_string())?;
        let base64_audio = base64::engine::general_purpose::STANDARD.encode(bytes);
        let prompt = format!(
            "Bạn đang xử lý một chunk audio cho subtitle. Nguồn ngôn ngữ: {}. Instruction dịch: {}. Hãy trả về JSON object dạng {{\"source_text\":\"...\",\"translated_text\":\"...\"}}. Không thêm markdown.",
            payload.source_language, payload.translation_instruction
        );

        let body = json!({
            "contents": [
                {
                    "role": "user",
                    "parts": [
                        { "text": prompt },
                        {
                            "inlineData": {
                                "mimeType": "audio/mpeg",
                                "data": base64_audio
                            }
                        }
                    ]
                }
            ],
            "generationConfig": {
                "responseMimeType": "application/json"
            }
        });

        let response = call_gemini(&payload.api_key, &payload.model_id, body).await?;
        let text = extract_text_part(&response)?;
        let parsed: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;

        segments.push(SubtitleSegment {
            id: format!("segment-{}", index + 1),
            start_ms,
            end_ms,
            source_text: parsed["source_text"]
                .as_str()
                .unwrap_or("Transcription unavailable")
                .to_string(),
            translated_text: parsed["translated_text"]
                .as_str()
                .unwrap_or("Translation unavailable")
                .to_string(),
        });
    }

    Ok(segments)
}

#[tauri::command]
async fn fetch_supported_gemini_models(api_key: String) -> Result<Vec<GeminiModelOption>, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!(
            "Không lấy được Gemini models với status {}",
            response.status()
        ));
    }

    let payload = response
        .json::<Value>()
        .await
        .map_err(|error| error.to_string())?;
    let models = payload["models"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|model| {
            let name = model["name"].as_str()?.split('/').last()?.to_string();
            if !GEMINI_MODELS_ALLOWLIST.contains(&name.as_str()) {
                return None;
            }

            Some(GeminiModelOption {
                label: name.clone(),
                id: name,
                description: model["displayName"]
                    .as_str()
                    .unwrap_or("Supported by EasyVietsub")
                    .to_string(),
                experimental: None,
            })
        })
        .collect::<Vec<_>>();

    Ok(models)
}

/// Escape a filesystem path for use inside an FFmpeg filter string.
///
/// FFmpeg filter syntax treats backslashes and colons as special characters.
/// On Windows, paths like `C:\Users\foo\sub.ass` must be converted carefully:
///   - backslashes → forward slashes
///   - colons are escaped with `\:`, **except** the drive-letter colon (e.g. `C:`)
fn escape_path_for_ffmpeg_filter(path: &std::path::Path) -> String {
    let s = path.to_string_lossy();

    #[cfg(windows)]
    {
        // Replace backslashes with forward slashes first.
        let forward = s.replace('\\', "/");
        // Preserve the drive-letter colon (e.g. "C:"), escape all others.
        if forward.len() >= 2
            && forward.as_bytes()[0].is_ascii_alphabetic()
            && forward.as_bytes()[1] == b':'
        {
            let (drive, rest) = forward.split_at(2);
            format!("{}{}", drive, rest.replace(':', "\\:"))
        } else {
            forward.replace(':', "\\:")
        }
    }

    #[cfg(not(windows))]
    {
        s.replace(':', "\\:")
    }
}

#[tauri::command]
async fn render_hard_subtitle(
    app: AppHandle,
    state: State<'_, RunningProcesses>,
    job_id: String,
    payload: RenderRequest,
    duration_seconds: f64,
) -> Result<String, String> {
    let segments = parse_srt(&payload.subtitle_content)?;
    let ass_path = app_cache_dir(&app)?.join("render-preview.ass");
    write_ass_file(&ass_path, &segments, &payload.style)?;
    
    // Get fonts directory and escape path for FFmpeg
    let fonts_directory = fonts_dir(&app)?;
    let fonts_dir_str = escape_path_for_ffmpeg_filter(&fonts_directory);
    
    // Escape ASS path for FFmpeg ass filter
    let ass_path_str = escape_path_for_ffmpeg_filter(&ass_path);
    
    // Build FFmpeg filter with fontsdir to use bundled fonts
    let filter_str = format!("ass='{}':fontsdir='{}'", ass_path_str, fonts_dir_str);


    let (mut rx, child) = app
        .shell()
        .command(resolve_ffmpeg_path(&app)?)
        .args([
            "-y",
            "-i",
            &payload.input_path,
            "-vf",
            &filter_str,
            "-c:a",
            "copy",
            "-progress",
            "pipe:1",
            "-nostats",
            &payload.output_path,
        ])
        .spawn()
        .map_err(|error| error.to_string())?;

        
    register_running_process(&state, &job_id, child)?;
    emit_progress(
        &app,
        &job_id,
        "render",
        72.0,
        "FFmpeg đang render hard subtitle.",
        None,
    );

    let mut stderr_lines: Vec<String> = Vec::new();

    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(line) => {
                let text = String::from_utf8_lossy(&line).trim().to_string();
                if let Some(current_ms) = parse_ffmpeg_progress_ms(&text) {
                    let total_ms = (duration_seconds.max(1.0) * 1000.0) as u64;
                    let ratio = (current_ms as f64 / total_ms as f64).clamp(0.0, 1.0);
                    let progress = 72.0 + ratio * 26.0;
                    let eta = if current_ms > 0 {
                        let remaining_ms = total_ms.saturating_sub(current_ms);
                        Some(remaining_ms / 1000)
                    } else {
                        None
                    };
                    emit_progress(
                        &app,
                        &job_id,
                        "render",
                        progress,
                        "Đang render video output.",
                        eta,
                    );
                }
            }
            CommandEvent::Stderr(line) => {
                let text = String::from_utf8_lossy(&line).trim().to_string();
                if !text.is_empty() {
                    stderr_lines.push(text.clone());
                    emit_progress(&app, &job_id, "render", 72.0, text, None);
                }
            }
            CommandEvent::Error(message) => {
                let _ = take_running_process(&state, &job_id)?;
                return Err(message);
            }
            CommandEvent::Terminated(payload) => {
                let removed = take_running_process(&state, &job_id)?;
                if payload.code == Some(0) {
                    emit_progress(&app, &job_id, "render", 100.0, "Render hoàn tất.", Some(0));
                    if removed.is_some() {
                        break;
                    }
                } else if removed.is_none() {
                    return Err("Job đã bị hủy bởi người dùng.".into());
                } else {
                    let ffmpeg_log = stderr_lines.join("\n");
                    return Err(format!(
                        "Render thất bại với code {:?}\n\n--- FFmpeg log ---\n{}",
                        payload.code, ffmpeg_log
                    ));
                }
            }
            _ => {}
        }
    }

    Ok(payload.output_path)
}

#[tauri::command]
fn load_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        return Ok(AppSettings {
            api_key: String::new(),
            output_directory: String::new(),
            last_opened_project_path: String::new(),
            saved_translation_instruction: String::new(),
        });
    }
    
    let raw = fs::read(path).map_err(|error| error.to_string())?;
    let settings: AppSettings = serde_json::from_slice(&raw).map_err(|error| error.to_string())?;
    Ok(settings)
}

#[tauri::command]
fn save_project_snapshot(path: String, snapshot: ProjectSnapshot) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(&snapshot).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| error.to_string())
}

#[tauri::command]
fn load_project_snapshot(path: String) -> Result<ProjectSnapshot, String> {
    let raw = fs::read(path).map_err(|error| error.to_string())?;
    let snapshot: ProjectSnapshot =
        serde_json::from_slice(&raw).map_err(|error| error.to_string())?;
    migrate_project_snapshot(snapshot)
}

#[tauri::command]
fn persist_app_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?;
    fs::write(settings_path(&app)?, bytes).map_err(|error| error.to_string())
}

#[tauri::command]
fn cancel_pipeline_process(
    app: AppHandle,
    state: State<'_, RunningProcesses>,
    job_id: String,
) -> Result<(), String> {
    // Cancel ffmpeg/sidecar processes.
    if let Some(child) = take_running_process(&state, &job_id)? {
        child.kill().map_err(|error| error.to_string())?;
    }

    // Cancel whisper-rs jobs via AtomicBool token.
    {
        let mut tokens = state
            .whisper_cancel_tokens
            .lock()
            .map_err(|_| "Không khóa được cancel tokens.".to_string())?;
        if let Some(token) = tokens.remove(&job_id) {
            token.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    emit_progress(
        &app,
        &job_id,
        "cancelled",
        0.0,
        "Đã gửi tín hiệu hủy.",
        None,
    );

    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(RunningProcesses::default())
        .invoke_handler(tauri::generate_handler![
            detect_runtime_capabilities,
            install_local_ffmpeg_runtime,
            remove_local_ffmpeg_runtime,
            list_whisper_models,
            download_whisper_model,
            remove_whisper_model,
            inspect_media,
            normalize_audio,
            transcribe_with_whisper,
            translate_segments_with_gemini,
            transcribe_direct_with_gemini,
            fetch_supported_gemini_models,
            render_hard_subtitle,
            export_ass_subtitle,
            load_app_settings,
            save_project_snapshot,
            load_project_snapshot,
            persist_app_settings,
            cancel_pipeline_process
        ])
        .run(tauri::generate_context!())
        .expect("error while running easyvietsub");
}

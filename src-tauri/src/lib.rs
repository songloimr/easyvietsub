mod error;
mod models;

use base64::Engine;
use error::{AppError, AppResult};
use models::*;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tauri::http::Response as HttpResponse;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_shell::{
    process::{CommandChild, CommandEvent},
    ShellExt,
};

use tokio::io::AsyncWriteExt;
use memmap2::Mmap;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

// ==================== CONSTANTS ====================

// Stream protocol configuration
/// Stream chunk size for video streaming (2MB).
const STREAM_CHUNK_SIZE: u64 = 2 * 1024 * 1024;

// ==================== HELPER FUNCTIONS ====================

fn app_data_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        ?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn app_cache_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_cache_dir()
        ?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn models_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app_data_dir(app)?.join("models");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn tools_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app_data_dir(app)?.join("tools");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn fonts_dir(app: &AppHandle) -> AppResult<PathBuf> {
    // Priority 1: Try bundled resources directory (production)
    let resources_dir = app
        .path()
        .resource_dir()
        ?;
    
    if resources_dir.join("Arial.ttf").exists() {
        return Ok(resources_dir);
    }
    
    // Priority 2: Development mode - walk up from exe dir to find fonts/ folder.
    // macOS dev:   target/debug/bundle/macos/App.app/Contents/MacOS/exe (many levels)
    // Windows dev: target/debug/exe.exe (fewer levels)
    // Walk up to a reasonable depth (max 8 ancestors) to stay robust across OSes.
    let exe_path = std::env::current_exe()?;
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
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn settings_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app_data_dir(app)?.join("settings.json"))
}

fn command_filename(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn managed_binary_path(app: &AppHandle, name: &str) -> AppResult<PathBuf> {
    Ok(tools_dir(app)?.join(command_filename(name)))
}

fn bundled_sidecar_path(name: &str) -> AppResult<PathBuf> {
    let exe_path = std::env::current_exe()?;
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
) -> AppResult<PathBuf> {
    let managed = managed_binary_path(app, managed_name)?;
    if managed.exists() {
        return Ok(managed);
    }

    let sidecar = bundled_sidecar_path(sidecar_name)?;
    if sidecar.exists() {
        return Ok(sidecar);
    }

    Ok(which::which(system_name)?)
}

fn resolve_ffmpeg_path(app: &AppHandle) -> AppResult<PathBuf> {
    resolve_runtime_binary_path(app, "ffmpeg", "ffmpeg-sidecar", "ffmpeg")
}

fn resolve_ffprobe_path(app: &AppHandle) -> AppResult<PathBuf> {
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
) -> AppResult<()> {
    let mut response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        ?;

    if !response.status().is_success() {
        return Err(AppError::network(format!("Tải runtime thất bại với status {}", response.status()), None, None));
    }

    let total_bytes = response.content_length().unwrap_or_default();
    let mut downloaded_bytes = 0_u64;
    let mut file = tokio::fs::File::create(destination)
        .await
        ?;

    emit_runtime_download_progress(
        app,
        stage,
        start_progress,
        downloaded_bytes,
        total_bytes,
        message.to_string(),
    );

    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk)
            .await
            ?;
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

    file.flush().await?;

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

fn extract_first_file_from_zip(zip_path: &Path, destination: &Path) -> AppResult<()> {
    let source = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(source)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if !entry.is_file() {
            continue;
        }

        let mut output = fs::File::create(destination)?;
        io::copy(&mut entry, &mut output)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = output
                .metadata()
                ?
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(destination, permissions)?;
        }

        return Ok(());
    }

    Err(AppError::file_system("Archive runtime không chứa binary hợp lệ.", None))
}

/// Extract a named file from a 7z archive to destination path.
/// Searches for file_name in any subfolder (e.g., "ffmpeg.exe" matches "*/bin/ffmpeg.exe").
fn extract_named_file_from_7z(
    archive_path: &Path,
    file_name: &str,
    destination: &Path,
) -> AppResult<()> {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    // Create temp directory to extract full archive
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let temp_dir = std::env::temp_dir().join(format!("easyvietsub_7z_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    // Extract full archive to temp dir
    sevenz_rust::decompress_file(archive_path, &temp_dir)
        .map_err(|e| AppError::file_system(format!("Không thể giải nén 7z archive: {}", e), None))?;

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

    walk_dir(&temp_dir, file_name, &mut found_path)?;

    if let Some(source) = found_path {
        // Copy to destination
        fs::copy(&source, destination)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(destination)
                ?
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(destination, permissions)?;
        }

        // Cleanup temp dir
        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    } else {
        let _ = fs::remove_dir_all(&temp_dir);
        Err(AppError::file_system(format!("Không tìm thấy file '{}' trong 7z archive.", file_name), None))
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
) -> AppResult<()> {
    let mut jobs = state
        .jobs
        .lock()
        ?;
    if let Some(existing) = jobs.remove(job_id) {
        let _ = existing.kill();
    }
    jobs.insert(job_id.to_string(), child);
    Ok(())
}

fn take_running_process(
    state: &State<RunningProcesses>,
    job_id: &str,
) -> AppResult<Option<CommandChild>> {
    let mut jobs = state
        .jobs
        .lock()
        ?;
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

fn srt_timestamp_to_ms(input: &str) -> AppResult<u64> {
    let parts: Vec<&str> = input.split([':', ',']).collect();
    if parts.len() != 4 {
        return Err(AppError::validation(format!("Timestamp không hợp lệ: {input}"), None));
    }

    let hours = parts[0].parse::<u64>()?;
    let minutes = parts[1].parse::<u64>()?;
    let seconds = parts[2].parse::<u64>()?;
    let millis = parts[3].parse::<u64>()?;
    Ok(hours * 3_600_000 + minutes * 60_000 + seconds * 1_000 + millis)
}

fn ms_to_ass_timestamp(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let centiseconds = (ms % 1_000) / 10;
    format!("{hours}:{minutes:02}:{seconds:02}.{centiseconds:02}")
}

fn ms_to_srt_timestamp(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

fn segments_to_srt_string(segments: &[SubtitleSegment]) -> String {
    segments
        .iter()
        .enumerate()
        .map(|(i, seg)| {
            format!(
                "{}\n{} --> {}\n{}",
                i + 1,
                ms_to_srt_timestamp(seg.start_ms),
                ms_to_srt_timestamp(seg.end_ms),
                seg.source_text.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn strip_markdown_code_block(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        // Skip the language tag line (e.g. "```srt\n")
        let rest = rest
            .find('\n')
            .map(|pos| &rest[pos + 1..])
            .unwrap_or(rest);
        rest.strip_suffix("```").unwrap_or(rest).trim()
    } else {
        trimmed
    }
}

fn parse_srt(input: &str) -> AppResult<Vec<SubtitleSegment>> {
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
            return Err(AppError::validation(format!("Block SRT không hợp lệ ở index {index}"), None));
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
) -> AppResult<()> {
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
            .replace('{', "\\{")
            .replace('}', "\\}");
        content.push_str(&format!(
            "Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
            ms_to_ass_timestamp(segment.start_ms),
            ms_to_ass_timestamp(segment.end_ms),
            text
        ));
    }

    Ok(fs::write(path, content)?)
}

#[tauri::command]
fn export_ass_subtitle(
    path: String,
    segments: Vec<SubtitleSegment>,
    style: RenderStyle,
) -> AppResult<()> {
    let path = Path::new(&path);
    write_ass_file(path, &segments, &style)
}

#[tauri::command]
fn detect_runtime_capabilities(app: AppHandle) -> RuntimeCapabilities {
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
    }
}

#[tauri::command]
async fn install_local_ffmpeg_runtime(app: AppHandle) -> AppResult<RuntimeCapabilities> {
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
            return Err(AppError::configuration(format!("Hệ điều hành '{}' chưa được hỗ trợ tải FFmpeg tự động. Vui lòng cài FFmpeg thủ công.", other), None));
        }
    }

    Ok(detect_runtime_capabilities(app))
}

#[tauri::command]
fn remove_local_ffmpeg_runtime(app: AppHandle) -> AppResult<RuntimeCapabilities> {
    let ffmpeg_path = managed_binary_path(&app, "ffmpeg")?;
    let ffprobe_path = managed_binary_path(&app, "ffprobe")?;

    if ffmpeg_path.exists() {
        fs::remove_file(&ffmpeg_path)?;
    }

    if ffprobe_path.exists() {
        fs::remove_file(&ffprobe_path)?;
    }

    Ok(detect_runtime_capabilities(app))
}

#[tauri::command]
fn list_whisper_models(app: AppHandle) -> AppResult<Vec<WhisperModelOption>> {
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
) -> AppResult<WhisperModelOption> {
    let dir = models_dir(&app)?;
    let (_, label, filename, size_bytes, description, url) = whisper_model_catalog()
        .into_iter()
        .find(|(id, _, _, _, _, _)| id == &model_id)
        .ok_or_else(|| "Whisper model không hợp lệ.".to_string())?;

    let mut response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        ?;

    if !response.status().is_success() {
        return Err(AppError::network(format!("Tải model thất bại với status {}", response.status()), None, None));
    }

    let path = dir.join(&filename);
    let mut file = tokio::fs::File::create(&path)
        .await
        ?;
    let total_bytes = response.content_length().unwrap_or(size_bytes);
    let mut downloaded_bytes = 0_u64;

    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk)
            .await
            ?;
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
        ?;
    }

    file.flush().await?;

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
fn remove_whisper_model(app: AppHandle, model_id: String) -> AppResult<WhisperModelOption> {
    let dir = models_dir(&app)?;
    let (_, label, filename, size_bytes, description, _) = whisper_model_catalog()
        .into_iter()
        .find(|(id, _, _, _, _, _)| id == &model_id)
        .ok_or_else(|| "Whisper model không hợp lệ.".to_string())?;

    let path = dir.join(&filename);
    if path.exists() {
        fs::remove_file(&path)?;
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
async fn inspect_media(app: AppHandle, file_path: String) -> AppResult<MediaInspection> {
    log::info!("[media] Inspecting media file: {}", file_path);
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
        ?;

    if !output.status.success() {
        return Err(AppError::media_processing(String::from_utf8_lossy(&output.stderr).trim().to_string(), None));
    }

    let raw = String::from_utf8_lossy(&output.stdout).to_string();

    let value: Value = serde_json::from_str(&raw)?;
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

    let inspection = MediaInspection {
        path: file_path.clone(),
        kind: kind.to_string(),
        duration_seconds,
        file_size_bytes,
        audio_tracks: audio_tracks.clone(),
        sample_rate,
    };
    
    log::info!(
        "[media] Inspected {}: {:.1}s, {} audio tracks",
        file_path,
        duration_seconds,
        audio_tracks.len()
    );
    
    Ok(inspection)
}

#[tauri::command]
async fn transcribe_with_whisper(
    app: AppHandle,
    state: State<'_, RunningProcesses>,
    job_id: String,
    payload: WhisperRequest,
) -> AppResult<Vec<SubtitleSegment>> {
    let start = std::time::Instant::now();
    
    log::info!(
        "[whisper] Starting transcription: job_id={}, model={}, cpu_only={}",
        job_id,
        payload.model_id,
        payload.cpu_only
    );
    
    let model = whisper_model_catalog()
        .into_iter()
        .find(|(id, _, _, _, _, _)| id == &payload.model_id)
        .ok_or_else(|| AppError::model("Model Whisper không hợp lệ.", None))?;
    let model_path = models_dir(&app)?.join(model.2);
    if !model_path.exists() {
        return Err(AppError::model("Whisper model chưa được tải. Hãy tải model trong phần setup trước khi chạy.", None));
    }

    let use_gpu = !payload.cpu_only;

    if payload.cpu_only {
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
            ?;
        tokens.insert(job_id.clone(), Arc::clone(&cancel_token));
    }

    let app_clone = app.clone();
    let job_id_clone = job_id.clone();
    let model_path_str = model_path.to_string_lossy().to_string();
    let audio_path = payload.audio_path.clone();
    let track_index = payload.track_index;
    let source_language = payload.source_language.clone();

    // Retrieve or create a cached WhisperContext to avoid reloading the model every time.
    let whisper_ctx: Arc<WhisperContext> = {
        let mut cache = state
            .whisper_ctx_cache
            .lock()
            ?;

        if let Some((ref cached_path, cached_gpu, ref ctx)) = *cache {
            if cached_path == &model_path_str && cached_gpu == use_gpu {
                Arc::clone(ctx)
            } else {
                // Model or GPU setting changed — drop old, load new.
                let mut ctx_params = WhisperContextParameters::default();
                ctx_params.use_gpu(use_gpu);
                let new_ctx = Arc::new(
                    WhisperContext::new_with_params(&model_path_str, ctx_params)
                        .map_err(|e| AppError::model(format!("Không tải được Whisper model: {e}"), None))?,
                );
                *cache = Some((model_path_str.clone(), use_gpu, Arc::clone(&new_ctx)));
                new_ctx
            }
        } else {
            let mut ctx_params = WhisperContextParameters::default();
            ctx_params.use_gpu(use_gpu);
            let new_ctx = Arc::new(
                WhisperContext::new_with_params(&model_path_str, ctx_params)
                    .map_err(|e| AppError::model(format!("Không tải được Whisper model: {e}"), None))?,
            );
            *cache = Some((model_path_str.clone(), use_gpu, Arc::clone(&new_ctx)));
            new_ctx
        }
    };

    // Run blocking whisper-rs inference on a dedicated thread.
    let result = tokio::task::spawn_blocking(move || {

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

        // Decode audio directly from source to 16kHz mono f32 PCM (cached per job).
        let mapped_audio = load_audio_as_f32(&app_clone, &audio_path, track_index, &job_id_clone)?;

        emit_progress(
            &app_clone,
            &job_id_clone,
            "transcribe",
            50.0,
            "Đang phân tích audio.",
            None,
        );

        // Set up progress callback to emit real-time progress during inference.
        {
            let cb_app = app_clone.clone();
            let cb_job_id = job_id_clone.clone();
            params.set_progress_callback_safe(move |progress| {
                // Map whisper 0-100% to pipeline 50-63%.
                let pipeline_pct = 50.0 + (progress as f64 * 0.13);
                emit_progress(
                    &cb_app,
                    &cb_job_id,
                    "transcribe",
                    pipeline_pct,
                    &format!("Đang nhận dạng giọng nói... {}%", progress),
                    None,
                );
            });
        }

        // Set up abort callback to allow cancellation during inference.
        {
            let cb_cancel_token = Arc::clone(&cancel_token);
            params.set_abort_callback_safe(move || {
                cb_cancel_token.load(std::sync::atomic::Ordering::Relaxed)
            });
        }

        let mut state_whisper = whisper_ctx
            .create_state()
            .map_err(|error| AppError::model(format!("Không khởi tạo được Whisper state: {error}"), None))?;

        state_whisper
            .full(params, mapped_audio.samples())
            .map_err(|error| AppError::model(format!("Whisper inference thất bại: {error}"), None))?;

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
            .map_err(|error| AppError::model(format!("Không đọc được số segments: {error}"), None))?;

        let mut subtitle_segments = Vec::new();
        for i in 0..num_segments {
            if cancel_token.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(AppError::cancelled("Job đã bị hủy bởi người dùng.", None));
            }

            let text = state_whisper
                .full_get_segment_text(i)
                .map_err(|error| AppError::model(format!("Không đọc được segment text {i}: {error}"), None))?;
            let start_ms = (state_whisper
                .full_get_segment_t0(i)
                .map_err(|error| AppError::model(format!("Không đọc được segment timestamp {i}: {error}"), None))?
                * 10) as u64;
            let end_ms = (state_whisper
                .full_get_segment_t1(i)
                .map_err(|error| AppError::model(format!("Không đọc được segment timestamp {i}: {error}"), None))?
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
    .map_err(|error| AppError::model(format!("Whisper thread panic: {error}"), None))?;

    // Clean up cancel token.
    {
        let mut tokens = state
            .whisper_cancel_tokens
            .lock()
            ?;
        tokens.remove(&job_id);
    }

    // PCM cache file is kept for potential retries — cleanup via cleanup_job_cache.

    match result {
        Ok(segments) => {
            let elapsed = start.elapsed();
            log::info!(
                "[whisper] Transcription complete: {} segments in {:.1}s",
                segments.len(),
                elapsed.as_secs_f64()
            );
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
        Err(error) => {
            log::error!("[whisper] Transcription failed: {}", error);
            Err(error)
        }
    }
}

/// Audio samples backed by a memory-mapped temp file.
/// Keeps the mmap alive so the `&[f32]` slice remains valid.
struct MappedAudio {
    #[allow(dead_code)]
    mmap: Mmap,
    sample_count: usize,
}

impl MappedAudio {
    /// Get the audio samples as a `&[f32]` slice (zero-copy from mmap).
    fn samples(&self) -> &[f32] {
        // SAFETY: mmap is aligned to page boundary (always >= 4-byte aligned),
        // sample_count was validated so sample_count * 4 <= mmap.len(),
        // and the file was written as f32le by ffmpeg.
        unsafe { std::slice::from_raw_parts(self.mmap.as_ptr() as *const f32, self.sample_count) }
    }
}

/// Load audio from a source media file to 16kHz mono f32 PCM samples using ffmpeg.
/// Decodes directly from the source (mp4/mkv/mp3/etc.) — no intermediate WAV needed.
/// Output is cached per job_id so retries skip the decode step.
/// This is a blocking function, to be called inside spawn_blocking.
fn load_audio_as_f32(
    app: &AppHandle,
    source_path: &str,
    track_index: u32,
    job_id: &str,
) -> AppResult<MappedAudio> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| AppError::file_system(format!("Không lấy được cache dir: {error}"), None))?;
    let pcm_dir = cache_dir.join("whisper-pcm");
    fs::create_dir_all(&pcm_dir)
        .map_err(|error| AppError::file_system(format!("Không tạo được whisper-pcm dir: {error}"), None))?;

    let pcm_path = pcm_dir.join(format!("{job_id}.raw"));

    // Reuse cached PCM file if it exists and is valid.
    if pcm_path.exists() {
        if let Ok(meta) = fs::metadata(&pcm_path) {
            let len = meta.len() as usize;
            if len > 0 && len % 4 == 0 {
                let file = fs::File::open(&pcm_path)
                    .map_err(|e| AppError::file_system(format!("Không mở được cached PCM file: {e}"), None))?;
                let mmap = unsafe { Mmap::map(&file) }
                    .map_err(|e| AppError::file_system(format!("Không mmap được cached PCM file: {e}"), None))?;
                return Ok(MappedAudio {
                    mmap,
                    sample_count: len / 4,
                });
            }
        }
        // Invalid cache file — remove and re-decode.
        let _ = fs::remove_file(&pcm_path);
    }

    let ffmpeg = resolve_ffmpeg_path(app)?;
    let track_map = format!("0:{}", track_index);

    let output = Command::new(&ffmpeg)
        .args([
            "-y",
            "-i",
            source_path,
            "-map",
            &track_map,
            "-vn",
            "-f",
            "f32le",
            "-acodec",
            "pcm_f32le",
            "-ac",
            "1",
            "-ar",
            "16000",
        ])
        .arg(pcm_path.as_os_str())
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .output()
        .map_err(|error| AppError::media_processing(format!("Không thể chạy ffmpeg để decode audio: {error}"), None))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = fs::remove_file(&pcm_path);
        return Err(AppError::media_processing(format!("ffmpeg decode audio thất bại: {stderr}"), None));
    }

    let file = fs::File::open(&pcm_path)
        .map_err(|error| AppError::file_system(format!("Không mở được PCM file: {error}"), None))?;
    let file_len = file
        .metadata()
        .map_err(|error| AppError::file_system(format!("Không đọc được metadata PCM: {error}"), None))?
        .len() as usize;

    if file_len == 0 {
        let _ = fs::remove_file(&pcm_path);
        return Err(AppError::media_processing("ffmpeg decode audio ra file rỗng.", None));
    }
    if file_len % 4 != 0 {
        let _ = fs::remove_file(&pcm_path);
        return Err(AppError::validation("Dữ liệu audio PCM không hợp lệ (không chia hết cho 4).", None));
    }

    // SAFETY: The file was just written by ffmpeg and is not modified concurrently.
    let mmap = unsafe { Mmap::map(&file) }
        .map_err(|error| AppError::file_system(format!("Không mmap được PCM file: {error}"), None))?;

    let sample_count = file_len / 4;
    Ok(MappedAudio { mmap, sample_count })
}

fn extract_text_part(response: &Value) -> AppResult<String> {
    response["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|value| value.to_string())
        .ok_or_else(|| AppError::model("Gemini response không chứa text hợp lệ.", None))
}

/// Extract token usage metadata from Gemini API response.
fn extract_token_usage(response: &Value) -> TokenUsage {
    let meta = &response["usageMetadata"];
    TokenUsage {
        prompt_tokens: meta["promptTokenCount"].as_u64().unwrap_or(0),
        completion_tokens: meta["candidatesTokenCount"].as_u64().unwrap_or(0),
        total_tokens: meta["totalTokenCount"].as_u64().unwrap_or(0),
    }
}

async fn call_gemini(
    client: &reqwest::Client,
    api_key: &str,
    model_id: &str,
    body: Value,
) -> AppResult<Value> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model_id
    );
    
    log::debug!("[gemini] API call: model={}", model_id);
    let start = std::time::Instant::now();
    
    let mut last_error = None;
    
    for attempt in 0..=MAX_GEMINI_RETRIES {
        // Apply exponential backoff with jitter (except for first attempt)
        if attempt > 0 {
            let backoff = GEMINI_INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1);
            // Add jitter: ±25%
            let jitter_factor = 0.25 * (rand::random::<f64>() * 2.0 - 1.0);
            let jittered_backoff = (backoff as f64 * (1.0 + jitter_factor)) as u64;
            
            log::warn!(
                "[gemini] Retry {}/{} after {}ms...",
                attempt, MAX_GEMINI_RETRIES, jittered_backoff
            );
            tokio::time::sleep(tokio::time::Duration::from_millis(jittered_backoff)).await;
        }
        
        // Make the request
        let url_with_key = format!("{}?key={}", url, api_key);
        let response = match client
            .post(&url_with_key)
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) if e.is_timeout() || e.is_connect() => {
                let err_msg = format!("Network error: {}", e);
                log::warn!("[gemini] Attempt {}: {}", attempt + 1, err_msg);
                last_error = Some(AppError::network(err_msg, None, None));
                continue;
            }
            Err(e) => {
                return Err(AppError::network(format!("Request failed: {}", e), None, None));
            }
        };
        
        let status = response.status();
        
        // Success case
        if status.is_success() {
            let elapsed = start.elapsed();
            log::debug!("[gemini] API call success: {}ms", elapsed.as_millis());
            return response
                .json::<Value>()
                .await
                .map_err(|e| AppError::network(format!("Failed to parse response: {}", e), None, None));
        }
        
        // Retryable status codes
        match status.as_u16() {
            429 | 500 | 502 | 503 | 529 => {
                let err_msg = format!("Gemini API returned status {}", status);
                log::warn!("[gemini] Attempt {}: {}", attempt + 1, err_msg);
                last_error = Some(AppError::network(err_msg, None, None));
                continue;
            }
            _ => {
                // Non-retryable error
                return Err(AppError::network(
                    format!("Gemini request failed with status {}", status),
                    None,
                    None,
                ));
            }
        }
    }
    
    // All retries exhausted
    Err(last_error.unwrap_or_else(|| {
        AppError::network(
            format!("Gemini request failed after {} retries", MAX_GEMINI_RETRIES),
            None,
            None,
        )
    }))
}

/// Build a context-aware SRT string for a translation chunk.
/// `context_pairs` contains previously-translated segments (source + translated)
/// shown to the model for continuity but NOT included in its output.
fn build_chunk_prompt(
    instruction: &str,
    chunk: &[SubtitleSegment],
    context_pairs: &[(String, String)], // (source_text, translated_text)
    chunk_idx: usize,
    total_chunks: usize,
) -> String {
    let srt_body = segments_to_srt_string(chunk);
    let segment_count = chunk.len();

    let context_section = if context_pairs.is_empty() {
        String::new()
    } else {
        let lines: Vec<String> = context_pairs
            .iter()
            .map(|(src, tl)| format!("[SRC] {} → [VI] {}", src, tl))
            .collect();
        format!(
            "\n=== PREVIOUSLY TRANSLATED (for context only, do NOT include in output) ===\n{}\n\n",
            lines.join("\n")
        )
    };

    let part_label = if total_chunks > 1 {
        format!(" (Part {}/{})", chunk_idx + 1, total_chunks)
    } else {
        String::new()
    };

    format!(
        "User instruction: {instruction}\n\n\
        Translate the following SRT subtitle file{part_label}. \
        Return ONLY the translated SRT file with the exact same format and same number of segments ({segment_count}). \
        Keep all segment numbers and timestamps exactly as they are. \
        Only translate the text content. \
        Do NOT add any explanation, commentary, or markdown formatting.\
        {context_section}\n\
        === TRANSLATE THE FOLLOWING ===\n\
        {srt_body}"
    )
}

/// Translate a single chunk of segments via Gemini.
/// Returns Ok((translated_segments, token_usage)) on success, Err(reason) on failure.
async fn translate_single_chunk(
    client: &reqwest::Client,
    api_key: &str,
    model_id: &str,
    instruction: &str,
    chunk: &[SubtitleSegment],
    context_pairs: &[(String, String)],
    chunk_idx: usize,
    total_chunks: usize,
) -> AppResult<(Vec<SubtitleSegment>, TokenUsage)> {
    let prompt = build_chunk_prompt(instruction, chunk, context_pairs, chunk_idx, total_chunks);
    let expected_count = chunk.len();

    let body = json!({
        "contents": [
            {
                "role": "user",
                "parts": [{ "text": prompt }]
            }
        ],
        "generationConfig": {
            "maxOutputTokens": 1048576
        }
    });

    let response = call_gemini(client, api_key, model_id, body).await?;

    // Extract token usage
    let token_usage = extract_token_usage(&response);

    // Check finishReason
    let finish_reason = response["candidates"][0]["finishReason"]
        .as_str()
        .unwrap_or("UNKNOWN");

    if finish_reason == "MAX_TOKENS" {
        return Err(AppError::model("MAX_TOKENS", None));
    }
    if finish_reason != "STOP" {
        return Err(AppError::model(format!("finishReason: {}", finish_reason), None));
    }

    // Extract and parse
    let raw_text = extract_text_part(&response)?;
    let srt_text = strip_markdown_code_block(&raw_text);
    let translated = parse_srt(srt_text)?;

    // Reconcile segment count mismatch by mapping to original timing
    let final_segments = if translated.len() == expected_count {
        // Perfect match - use Gemini's output as-is
        translated
    } else {
        log::warn!(
            "[translate] Chunk {}/{}: segment count mismatch (expected {}, got {}). Reconciling with original timing...",
            chunk_idx + 1,
            total_chunks,
            expected_count,
            translated.len()
        );
        
        let mut reconciled = Vec::with_capacity(expected_count);
        for (i, original) in chunk.iter().enumerate() {
            // Use Gemini's translation if available, otherwise copy source text
            let translated_text = translated.get(i)
                .map(|t| t.source_text.clone()) // parse_srt puts text into source_text field
                .unwrap_or_else(|| {
                    log::warn!(
                        "[translate] Chunk {}/{}: Missing translation for segment {} ({}..{}ms), using source text as fallback",
                        chunk_idx + 1,
                        total_chunks,
                        i + 1,
                        original.start_ms,
                        original.end_ms
                    );
                    original.source_text.clone()
                });
            
            reconciled.push(SubtitleSegment {
                id: original.id.clone(),
                start_ms: original.start_ms,
                end_ms: original.end_ms,
                source_text: translated_text,
                translated_text: String::new(), // Will be used downstream
            });
        }
        
        if translated.len() > expected_count {
            log::warn!(
                "[translate] Chunk {}/{}: Gemini returned {} extra segments, ignoring them.",
                chunk_idx + 1,
                total_chunks,
                translated.len() - expected_count
            );
        }
        
        reconciled
    };

    Ok((final_segments, token_usage))
}

/// Translate a chunk with adaptive retry: on failure, split in half and retry each sub-chunk.
/// `depth` limits recursive splitting (0 = first attempt, MAX_CHUNK_SPLIT_DEPTH = stop splitting).
/// Returns (translated_segments, accumulated_token_usage).
fn translate_chunk_with_retry<'a>(
    client: &'a reqwest::Client,
    api_key: &'a str,
    model_id: &'a str,
    instruction: &'a str,
    chunk: &'a [SubtitleSegment],
    context_pairs: &'a [(String, String)],
    chunk_idx: usize,
    total_chunks: usize,
    depth: usize,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = AppResult<(Vec<SubtitleSegment>, TokenUsage)>> + Send + 'a>> {
    Box::pin(async move {
    // Try translating the whole chunk first
    match translate_single_chunk(
        client,
        api_key,
        model_id,
        instruction,
        chunk,
        context_pairs,
        chunk_idx,
        total_chunks,
    )
    .await
    {
        Ok((segments, token_usage)) => return Ok((segments, token_usage)),
        Err(error) => {
            log::warn!(
                "[translate] Chunk {}/{} failed (depth {}): {}",
                chunk_idx + 1,
                total_chunks,
                depth,
                error
            );

            // If we haven't reached max split depth and chunk is splittable, split in half
            if depth < MAX_CHUNK_SPLIT_DEPTH && chunk.len() > 1 {
                let mid = chunk.len() / 2;
                let (first_half, second_half) = chunk.split_at(mid);

                log::info!(
                    "[translate] Splitting chunk into {} + {} segments (depth {})",
                    first_half.len(),
                    second_half.len(),
                    depth + 1
                );

                // Translate first half
                let (first_result, first_tokens) = translate_chunk_with_retry(
                    client,
                    api_key,
                    model_id,
                    instruction,
                    first_half,
                    context_pairs,
                    chunk_idx,
                    total_chunks,
                    depth + 1,
                )
                .await?;

                // Build context from the tail of first_result for the second half
                let tail_count = first_result.len().min(TRANSLATION_CONTEXT_SIZE);
                let mut second_context = context_pairs.to_vec();
                for seg in first_result.iter().skip(first_result.len() - tail_count) {
                    second_context.push((
                        seg.source_text.clone(),
                        seg.translated_text.clone(),
                    ));
                }

                // Translate second half
                let (second_result, second_tokens) = translate_chunk_with_retry(
                    client,
                    api_key,
                    model_id,
                    instruction,
                    second_half,
                    &second_context,
                    chunk_idx,
                    total_chunks,
                    depth + 1,
                )
                .await?;

                // Combine results and accumulate tokens
                let mut combined = first_result;
                combined.extend(second_result);
                
                let mut total_tokens = first_tokens;
                total_tokens.accumulate(&second_tokens);
                
                return Ok((combined, total_tokens));
            }

            // Can't split further – propagate error
            return Err(error);
        }
    }
    })
}

/// Calculate adaptive chunk size based on segment characteristics.
/// - For small sets (<100 segments): use smaller chunks for faster feedback
/// - For medium sets (100-500): use standard chunk size
/// - For large sets (>500): use larger chunks for efficiency
/// - Adjust based on average text length per segment
fn calculate_adaptive_chunk_size(segments: &[SubtitleSegment]) -> usize {
    let total_segments = segments.len();

    if total_segments == 0 {
        return MIN_CHUNK_SIZE;
    }
    
    // Calculate average text length (safe: total_segments > 0 guaranteed)
    let avg_text_len: usize = segments.iter()
        .map(|s| s.source_text.len())
        .sum::<usize>() / total_segments;
    
    // Base chunk size on total segment count
    let base_chunk_size = if total_segments < 100 {
        MIN_CHUNK_SIZE
    } else if total_segments < 500 {
        TRANSLATION_CHUNK_SIZE
    } else {
        MAX_CHUNK_SIZE
    };
    
    // Adjust based on text complexity
    // Longer text = smaller chunks to avoid token limits
    let adjusted_size = if avg_text_len > 200 {
        // Long text (>200 chars avg): reduce chunk size by 30%
        (base_chunk_size as f64 * 0.7) as usize
    } else if avg_text_len > 100 {
        // Medium text (100-200 chars): reduce by 15%
        (base_chunk_size as f64 * 0.85) as usize
    } else {
        // Short text (<100 chars): keep base size
        base_chunk_size
    };
    
    // Clamp to min/max bounds
    let final_size = adjusted_size.clamp(MIN_CHUNK_SIZE, MAX_CHUNK_SIZE);
    
    log::debug!(
        "[translate] Adaptive chunk size: {} segments, avg_len={}, chunk_size={}",
        total_segments,
        avg_text_len,
        final_size
    );
    
    final_size
}

#[tauri::command]
async fn translate_segments_with_gemini(
    app: AppHandle,
    state: State<'_, RunningProcesses>,
    payload: GeminiTranslateRequest,
) -> AppResult<TranslationResult> {
    let start = std::time::Instant::now();
    let all_segments = payload.segments;
    let total_count = all_segments.len();
    
    log::info!(
        "[translate] Starting translation: job_id={}, model={}, {} segments",
        payload.job_id,
        payload.model_id,
        total_count
    );

    if total_count == 0 {
        return Ok(TranslationResult {
            segments: vec![],
            token_usage: TokenUsage::default(),
        });
    }

    // Calculate adaptive chunk size based on total segments and average text length
    let chunk_size = calculate_adaptive_chunk_size(&all_segments);
    
    // Split segments into chunks
    let chunks: Vec<&[SubtitleSegment]> = all_segments.chunks(chunk_size).collect();
    let total_chunks = chunks.len();

    log::info!(
        "[translate] {} segments → {} chunk(s) of ~{} each (adaptive)",
        total_count, total_chunks, chunk_size
    );

    emit_progress(
        &app,
        &payload.job_id,
        "translate",
        70.0,
        format!("Bắt đầu dịch {} phần...", total_chunks),
        None,
    );

    // Register a cancel token for this Gemini job.
    let cancel_token = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut tokens = state
            .gemini_cancel_tokens
            .lock()
            ?;
        tokens.insert(payload.job_id.clone(), Arc::clone(&cancel_token));
    }

    let mut translated_all: Vec<SubtitleSegment> = Vec::with_capacity(total_count);
    let mut total_usage = TokenUsage::default();

    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        // Check for cancellation before processing each chunk.
        if cancel_token.load(std::sync::atomic::Ordering::Relaxed) {
            // Clean up cancel token.
            {
                let mut tokens = state
                    .gemini_cancel_tokens
                    .lock()
                    ?;
                tokens.remove(&payload.job_id);
            }
            return Err(AppError::cancelled("Job đã bị hủy bởi người dùng.", None));
        }

        // Build context from the tail of previously translated segments
        let context_pairs: Vec<(String, String)> = if translated_all.is_empty() {
            vec![]
        } else {
            let tail_count = translated_all.len().min(TRANSLATION_CONTEXT_SIZE);
            translated_all[translated_all.len() - tail_count..]
                .iter()
                .map(|seg| (seg.source_text.clone(), seg.translated_text.clone()))
                .collect()
        };

        // Progress: 70% → 95% spread across chunks
        let progress = 70.0 + (chunk_idx as f64 / total_chunks as f64) * 25.0;
        emit_progress(
            &app,
            &payload.job_id,
            "translate",
            progress,
            format!(
                "Đang dịch phần {}/{}... ({} segments)",
                chunk_idx + 1,
                total_chunks,
                chunk.len()
            ),
            None,
        );

        match translate_chunk_with_retry(
            &state.http_client,
            &payload.api_key,
            &payload.model_id,
            &payload.translation_instruction,
            chunk,
            &context_pairs,
            chunk_idx,
            total_chunks,
            0, // initial depth
        )
        .await
        {
            Ok((translated_chunk, chunk_usage)) => {
                // Accumulate token usage
                total_usage.accumulate(&chunk_usage);
                
                // Push Gemini's translated segments directly without mapping back to original IDs
                // (SRT gốc và SRT dịch là hai phần riêng biệt, không cần đối chiếu)
                for translated in translated_chunk.into_iter() {
                    translated_all.push(SubtitleSegment {
                        id: translated.id.clone(),
                        start_ms: translated.start_ms,
                        end_ms: translated.end_ms,
                        source_text: String::new(), // SRT dịch không cần source_text
                        translated_text: translated.source_text, // parse_srt puts text into source_text
                    });
                }
            }
            Err(error) => {
                log::error!(
                    "[translate] Chunk {}/{} failed after retries: {}. Fallback cho chunk này.",
                    chunk_idx + 1,
                    total_chunks,
                    error
                );
                // Fallback: copy sourceText for this chunk's segments only
                let original_offset = chunk_idx * chunk_size;
                for (i, _seg) in chunk.iter().enumerate() {
                    let original = &all_segments[original_offset + i];
                    translated_all.push(SubtitleSegment {
                        id: original.id.clone(),
                        start_ms: original.start_ms,
                        end_ms: original.end_ms,
                        source_text: original.source_text.clone(),
                        translated_text: original.source_text.clone(),
                    });
                }
            }
        }
    }

    // Final progress
    emit_progress(
        &app,
        &payload.job_id,
        "translate",
        95.0,
        "Hoàn tất dịch thuật.",
        None,
    );

    // Clean up cancel token.
    {
        let mut tokens = state
            .gemini_cancel_tokens
            .lock()
            ?;
        tokens.remove(&payload.job_id);
    }
    
    let elapsed = start.elapsed();
    log::info!(
        "[translate] Translation complete: {} segments, {} total tokens in {:.1}s",
        translated_all.len(),
        total_usage.total_tokens,
        elapsed.as_secs_f64()
    );

    Ok(TranslationResult {
        segments: translated_all,
        token_usage: total_usage,
    })
}

fn chunk_audio(
    app: &AppHandle,
    source_path: &str,
    track_index: u32,
    job_id: &str,
    duration: f64,
) -> AppResult<Vec<(PathBuf, u64, u64)>> {
    let cache_dir = app_cache_dir(app)?;
    let chunks_dir = cache_dir.join("gemini-chunks").join(job_id);

    // Reuse cached chunks if they exist and look valid.
    if chunks_dir.exists() {
        if let Ok(entries) = fs::read_dir(&chunks_dir) {
            let existing: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map_or(false, |ext| ext == "mp3")
                })
                .collect();
            if !existing.is_empty() {
                let chunk_seconds = GEMINI_AUDIO_CHUNK_DURATION_SECS as u64;
                let total_seconds = duration.ceil() as u64;
                let expected_count = (total_seconds + chunk_seconds - 1) / chunk_seconds;
                if existing.len() as u64 >= expected_count {
                    // Rebuild chunks list from cached files.
                    let mut chunks = Vec::new();
                    let mut offset = 0u64;
                    while offset < total_seconds {
                        let chunk_path = chunks_dir
                            .join(format!("chunk-{}-{}.mp3", offset, offset + chunk_seconds));
                        if chunk_path.exists() {
                            chunks.push((
                                chunk_path,
                                offset * 1000,
                                (offset + chunk_seconds).min(total_seconds) * 1000,
                            ));
                        }
                        offset += chunk_seconds;
                    }
                    if !chunks.is_empty() {
                        log::debug!("[chunk_audio] Cache hit: {} chunks for job {}", chunks.len(), job_id);
                        return Ok(chunks);
                    }
                }
            }
        }
        // Invalid cache — clean and re-chunk.
        let _ = fs::remove_dir_all(&chunks_dir);
    }

    fs::create_dir_all(&chunks_dir)?;

    let ffmpeg = resolve_ffmpeg_path(app)?;
    let track_map = format!("0:{}", track_index);
    let mut chunks = Vec::new();
    let chunk_seconds = GEMINI_AUDIO_CHUNK_DURATION_SECS as u64;
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
            .arg(source_path)
            .arg("-map")
            .arg(&track_map)
            .arg("-vn")
            .arg("-acodec")
            .arg("libmp3lame")
            .arg("-b:a")
            .arg("160k")
            .arg(&output_path)
            .status()
            ?;

        if !status.success() {
            return Err(AppError::media_processing("Không thể chunk audio cho Gemini direct mode.", None));
        }

        chunks.push((
            output_path,
            offset * 1000,
            (offset + chunk_seconds).min(total_seconds) * 1000,
        ));
        offset += chunk_seconds;
    }
    
    log::debug!("[chunk_audio] Created {} audio chunks for job {}", chunks.len(), job_id);

    Ok(chunks)
}

#[tauri::command]
async fn transcribe_direct_with_gemini(
    app: AppHandle,
    state: State<'_, RunningProcesses>,
    payload: GeminiDirectRequest,
) -> AppResult<TranslationResult> {
    let start = std::time::Instant::now();
    
    log::info!(
        "[gemini-direct] Starting direct transcription: job_id={}, model={}, audio={}",
        payload.job_id,
        payload.model_id,
        payload.audio_path
    );
    
    // Chunk audio directly from source — no intermediate WAV needed.
    // Chunks are cached per job_id for retry reuse.
    let chunks = chunk_audio(
        &app,
        &payload.audio_path,
        payload.track_index,
        &payload.job_id,
        payload.duration_seconds,
    )?;
    
    log::debug!("[gemini-direct] Audio chunked into {} segments", chunks.len());
    
    // Register a cancel token for this Gemini job.
    let cancel_token = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut tokens = state
            .gemini_cancel_tokens
            .lock()
            ?;
        tokens.insert(payload.job_id.clone(), Arc::clone(&cancel_token));
    }
    
    let mut segments = Vec::new();
    let mut total_usage = TokenUsage::default();

    for (index, (chunk_path, start_ms, end_ms)) in chunks.into_iter().enumerate() {
        // Check for cancellation before processing each chunk.
        if cancel_token.load(std::sync::atomic::Ordering::Relaxed) {
            // Clean up cancel token.
            {
                let mut tokens = state
                    .gemini_cancel_tokens
                    .lock()
                    ?;
                tokens.remove(&payload.job_id);
            }
            return Err(AppError::cancelled("Job đã bị hủy bởi người dùng.", None));
        }
        
        let bytes = fs::read(&chunk_path)?;
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

        // Try processing this chunk with retry logic for JSON parse failures
        const MAX_CHUNK_RETRIES: u32 = 2;
        let mut chunk_result = None;
        let mut last_error = None;
        
        for chunk_attempt in 0..=MAX_CHUNK_RETRIES {
            if chunk_attempt > 0 {
                log::info!(
                    "[gemini-direct] Chunk {} retry {}/{}...",
                    index + 1, chunk_attempt, MAX_CHUNK_RETRIES
                );
            }
            
            match call_gemini(&state.http_client, &payload.api_key, &payload.model_id, body.clone()).await {
                Ok(response) => {
                    // Extract and accumulate token usage
                    let chunk_usage = extract_token_usage(&response);
                    
                    // Try to parse the response
                    match extract_text_part(&response) {
                        Ok(text) => {
                            match serde_json::from_str::<Value>(&text) {
                                Ok(parsed) => {
                                    total_usage.accumulate(&chunk_usage);
                                    chunk_result = Some((parsed, start_ms, end_ms));
                                    break;
                                }
                                Err(e) => {
                                    let err_msg = format!("JSON parse error: {}", e);
                                    log::warn!("[gemini-direct] Chunk {} attempt {}: {}", index + 1, chunk_attempt + 1, err_msg);
                                    last_error = Some(err_msg);
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            let err_msg = format!("Failed to extract text: {}", e);
                            log::warn!("[gemini-direct] Chunk {} attempt {}: {}", index + 1, chunk_attempt + 1, err_msg);
                            last_error = Some(err_msg);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    let err_msg = format!("API call failed: {}", e);
                    log::warn!("[gemini-direct] Chunk {} attempt {}: {}", index + 1, chunk_attempt + 1, err_msg);
                    last_error = Some(err_msg);
                    continue;
                }
            }
        }
        
        // Process the result or insert fallback
        match chunk_result {
            Some((parsed, start_ms, end_ms)) => {
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
            None => {
                log::error!(
                    "[gemini-direct] Chunk {} failed after {} retries. Using fallback. Last error: {}",
                    index + 1,
                    MAX_CHUNK_RETRIES,
                    last_error.unwrap_or_else(|| "Unknown error".to_string())
                );
                segments.push(SubtitleSegment {
                    id: format!("segment-{}", index + 1),
                    start_ms,
                    end_ms,
                    source_text: "[Transcription failed]".to_string(),
                    translated_text: "[Translation failed]".to_string(),
                });
            }
        }
    }

    // Clean up cancel token.
    {
        let mut tokens = state
            .gemini_cancel_tokens
            .lock()
            ?;
        tokens.remove(&payload.job_id);
    }
    
    let elapsed = start.elapsed();
    log::info!(
        "[gemini-direct] Direct transcription complete: {} segments, {} total tokens in {:.1}s",
        segments.len(),
        total_usage.total_tokens,
        elapsed.as_secs_f64()
    );

    Ok(TranslationResult {
        segments,
        token_usage: total_usage,
    })
}

#[tauri::command]
async fn fetch_supported_gemini_models(
    state: State<'_, RunningProcesses>,
    api_key: String,
) -> AppResult<Vec<GeminiModelOption>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );
    let response = state
        .http_client
        .get(url)
        .send()
        .await
        ?;

    if !response.status().is_success() {
        return Err(AppError::network(format!("Không lấy được Gemini models với status {}", response.status()), None, None));
    }

    let payload = response
        .json::<Value>()
        .await
        ?;
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
) -> AppResult<String> {
    let start = std::time::Instant::now();
    
    log::info!(
        "[render] Starting hard subtitle render: input={}, output={}",
        payload.input_path,
        payload.output_path
    );
    
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
        ?;

        
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
                return Err(AppError::media_processing(message, None));
            }
            CommandEvent::Terminated(payload_evt) => {
                let removed = take_running_process(&state, &job_id)?;
                if payload_evt.code == Some(0) {
                    let elapsed = start.elapsed();
                    log::info!(
                        "[render] Render complete: {} ({:.1}s)",
                        payload.output_path,
                        elapsed.as_secs_f64()
                    );
                    emit_progress(&app, &job_id, "render", 100.0, "Render hoàn tất.", Some(0));
                    if removed.is_some() {
                        break;
                    }
                } else if removed.is_none() {
                    log::warn!("[render] Render cancelled by user");
                    return Err(AppError::cancelled("Job đã bị hủy bởi người dùng.", None));
                } else {
                    log::error!("[render] Render failed with code {:?}", payload_evt.code);
                    let ffmpeg_log = stderr_lines.join("\n");
                    return Err(AppError::media_processing(format!("Render thất bại với code {:?}\n\n--- FFmpeg log ---\n{}", payload_evt.code, ffmpeg_log), None));
                }
            }
            _ => {}
        }
    }

    // Clean up temporary ASS subtitle file after render completes.
    let _ = fs::remove_file(&ass_path);

    Ok(payload.output_path)
}

#[tauri::command]
fn load_app_settings(app: AppHandle) -> AppResult<AppSettings> {
    let path = settings_path(&app)?;
    if !path.exists() {
        return Ok(AppSettings {
            api_key: String::new(),
            output_directory: String::new(),
            last_opened_project_path: String::new(),
            saved_translation_instruction: String::new(),
        });
    }
    
    let raw = fs::read(path)?;
    let settings: AppSettings = serde_json::from_slice(&raw)?;
    Ok(settings)
}

/// Write bytes to a file atomically: write to a temp file, then rename.
/// Prevents data corruption if the app crashes mid-write.
fn atomic_write(path: &std::path::Path, data: &[u8]) -> AppResult<()> {
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, data).map_err(|e| AppError::file_system(format!("Ghi file tạm thất bại: {e}"), None))?;
    fs::rename(&tmp_path, path).map_err(|e| AppError::file_system(format!("Rename file thất bại: {e}"), None))
}

#[tauri::command]
fn save_project_snapshot(path: String, snapshot: ProjectSnapshot) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(&snapshot)?;
    atomic_write(std::path::Path::new(&path), &bytes)
}

#[tauri::command]
fn load_project_snapshot(path: String) -> AppResult<ProjectSnapshot> {
    let raw = fs::read(path)?;
    let snapshot: ProjectSnapshot =
        serde_json::from_slice(&raw)?;
    migrate_project_snapshot(snapshot)
        .map_err(|e| AppError::validation(e, None))
}

#[tauri::command]
fn persist_app_settings(app: AppHandle, settings: AppSettings) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(&settings)?;
    atomic_write(&settings_path(&app)?, &bytes)
}

#[tauri::command]
fn cleanup_job_cache(app: AppHandle, job_id: String) -> AppResult<()> {
    if let Ok(cache_dir) = app.path().app_cache_dir() {
        // Remove cached whisper PCM file for this job.
        let _ = fs::remove_file(cache_dir.join("whisper-pcm").join(format!("{job_id}.raw")));
        // Remove cached gemini chunk directory for this job.
        let _ = fs::remove_dir_all(cache_dir.join("gemini-chunks").join(&job_id));
    }
    Ok(())
}

/// Get total cache size in bytes
#[tauri::command]
fn get_cache_size(app: AppHandle) -> AppResult<u64> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::file_system(format!("Failed to get cache dir: {e}"), None))?;
    
    let mut total_size = 0u64;
    
    // Calculate size of whisper-pcm cache
    if let Ok(entries) = fs::read_dir(cache_dir.join("whisper-pcm")) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
            }
        }
    }
    
    // Calculate size of gemini-chunks cache
    if let Ok(entries) = fs::read_dir(cache_dir.join("gemini-chunks")) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    total_size += calculate_dir_size(&entry.path()).unwrap_or(0);
                }
            }
        }
    }
    
    Ok(total_size)
}

/// Recursively calculate directory size
fn calculate_dir_size(path: &Path) -> Result<u64, std::io::Error> {
    let mut size = 0u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            size += calculate_dir_size(&entry.path())?;
        } else {
            size += metadata.len();
        }
    }
    Ok(size)
}

/// Clean all cache files
#[tauri::command]
fn cleanup_all_cache(app: AppHandle) -> AppResult<()> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::file_system(format!("Failed to get cache dir: {e}"), None))?;
    
    // Remove all whisper PCM files
    let _ = fs::remove_dir_all(cache_dir.join("whisper-pcm"));
    let _ = fs::create_dir_all(cache_dir.join("whisper-pcm"));
    
    // Remove all gemini chunks
    let _ = fs::remove_dir_all(cache_dir.join("gemini-chunks"));
    let _ = fs::create_dir_all(cache_dir.join("gemini-chunks"));
    
    Ok(())
}

/// Clean old cache files based on access time
#[tauri::command]
fn cleanup_old_cache(app: AppHandle, days: u64) -> AppResult<u64> {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::file_system(format!("Failed to get cache dir: {e}"), None))?;
    
    let cutoff_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::unknown(format!("Time error: {e}")))?
        .as_secs() - (days * 24 * 3600);
    
    let mut cleaned_bytes = 0u64;
    
    // Clean old whisper PCM files
    if let Ok(entries) = fs::read_dir(cache_dir.join("whisper-pcm")) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(accessed) = metadata.accessed() {
                    if let Ok(duration) = accessed.duration_since(UNIX_EPOCH) {
                        if duration.as_secs() < cutoff_time {
                            cleaned_bytes += metadata.len();
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }
    
    // Clean old gemini chunk directories
    if let Ok(entries) = fs::read_dir(cache_dir.join("gemini-chunks")) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    if let Ok(accessed) = metadata.accessed() {
                        if let Ok(duration) = accessed.duration_since(UNIX_EPOCH) {
                            if duration.as_secs() < cutoff_time {
                                cleaned_bytes += calculate_dir_size(&entry.path()).unwrap_or(0);
                                let _ = fs::remove_dir_all(entry.path());
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(cleaned_bytes)
}

#[tauri::command]
fn cancel_pipeline_process(
    app: AppHandle,
    state: State<'_, RunningProcesses>,
    job_id: String,
) -> AppResult<()> {
    log::info!("[cancel] Cancelling pipeline: job_id={}", job_id);
    
    let mut cancelled_types = Vec::new();
    
    // Cancel ffmpeg/sidecar processes.
    if let Some(child) = take_running_process(&state, &job_id)? {
        child.kill()?;
        cancelled_types.push("process");
    }

    // Cancel whisper-rs jobs via AtomicBool token.
    {
        let mut tokens = state
            .whisper_cancel_tokens
            .lock()
            ?;
        if let Some(token) = tokens.remove(&job_id) {
            token.store(true, std::sync::atomic::Ordering::Relaxed);
            cancelled_types.push("whisper");
        }
    }

    // Cancel Gemini translation jobs via AtomicBool token.
    {
        let mut tokens = state
            .gemini_cancel_tokens
            .lock()
            ?;
        if let Some(token) = tokens.remove(&job_id) {
            token.store(true, std::sync::atomic::Ordering::Relaxed);
            cancelled_types.push("gemini");
        }
    }
    
    if !cancelled_types.is_empty() {
        log::info!("[cancel] Cancelled: {}", cancelled_types.join(", "));
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
    // Initialize logger with millisecond timestamps
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(RunningProcesses::default())
        .register_uri_scheme_protocol("stream", |_ctx, request| {
            let uri = request.uri().to_string();
            // Parse path from stream://localhost/<encoded_path>
            let path = uri
                .strip_prefix("stream://localhost/")
                .or_else(|| uri.strip_prefix("stream://localhost"))
                .unwrap_or("");
            let path = percent_encoding::percent_decode_str(path)
                .decode_utf8_lossy()
                .to_string();

            let file_path = std::path::Path::new(&path);
            if !file_path.exists() {
                return HttpResponse::builder()
                    .status(404)
                    .body(Vec::new())
                    .unwrap();
            }

            let file_size = match fs::metadata(&path) {
                Ok(m) => m.len(),
                Err(_) => {
                    return HttpResponse::builder()
                        .status(500)
                        .body(Vec::new())
                        .unwrap();
                }
            };

            // Determine MIME type from extension
            let mime = match file_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .as_deref()
            {
                Some("mp4") => "video/mp4",
                Some("webm") => "video/webm",
                Some("mkv") => "video/x-matroska",
                Some("avi") => "video/x-msvideo",
                Some("mov") => "video/quicktime",
                Some("m4v") => "video/x-m4v",
                _ => "application/octet-stream",
            };

            // Parse Range header
            let range_header = request.headers().get("range").and_then(|v| v.to_str().ok());

            if let Some(range_str) = range_header {
                // Parse "bytes=START-END" or "bytes=START-"
                let range_str = range_str.trim_start_matches("bytes=");
                let parts: Vec<&str> = range_str.splitn(2, '-').collect();
                let start: u64 = parts[0].parse().unwrap_or(0);
                let end: u64 = if parts.len() > 1 && !parts[1].is_empty() {
                    parts[1].parse().unwrap_or(file_size - 1)
                } else {
                    // Limit chunk to STREAM_CHUNK_SIZE for large files to avoid memory issues
                    std::cmp::min(start + STREAM_CHUNK_SIZE - 1, file_size - 1)
                };
                let end = std::cmp::min(end, file_size - 1);
                let content_length = end - start + 1;

                let mut file = match fs::File::open(&path) {
                    Ok(f) => f,
                    Err(_) => {
                        return HttpResponse::builder()
                            .status(500)
                            .body(Vec::new())
                            .unwrap();
                    }
                };

                if file.seek(SeekFrom::Start(start)).is_err() {
                    return HttpResponse::builder()
                        .status(500)
                        .body(Vec::new())
                        .unwrap();
                }

                let mut buf = vec![0u8; content_length as usize];
                if let Err(_) = file.read_exact(&mut buf) {
                    // Read as much as possible
                    let _ = file.seek(SeekFrom::Start(start));
                    buf.resize(content_length as usize, 0);
                    let _ = file.read(&mut buf);
                }

                HttpResponse::builder()
                    .status(206)
                    .header("Content-Type", mime)
                    .header("Content-Length", content_length.to_string())
                    .header(
                        "Content-Range",
                        format!("bytes {}-{}/{}", start, end, file_size),
                    )
                    .header("Accept-Ranges", "bytes")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(buf)
                    .unwrap()
            } else {
                // No Range header — return the first chunk as 206 Partial Content
                // to force the browser to follow up with proper Range requests.
                // This avoids reading entire multi-GB files into memory.
                let end = std::cmp::min(STREAM_CHUNK_SIZE - 1, file_size - 1);
                let content_length = end + 1;

                let mut file = match fs::File::open(&path) {
                    Ok(f) => f,
                    Err(_) => {
                        return HttpResponse::builder()
                            .status(500)
                            .body(Vec::new())
                            .unwrap();
                    }
                };

                let mut buf = vec![0u8; content_length as usize];
                if let Err(_) = file.read_exact(&mut buf) {
                    buf.clear();
                    let _ = file.seek(SeekFrom::Start(0));
                    let _ = file.read_to_end(&mut buf);
                }

                HttpResponse::builder()
                    .status(206)
                    .header("Content-Type", mime)
                    .header("Content-Length", content_length.to_string())
                    .header(
                        "Content-Range",
                        format!("bytes 0-{}/{}", end, file_size),
                    )
                    .header("Accept-Ranges", "bytes")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(buf)
                    .unwrap()
            }
        })
        .invoke_handler(tauri::generate_handler![
            detect_runtime_capabilities,
            install_local_ffmpeg_runtime,
            remove_local_ffmpeg_runtime,
            list_whisper_models,
            download_whisper_model,
            remove_whisper_model,
            inspect_media,
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
            cleanup_job_cache,
            get_cache_size,
            cleanup_all_cache,
            cleanup_old_cache,
            cancel_pipeline_process
        ])
        .run(tauri::generate_context!())
        .expect("error while running easyvietsub");
}

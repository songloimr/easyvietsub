use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

pub fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

pub fn app_cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

pub fn models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("models");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

pub fn tools_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("tools");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

pub fn fonts_dir(app: &AppHandle) -> Result<PathBuf, String> {
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

pub fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("settings.json"))
}

pub fn command_filename(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

pub fn managed_binary_path(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    Ok(tools_dir(app)?.join(command_filename(name)))
}

pub fn bundled_sidecar_path(name: &str) -> Result<PathBuf, String> {
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

pub fn resolve_runtime_binary_path(
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

pub fn resolve_ffmpeg_path(app: &AppHandle) -> Result<PathBuf, String> {
    resolve_runtime_binary_path(app, "ffmpeg", "ffmpeg-sidecar", "ffmpeg")
}

pub fn resolve_ffprobe_path(app: &AppHandle) -> Result<PathBuf, String> {
    resolve_runtime_binary_path(app, "ffprobe", "ffprobe-sidecar", "ffprobe")
}

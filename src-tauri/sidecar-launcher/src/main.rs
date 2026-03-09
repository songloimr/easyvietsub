use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process::{exit, Command};

fn select_program(binary_name: &str) -> &'static str {
    if binary_name.contains("ffprobe-sidecar") {
        "ffprobe"
    } else if binary_name.contains("whisper-cli-sidecar") {
        "whisper-cli"
    } else {
        "ffmpeg"
    }
}

fn resolve_override(program: &str) -> Option<OsString> {
    let key = match program {
        "ffprobe" => "EASYVIETSUB_FFPROBE_BIN",
        "whisper-cli" => "EASYVIETSUB_WHISPER_BIN",
        _ => "EASYVIETSUB_FFMPEG_BIN"
    };

    env::var_os(key)
}

fn main() {
    let argv0 = env::args_os().next().unwrap_or_else(|| OsString::from("sidecar"));
    let binary_name = Path::new(&argv0)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("sidecar");
    let program = select_program(binary_name);
    let resolved = resolve_override(program).unwrap_or_else(|| OsString::from(program));

    let status = Command::new(resolved)
        .args(env::args_os().skip(1))
        .status()
        .unwrap_or_else(|error| {
            eprintln!("failed to launch delegated binary: {error}");
            exit(1);
        });

    exit(status.code().unwrap_or(1));
}

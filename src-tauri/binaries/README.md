# Sidecar Binaries

This project is configured to bundle `ffprobe`, `ffmpeg` and `whisper-cli` as Tauri sidecars.

Current repository contents:
- `ffmpeg-sidecar-aarch64-apple-darwin`
- `ffprobe-sidecar-aarch64-apple-darwin`
- `whisper-cli-sidecar-aarch64-apple-darwin`

These wrappers delegate to the host-installed binaries, or to:
- `EASYVIETSUB_FFPROBE_BIN`
- `EASYVIETSUB_FFMPEG_BIN`
- `EASYVIETSUB_WHISPER_BIN`

## Building Sidecar Binaries

Before running `tauri build` or `tauri dev`, you **must** build sidecars for your target platform. If the matching binaries are missing Tauri will fail to bundle.

### macOS (Apple Silicon)

```bash
npm run sidecars:build
```

Produces:
- `ffmpeg-sidecar-aarch64-apple-darwin`
- `ffprobe-sidecar-aarch64-apple-darwin`
- `whisper-cli-sidecar-aarch64-apple-darwin`

### Windows (x64)

```powershell
npm run sidecars:build:windows
```

Produces:
- `ffmpeg-sidecar-x86_64-pc-windows-msvc.exe`
- `ffprobe-sidecar-x86_64-pc-windows-msvc.exe`
- `whisper-cli-sidecar-x86_64-pc-windows-msvc.exe`

### Custom target

```bash
# Shell
./scripts/build-sidecars.sh <target-triple>

# PowerShell
./scripts/build-sidecars.ps1 -Target <target-triple>
```

## Naming Convention

Tauri appends the Rust target triple (and `.exe` on Windows) to each sidecar key in `tauri.conf.json`:
- `binaries/ffmpeg-sidecar` → `ffmpeg-sidecar-<target>[.exe]`

For production packaging on other targets, add target-specific binaries using Tauri's expected naming convention:
- `*-x86_64-pc-windows-msvc.exe`
- `*-aarch64-apple-darwin`
- `*-x86_64-unknown-linux-gnu`

## Source

The `sidecar-launcher` helper source lives in `src-tauri/sidecar-launcher`. It produces tiny native launcher binaries that delegate to `ffprobe`, `ffmpeg`, or `whisper-cli` via PATH or the corresponding `EASYVIETSUB_*_BIN` override env var.

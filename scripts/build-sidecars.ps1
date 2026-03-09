$ErrorActionPreference = "Stop"

$RootDir = Resolve-Path (Join-Path $PSScriptRoot "..")
$LauncherManifest = Join-Path $RootDir "src-tauri/sidecar-launcher/Cargo.toml"
$TargetTriple = if ($args.Length -gt 0) { $args[0] } else { "x86_64-pc-windows-msvc" }
$BinDir = Join-Path $RootDir "src-tauri/binaries"

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

cargo build --manifest-path $LauncherManifest --release --target $TargetTriple

$OutputDir = Join-Path $RootDir "src-tauri/sidecar-launcher/target/$TargetTriple/release"
$Launcher = Join-Path $OutputDir "easyvietsub-sidecar-launcher.exe"

Copy-Item $Launcher (Join-Path $BinDir "ffmpeg-sidecar-$TargetTriple.exe") -Force
Copy-Item $Launcher (Join-Path $BinDir "ffprobe-sidecar-$TargetTriple.exe") -Force
Copy-Item $Launcher (Join-Path $BinDir "whisper-cli-sidecar-$TargetTriple.exe") -Force

Write-Host "Built sidecars for $TargetTriple into $BinDir"

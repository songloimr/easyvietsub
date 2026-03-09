# EasyVietsub

EasyVietsub là ứng dụng desktop dùng `SvelteKit + Tauri` để xử lý quy trình làm phụ đề tiếng Việt cho video/audio: inspect media, chuyển giọng nói thành văn bản, dịch sang tiếng Việt, xuất subtitle và render hard-sub.

## Tính năng chính

- Chọn file video/audio và inspect metadata, audio track, thời lượng.
- Hỗ trợ 2 chế độ xử lý:
  - `Whisper local + Gemini translate`
  - `Gemini direct audio` (experimental)
- Tùy chỉnh prompt dịch, chọn Gemini model và Whisper model.
- Quản lý FFmpeg local và tải/xóa Whisper model ngay trong app.
- Chỉnh style subtitle, preview và render video hard-sub.
- Export `SRT` / `ASS`, lưu và import project snapshot `.easyvietsub.json`.
- Có history và process log để theo dõi từng job.

## Stack

- Frontend: `SvelteKit 2`, `Svelte 5`, `TypeScript`, `Tailwind CSS v4`
- Desktop runtime: `Tauri 2`
- Backend native: `Rust`
- Subtitle editor: `TipTap`
- Media pipeline: `FFmpeg`, `ffprobe`, `whisper-rs`, Gemini API

## Yêu cầu môi trường

- `Node.js` 20+
- `npm`
- `Rust` toolchain stable
- Máy có thể chạy Tauri 2
- Gemini API key để dịch / dùng chế độ Gemini direct

Ghi chú:

- App có thể cài FFmpeg local trong phần Settings trên một số nền tảng hỗ trợ.
- Trước khi chạy Tauri, cần có sidecar binary đúng target.

## Cài đặt

```bash
npm install
```

## Chạy dự án

Chạy frontend:

```bash
npm run dev
```

Chạy app desktop với Tauri:

```bash
npm run sidecars:build
npm run tauri dev
```

Build production:

```bash
npm run build
npm run tauri build
```

## Sidecars

Repo này dùng các sidecar launcher cho:

- `ffmpeg`
- `ffprobe`
- `whisper-cli`

Build sidecar theo target hiện tại:

```bash
npm run sidecars:build
```

Build sidecar cho Windows:

```powershell
npm run sidecars:build:windows
```

Các file sidecar sinh ra trong `src-tauri/binaries/` hiện đã được thêm vào `.gitignore` vì đây là artifact build theo máy/target. Khi clone repo mới, chỉ cần build lại sidecar trước khi chạy Tauri.

## Cách dùng cơ bản

1. Mở app và nhập Gemini API key trong tab `Settings`.
2. Cài FFmpeg local nếu máy chưa có `ffmpeg` / `ffprobe`.
3. Nếu dùng chế độ Whisper, tải model phù hợp trong `Settings`.
4. Chọn file media ở tab `Translate`.
5. Chạy pipeline để tạo transcript và subtitle tiếng Việt.
6. Kiểm tra/chỉnh subtitle, chọn thư mục output.
7. Export subtitle hoặc render video hard-sub.

## Scripts hữu ích

```bash
npm run dev
npm run check
npm run build
npm run preview
npm run tauri dev
npm run tauri build
npm run sidecars:build
npm run sidecars:build:windows
```

## Cấu trúc thư mục

```text
src/                 giao diện SvelteKit
src/lib/             store, service, type, utility
src-tauri/           mã Rust + cấu hình Tauri
scripts/             script build sidecar
static/              asset tĩnh
fonts/               font dùng khi render subtitle
```

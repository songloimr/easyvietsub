# EasyVietsub

Ứng dụng desktop chuyên vietsub phim JAV — hỗ trợ tạo, chỉnh sửa và gắn phụ đề tiếng Việt vào video.

## Screenshots

<!-- Thêm screenshots tại đây -->

*Screenshots coming soon.*

## Tính năng chính

- **Nhận dạng & dịch tự động** — chuyển giọng nói thành text (Whisper) và dịch sang tiếng Việt (Gemini)
- **Chỉnh sửa phụ đề** — editor trực quan, preview realtime ngay trong app
- **Export & render** — xuất file SRT / ASS hoặc render hard-sub trực tiếp vào video

## Yêu cầu

- [Node.js](https://nodejs.org/) >= 20 và npm
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Tauri 2 Prerequisites](https://v2.tauri.app/start/prerequisites/)
- Gemini API key — lấy tại [Google AI Studio](https://aistudio.google.com/apikey)

> FFmpeg có thể cài global hoặc để app dùng sidecar đi kèm. Xem phần [Sidecars](#sidecars) bên dưới.

## Cài đặt & Chạy

```bash
# 1. Clone repo
git clone https://github.com/<your-username>/easyvietsub.git
cd easyvietsub

# 2. Cài dependencies
npm install

# 3. Build sidecars (xem phần Sidecars)
npm run sidecars:build

# 4. Chạy dev
npm run tauri dev

# 5. Build production
npm run tauri build
```

## Sidecars

App đi kèm 3 sidecar binary: **ffmpeg**, **ffprobe** và **whisper-cli**.

Các binary này cần được build/copy vào `src-tauri/binaries/` theo đúng naming convention của Tauri (kèm target triple).

```bash
# macOS / Linux
npm run sidecars:build

# Windows (PowerShell)
npm run sidecars:build:windows
```

> Thư mục `src-tauri/binaries/` đã được thêm vào `.gitignore`. Chi tiết xem tại [`src-tauri/binaries/README.md`](src-tauri/binaries/README.md).

## Contributing

1. Fork repo
2. Tạo branch mới (`git checkout -b feature/ten-tinh-nang`)
3. Commit changes
4. Mở Pull Request

## License

[MIT](LICENSE)

# Video Manager — Rust + egui

Native desktop replacement for the YouTube, local-video workspace, Cloudflare R2, and `data.json` workflows in the uploaded Flutter project.

## What is included

### YouTube

- Paste a YouTube URL and fetch metadata/chapters with `yt-dlp`.
- Download the full video as MP4 into its own workspace folder.
- Download subtitles when available and convert VTT into a readable **timestamped** `transcript.txt`.
- Save normalized `chapters.json`, an optional thumbnail, and `manifest.json`.

### Local files

- Scan the configured workspace for downloaded or manually created video folders.
- Read the full video, optional transcript/thumbnail, `chapters.json`, and `manifest.json`.
- Cut one chapter or all chapters with FFmpeg.
- **Cut & upload** one chapter or all chapters directly to R2.
- Upload the full local video to R2.
- Store successful R2 URLs in the workspace `manifest.json`.
- Sync the selected workspace into `data.json`, using uploaded clip URLs as each chapter's `videoUrl`.

### Cloudflare R2

- Native S3-compatible AWS Signature V4 implementation in Rust.
- List objects / test credentials.
- Stream file uploads instead of loading large videos fully into memory.
- Namespace workspace uploads as `<prefix>/<video-title>/...`.
- Generate public URLs from `R2_PUBLIC_BASE_URL`.
- Pick an arbitrary local file for upload and pass its resulting URL into the Data JSON editor.

### Data JSON

The editor uses the same shape as the Flutter video player:

```json
[
  {
    "name": "Training",
    "type": "direct",
    "videoId": "SOURCE_VIDEO_ID",
    "videoUrl": "https://media.example.com/full-video.mp4",
    "thumbnailUrl": "https://media.example.com/thumb.webp",
    "chapters": [
      {
        "name": "Pressing",
        "startSeconds": 10.0,
        "endSeconds": 25.0,
        "videoUrl": "https://media.example.com/01-Pressing.mp4"
      }
    ]
  }
]
```

Fields with empty values are omitted when saved. The GUI can:

- create/delete playlists;
- edit `type`, `videoId`, `videoUrl`, and `thumbnailUrl`;
- add/remove/reorder chapters;
- edit chapter timestamps and `videoUrl` values;
- add/sync a selected local workspace;
- use the last uploaded R2 URL or an object selected in the R2 tab.

## Workspace format

```text
VideoManager/
└── Video Title/
    ├── video.mp4
    ├── chapters.json
    ├── transcript.txt       # optional
    ├── thumbnail.webp       # optional; extension can vary
    ├── manifest.json
    └── cuts/
        ├── 01-Intro.mp4
        └── 02-Drill.mp4
```

`manifest.json` records source metadata and the public R2 URL of every uploaded local file. The Data JSON sync uses those manifest entries rather than guessing URLs.

## Settings and secrets

R2 credentials are **not hardcoded**. Configure them in the Settings tab or through environment variables / a `.env` file beside the executable or in its working directory:

- `VIDEO_MANAGER_WORKSPACE`
- `VIDEO_MANAGER_DATA_JSON`
- `YT_DLP_PATH`
- `FFMPEG_PATH`
- `R2_ACCOUNT_ID`
- `R2_BUCKET`
- `R2_PREFIX`
- `R2_PUBLIC_BASE_URL`
- `R2_ACCESS_KEY_ID`
- `R2_SECRET_ACCESS_KEY`

A public base URL is required for uploads because the generated URL is written into manifests and `data.json`.

## Standalone/runtime model

The GUI is one native Rust executable and does not need Flutter, Dart, Node, or Python. Media extraction/encoding intentionally delegates to the established command-line tools:

- `yt-dlp`
- `ffmpeg`

Install them on `PATH`, place them beside the app and configure their path, or point the Settings fields at their executables. The app itself is still a single native executable; these two media tools are intentionally kept external so they can be updated independently.

The Settings tab stores its local settings JSON in your OS config directory. R2 secrets saved there are plain local configuration values; use environment variables / `.env` instead if you do not want credentials persisted in that file.

## Build

Rust 1.97.1 is the project baseline.

```bash
cargo build --release
```

Outputs:

- Linux: `target/release/video-manager-egui`
- Windows: `target/release/video-manager-egui.exe`

The included GitHub Actions workflow runs formatting, tests, Clippy, then builds Linux x86_64 and Windows x86_64 artifacts.

# Native Messaging Design

## Goal

Make the existing `video-manager-egui` executable also act as the Chrome/Chromium Native Messaging host used by the YouTube Chapter Clipper extension, while automatically registering the host on normal GUI startup.

## Architecture

Use one executable in two runtime modes. A normal launch registers/repairs user-scope manifests and starts egui. A browser launch is detected from the `chrome-extension://.../` origin argument and runs a stdio Native Messaging loop instead of opening the GUI.

Use `native_messaging` 0.3 for protocol framing and cross-platform manifest/registry installation. Keep the host synchronous to avoid adding a second async runtime. The Rust host accepts the existing extension actions `ping`, `download_chapter`, `download_full_video`, and `open_clip` and preserves the response field names used by the extension.

## Development origin policy

Chromium manifests do not support wildcard `allowed_origins`. Development-permissive registration therefore includes the known Chapter Clipper extension ID and every valid installed Chromium extension ID discoverable from browser profile Preferences and Extensions directories. The Rust host applies no additional origin authorization.

## Command policy

Native messages are not mapped to arbitrary shell commands. The host only dispatches explicit app-owned actions. External media tools are invoked with argument arrays, not a shell.

## Media behavior

- `download_chapter` uses yt-dlp section downloading and the configured FFmpeg path. By default clips are written under `<workspace>/Youtube_Clips`; an explicit `outputDir` from the extension is honored.
- `download_full_video` reuses the Rust workspace downloader with a temporary workspace root of `<workspace>/Youtube_Videos` unless `outputDir` is supplied. If yt-dlp metadata has no chapters, valid chapters supplied by the extension are written into `chapters.json`.
- `open_clip` opens an existing file or folder with the platform default handler.

## Registration

On GUI startup, resolve the absolute current executable path and install/repair the user-scope native host for Chrome, Chromium, Edge, Brave, and Vivaldi. Registration is best-effort per browser; a failure for one browser does not prevent the app from starting or other browsers from being registered.

## Testing

Tests cover browser-launch detection, development origin discovery, dispatch/response compatibility, framed multi-message I/O, request parsing, and argument generation for section downloads. Existing tests, strict Clippy, and Linux/Windows release builds remain required in CI.

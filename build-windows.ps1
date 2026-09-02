$ErrorActionPreference = 'Stop'
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
Write-Host 'Built: target\release\video-manager-egui.exe'

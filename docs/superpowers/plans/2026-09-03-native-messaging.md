# Native Messaging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add automatic Chrome/Chromium Native Messaging registration and host mode to the existing Rust desktop executable.

**Architecture:** The single executable has GUI mode and browser-host mode. `src/native_host.rs` owns registration, origin discovery, framing/dispatch, browser actions, and platform open behavior. `src/main.rs` selects host mode before egui initialization and performs best-effort automatic registration for normal GUI launches.

**Tech Stack:** Rust 1.97.1, `native_messaging` 0.3 sync framing + installer, serde/serde_json, existing yt-dlp/FFmpeg pipeline.

**Spec:** `docs/superpowers/specs/2026-09-03-native-messaging-design.md`

## Global Constraints

- Keep host name exactly `com.fluent_desktop.youtube_clipper`.
- Preserve extension actions `ping`, `download_chapter`, `download_full_video`, `open_clip`.
- Do not expose arbitrary shell command execution.
- Register at user scope automatically on GUI startup.
- Allow the known Chapter Clipper extension plus discovered Chromium extension IDs; no extra runtime origin check.
- Keep Linux and Windows CI green with strict Clippy.

---

### Task 1: Protocol and dispatch

**Files:**
- Create: `src/native_host.rs`
- Modify: `src/lib.rs`
- Test: `tests/native_host.rs`

**Interfaces:**
- Produces `NativeHostBackend`, `dispatch_message`, `run_host_loop_with_io`, `is_native_host_launch`.

- [x] **Step 1: Write failing protocol tests**
- [x] **Step 2: Run tests and confirm missing-module failure**
- [ ] **Step 3: Add `native_messaging` dependency and minimal protocol implementation**
- [ ] **Step 4: Run protocol tests until green**

### Task 2: Automatic registration and origin discovery

**Files:**
- Modify: `src/native_host.rs`
- Modify: `src/main.rs`
- Test: `tests/native_host.rs`

**Interfaces:**
- Produces `auto_register_current_executable` and permissive Chromium origin discovery.

- [ ] **Step 1: Implement/test valid extension ID extraction from Preferences and Extensions directories**
- [ ] **Step 2: Register each Chromium browser independently with `native_messaging::install(..., Scope::User)`**
- [ ] **Step 3: Branch `main` into host mode before egui and auto-register only in GUI mode**

### Task 3: Extension actions

**Files:**
- Modify: `src/native_host.rs`
- Test: `tests/native_host.rs`

**Interfaces:**
- Production backend uses `AppConfig`, `youtube::download_bundle`, yt-dlp section arguments, and platform file opening.

- [ ] **Step 1: Add request validation and chapter download arguments**
- [ ] **Step 2: Implement chapter clip response contract**
- [ ] **Step 3: Implement full-video workspace response contract with browser-chapter fallback**
- [ ] **Step 4: Implement `open_clip` with platform argument-safe process launch**

### Task 4: Verification and delivery

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Run `cargo fmt -- --check`**
- [ ] **Step 2: Run `cargo test --all-targets`**
- [ ] **Step 3: Run `cargo clippy --all-targets -- -D warnings`**
- [ ] **Step 4: Run release build where practical and rely on Linux/Windows Actions matrix for final artifacts**
- [ ] **Step 5: Open PR, inspect diff and CI, fix failures, then merge to `main`**

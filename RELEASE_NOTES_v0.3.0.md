# Release Notes — v0.3.0

This release focuses on repository migration, CI/CD improvements, installation ease, and maintaining a strict zero-warning compilation baseline.

## 🚀 Enhancements & Features

- **Project Migration**: Properly migrated all project URL references, updater endpoints, and security contacts to the new namespace `YoannDev90/lotion-rs`.
- **Quick Installation Script**: Added a one-liner `curl` install script to completely automate installation across different Linux distributions and macOS. It automatically detects your OS and pulls the correct pre-built binary.
- **Improved CI/CD Pipelines**: Added `workflow_dispatch` triggers to all GitHub Actions workflows (`build`, `ci`, `integrity`, `rust-clippy`, `test`). You can now manually trigger any of these jobs directly from the GitHub UI.
- **Tauri Auto-Updater**: Regenerated and implemented new cryptographic signature keys for the Tauri auto-update system (`tauri.conf.json`).

## 🐞 Bug Fixes

- **Windows Sandboxing (LiteBox)**: Fixed a cross-platform compilation error in `src-tauri/src/litebox/windows.rs` where the `CreateJobObjectW` handle check wrongly expected a `usize` instead of a raw pointer (`*mut c_void`).
- **Zero-Warning Builds**: Cleaned up the Rust codebase to compile completely silently across all OS profiles:
  - Removed unused `super::*` imports in `config.rs`.
  - Silenced an `unused_mut` warning on `window_builder` in `window_controller.rs` specifically popping up on macOS builds.
  - Handled unused `event` variables cleanly in `main.rs` when compiled in release mode (where network logging is stripped).

## 🛡️ Security

- **Updated Security Channels**: Security vulnerability reporting email and GitHub advisory links have been successfully routed to `@YoannDev90`.
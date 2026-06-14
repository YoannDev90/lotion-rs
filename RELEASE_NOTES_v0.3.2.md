# Release Notes — v0.3.2

This release fixes the window icon not appearing in the KDE taskbar, enables standalone executable binaries in CI artifacts, and registers the previously unused opener plugin.

## Highlights

- **Window icon fix on Linux**: The icon is now embedded at compile time via `include_bytes!` and applied after window creation, ensuring it appears in the KDE taskbar regardless of working directory.
- **GTK app ID enabled**: Setting `enableGTKAppId` in `tauri.conf.json` gives the window a proper `WM_CLASS` / Wayland `app_id`, allowing the compositor to associate it with the `.desktop` file.
- **Standalone binaries in CI**: Both `build.yml` and `release.yml` now collect the raw executable binary alongside installer packages (`lotion-rs`, `lotion-rs-x64`, `lotion-rs-arm64`, `lotion-rs.exe`).
- **Opener plugin registered**: `tauri-plugin-opener` is now properly initialized in the Tauri builder, enabling system-browser URL opening from the frontend.
- **No duplicate tag builds**: `build.yml` no longer triggers on tags (`v*`), avoiding redundant builds when a release tag is pushed.

## Bug Fixes

- **Window icon missing in KDE taskbar**: Replaced the broken runtime path loading with compile-time embedded bytes; replaced the builder `icon()` call (which consumed ownership) with `set_icon()` on the built window.
- **Builder error handling**: Fixed the `unwrap_or_else` in `window_controller.rs` that silently created a new window builder without icon on failure.

## CI / Build

- **Executable binaries in artifacts**: Build and release workflows now copy `lotion-rs` (Linux/macOS) and `lotion-rs.exe` (Windows) to the staging directory.
- **Build workflow scope reduced**: Removed `tags: ["v*"]` from `build.yml` triggers so it only runs on branch pushes and PRs, avoiding duplicate work with `release.yml`.

## Platform

- **Wayland app ID**: Environment variable `APP_ID=lotion-rs` is now set on Linux for better Wayland integration.
- **GTK app ID**: `enableGTKAppId: true` in `tauri.conf.json` uses the app identifier as the GTK application ID.

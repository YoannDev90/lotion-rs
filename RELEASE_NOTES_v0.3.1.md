# Release Notes — v0.3.1

This release focuses on stabilizing the app runtime, improving error handling, and cleaning up the menu and window behavior after the larger UI refactor.

## Highlights

- **Stronger command error handling**: Tauri commands now return explicit `Result<T, String>` values where it matters, allowing errors to propagate cleanly instead of failing silently.
- **Async state persistence**: App state save/load paths were moved to async I/O to reduce blocking and improve responsiveness.
- **Config-driven trust policy**: Trusted navigation domains are now sourced from persisted config instead of being hardcoded.
- **Tracing-based logging**: Replaced the legacy logging setup with `tracing` and `tracing-subscriber` for more structured runtime diagnostics.

## Bug Fixes

- **Menu/window issues**: Fixed the broken custom Preferences/About window experiments and then removed them to keep the menu reliable and maintainable.
- **Menu simplification**: Reduced the top-level menu to the stable View actions, keeping theme, language, reload, fullscreen, and developer tools behavior intact.
- **Startup stability**: Tightened initialization flow around config, state loading, and background save tasks.

## Security & Reliability

- **Safer origin checks**: Origin validation now uses the persisted allowlist, making the policy easier to audit and change.
- **Less blocking work**: Reduced synchronous file and lock-heavy paths in the runtime.
- **Better runtime observability**: Added targeted logs around menu actions and state persistence to make regressions easier to diagnose.

## Notes

- The app remains centered on its core browsing workflow and the View menu actions.
- Preferences/About were intentionally removed after proving too heavy and unreliable in this branch.
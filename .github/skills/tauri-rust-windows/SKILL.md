---
name: tauri-rust-windows
description: 'Use when implementing or debugging What-N-When Tauri 2 and Rust behavior, Windows APIs, window or webview lifecycle, IPC commands, persistence, autostart, desktop interaction, or native integration.'
---

# Tauri and Rust for Windows

Use this workflow for native application changes in What-N-When.

## Procedure

1. Trace the relevant Tauri command, event, or window flow in `src-tauri/src/lib.rs` and check its frontend callers before editing.
2. Keep platform-specific code behind Windows-specific modules or `cfg(windows)` where appropriate. Reuse the existing Windows bindings and Tauri plugins rather than introducing overlapping APIs or dependencies.
3. Keep frontend command names, argument names, serialized settings fields, and Rust types in sync. Tauri/Serde exposes the existing option fields to JavaScript in camelCase.
4. Preserve lifecycle and threading requirements. The desktop double-click hook, Tauri window operations, and cross-process widget activation have distinct threading and OS constraints; do not block the UI thread or call UI APIs from native callbacks without the established dispatch mechanism.
5. Preserve settings compatibility and existing behavior for window bounds, startup, desktop toggles, transparency limits, child webviews, and popup handling. Update related tests or documentation when behavior changes.
6. Run `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo check --manifest-path src-tauri/Cargo.toml`, `npm run check`, and, for integrated application changes, `npm run build`.

## Review checklist

- Tauri commands are registered and their frontend invocation shapes still match.
- Errors are surfaced to the caller or logged with enough context; new failures are not silently ignored.
- OS hooks, handles, and background tasks are cleaned up correctly.
- Window geometry, z-order, taskbar behavior, and child-webview positioning remain intentional.
- Windows-only code is validated on Windows or clearly marked as not validated.

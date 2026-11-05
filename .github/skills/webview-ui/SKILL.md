---
name: webview-ui
description: 'Use when changing What-N-When HTML, CSS, or vanilla JavaScript in web/, including the frameless widget, resize handles, options form, Tauri IPC, accessibility, or WebView security policy.'
---

# WebView UI Development

Use this workflow for the static frontend bundled by Tauri.

## Procedure

1. Inspect the related page and script in `web/` and identify the backend command or event they interact with. This project uses plain HTML, CSS, and JavaScript; do not assume a frontend framework, bundler, or TypeScript build.
2. Preserve compatibility with `npm run check` and the supported Node syntax checker. Keep changes consistent with the existing DOM-based implementation.
3. For Tauri IPC, verify command names, camelCase argument/property names, return shapes, and error behavior against `src-tauri/src/lib.rs`. Show a useful status or log on failures; do not make a failed command appear successful.
4. Preserve the frameless window layout: resize-handle geometry must continue to match the native resize directions, and the calendar child webview must remain below the custom title bar.
5. Keep the content security policy in `src-tauri/tauri.conf.json` restrictive. Only add an origin or directive when a concrete feature requires it.
6. For options changes, keep backend defaults and limits, current-form values, live preview/revert behavior, and save/reset/cancel flows consistent.
7. Run `npm run check`. For changes involving native integration or overall layout behavior, also run `cargo check --manifest-path src-tauri/Cargo.toml` or `npm run build` as appropriate.

## Review checklist

- Controls work with keyboard as well as pointer input where applicable.
- User-visible status and error states are understandable.
- No new inline script or unnecessary CSP relaxation is introduced.
- Desktop and WebView sizing, resize handles, and window controls remain aligned.

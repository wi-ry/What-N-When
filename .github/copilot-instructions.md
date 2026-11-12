<!-- Workspace-specific instructions for GitHub Copilot. -->

# What-N-When - Development Instructions

This is a Windows desktop widget built with Tauri 2, Rust, and WebView2. The frontend is a small set of static HTML, CSS, and JavaScript files in `web/`. The repository also contains a .NET 8 Windows widget-provider project under `src-tauri/widget-provider/` and its unit tests under `src-tauri/widget-provider.Tests/`; do not describe the repository as having no .NET project.

## Prerequisites

- Windows 10 or 11
- Node.js 22 (the CI workflow uses Node 22; Node 18+ is sufficient for normal development)
- Rust stable with the MSVC toolchain and Windows build tools
- WebView2 Runtime, usually already installed on modern Windows systems

Install JavaScript dependencies with:

```powershell
npm ci
```

## Development Commands

Run these from the repository root:

```powershell
# Validate both frontend scripts
npm run check

# Start the Tauri development application
npm run dev

# Build the Windows application and installers
npm run build

# Build without bundling installers, useful for CI-style validation
npm run build -- --no-bundle

# Check the Rust crate directly
cargo check --manifest-path src-tauri/Cargo.toml
```

`npm run check` runs `node --check` against the frontend JavaScript files listed in `package.json`. Automated tests exist in all three code areas: run `npm test` for Node.js tests, `cargo test --manifest-path src-tauri/Cargo.toml` for Rust tests, and `dotnet test src-tauri/widget-provider.Tests/WhatNWhenWidgetProvider.Tests.csproj --configuration Release` for the .NET widget-provider tests.

## Project Structure

```text
├── web/
│   ├── index.html             - Main widget UI and resize handles
│   ├── renderer.js            - Main window controls and manual resizing
│   ├── options.html           - Settings window UI
│   ├── optionsRenderer.js     - Settings commands and debug diagnostics
│   └── assets/                - Application icons and frontend assets
├── src-tauri/
│   ├── src/lib.rs             - Tauri setup, native commands, windows, and persistence
│   ├── widget-provider/      - .NET 8 Windows widget provider included in MSIX
│   ├── src/main.rs            - Windows entry point
│   ├── tauri.conf.json        - Frontend path, window, CSP, and bundle configuration
│   ├── Cargo.toml             - Rust dependencies and Windows bindings
│   └── build.rs               - Tauri build integration
├── .github/workflows/         - CI, semantic-release, and release-asset recovery workflows
├── src-tauri/widget-provider.Tests/ - .NET widget-provider unit tests
├── package.json               - npm scripts and release tooling
└── README.md                  - User and contributor documentation
```

## Implementation Guidance

- The calendar is loaded as a child webview in `src-tauri/src/lib.rs` from `https://calendar.notion.so/`. Keep the parent window and child webview positioned below the custom title bar when changing layout or resize behavior.
- Change the calendar URL in the `WebviewUrl::External` call in `src-tauri/src/lib.rs`.
- Change default window dimensions, decorations, taskbar behavior, minimum size, background color, and bundle settings in `src-tauri/tauri.conf.json`.
- The main window is frameless and uses frontend resize handles that call Tauri's `startResizeDragging`; preserve the stable handle geometry when changing `web/index.html` or `web/renderer.js`.
- Options are serialized in `%APPDATA%\ca.willryan.notioncalendarwidget\settings.json`. Keep `Settings`, `Options`, and the frontend option names in sync when adding or renaming settings.
- Native desktop double-click detection is Windows-only and uses a low-level mouse hook in `src-tauri/src/lib.rs`. It must distinguish the desktop surface from normal application and Explorer windows.
- New windows opened by the calendar are created as Tauri webview windows. Preserve the existing behavior that mirrors the document title and denies failed popup creation.
- The CSP is intentionally restrictive in `src-tauri/tauri.conf.json`; update it only when a required local resource or external origin is added.
- Keep Rust code formatted with `cargo fmt` and frontend JavaScript compatible with the current Node syntax checker. Avoid adding a frontend framework unless the architecture is intentionally being changed.

## Packaging and Releases

`npm run build` writes release output under `src-tauri/target/release/`, including NSIS and MSI installers under `bundle/` and the portable executable at `src-tauri/target/release/what-n-when.exe`.

The CI workflow runs on pull requests only. Its parallel jobs run Node.js checks and tests, Rust formatting/compilation/tests, .NET widget-provider tests, and a Windows Tauri build plus MSIX packaging. The CI gate requires every job to pass.

The semantic-release workflow runs separately on pushes to `main`; it does not wait for or depend on pull-request CI. It checks out the pushed commit, creates a GitHub release when the Conventional Commit history requires one, and then builds the NSIS/MSI installers, portable executable, and MSIX package. The release validator smoke-tests the portable, NSIS-installed, and MSI-installed applications, and checks the unpacked MSIX identity, publisher, version, and required files. The release workflow uploads the installers and portable executable to GitHub Releases and submits the MSIX to the Microsoft Store.

The manually triggered `.github/workflows/recover-release-assets.yml` workflow rebuilds the NSIS installer, MSI installer, and portable executable from a selected existing release tag, verifies the GitHub release exists, validates the artifacts, and uploads them to that release. It does not create a release or submit the MSIX to the Store; update its configured tag choices when recovery targets change.

Tauri reads the app version from the root `package.json` via `src-tauri/tauri.conf.json`. The release workflow sets the package version to the semantic-release version before building, so the tagged source does not need a version-bump commit.

Use Conventional Commits for changes, for example `fix: ignore Explorer double-clicks` or `feat(options): add a startup setting`. The pre-push hook runs `npm run check && npm run build:debug`.

## Troubleshooting

- If dependencies are missing or stale, run `npm ci`.
- If frontend syntax fails, run `npm run check` and fix the reported file before building.
- If the Rust build fails, run `cargo fmt --check` and `cargo check --manifest-path src-tauri/Cargo.toml`; confirm the MSVC build tools and Rust stable toolchain are installed.
- If the calendar does not load, verify network access and the WebView2 Runtime.
- If window resizing or transparency flickers on Windows, preserve the matching dark background colors in both `tauri.conf.json` and the child webview builder.

## References

- [Tauri 2 documentation](https://v2.tauri.app/)
- [Rust documentation](https://www.rust-lang.org/learn)
- [WebView2 documentation](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

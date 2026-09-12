# Notion Calendar Widget

A desktop widget for Windows that embeds https://calendar.notion.so/ in a frameless Tauri (Rust + WebView2) window.

<img width="1028" height="965" alt="Main UI - Dark Mode" src="https://github.com/user-attachments/assets/4f1f151c-05a9-48e6-9ec5-019de63e2193" />

- Main UI - Dark Mode

<img width="1029" height="967" alt="Main UI - Light Mode" src="https://github.com/user-attachments/assets/27f9a768-d666-4ea6-aee7-9fdbe4f820a1" />

- Main UI - Light Mode

<img width="747" height="606" alt="Settings window" src="https://github.com/user-attachments/assets/89b3ba8c-7062-4555-91e5-73543e2f5d96" />

- Settings window


## Features

- Frameless floating widget with custom title bar
- Refresh, options, and close buttons
- Widget Options window (remember window bounds, launch at Windows startup)
- Optional desktop double-click toggle to hide or show the widget
- Skipped from the taskbar
- Persistent window size and position
- Custom edge and corner resizing

## Requirements

- Windows 10/11
- Node.js 18+
- Rust (stable toolchain) with the MSVC build tools
- [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (preinstalled on most modern Windows systems)

## Install

```powershell
npm install
```

## License

This project is licensed under the [MIT License](LICENSE).

## Run (development)

```powershell
npm run dev
```

## Build

```powershell
npm run build
```

Build output includes NSIS and MSI installers in `src-tauri/target/release/bundle/` and the portable executable at `src-tauri/target/release/notion-calendar-widget.exe`. GitHub releases include a versioned portable executable alongside the installers.

## Development

### Pre-push Hooks

A Husky pre-push hook runs `npm run check` and `npm run build` before every push. This catches syntax errors and build failures locally, preventing broken commits from reaching GitHub.

If you need to bypass the hook (not recommended):
```powershell
git push --no-verify
```

## CI/CD

- CI workflow: `.github/workflows/ci.yml`
	- Runs on push and pull request
	- Installs dependencies with `npm ci`
	- Runs syntax validation (`npm run check`)
	- Builds the Tauri app (`npm run build`) and uploads the NSIS/MSI installers as artifacts
- Commit policy workflow: `.github/workflows/conventional-commits.yml`
	- Runs on pull requests
	- Validates PR title follows Conventional Commits
	- Lints commit messages in the PR range with commitlint
- Release workflow: `.github/workflows/release.yml`
	- Runs after `CI` completes successfully for `main`
	- Uses `semantic-release` to generate version tags and GitHub releases
	- Builds the Windows installers and uploads them as release assets when a new release is published

## Semantic Versioning

Releases are automatically generated from commit messages using [Conventional Commits](https://www.conventionalcommits.org/) and semantic-release.

### Commit Format

Commit messages follow this structure:

```
<type>(<scope>): <subject>

<body>
```

### Release Mapping

The commit type determines the version bump:

- `feat:` → minor version bump (e.g., 1.0.0 → 1.1.0)
- `fix:` → patch version bump (e.g., 1.0.0 → 1.0.1)
- `feat!:` or `BREAKING CHANGE:` → major version bump (e.g., 1.0.0 → 2.0.0)
- `refactor:`, `style:`, `docs:`, `chore:`, etc. → patch version bump
- Other commit types do not trigger a release

### Examples

```
feat: add custom app icon to titlebar

fix: resolve memory leak in BrowserView cleanup

feat(ui): redesign settings panel
Fixes #42

fix!: change settings storage format
BREAKING CHANGE: old settings.json format no longer compatible
```

### Best Practices

- **Be specific**: Use lowercase, imperative mood ("add", not "adds" or "added")
- **Scope is optional**: Add `(scope)` for organized commits (e.g., `fix(release):`)
- **One feature per commit**: Don't mix multiple features or fixes in one commit
- **Write a body for complex changes**: Explain the *why*, not the *what* (the diff shows that)
- **Link issues**: Reference related issues: `Fixes #42` or `Relates to #99`

## Project Files

- `src-tauri/src/lib.rs` - Tauri app setup, window/webview lifecycle, and commands
- `src-tauri/src/main.rs` - Entry point
- `src-tauri/tauri.conf.json` - Window, bundle, and CSP configuration
- `web/index.html` - Widget UI, title bar, and resize handles
- `web/renderer.js` - Button behavior and manual resize interactions
- `web/options.html` / `web/optionsRenderer.js` - Widget Options window
- `package.json` - npm scripts and tooling dependencies

## Settings

Window bounds and options are stored in:

`%APPDATA%\ca.willryan.notioncalendarwidget\settings.json`

## Notes

- Internet access is required for Notion Calendar.
- The calendar is loaded as a child webview pointed at `https://calendar.notion.so/`; sign-in state is persisted by WebView2 between launches.

# Notion Calendar Widget

A desktop widget for Windows that embeds https://calendar.notion.so/ in an Electron frameless window.

<img width="852" height="917" alt="image" src="https://github.com/user-attachments/assets/70895e72-4a71-4e9c-b11f-0b555e9d2954" />


## Features

- Frameless floating widget with custom title bar
- Refresh and close buttons
- Always on top and hidden from taskbar
- Persistent Notion session via webview partition
- Persistent window size and position
- Custom edge and corner resizing

## Requirements

- Windows 10/11
- Node.js 18+

## Install

```powershell
npm install
```

## Run

```powershell
npm start
```

## Build

```powershell
npm run build:win
```

Build output is written to `release/`.

## Development

### Pre-push Hooks

A Husky pre-push hook runs `npm run check` and `npm run build:win` before every push. This catches syntax errors and build failures locally, preventing broken commits from reaching GitHub.

If you need to bypass the hook (not recommended):
```powershell
git push --no-verify
```

## CI/CD

- CI workflow: `.github/workflows/ci.yml`
	- Runs on push and pull request
	- Installs dependencies with `npm ci`
	- Runs syntax validation (`npm run check`)
	- Builds Windows artifact (`npm run build:win`)
- Commit policy workflow: `.github/workflows/conventional-commits.yml`
	- Runs on pull requests
	- Validates PR title follows Conventional Commits
	- Lints commit messages in the PR range with commitlint
- Release workflow: `.github/workflows/release.yml`
	- Runs after `CI` completes successfully for `main`
	- Uses `semantic-release` to generate version tags and GitHub releases
	- Builds the Windows executable and uploads it as a release asset when a new release is published

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

- `main.js` - Electron main process and window lifecycle
- `preload.js` - Safe IPC bridge for renderer
- `index.html` - Widget UI, title bar, and resize handles
- `renderer.js` - Button behavior and manual resize interactions
- `package.json` - Scripts and dependencies

## Settings

Window bounds are stored in:

`%APPDATA%\notion-calendar-widget\settings.json`

## Notes

- First launch may take longer while Electron finishes setup.
- Internet access is required for Notion Calendar.

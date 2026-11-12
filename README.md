# What-N-When for Notion Calendar

A desktop widget for Windows that embeds [Notion Calendar](https://calendar.notion.so) in a frameless Tauri (Rust + WebView2) window.

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
- Windows 11 widget with today's date that opens the desktop app (MSIX installation)
- Skipped from the taskbar
- Persistent window size and position
- Custom edge and corner resizing

## Requirements

- Windows 10/11
- Node.js 18+
- Rust (stable toolchain) with the MSVC build tools
- [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (preinstalled on most modern Windows systems)
- .NET 8 SDK and Windows SDK (only needed to build the MSIX lock-screen widget package)

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

Build output includes NSIS and MSI installers in `src-tauri/target/release/bundle/` and the portable executable at `src-tauri/target/release/what-n-when.exe`. GitHub releases include a versioned portable executable alongside the installers.

To build the MSIX package with the lock-screen widget, run this after `npm run build` in PowerShell:

```powershell
.\.github\scripts\package-msix.ps1
```

The package is unsigned. To sign it, install a trusted code-signing certificate with its private key in the current user's `Personal` certificate store (`Cert:\CurrentUser\My`). Its subject must match the `Publisher` value in `.github/scripts/AppxManifest.template.xml`. Find its thumbprint and save it in the ignored, machine-local `.env.local` file:

```powershell
Get-ChildItem Cert:\CurrentUser\My | Select-Object Subject, Thumbprint
Set-Content .env.local 'WNW_MSIX_CERTIFICATE_THUMBPRINT=<certificate-thumbprint>'
.\.github\scripts\sign-msix.ps1
```

The script reads `.env.local` by default; alternatively, pass `-CertificateThumbprint` or set the `WNW_MSIX_CERTIFICATE_THUMBPRINT` environment variable. To sign a specific package, pass `-PackagePath`. The script timestamps the signature and verifies it after signing. The certificate must also be trusted by Windows on the machine where the MSIX is installed.

The Windows widget is registered only by the MSIX package. It is not available in the NSIS/MSI installers or the portable executable. All sizes share a date summary with the day and month on the left and the weekday separated on the right. Medium and large layouts add a full-width, six-week calendar panel; today's cell is marked with a round image colored from the current Windows accent color. Clicking anywhere on the widget card opens the packaged desktop app.

## Development

### Git Hooks

Husky installs two hooks when dependencies are installed:

- **Pre-commit:** runs `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` to ensure Rust code is formatted.
- **Pre-push:** runs `npm run check` and `npm run build:debug` to catch JavaScript syntax errors and debug-build failures before pushing.

The pre-commit hook requires Rust stable and the `rustfmt` component. See the Rust setup instructions in the repository's developer guide.

To bypass a hook (not recommended), use `git commit --no-verify` or `git push --no-verify` for the corresponding operation.

## CI/CD

The CI workflow (`.github/workflows/ci.yml`) runs on pull requests and pushes to `main`. Independent jobs run in parallel:

- **Node.js:** installs dependencies, runs `npm run check` and `npm test`, and validates PR titles and commit messages on pull requests.
- **Rust:** checks formatting, compiles with `cargo check`, and runs `cargo test`.
- **.NET:** runs the Windows widget-provider test project.
- **Windows build:** validates the Tauri app build on pull requests and creates the application installers on pushes to `main`.
- **CI gate:** fails unless every required job succeeds.

Superseded runs in the same workflow/ref are cancelled automatically.

The release workflow (`.github/workflows/release.yml`) runs after successful CI on `main`. It uses semantic-release to create version tags and GitHub releases, builds the Windows installers and portable executable, and publishes the MSIX package to the Microsoft Store when a release is created.

Pull request titles and commit messages are checked against Conventional Commits by the CI workflow.
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

`%APPDATA%\ca.willryan.whatnwhen\settings.json`

## Notes

- Internet access is required for Notion Calendar.
- The calendar is loaded as a child webview pointed at `https://calendar.notion.so/`; sign-in state is persisted by WebView2 between launches.

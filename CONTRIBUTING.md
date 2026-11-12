# Development Setup

## Prerequisites

- Windows 10 or 11
- Node.js 22 (use the same major version as CI)
- Rust stable with the MSVC toolchain and the `rustfmt` component
- Visual Studio Build Tools with the C++ build tools
- WebView2 Runtime
- .NET 8 SDK and Windows SDK only when working on the MSIX widget provider

Install the Rust formatting component if it is not already available:

```powershell
rustup component add rustfmt
```

## Install dependencies

From the repository root:

```powershell
npm ci
```

## Run locally

Start the development app:

```powershell
npm run dev
```

Run the JavaScript checks and tests:

```powershell
npm run check
npm test
```

Run the Rust checks:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

To apply Rust formatting instead of checking it:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --all
```

## Git hooks

Husky installs the repository hooks when npm dependencies are installed:

- **Pre-commit:** runs `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- **Pre-push:** runs `npm run check` and `npm run build:debug`.

The pre-commit hook requires Rust and rustfmt to be installed. Run the corresponding commands above manually if hooks are unavailable.

## Pull requests and CI

The CI workflow runs on pull requests. It runs the Node.js syntax checks and unit tests, Rust formatting/compilation/unit tests, .NET widget-provider tests, and a Windows application build/MSIX packaging job as separate jobs. All jobs must pass the CI gate before a pull request is considered green.

The semantic-release workflow is separate: it runs on pushes to `main` and is not configured to wait for the pull-request CI workflow. When semantic-release publishes a new version, the release workflow builds and validates the Windows release artifacts, uploads the NSIS/MSI installers and portable executable to the GitHub release, and submits the MSIX package to the Microsoft Store.

For a missing GitHub release asset, the manually triggered **Recover Release Assets** workflow can rebuild and upload the NSIS installer, MSI installer, and portable executable to an existing release. It verifies that the release already exists before uploading. Its selectable tag list is maintained in `.github/workflows/recover-release-assets.yml`.

See [README.md](README.md) for application features, packaging, release validation, recovery, and signing instructions.

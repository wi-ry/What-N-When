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

- **Pre-commit:** checks Rust formatting with `cargo fmt --check`.
- **Pre-push:** runs `npm run check` and `npm run build:debug`.

The pre-commit hook requires Rust and rustfmt to be installed. Run the corresponding commands above manually if hooks are unavailable.

## Pull requests and CI

GitHub Actions runs the Node.js checks and tests, Rust formatting/compilation/tests, .NET widget-provider tests, and a Windows application build as separate jobs. All jobs must pass the CI gate before a change is considered green.

See [README.md](README.md) for application features, packaging, release, and signing instructions.

---
name: msix-release-packaging
description: 'Use when changing or validating What-N-When MSIX, NSIS/MSI, portable builds, Windows widget packaging, signing, Store/release workflows, versions, or package identity.'
---

# Windows Packaging and Releases

Use this workflow to keep What-N-When build and release artifacts consistent.

## Procedure

1. Inspect `README.md`, `src-tauri/tauri.conf.json`, `.github/workflows/`, and `.github/scripts/` to identify the affected artifact: Tauri installers, portable executable, MSIX, optional widget provider, or signed package.
2. Preserve the build boundaries: `npm run build` creates Tauri artifacts; `.github/scripts/package-msix.ps1` assembles the MSIX and widget provider; signing is a separate step using a locally configured certificate.
3. Keep package identity, publisher, version, architecture, executable paths, widget provider registration, and activation IDs consistent between manifests, scripts, and application code. Release version synchronization must remain compatible with semantic-release.
4. Never commit certificate material, private keys, thumbprints intended to remain local, or Store credentials. Do not ask the user to paste secrets into chat or logs; use the documented machine-local configuration or CI secret mechanism.
5. Do not weaken signature verification or trust checks to make packaging pass. Report missing SDKs, identity configuration, or certificates explicitly.
6. Run `npm run check`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `npm run build` for application packaging changes. Build the .NET provider separately when it changes. Run package assembly or signing only when the required local Windows SDK and configuration are available.

## Review checklist

- Portable and installer artifacts still use the intended paths and runtime assumptions.
- MSIX includes the widget provider only in the package configuration that supports it.
- Unsigned and signed package flows remain distinct and verifiable.
- Release metadata and versions are not hard-coded inconsistently.
- Validation reports distinguish a successful build from successful installation, signing, or Store submission.

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

## Release artifact validation and recovery

The semantic-release workflow invokes `.github/scripts/validate-release.ps1` before uploading release assets. Keep its checks aligned with the package contents and installation behavior:

- The portable executable is checked for presence, non-empty size, compatible file version metadata when available, and successful startup.
- The NSIS installer is installed silently; the installed application is launched, then the installer is uninstalled.
- The MSI installer is installed silently; the installed executable is located through uninstall registry metadata and standard Program Files locations, launched, and then uninstalled.
- The MSIX is unpacked with `makeappx.exe`. Its manifest identity, publisher, and version, plus required application/provider executables and image assets, are checked. This is not an MSIX installation or launch test.

The manual `.github/workflows/recover-release-assets.yml` workflow is for restoring missing assets on an existing GitHub release. It verifies the release exists, checks out the selected release tag, uses the current validator from `main`, and uploads the NSIS installer, MSI installer, and portable executable. It does not create a release or publish the MSIX to the Microsoft Store. Its selectable tag list is intentionally explicit and must be maintained as recovery targets change.

## Review checklist

- Portable and installer artifacts still use the intended paths and runtime assumptions.
- MSIX includes the widget provider only in the package configuration that supports it.
- Unsigned and signed package flows remain distinct and verifiable.
- Release metadata and versions are not hard-coded inconsistently.
- Validation reports distinguish a successful build from successful installation, signing, or Store submission.

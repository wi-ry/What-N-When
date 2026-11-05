---
name: windows-widget-provider
description: 'Use when changing or troubleshooting the Windows widget provider in src-tauri/widget-provider, including Windows App SDK, widget templates, COM activation, widget actions, refresh, or app activation.'
---

# Windows Widget Provider

Use this workflow for the Windows App SDK widget provider packaged with What-N-When.

## Procedure

1. Inspect `src-tauri/widget-provider/WhatNWhenWidgetProvider.csproj`, the provider implementation, and `.github/scripts` packaging configuration. This provider is a separate .NET 8 Windows executable, not part of the Tauri Rust crate.
2. Preserve the Windows widget host contract: COM class/interface IDs, provider activation arguments, manifest declarations, app identity, and widget definition ID must remain consistent across the provider and MSIX packaging.
3. Keep widget template JSON valid for the Windows widget schema and maintain separate, intentional layouts for supported sizes. Use invariant formatting for dates and values serialized into templates.
4. Ensure widget actions handle unknown verbs safely and that opening the desktop app follows the existing activation event / packaged-app launch behavior. Keep failures diagnosable without crashing the widget host.
5. Preserve lifecycle behavior for creation, deletion, refresh, and timers. Avoid unbounded background work or retaining deleted widget state.
6. Build the provider with `dotnet build src-tauri/widget-provider/WhatNWhenWidgetProvider.csproj -c Release`. Validate installation, rendering, refresh, and activation on Windows; a successful .NET build does not verify widget-host integration.

## Review checklist

- Provider IDs, entry point, executable path, and manifest declarations match.
- Template JSON and click-action identifiers match the expected host contract.
- Timers and active-widget state are safe across create, delete, and process shutdown.
- Widget activation handles both an already-running app and a cold launch.
- Windows widget scenarios are reported separately from automated build validation.

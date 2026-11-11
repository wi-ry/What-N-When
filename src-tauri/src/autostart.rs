use super::*;

// ---------------------------------------------------------------------------
// Autostart
// ---------------------------------------------------------------------------

/// Dev builds never register themselves to run at sign-in (the entry would point at
/// `target\debug`), so the option is greyed out in the UI and refused here.
pub(super) const AUTOSTART_AVAILABLE: bool = !cfg!(debug_assertions);

/// Removes the Run-key entry left behind by the old Electron version of the app.
#[cfg(windows)]
pub(super) fn remove_legacy_electron_autostart(current_name: &str) {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;

    const LEGACY_ELECTRON_AUTOSTART_NAME: &str = "ca.willryan.notioncalendarwidget";

    // Never delete the entry this app writes for itself.
    if current_name == LEGACY_ELECTRON_AUTOSTART_NAME {
        return;
    }
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(run_key) = hkcu.open_subkey_with_flags(
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
        KEY_SET_VALUE,
    ) {
        let _ = run_key.delete_value(LEGACY_ELECTRON_AUTOSTART_NAME);
    }
}

#[cfg(not(windows))]
pub(super) fn remove_legacy_electron_autostart(_current_name: &str) {}

#[cfg(windows)]
mod platform {
    //! Writes the per-user Run key directly instead of going through the plugin
    //! (which registers a canonicalized `\\?\` path on Windows).

    use std::io;
    use tauri::AppHandle;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
    use winreg::RegKey;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    /// Task Manager's Enable/Disable toggle lives here, separately from the Run key.
    const APPROVED_KEY: &str =
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

    fn value_name(app: &AppHandle) -> &str {
        &app.package_info().name
    }

    fn ignore_not_found(result: io::Result<()>) -> io::Result<()> {
        match result {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }

    fn read_command(name: &str) -> io::Result<Option<String>> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run = match hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ) {
            Ok(key) => key,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        match run.get_value::<String, _>(name) {
            Ok(command) => Ok(Some(command)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn remove_entry(name: &str) -> io::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(run) = hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE) {
            ignore_not_found(run.delete_value(name))?;
        }
        if let Ok(approved) = hkcu.open_subkey_with_flags(APPROVED_KEY, KEY_SET_VALUE) {
            ignore_not_found(approved.delete_value(name))?;
        }
        Ok(())
    }

    fn is_enabled_io(name: &str) -> io::Result<bool> {
        if read_command(name)?.is_none() {
            return Ok(false);
        }
        // An odd first byte means the user disabled it in Task Manager > Startup.
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let disabled = hkcu
            .open_subkey_with_flags(APPROVED_KEY, KEY_READ)
            .ok()
            .and_then(|key| key.get_raw_value(name).ok())
            .and_then(|value| value.bytes.first().copied())
            .is_some_and(|byte| byte & 1 == 1);
        Ok(!disabled)
    }

    fn enable_io(name: &str) -> io::Result<()> {
        let exe = std::env::current_exe()?;
        let exe = exe.to_string_lossy();
        let exe = exe.strip_prefix(r"\\?\").unwrap_or(&exe);
        let command = format!("\"{exe}\"");

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (run, _) = hkcu.create_subkey(RUN_KEY)?;
        run.set_value(name, &command)?;

        // Clear a "disabled in Task Manager" marker from an earlier session.
        if let Ok(approved) = hkcu.open_subkey_with_flags(APPROVED_KEY, KEY_SET_VALUE) {
            ignore_not_found(approved.delete_value(name))?;
        }
        Ok(())
    }

    pub fn is_enabled(app: &AppHandle) -> Result<bool, String> {
        is_enabled_io(value_name(app))
            .map_err(|error| format!("Could not read launch-at-sign-in status: {error}"))
    }

    pub fn set(app: &AppHandle, enabled: bool) -> Result<(), String> {
        let name = value_name(app);
        let result = if enabled { enable_io(name) } else { remove_entry(name) };
        result.map_err(|error| {
            format!("Could not set launch-at-sign-in to {enabled}: {error}")
        })
    }

    fn normalize(text: &str) -> String {
        text.replace('/', "\\").replace(r"\\?\", "").to_lowercase()
    }

    fn is_dev_command(command: &str) -> bool {
        let command = normalize(command);
        if command.contains(r"\target\debug\") {
            return true;
        }
        // Covers a custom CARGO_TARGET_DIR when this *is* the dev build.
        cfg!(debug_assertions)
            && std::env::current_exe()
                .is_ok_and(|exe| command.contains(&normalize(&exe.to_string_lossy())))
    }

    /// Removes a Run entry that points at a dev build (one registered before dev
    /// builds were blocked from doing so). A release install's entry is left alone.
    pub fn remove_stale_dev_entry(app: &AppHandle) {
        let name = value_name(app);
        match read_command(name) {
            Ok(Some(command)) if is_dev_command(&command) => {
                match remove_entry(name) {
                    Ok(()) => eprintln!("autostart: removed stale dev-build entry: {command}"),
                    Err(error) => eprintln!("autostart: could not remove dev entry: {error}"),
                }
            }
            Ok(_) => {}
            Err(error) => eprintln!("autostart: could not inspect Run entry: {error}"),
        }
    }
}

#[cfg(not(windows))]
mod autostart {
    use tauri::AppHandle;
    use tauri_plugin_autostart::ManagerExt;

    pub fn is_enabled(app: &AppHandle) -> Result<bool, String> {
        app.autolaunch()
            .is_enabled()
            .map_err(|error| format!("Could not read launch-at-sign-in status: {error}"))
    }

    pub fn set(app: &AppHandle, enabled: bool) -> Result<(), String> {
        let autolaunch = app.autolaunch();
        let result = if enabled { autolaunch.enable() } else { autolaunch.disable() };
        result.map_err(|error| format!("Could not set launch-at-sign-in to {enabled}: {error}"))
    }

    pub fn remove_stale_dev_entry(_app: &AppHandle) {}
}

pub(super) fn autostart_enabled(app: &AppHandle) -> Result<bool, String> {
    if !AUTOSTART_AVAILABLE {
        return Ok(false);
    }
    platform::is_enabled(app)
}

pub(super) fn set_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    if !AUTOSTART_AVAILABLE {
        // Disabling is a no-op so a dev "reset" can't touch a real install's entry.
        return if enabled {
            Err("Open at sign-in isn't available in dev builds".to_owned())
        } else {
            Ok(())
        };
    }
    platform::set(app, enabled).inspect_err(|message| eprintln!("autostart: {message}"))
}

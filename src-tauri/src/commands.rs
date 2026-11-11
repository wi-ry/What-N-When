use super::*;

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_options(app: AppHandle, store: State<'_, SettingsStore>) -> Result<OptionsView, String> {
    options_view(&app, store.options())
}

#[tauri::command]
fn is_debug_build() -> bool {
    cfg!(debug_assertions)
}

#[tauri::command]
fn get_desktop_listener_debug() -> desktop::DesktopListenerDebug {
    desktop::debug_snapshot()
}

#[tauri::command]
fn test_desktop_toggle(app: AppHandle) -> Result<(), String> {
    desktop::test_toggle(&app)
}

#[tauri::command]
fn save_options(
    app: AppHandle,
    store: State<'_, SettingsStore>,
    patch: OptionsPatch,
) -> Result<OptionsView, String> {
    if !AUTOSTART_AVAILABLE && patch.open_at_login.is_some() {
        return Err("Open at sign-in isn't available in dev builds".to_owned());
    }
    let previous_autostart = patch
        .open_at_login
        .map(|_| autostart_enabled(&app))
        .transpose()?;
    let open_at_login = match patch.open_at_login {
        Some(enabled) => {
            set_autostart(&app, enabled)?;
            enabled
        }
        None => autostart_enabled(&app)?,
    };

    let options = match store.update(|settings| {
        if let Some(v) = patch.remember_window_bounds {
            settings.options.remember_window_bounds = v;
            if !v {
                settings.window_bounds = None;
            }
        }
        if let Some(v) = patch.toggle_on_desktop_double_click {
            settings.options.toggle_on_desktop_double_click = v;
        }
        if let Some(v) = patch.desktop_toggle_action {
            settings.options.desktop_toggle_action = v;
        }
        if let Some(v) = patch.transparency_level {
            settings.options.transparency_level = v.min(MAX_TRANSPARENCY_LEVEL);
        }
        settings.options.clone()
    }) {
        Ok(options) => options,
        Err(error) => {
            let message = format!("Could not save settings: {error}");
            if let Some(previous) = previous_autostart {
                if let Err(rollback_error) = set_autostart(&app, previous) {
                    return Err(format!(
                        "{message}; could not restore the previous launch-at-sign-in setting: {rollback_error}"
                    ));
                }
            }
            return Err(message);
        }
    };

    // Side effects run after the settings lock is released.
    if let Some(enabled) = patch.toggle_on_desktop_double_click {
        desktop::set_listener_enabled(enabled);
        if !enabled {
            restore_widget(&app);
        }
    }
    if let Some(action) = patch.desktop_toggle_action {
        desktop::set_toggle_action(action);
        if action == DesktopToggleAction::Hide {
            restore_widget(&app);
        }
    }
    if patch.transparency_level.is_some() {
        desktop::apply_transparency_level(&app);
    }

    Ok(options_view_with_autostart(options, open_at_login))
}

#[tauri::command]
fn update_transparency_level(
    app: AppHandle,
    store: State<'_, SettingsStore>,
    transparency_level: u8,
) -> Result<(), String> {
    let level = transparency_level.min(MAX_TRANSPARENCY_LEVEL);
    store
        .update(|settings| settings.options.transparency_level = level)
        .map_err(|error| format!("Could not save transparency setting: {error}"))?;
    desktop::apply_transparency_level(&app);
    Ok(())
}

#[tauri::command]
fn reset_options(app: AppHandle, store: State<'_, SettingsStore>) -> Result<OptionsView, String> {
    let previous_autostart = autostart_enabled(&app)?;
    set_autostart(&app, false)?;

    let options = match store.update(|settings| {
        settings.options = Options::default();
        settings.options.clone()
    }) {
        Ok(options) => options,
        Err(error) => {
            let message = format!("Could not reset settings: {error}");
            if let Err(rollback_error) = set_autostart(&app, previous_autostart) {
                return Err(format!(
                    "{message}; could not restore the previous launch-at-sign-in setting: {rollback_error}"
                ));
            }
            return Err(message);
        }
    };

    desktop::set_listener_enabled(false);
    desktop::set_toggle_action(DesktopToggleAction::default());
    restore_widget(&app);
    Ok(options_view_with_autostart(options, false))
}

#[tauri::command]
async fn close_window(window: tauri::Window) {
    if let Err(error) = window.close() {
        eprintln!("could not close window: {error}");
    }
}

#[tauri::command]
fn refresh_calendar(app: AppHandle) {
    if let Some(webview) = app.get_webview(CALENDAR_LABEL) {
        if let Err(error) = webview.eval("location.reload()") {
            eprintln!("could not reload calendar: {error}");
        }
    }
}

// Async on purpose: creating windows from a sync command can deadlock on Windows.
#[tauri::command]
async fn open_options_window(app: AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(OPTIONS_LABEL) {
        return existing.set_focus().map_err(|e| e.to_string());
    }

    let height = if cfg!(debug_assertions) { 720.0 } else { 600.0 };

    let mut builder =
        WebviewWindowBuilder::new(&app, OPTIONS_LABEL, WebviewUrl::App("options.html".into()))
            .title("Widget Options")
            .inner_size(560.0, height)
            .resizable(false)
            .minimizable(false)
            .maximizable(false);

    if let Some(main_window) = app.get_window(MAIN_LABEL) {
        if let (Ok(position), Ok(scale)) =
            (main_window.outer_position(), main_window.scale_factor())
        {
            builder = builder.position(
                f64::from(position.x) / scale + 32.0,
                f64::from(position.y) / scale + 32.0,
            );
        }
    }

    builder.build().map(|_| ()).map_err(|e| e.to_string())
}

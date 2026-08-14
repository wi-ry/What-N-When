use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindowBuilder,
};
use tauri_plugin_autostart::ManagerExt;

const TITLEBAR_HEIGHT: f64 = 40.0;
const LEGACY_ELECTRON_AUTOSTART_NAME: &str = "ca.willryan.notioncalendarwidget";
static POPUP_WINDOW_ID: AtomicUsize = AtomicUsize::new(0);

#[cfg(windows)]
fn remove_legacy_electron_autostart() {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(run_key) = hkcu.open_subkey_with_flags(
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
        KEY_SET_VALUE,
    ) {
        let _ = run_key.delete_value(LEGACY_ELECTRON_AUTOSTART_NAME);
    }
}

#[cfg(not(windows))]
fn remove_legacy_electron_autostart() {}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WindowBounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Options {
    remember_window_bounds: bool,
    open_at_login: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            remember_window_bounds: true,
            open_at_login: false,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Settings {
    options: Options,
    window_bounds: Option<WindowBounds>,
}

fn settings_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("failed to resolve app data dir")
        .join("settings.json")
}

fn load_settings(app: &AppHandle) -> Settings {
    let path = settings_path(app);
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings(app: &AppHandle, settings: &Settings) {
    let path = settings_path(app);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, json);
    }
}

#[tauri::command]
fn get_options(app: AppHandle) -> Options {
    let mut settings = load_settings(&app);
    // Always reflect the actual OS autostart state.
    settings.options.open_at_login = app.autolaunch().is_enabled().unwrap_or(false);
    settings.options
}

#[tauri::command]
fn save_options(
    app: AppHandle,
    remember_window_bounds: Option<bool>,
    open_at_login: Option<bool>,
) -> Options {
    let mut settings = load_settings(&app);

    if let Some(v) = remember_window_bounds {
        settings.options.remember_window_bounds = v;
        if !v {
            settings.window_bounds = None;
        }
    }
    if let Some(v) = open_at_login {
        settings.options.open_at_login = v;
        let autolaunch = app.autolaunch();
        let _ = if v {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
    }

    save_settings(&app, &settings);
    settings.options
}

#[tauri::command]
fn reset_options(app: AppHandle) -> Options {
    let mut settings = load_settings(&app);
    settings.options = Options::default();
    let _ = app.autolaunch().disable();
    save_settings(&app, &settings);
    settings.options
}

#[tauri::command]
async fn close_window(window: tauri::Window) {
    let _ = window.close();
}

#[tauri::command]
fn refresh_calendar(app: AppHandle) {
    if let Some(webview) = app.get_webview("calendar") {
        let _ = webview.eval("location.reload()");
    }
}

#[tauri::command]
async fn open_options_window(app: AppHandle) {
    if let Some(existing) = app.get_webview_window("options") {
        let _ = existing.set_focus();
        return;
    }

    let result = (|| -> tauri::Result<()> {
        let mut builder =
            WebviewWindowBuilder::new(&app, "options", WebviewUrl::App("options.html".into()))
                .title("Widget Options")
                .inner_size(420.0, 360.0)
                .resizable(false)
                .minimizable(false)
                .maximizable(false)
                .fullscreen(false);

        if let Some(main_win) = app.get_webview_window("main") {
            builder = builder.parent(&main_win)?;
        }

        builder.build()?;
        Ok(())
    })();

    let _ = result;
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let handle = app.handle().clone();
            let settings = load_settings(&handle);

            remove_legacy_electron_autostart();

            let autolaunch = handle.autolaunch();
            let _ = if settings.options.open_at_login {
                autolaunch.enable()
            } else {
                autolaunch.disable()
            };

            let main_window = app.get_webview_window("main").unwrap();
            let main_base_window = app.get_window("main").unwrap();

            // Restore saved window position and size (stored as physical pixels).
            if settings.options.remember_window_bounds {
                if let Some(bounds) = &settings.window_bounds {
                    let _ = main_window.set_position(PhysicalPosition::new(bounds.x, bounds.y));
                    let _ = main_window.set_size(PhysicalSize::new(bounds.width, bounds.height));
                }
            }

            // Add the calendar as a child webview occupying the space below the titlebar.
            let phys = main_window.inner_size()?;
            let scale = main_window.scale_factor()?;
            let lw = phys.width as f64 / scale;
            let lh = phys.height as f64 / scale;

            main_base_window.add_child(
                tauri::webview::WebviewBuilder::new(
                    "calendar",
                    WebviewUrl::External("https://calendar.notion.so/".parse().unwrap()),
                )
                .on_new_window({
                    let handle = handle.clone();
                    move |_url, features| {
                        let label = format!(
                            "notion-calendar-popup-{}",
                            POPUP_WINDOW_ID.fetch_add(1, Ordering::Relaxed)
                        );
                        let result = WebviewWindowBuilder::new(
                            &handle,
                            &label,
                            WebviewUrl::External("about:blank".parse().unwrap()),
                        )
                        .window_features(features)
                        .title("Notion Calendar Sign In")
                        .on_document_title_changed(|window, title| {
                            let _ = window.set_title(&title);
                        })
                        .build();

                        match result {
                            Ok(window) => tauri::webview::NewWindowResponse::Create { window },
                            Err(_) => tauri::webview::NewWindowResponse::Deny,
                        }
                    }
                })
                .background_color(tauri::webview::Color(0x1a, 0x1a, 0x1a, 0xff)),
                LogicalPosition::new(0.0, TITLEBAR_HEIGHT),
                LogicalSize::new(lw, lh - TITLEBAR_HEIGHT),
            )?;

            // Update calendar webview bounds on resize; save bounds on close.
            main_window.on_window_event({
                let handle = handle.clone();
                let base_window = main_base_window.clone();
                move |event| match event {
                    tauri::WindowEvent::Resized(phys_size) => {
                        if let Some(cal) = handle.get_webview("calendar") {
                            let scale = base_window.scale_factor().unwrap_or(1.0);
                            let lw = phys_size.width as f64 / scale;
                            let lh = phys_size.height as f64 / scale;
                            let _ = cal.set_position(tauri::Position::Logical(
                                LogicalPosition::new(0.0, TITLEBAR_HEIGHT),
                            ));
                            let _ = cal.set_size(tauri::Size::Logical(LogicalSize::new(
                                lw,
                                (lh - TITLEBAR_HEIGHT).max(0.0),
                            )));
                        }
                    }
                    tauri::WindowEvent::CloseRequested { .. } => {
                        let s = load_settings(&handle);
                        if s.options.remember_window_bounds {
                            // set_position uses outer coords but set_size uses inner coords, so match those on save.
                            if let (Ok(pos), Ok(size)) =
                                (base_window.outer_position(), base_window.inner_size())
                            {
                                let mut s2 = load_settings(&handle);
                                s2.window_bounds = Some(WindowBounds {
                                    x: pos.x,
                                    y: pos.y,
                                    width: size.width,
                                    height: size.height,
                                });
                                save_settings(&handle, &s2);
                            }
                        }
                    }
                    _ => {}
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_options,
            save_options,
            reset_options,
            close_window,
            refresh_calendar,
            open_options_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

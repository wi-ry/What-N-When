//! What-N-When: a Notion Calendar desktop widget (Tauri 2).
//!
//! Module layout:
//! - navigation: URL routing, popup handling, and webview diagnostics
//! - settings: persisted options and window bounds
//! - autostart: platform-specific launch-at-sign-in integration
//! - commands: Tauri command handlers
//! - windows: app setup, webview lifecycle, and window layout
//! - desktop: Windows desktop toggle, effects, mouse hook, and hit-testing

mod autostart;
mod commands;
mod desktop;
mod navigation;
mod settings;
mod windows;

use autostart::*;
use commands::*;
use navigation::*;
use settings::*;

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use tauri::webview::{Color, NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, State, Url,
    WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_opener::OpenerExt;

const MAIN_LABEL: &str = "main";
const CALENDAR_LABEL: &str = "calendar";
const OPTIONS_LABEL: &str = "options";
const CALENDAR_URL: &str = "https://calendar.notion.so/";
const TITLEBAR_HEIGHT: f64 = 40.0;
const MAX_TRANSPARENCY_LEVEL: u8 = 230;
const MIN_VISIBLE_PX: i64 = 100;

static POPUP_WINDOW_ID: AtomicUsize = AtomicUsize::new(0);
static WIDGET_ACTIVATION_FOREGROUND: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
mod widget_activation {
    use super::{desktop, WIDGET_ACTIVATION_FOREGROUND};
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::sync::atomic::Ordering;
    use tauri::AppHandle;
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_FAILED, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject, INFINITE};

    pub const EVENT_NAME: &str = "Local\\WhatNWhen.DesktopActivation.v1";

    pub fn listen(app: AppHandle, window: tauri::Window) -> io::Result<()> {
        let name: Vec<u16> = std::ffi::OsStr::new(EVENT_NAME)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let event = unsafe { CreateEventW(std::ptr::null(), 0, 0, name.as_ptr()) };
        if event.is_null() {
            return Err(io::Error::last_os_error());
        }
        let event_handle = event as usize;
        if let Err(error) = std::thread::Builder::new()
            .name("widget-activation-listener".into())
            .spawn(move || {
                let event = event_handle as windows_sys::Win32::Foundation::HANDLE;
                loop {
                    let result = unsafe { WaitForSingleObject(event, INFINITE) };
                    if result != WAIT_OBJECT_0 {
                        if result == WAIT_FAILED {
                            eprintln!(
                                "widget activation listener: wait failed: {}",
                                io::Error::last_os_error()
                            );
                        } else {
                            eprintln!(
                                "widget activation listener: unexpected wait result {result}"
                            );
                        }
                        break;
                    }
                    activate_main_window(&app, &window);
                }
                unsafe {
                    CloseHandle(event);
                }
            })
        {
            unsafe {
                CloseHandle(event);
            }
            return Err(error);
        }

        Ok(())
    }

    fn activate_main_window(app: &AppHandle, window: &tauri::Window) {
        if let Err(error) = window.set_always_on_bottom(false) {
            eprintln!("widget activation: could not raise main window: {error}");
            return;
        }
        if let Err(error) = window.set_always_on_top(true) {
            eprintln!("widget activation: could not bring main window to front: {error}");
            return;
        }
        WIDGET_ACTIVATION_FOREGROUND.store(true, Ordering::Release);

        if window.is_minimized().unwrap_or(false) {
            if let Err(error) = window.unminimize() {
                eprintln!("widget activation: could not unminimize main window: {error}");
            }
        }
        if let Err(error) = desktop::restore_with_window(app, window) {
            eprintln!("widget activation: could not restore main window: {error}");
        }
        if let Err(error) = window.set_focus() {
            eprintln!("widget activation: could not focus main window: {error}");
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(windows::setup)
        .invoke_handler(tauri::generate_handler![
            get_options,
            is_debug_build,
            get_desktop_listener_debug,
            test_desktop_toggle,
            save_options,
            update_transparency_level,
            reset_options,
            close_window,
            refresh_calendar,
            open_options_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::windows::calendar_bounds;
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_ID: AtomicUsize = AtomicUsize::new(0);

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "what-n-when-tests-{}-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed),
            name
        ))
    }

    #[test]
    fn host_matching_requires_a_domain_boundary() {
        assert!(host_matches("notion.so", "notion.so"));
        assert!(host_matches("calendar.notion.so", "notion.so"));
        assert!(!host_matches("notion.so.attacker.example", "notion.so"));
        assert!(!host_matches("notion-so", "notion.so"));
    }

    #[test]
    fn popup_routing_keeps_notion_and_identity_providers_in_app() {
        for address in [
            "https://calendar.notion.so/",
            "https://notion.so/login",
            "https://accounts.google.com/signin",
            "https://login.microsoftonline.com/",
            "about:blank",
        ] {
            assert!(is_auth_popup(&Url::parse(address).unwrap()), "{address}");
        }

        for address in [
            "https://example.com/",
            "https://notion.so.attacker.example/",
            "file:///tmp/example",
        ] {
            assert!(!is_auth_popup(&Url::parse(address).unwrap()), "{address}");
        }
    }

    #[test]
    fn internal_navigation_only_allows_trusted_hosts_and_embedded_schemes() {
        for address in [
            "https://calendar.notion.so/",
            "https://idmsa.apple.com/",
            "about:blank",
            "blob:https://calendar.notion.so/id",
            "data:text/plain,hello",
        ] {
            assert!(is_internal_url(&Url::parse(address).unwrap()), "{address}");
        }

        for address in [
            "https://example.com/",
            "https://notion.so.attacker.example/",
        ] {
            assert!(!is_internal_url(&Url::parse(address).unwrap()), "{address}");
        }
    }

    #[test]
    fn popup_callback_requires_a_notion_host_and_callback_path() {
        assert!(is_popup_callback(
            &Url::parse("https://notion.so/googlepopupcallback").unwrap()
        ));
        assert!(is_popup_callback(
            &Url::parse("https://www.notion.com/microsoftpopupcallback").unwrap()
        ));
        assert!(!is_popup_callback(
            &Url::parse("https://notion.so/login").unwrap()
        ));
        assert!(!is_popup_callback(
            &Url::parse("https://example.com/googlepopupcallback").unwrap()
        ));
    }

    #[test]
    fn settings_store_loads_defaults_and_persists_updates() {
        let directory = temp_path("settings");
        let path = directory.join("settings.json");
        let store = SettingsStore::load(path.clone());

        assert!(store.options().remember_window_bounds);
        assert!(!store.options().toggle_on_desktop_double_click);
        assert_eq!(store.options().transparency_level, 64);

        store
            .update(|settings| {
                settings.options.remember_window_bounds = false;
                settings.options.transparency_level = 120;
                settings.window_bounds = Some(WindowBounds {
                    x: -100,
                    y: 50,
                    width: 800,
                    height: 600,
                });
            })
            .unwrap();

        let reloaded = SettingsStore::load(path);
        let settings = reloaded.snapshot();
        assert!(!settings.options.remember_window_bounds);
        assert_eq!(settings.options.transparency_level, 120);
        let bounds = settings.window_bounds.unwrap();
        assert_eq!(
            (bounds.x, bounds.y, bounds.width, bounds.height),
            (-100, 50, 800, 600)
        );

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn settings_store_preserves_defaults_for_omitted_options() {
        let directory = temp_path("partial-settings");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        fs::write(
            &path,
            r#"{ "options": { "rememberWindowBounds": false } }"#,
        )
        .unwrap();

        let store = SettingsStore::load(path);
        let options = store.options();

        assert!(!options.remember_window_bounds);
        assert!(!options.toggle_on_desktop_double_click);
        assert_eq!(options.desktop_toggle_action, DesktopToggleAction::Hide);
        assert_eq!(options.transparency_level, 64);

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn settings_store_recovers_from_malformed_json() {
        let directory = temp_path("malformed-settings");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        fs::write(&path, r#"{ "options": "#).unwrap();

        let store = SettingsStore::load(path);
        let options = store.options();

        assert!(options.remember_window_bounds);
        assert!(!options.toggle_on_desktop_double_click);
        assert_eq!(options.desktop_toggle_action, DesktopToggleAction::Hide);
        assert_eq!(options.transparency_level, 64);

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn settings_store_clamps_transparency_when_loading() {
        let directory = temp_path("clamp");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        fs::write(
            &path,
            r#"{"options":{"transparencyLevel":255},"windowBounds":null}"#,
        )
        .unwrap();

        let store = SettingsStore::load(path);
        assert_eq!(store.options().transparency_level, MAX_TRANSPARENCY_LEVEL);

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn failed_settings_write_does_not_change_cached_state() {
        let directory = temp_path("write-failure");
        fs::create_dir_all(&directory).unwrap();
        let blocked_parent = directory.join("not-a-directory");
        fs::write(&blocked_parent, "file").unwrap();
        let store = SettingsStore::load(blocked_parent.join("settings.json"));

        let result = store.update(|settings| {
            settings.options.remember_window_bounds = false;
        });

        assert!(result.is_err());
        assert!(store.options().remember_window_bounds);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn calendar_bounds_convert_physical_size_and_reserve_titlebar() {
        let (position, size) = calendar_bounds(PhysicalSize::new(1600, 1200), 2.0);
        assert_eq!((position.x, position.y), (0.0, TITLEBAR_HEIGHT));
        assert_eq!((size.width, size.height), (800.0, 560.0));
    }

    #[test]
    fn calendar_bounds_never_return_a_negative_height() {
        let (_, size) = calendar_bounds(PhysicalSize::new(100, 20), 1.0);
        assert_eq!(size.height, 0.0);
    }
}

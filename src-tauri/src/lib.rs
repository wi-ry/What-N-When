//! What-N-When: a Notion Calendar desktop widget (Tauri 2).

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

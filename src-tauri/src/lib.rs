use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
#[cfg(windows)]
use std::sync::{Mutex, Once, OnceLock};
#[cfg(windows)]
use std::time::Duration;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindowBuilder,
};
use tauri_plugin_autostart::ManagerExt;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, LRESULT, POINT, WPARAM};
#[cfg(windows)]
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
#[cfg(windows)]
use windows_sys::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
#[cfg(windows)]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows_sys::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
#[cfg(windows)]
use windows_sys::Win32::System::SystemInformation::GetTickCount;
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetClassNameW, GetMessageW, GetWindowLongPtrW, GetParent,
    GetWindowThreadProcessId, SendMessageW, SetLayeredWindowAttributes, SetWindowLongPtrW,
    SetWindowsHookExW, GWL_EXSTYLE, HC_ACTION, LWA_ALPHA, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL,
    WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WS_EX_LAYERED, WindowFromPoint,
};

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn GetDoubleClickTime() -> u32;
}

const TITLEBAR_HEIGHT: f64 = 40.0;
const LEGACY_ELECTRON_AUTOSTART_NAME: &str = "ca.willryan.notioncalendarwidget";
static POPUP_WINDOW_ID: AtomicUsize = AtomicUsize::new(0);
static DESKTOP_DOUBLE_CLICK_ENABLED: AtomicBool = AtomicBool::new(false);
static DESKTOP_TOGGLE_ACTION: AtomicU8 = AtomicU8::new(0);
static WIDGET_IS_TRANSPARENT: AtomicBool = AtomicBool::new(false);
static WIDGET_IS_HIDING: AtomicBool = AtomicBool::new(false);
static TOGGLE_FADE_GENERATION: AtomicUsize = AtomicUsize::new(0);
#[cfg(windows)]
static DESKTOP_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
static DESKTOP_MOUSE_DOWN_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(windows)]
static DESKTOP_SURFACE_CLICK_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(windows)]
static DESKTOP_DOUBLE_CLICK_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(windows)]
static DESKTOP_TOGGLE_DISPATCH_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(windows)]
static DESKTOP_TOGGLE_UI_THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(windows)]
static DESKTOP_MAIN_WINDOW_FOUND_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(windows)]
static DESKTOP_TOGGLE_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(windows)]
static DESKTOP_ICON_CLICK_IGNORED_COUNT: AtomicUsize = AtomicUsize::new(0);

const HIDE_WIDGET_ACTION: u8 = 0;
const TRANSPARENT_WIDGET_ACTION: u8 = 1;
const TOGGLE_FADE_STEPS: u16 = 13;
const TOGGLE_FADE_STEP_DURATION_MS: u64 = 20;

// LVM_HITTEST (LVM_FIRST + 18): asks a SysListView32 control whether a given
// client-coordinate point falls on an item (icon) or on empty list space.
#[cfg(windows)]
const LVM_FIRST: u32 = 0x1000;
#[cfg(windows)]
const LVM_HITTEST: u32 = LVM_FIRST + 18;

#[cfg(windows)]
#[repr(C)]
struct LvHitTestInfo {
    pt: POINT,
    flags: u32,
    i_item: i32,
    i_sub_item: i32,
    i_group: i32,
}

#[cfg(windows)]
#[derive(Clone, Copy)]
struct DesktopClick {
    tick: u32,
}

#[cfg(windows)]
static DESKTOP_DOUBLE_CLICK_APP: OnceLock<AppHandle> = OnceLock::new();
#[cfg(windows)]
static DESKTOP_CLICK: OnceLock<Mutex<Option<DesktopClick>>> = OnceLock::new();
#[cfg(windows)]
static DESKTOP_HOOK_STARTED: Once = Once::new();
#[cfg(windows)]
static DESKTOP_LAST_TARGET: OnceLock<Mutex<String>> = OnceLock::new();
#[cfg(windows)]
static DESKTOP_LAST_TOGGLE_RESULT: OnceLock<Mutex<String>> = OnceLock::new();

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
#[serde(default)]
struct Options {
    remember_window_bounds: bool,
    open_at_login: bool,
    toggle_on_desktop_double_click: bool,
    desktop_toggle_action: DesktopToggleAction,
    transparency_level: u8,
}

#[derive(Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum DesktopToggleAction {
    #[default]
    Hide,
    Transparent,
}

impl DesktopToggleAction {
    fn as_u8(self) -> u8 {
        match self {
            Self::Hide => HIDE_WIDGET_ACTION,
            Self::Transparent => TRANSPARENT_WIDGET_ACTION,
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            remember_window_bounds: true,
            open_at_login: false,
            toggle_on_desktop_double_click: false,
            desktop_toggle_action: DesktopToggleAction::default(),
            transparency_level: 64,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Settings {
    options: Options,
    window_bounds: Option<WindowBounds>,
}

#[cfg(windows)]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopListenerDebug {
    hook_installed: bool,
    listener_enabled: bool,
    mouse_down_count: usize,
    desktop_surface_click_count: usize,
    double_click_count: usize,
    toggle_dispatch_count: usize,
    toggle_ui_thread_count: usize,
    main_window_found_count: usize,
    toggle_count: usize,
    icon_click_ignored_count: usize,
    last_target: String,
    last_toggle_result: String,
}

fn settings_path(app: &AppHandle) -> PathBuf {
    // In debug/dev mode, use a local settings.json for easier testing
    if cfg!(debug_assertions) {
        std::path::PathBuf::from("settings.json")
    } else {
        app.path()
            .app_data_dir()
            .expect("failed to resolve app data dir")
            .join("settings.json")
    }
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

/// Walks up the ancestor chain starting at `window`, classifying whether the click
/// landed on the desktop surface (Progman/WorkerW/SHELLDLL_DefView/SysListView32)
/// as opposed to a File Explorer window. Returns
/// (is_desktop_surface, joined class-name chain for diagnostics, is_progman_window).
///
/// Note: this alone cannot distinguish a click on a desktop *icon* from a click on
/// empty desktop space, since both live inside the same SysListView32 control and
/// report identical class names. See `desktop_click_hits_icon` for that distinction.
#[cfg(windows)]
unsafe fn desktop_window_details(mut window: HWND) -> (bool, String, bool) {
    if window.is_null() {
        return (false, "No window at pointer".into(), false);
    }

    let mut classes = Vec::new();
    let mut is_file_explorer_window = false;
    let mut is_desktop_surface = false;
    let mut is_progman_window = false;
    for _ in 0..8 {
        let mut class_name = [0u16; 32];
        let length = GetClassNameW(window, class_name.as_mut_ptr(), class_name.len() as i32);
        let class_name = String::from_utf16_lossy(&class_name[..length.max(0) as usize]);
        is_file_explorer_window |= matches!(class_name.as_str(), "CabinetWClass" | "ExploreWClass");
        is_desktop_surface |= matches!(
            class_name.as_str(),
            "Progman" | "WorkerW" | "SHELLDLL_DefView" | "SysListView32"
        );
        is_progman_window |= class_name.as_str() == "Progman";
        classes.push(class_name);

        window = GetParent(window);
        if window.is_null() {
            break;
        }
    }

    (
        is_desktop_surface && !is_file_explorer_window && is_progman_window,
        classes.join(" > "),
        is_progman_window,
    )
}

/// True if `screen_point` lands on an actual desktop icon inside the SysListView32
/// control at `hwnd`, rather than on empty desktop space. Used to make sure
/// double-clicking an icon (to open it) never also toggles the widget.
///
/// The desktop's list view is owned by explorer.exe, a different process from ours,
/// so LVM_HITTEST's LPARAM can't just point at a struct in our own memory - explorer
/// would try to read/write that address in *its* address space, which is invalid and
/// crashes it. Instead we allocate a small buffer inside explorer's process, write the
/// hit-test struct there, send the message, read the result back, then free the buffer.
#[cfg(windows)]
unsafe fn desktop_click_hits_icon(hwnd: HWND, screen_point: POINT) -> bool {
    let mut client_point = screen_point;
    if ScreenToClient(hwnd, &mut client_point) == 0 {
        // If we can't map the point, don't assume it's an icon - fall back to the
        // old class-name-only behavior for this click.
        return false;
    }

    let mut process_id: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut process_id);
    if process_id == 0 {
        return false;
    }

    let process = OpenProcess(
        PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
        0,
        process_id,
    );
    if process.is_null() {
        // Can't open the owning process (e.g. permissions) - don't guess, just treat
        // this click as not being on an icon so we fall back to prior behavior.
        return false;
    }

    let hit = desktop_hit_test_remote(process, hwnd, client_point);
    CloseHandle(process);
    hit
}

/// Runs the actual VirtualAllocEx/WriteProcessMemory/SendMessage/ReadProcessMemory
/// dance against `process` (which owns `hwnd`) and returns whether `client_point`
/// (already in `hwnd`'s client coordinates) landed on a list view item.
#[cfg(windows)]
unsafe fn desktop_hit_test_remote(process: HANDLE, hwnd: HWND, client_point: POINT) -> bool {
    let size = std::mem::size_of::<LvHitTestInfo>();
    let remote_buffer = VirtualAllocEx(
        process,
        std::ptr::null(),
        size,
        MEM_COMMIT | MEM_RESERVE,
        PAGE_READWRITE,
    );
    if remote_buffer.is_null() {
        return false;
    }

    let hit_test = LvHitTestInfo {
        pt: client_point,
        flags: 0,
        i_item: -1,
        i_sub_item: 0,
        i_group: 0,
    };

    let mut hit_result = false;
    let write_ok = WriteProcessMemory(
        process,
        remote_buffer,
        &hit_test as *const LvHitTestInfo as *const _,
        size,
        std::ptr::null_mut(),
    );

    if write_ok != 0 {
        SendMessageW(hwnd, LVM_HITTEST, 0, remote_buffer as LPARAM);

        let mut readback = hit_test;
        let read_ok = ReadProcessMemory(
            process,
            remote_buffer,
            &mut readback as *mut LvHitTestInfo as *mut _,
            size,
            std::ptr::null_mut(),
        );
        if read_ok != 0 {
            hit_result = readback.i_item >= 0;
        }
    }

    VirtualFreeEx(process, remote_buffer, 0, MEM_RELEASE);
    hit_result
}

#[cfg(windows)]
fn update_desktop_listener_target(target: String) {
    let last_target = DESKTOP_LAST_TARGET.get_or_init(|| Mutex::new(String::new()));
    if let Ok(mut value) = last_target.lock() {
        *value = target;
    }
}

#[cfg(windows)]
fn update_desktop_toggle_result(result: impl Into<String>) {
    let last_result = DESKTOP_LAST_TOGGLE_RESULT.get_or_init(|| Mutex::new(String::new()));
    if let Ok(mut value) = last_result.lock() {
        *value = result.into();
    }
}

#[cfg(windows)]
fn set_main_window_opacity(window: &tauri::Window, opacity: u8) {
    let Ok(hwnd) = window.hwnd() else {
        return;
    };

    unsafe {
        let style = GetWindowLongPtrW(hwnd.0 as _, GWL_EXSTYLE);
        let _ = SetWindowLongPtrW(hwnd.0 as _, GWL_EXSTYLE, style | WS_EX_LAYERED as isize);
        let _ = SetLayeredWindowAttributes(hwnd.0 as _, 0, opacity, LWA_ALPHA);
    }
}

#[cfg(windows)]
fn fade_main_window(app: &AppHandle, from: u8, to: u8, hide_after_fade: bool, ignore_after_fade: bool) {
    let generation = TOGGLE_FADE_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let app = app.clone();

    std::thread::spawn(move || {
        for step in 0..=TOGGLE_FADE_STEPS {
            if TOGGLE_FADE_GENERATION.load(Ordering::Relaxed) != generation {
                return;
            }

            let progress = step as f32 / TOGGLE_FADE_STEPS as f32;
            let opacity = (from as f32 + (to as f32 - from as f32) * progress).round() as u8;
            let frame_app = app.clone();
            let _ = app.run_on_main_thread(move || {
                if TOGGLE_FADE_GENERATION.load(Ordering::Relaxed) == generation {
                    if let Some(window) = frame_app.get_window("main") {
                        set_main_window_opacity(&window, opacity);
                    }
                }
            });

            if step < TOGGLE_FADE_STEPS {
                std::thread::sleep(Duration::from_millis(TOGGLE_FADE_STEP_DURATION_MS));
            }
        }

        let final_app = app.clone();
        let _ = app.run_on_main_thread(move || {
            if TOGGLE_FADE_GENERATION.load(Ordering::Relaxed) != generation {
                return;
            }
            if let Some(window) = final_app.get_window("main") {
                if ignore_after_fade {
                    let _ = window.set_ignore_cursor_events(true);
                }
                if hide_after_fade {
                    let _ = window.hide();
                    WIDGET_IS_HIDING.store(false, Ordering::Relaxed);
                }
            }
        });
    });
}

#[cfg(windows)]
fn set_main_window_transparent(app: &AppHandle, transparent: bool, transparency_level: u8) {
    // Invert transparency_level: UI uses 0=opaque, 255=transparent; Windows API uses 0=transparent, 255=opaque
    let opacity = u8::MAX - transparency_level;
    if transparent {
        fade_main_window(
            app,
            u8::MAX,
            opacity,
            false,
            false, // Don't ignore cursor events - allow interaction with transparent widget
        );
    } else {
        if let Some(_window) = app.get_window("main") {
            let _ = _window.set_ignore_cursor_events(false);
        }
        fade_main_window(app, opacity, u8::MAX, false, false);
    }
    WIDGET_IS_TRANSPARENT.store(transparent, Ordering::Relaxed);
}

#[cfg(windows)]
fn restore_main_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_window("main") {
        if WIDGET_IS_TRANSPARENT.swap(false, Ordering::Relaxed) {
            // `window` above already confirms the main window exists.
            let options = load_settings(&app).options;
            set_main_window_transparent(app, false, options.transparency_level);
        }
        if !window.is_visible().map_err(|error| error.to_string())? {
            set_main_window_opacity(&window, 0);
            window.show().map_err(|error| error.to_string())?;
            fade_main_window(app, 0, u8::MAX, false, false);
        }
        WIDGET_IS_HIDING.store(false, Ordering::Relaxed);
        Ok(())
    } else {
        Err("Main window was not found".into())
    }
}

#[cfg(windows)]
fn perform_main_window_toggle(app: &AppHandle) -> Result<(), String> {
    let Some(window) = app.get_window("main") else {
        return Err("Main window was not found".into());
    };

    DESKTOP_MAIN_WINDOW_FOUND_COUNT.fetch_add(1, Ordering::Relaxed);
    if WIDGET_IS_HIDING.load(Ordering::Relaxed)
        || !window.is_visible().map_err(|error| error.to_string())?
        || WIDGET_IS_TRANSPARENT.load(Ordering::Relaxed)
    {
        restore_main_window(app)?;
    } else if DESKTOP_TOGGLE_ACTION.load(Ordering::Relaxed) == TRANSPARENT_WIDGET_ACTION {
        // `window` above already confirms the main window exists.
        let options = load_settings(&app).options;
        set_main_window_transparent(app, true, options.transparency_level);
    } else {
        WIDGET_IS_HIDING.store(true, Ordering::Relaxed);
        fade_main_window(app, u8::MAX, 0, true, false);
    }
    DESKTOP_TOGGLE_COUNT.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

#[cfg(windows)]
fn toggle_main_window() {
    if let Some(app) = DESKTOP_DOUBLE_CLICK_APP.get() {
        DESKTOP_TOGGLE_DISPATCH_COUNT.fetch_add(1, Ordering::Relaxed);
        let app = app.clone();
        if let Err(error) = app.clone().run_on_main_thread(move || {
            DESKTOP_TOGGLE_UI_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
            match perform_main_window_toggle(&app) {
                Ok(()) => update_desktop_toggle_result("Toggle completed"),
                Err(error) => update_desktop_toggle_result(format!("Toggle failed: {error}")),
            }
        }) {
            update_desktop_toggle_result(format!("Could not schedule UI-thread toggle: {error}"));
        }
    } else {
        update_desktop_toggle_result("Desktop listener has no application handle");
    }
}

#[cfg(windows)]
unsafe extern "system" fn desktop_mouse_hook(
    code: i32,
    message: WPARAM,
    data: LPARAM,
) -> LRESULT {
    if code == HC_ACTION as i32
        && matches!(message, value if value == WM_LBUTTONDOWN as usize || value == WM_LBUTTONDBLCLK as usize)
    {
        let is_double_click_message = message == WM_LBUTTONDBLCLK as usize;
        if !is_double_click_message {
            DESKTOP_MOUSE_DOWN_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        let point = (*(data as *const MSLLHOOKSTRUCT)).pt;
        let hovered_window = WindowFromPoint(point);
        let (is_desktop_surface, target, is_progman_window) =
            desktop_window_details(hovered_window);
        update_desktop_listener_target(target);
        if is_desktop_surface {
            DESKTOP_SURFACE_CLICK_COUNT.fetch_add(1, Ordering::Relaxed);
        }

        // A click on the desktop surface can still be a click on an icon rather than
        // empty space - both share the same SysListView32 class, so class name alone
        // can't tell them apart. Hit-test against the list view to find out, and treat
        // icon clicks like any other non-desktop-surface click (never toggles, and
        // resets any pending double-click state).
        let clicked_on_icon = is_desktop_surface && desktop_click_hits_icon(hovered_window, point);
        if clicked_on_icon {
            DESKTOP_ICON_CLICK_IGNORED_COUNT.fetch_add(1, Ordering::Relaxed);
        }

        if DESKTOP_DOUBLE_CLICK_ENABLED.load(Ordering::Relaxed)
            && is_desktop_surface
            && !clicked_on_icon
            && is_double_click_message
            && is_progman_window
        {
            DESKTOP_DOUBLE_CLICK_COUNT.fetch_add(1, Ordering::Relaxed);
            toggle_main_window();
        } else if DESKTOP_DOUBLE_CLICK_ENABLED.load(Ordering::Relaxed)
            && is_desktop_surface
            && !clicked_on_icon
        {
            let tick = GetTickCount();
            let clicks = DESKTOP_CLICK.get_or_init(|| Mutex::new(None));
            if let Ok(mut previous_click) = clicks.lock() {
                let is_double_click = previous_click.is_some_and(|previous| {
                    tick.wrapping_sub(previous.tick) <= GetDoubleClickTime()
                });
                *previous_click = None;

                if is_double_click {
                    DESKTOP_DOUBLE_CLICK_COUNT.fetch_add(1, Ordering::Relaxed);
                    toggle_main_window();
                } else {
                    *previous_click = Some(DesktopClick { tick });
                }
            }
        } else if let Some(clicks) = DESKTOP_CLICK.get() {
            if let Ok(mut previous_click) = clicks.lock() {
                *previous_click = None;
            }
        }
    }

    CallNextHookEx(std::ptr::null_mut(), code, message, data)
}

#[cfg(windows)]
fn start_desktop_double_click_listener(app: AppHandle) {
    let _ = DESKTOP_DOUBLE_CLICK_APP.set(app);
    DESKTOP_HOOK_STARTED.call_once(|| {
        std::thread::spawn(|| unsafe {
            let module = GetModuleHandleW(std::ptr::null());
            let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(desktop_mouse_hook), module, 0);
            if hook.is_null() {
                eprintln!("Failed to install desktop double-click listener.");
                return;
            }
            DESKTOP_HOOK_INSTALLED.store(true, Ordering::Relaxed);

            let mut message = std::mem::zeroed::<MSG>();
            while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {}
        });
    });
}

#[cfg(not(windows))]
fn start_desktop_double_click_listener(_app: AppHandle) {}

#[tauri::command]
fn get_options(app: AppHandle) -> Options {
    let mut settings = load_settings(&app);
    // Always reflect the actual OS autostart state.
    settings.options.open_at_login = app.autolaunch().is_enabled().unwrap_or(false);
    settings.options
}

#[tauri::command]
fn is_debug_build() -> bool {
    cfg!(debug_assertions)
}

#[cfg(windows)]
#[tauri::command]
fn get_desktop_listener_debug() -> DesktopListenerDebug {
    let last_target = DESKTOP_LAST_TARGET
        .get()
        .and_then(|value| value.lock().ok().map(|target| target.clone()))
        .unwrap_or_else(|| "No mouse-down event captured yet".into());
    let last_toggle_result = DESKTOP_LAST_TOGGLE_RESULT
        .get()
        .and_then(|value| value.lock().ok().map(|result| result.clone()))
        .unwrap_or_else(|| "No toggle attempted yet".into());

    DesktopListenerDebug {
        hook_installed: DESKTOP_HOOK_INSTALLED.load(Ordering::Relaxed),
        listener_enabled: DESKTOP_DOUBLE_CLICK_ENABLED.load(Ordering::Relaxed),
        mouse_down_count: DESKTOP_MOUSE_DOWN_COUNT.load(Ordering::Relaxed),
        desktop_surface_click_count: DESKTOP_SURFACE_CLICK_COUNT.load(Ordering::Relaxed),
        double_click_count: DESKTOP_DOUBLE_CLICK_COUNT.load(Ordering::Relaxed),
        toggle_dispatch_count: DESKTOP_TOGGLE_DISPATCH_COUNT.load(Ordering::Relaxed),
        toggle_ui_thread_count: DESKTOP_TOGGLE_UI_THREAD_COUNT.load(Ordering::Relaxed),
        main_window_found_count: DESKTOP_MAIN_WINDOW_FOUND_COUNT.load(Ordering::Relaxed),
        toggle_count: DESKTOP_TOGGLE_COUNT.load(Ordering::Relaxed),
        icon_click_ignored_count: DESKTOP_ICON_CLICK_IGNORED_COUNT.load(Ordering::Relaxed),
        last_target,
        last_toggle_result,
    }
}

#[cfg(windows)]
#[tauri::command]
fn test_desktop_toggle(app: AppHandle) -> Result<(), String> {
    match perform_main_window_toggle(&app) {
        Ok(()) => {
            update_desktop_toggle_result("Toggle completed by Options test button");
            Ok(())
        }
        Err(error) => {
            update_desktop_toggle_result(format!("Options test failed: {error}"));
            Err(error)
        }
    }
}

#[tauri::command]
fn save_options(
    app: AppHandle,
    remember_window_bounds: Option<bool>,
    open_at_login: Option<bool>,
    toggle_on_desktop_double_click: Option<bool>,
    desktop_toggle_action: Option<DesktopToggleAction>,
    transparency_level: Option<u8>,
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
    if let Some(v) = toggle_on_desktop_double_click {
        settings.options.toggle_on_desktop_double_click = v;
        DESKTOP_DOUBLE_CLICK_ENABLED.store(v, Ordering::Relaxed);
        if !v {
            let _ = restore_main_window(&app);
        }
    }
    if let Some(v) = desktop_toggle_action {
        settings.options.desktop_toggle_action = v;
        DESKTOP_TOGGLE_ACTION.store(v.as_u8(), Ordering::Relaxed);
        if v == DesktopToggleAction::Hide {
            let _ = restore_main_window(&app);
        }
    }
    if let Some(v) = transparency_level {
        settings.options.transparency_level = v;
    }

    save_settings(&app, &settings);

    // If widget is currently transparent, apply the new transparency level immediately
    #[cfg(windows)]
    if WIDGET_IS_TRANSPARENT.load(Ordering::Relaxed) {
        let opacity = u8::MAX - settings.options.transparency_level;
        let app_clone = app.clone();
        let _ = app_clone.clone().run_on_main_thread(move || {
            if let Some(window) = app_clone.get_window("main") {
                set_main_window_opacity(&window, opacity);
            }
        });
    }

    settings.options
}

#[tauri::command]
fn update_transparency_level(app: AppHandle, transparency_level: u8) {
    let mut settings = load_settings(&app);
    settings.options.transparency_level = transparency_level;
    save_settings(&app, &settings);

    // If widget is currently transparent, apply the new transparency level immediately
    #[cfg(windows)]
    if WIDGET_IS_TRANSPARENT.load(Ordering::Relaxed) {
        let opacity = u8::MAX - transparency_level;
        let app_clone = app.clone();
        let _ = app_clone.clone().run_on_main_thread(move || {
            if let Some(window) = app_clone.get_window("main") {
                set_main_window_opacity(&window, opacity);
            }
        });
    }
}

#[tauri::command]
fn reset_options(app: AppHandle) -> Options {
    let mut settings = load_settings(&app);
    settings.options = Options::default();
    DESKTOP_DOUBLE_CLICK_ENABLED.store(false, Ordering::Relaxed);
    DESKTOP_TOGGLE_ACTION.store(HIDE_WIDGET_ACTION, Ordering::Relaxed);
    let _ = restore_main_window(&app);
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
                .inner_size(560.0, 530.0)
                .resizable(false)
                .minimizable(false)
                .maximizable(false)
                .fullscreen(false);

        if let Some(main_window) = app.get_window("main") {
            if let (Ok(position), Ok(scale_factor)) =
                (main_window.outer_position(), main_window.scale_factor())
            {
                builder = builder.position(
                    position.x as f64 / scale_factor + 32.0,
                    position.y as f64 / scale_factor + 32.0,
                );
            }
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
            DESKTOP_DOUBLE_CLICK_ENABLED.store(
                settings.options.toggle_on_desktop_double_click,
                Ordering::Relaxed,
            );
            DESKTOP_TOGGLE_ACTION.store(
                settings.options.desktop_toggle_action.as_u8(),
                Ordering::Relaxed,
            );
            start_desktop_double_click_listener(handle.clone());

            remove_legacy_electron_autostart();

            let autolaunch = handle.autolaunch();
            let _ = if settings.options.open_at_login {
                autolaunch.enable()
            } else {
                autolaunch.disable()
            };

            let main_window = app.get_webview_window("main").unwrap();
            let main_base_window = app.get_window("main").unwrap();

            // Add "DEV BUILD" prefix to title in debug builds
            let title = if is_debug_build() {
                "What-N-When (DEV BUILD)"
            } else {
                "What-N-When"
            };
            let _ = main_window.set_title(title);

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

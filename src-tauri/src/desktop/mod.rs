use super::*;

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct DesktopListenerDebug {
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

#[cfg(windows)]
mod windows_impl {
    use super::*;
    #[path = "effects.rs"]
    mod effects;
    #[path = "hit_testing.rs"]
    mod hit_testing;
    #[path = "mouse_hook.rs"]
    mod mouse_hook;
    use effects::*;
    use mouse_hook::ClickEvent;

// Everything that touches Win32 lives here.
    //
    // Data flow: the `WH_MOUSE_LL` callback does almost nothing (it forwards the click
    // over a channel), because a slow low-level hook stalls mouse input system-wide.
    // A worker thread classifies the click, hit-tests desktop icons inside explorer.exe,
    // detects double-clicks, and asks the UI thread to toggle the widget.
    //
    // `restore`, `toggle` and `test_toggle` must be called on the main thread
    // (sync Tauri commands already run there).

use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::{Mutex, OnceLock, PoisonError};
    use std::time::Duration;
    use tauri::{AppHandle, Manager};
    use windows_sys::Win32::Foundation::{
        CloseHandle, HANDLE, HWND, LPARAM, LRESULT, POINT, WPARAM,
    };
    use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
    use windows_sys::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Memory::{
        VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetClassNameW, GetMessageW, GetParent, GetSystemMetrics, GetWindowLongPtrW,
        GetWindowThreadProcessId, SendMessageTimeoutW, SetLayeredWindowAttributes,
        SetWindowLongPtrW, SetWindowsHookExW, UnhookWindowsHookEx, WindowFromPoint, GWL_EXSTYLE,
        HC_ACTION, LWA_ALPHA, MSG, MSLLHOOKSTRUCT, SMTO_ABORTIFHUNG, SM_CXDOUBLECLK,
        SM_CYDOUBLECLK, WH_MOUSE_LL, WM_LBUTTONDOWN, WS_EX_LAYERED,
    };

    const FADE_STEPS: u16 = 13;
    const FADE_STEP: Duration = Duration::from_millis(20);
    const MAX_ANCESTOR_DEPTH: usize = 8;
    const HIT_TEST_TIMEOUT_MS: u32 = 200;

    /// LVM_HITTEST (LVM_FIRST + 18): asks a SysListView32 control whether a
    /// client-coordinate point falls on an item (icon) or on empty list space.
    const LVM_HITTEST: u32 = 0x1000 + 18;

    // ----- shared state ----------------------------------------------------

    static LISTENER_ENABLED: AtomicBool = AtomicBool::new(false);
    static FADE_INSTEAD_OF_HIDE: AtomicBool = AtomicBool::new(false);
    static WIDGET_STATE: AtomicU8 = AtomicU8::new(WidgetState::Visible as u8);
    static CURRENT_ALPHA: AtomicU8 = AtomicU8::new(u8::MAX);
    /// Bumped whenever a new fade starts (or is cancelled); older fades notice and stop.
    static FADE_GENERATION: AtomicUsize = AtomicUsize::new(0);
    static LAYERED_READY: AtomicBool = AtomicBool::new(false);
    static CLICK_TX: OnceLock<Sender<ClickEvent>> = OnceLock::new();
    static STATS: HookStats = HookStats::new();

    /// The widget's visibility state machine (one atomic instead of several booleans).
    #[derive(Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    enum WidgetState {
        Visible,
        Faded,
        Hiding,
        Hidden,
    }

    impl WidgetState {
        fn load() -> Self {
            match WIDGET_STATE.load(Ordering::Relaxed) {
                1 => Self::Faded,
                2 => Self::Hiding,
                3 => Self::Hidden,
                _ => Self::Visible,
            }
        }

        fn store(self) {
            WIDGET_STATE.store(self as u8, Ordering::Relaxed);
        }
    }

    struct HookStats {
        hook_installed: AtomicBool,
        mouse_down: AtomicUsize,
        desktop_surface_clicks: AtomicUsize,
        double_clicks: AtomicUsize,
        toggle_dispatches: AtomicUsize,
        toggle_ui_thread: AtomicUsize,
        main_window_found: AtomicUsize,
        toggles: AtomicUsize,
        icon_clicks_ignored: AtomicUsize,
        last_target: Mutex<String>,
        last_toggle_result: Mutex<String>,
    }

    impl HookStats {
        const fn new() -> Self {
            Self {
                hook_installed: AtomicBool::new(false),
                mouse_down: AtomicUsize::new(0),
                desktop_surface_clicks: AtomicUsize::new(0),
                double_clicks: AtomicUsize::new(0),
                toggle_dispatches: AtomicUsize::new(0),
                toggle_ui_thread: AtomicUsize::new(0),
                main_window_found: AtomicUsize::new(0),
                toggles: AtomicUsize::new(0),
                icon_clicks_ignored: AtomicUsize::new(0),
                last_target: Mutex::new(String::new()),
                last_toggle_result: Mutex::new(String::new()),
            }
        }

        fn set_target(&self, target: String) {
            *self
                .last_target
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = target;
        }

        fn set_toggle_result(&self, result: impl Into<String>) {
            *self
                .last_toggle_result
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = result.into();
        }

        fn snapshot(&self) -> DesktopListenerDebug {
            let read = |value: &Mutex<String>, fallback: &str| {
                let value = value.lock().unwrap_or_else(PoisonError::into_inner);
                if value.is_empty() {
                    fallback.to_owned()
                } else {
                    value.clone()
                }
            };
            let count = |counter: &AtomicUsize| counter.load(Ordering::Relaxed);
            DesktopListenerDebug {
                hook_installed: self.hook_installed.load(Ordering::Relaxed),
                listener_enabled: LISTENER_ENABLED.load(Ordering::Relaxed),
                mouse_down_count: count(&self.mouse_down),
                desktop_surface_click_count: count(&self.desktop_surface_clicks),
                double_click_count: count(&self.double_clicks),
                toggle_dispatch_count: count(&self.toggle_dispatches),
                toggle_ui_thread_count: count(&self.toggle_ui_thread),
                main_window_found_count: count(&self.main_window_found),
                toggle_count: count(&self.toggles),
                icon_click_ignored_count: count(&self.icon_clicks_ignored),
                last_target: read(&self.last_target, "No mouse-down event captured yet"),
                last_toggle_result: read(&self.last_toggle_result, "No toggle attempted yet"),
            }
        }
    }

    pub(super) fn bump(counter: &AtomicUsize) {
        counter.fetch_add(1, Ordering::Relaxed);
    }

    // ----- public API ------------------------------------------------------

    pub fn init(app: AppHandle, enabled: bool, action: DesktopToggleAction) {
        set_listener_enabled(enabled);
        set_toggle_action(action);

        let (tx, rx) = mpsc::channel();
        if CLICK_TX.set(tx).is_err() {
            return; // already initialised
        }
        if let Err(error) = std::thread::Builder::new()
            .name("desktop-click-worker".into())
            .spawn(move || mouse_hook::click_worker(app, rx))
        {
            eprintln!("desktop listener: could not start worker thread: {error}");
            return;
        }
        if let Err(error) = std::thread::Builder::new()
            .name("desktop-mouse-hook".into())
            .spawn(mouse_hook::run_hook_thread)
        {
            eprintln!("desktop listener: could not start hook thread: {error}");
        }
    }

    pub fn set_listener_enabled(enabled: bool) {
        LISTENER_ENABLED.store(enabled, Ordering::Relaxed);
    }

    pub fn set_toggle_action(action: DesktopToggleAction) {
        FADE_INSTEAD_OF_HIDE.store(
            action == DesktopToggleAction::Transparent,
            Ordering::Relaxed,
        );
    }

    pub fn debug_snapshot() -> DesktopListenerDebug {
        STATS.snapshot()
    }

    /// Brings the widget back to fully visible, whatever state it is in.
    pub fn restore(app: &AppHandle) -> Result<(), String> {
        let window = main_window(app)?;
        restore_with_window(app, &window)
    }

    pub fn restore_with_window(app: &AppHandle, window: &tauri::Window) -> Result<(), String> {
        let was_visible = window.is_visible().map_err(|e| e.to_string())?;
        if !was_visible {
            set_window_alpha(&window, 0);
            window.show().map_err(|e| e.to_string())?;
        }

        let previous = WidgetState::load();
        WidgetState::Visible.store();
        // Always start a fade when a hide/fade may be in flight: starting one also
        // cancels it (via the generation counter).
        if previous != WidgetState::Visible
            || !was_visible
            || CURRENT_ALPHA.load(Ordering::Relaxed) != u8::MAX
        {
            fade_to(app, u8::MAX, FadeEnd::Keep);
        }
        Ok(())
    }

    /// Toggles between visible and (hidden | faded), per the configured action.
    pub fn toggle(app: &AppHandle) -> Result<(), String> {
        let window = main_window(app)?;
        bump(&STATS.main_window_found);

        let visible = window.is_visible().map_err(|e| e.to_string())?;
        match (WidgetState::load(), visible) {
            (WidgetState::Visible, true) => {
                if FADE_INSTEAD_OF_HIDE.load(Ordering::Relaxed) {
                    WidgetState::Faded.store();
                    fade_to(app, alpha_from_level(configured_level(app)), FadeEnd::Keep);
                } else {
                    WidgetState::Hiding.store();
                    fade_to(app, 0, FadeEnd::Hide);
                }
            }
            _ => restore(app)?,
        }
        bump(&STATS.toggles);
        Ok(())
    }

    /// Backs the "test" button in the options window.
    pub fn test_toggle(app: &AppHandle) -> Result<(), String> {
        let result = toggle(app);
        STATS.set_toggle_result(match &result {
            Ok(()) => "Toggle completed by Options test button".to_owned(),
            Err(error) => format!("Options test failed: {error}"),
        });
        result
    }

    /// Applies the configured transparency level immediately if the widget is faded.
    pub fn apply_transparency_level(app: &AppHandle) {
        if WidgetState::load() != WidgetState::Faded {
            return;
        }
        let alpha = alpha_from_level(configured_level(app));
        FADE_GENERATION.fetch_add(1, Ordering::Relaxed); // cancel any in-flight fade
        let ui_app = app.clone();
        if let Err(error) = app.run_on_main_thread(move || apply_alpha(&ui_app, alpha)) {
            eprintln!("could not schedule transparency update: {error}");
        }
    }
}

#[cfg(windows)]
pub(super) use windows_impl::{
    apply_transparency_level, debug_snapshot, init, restore, restore_with_window,
    set_listener_enabled, set_toggle_action, test_toggle, toggle,
};

#[cfg(not(windows))]
pub(super) fn init(_app: AppHandle, _enabled: bool, _action: DesktopToggleAction) {}
#[cfg(not(windows))]
pub(super) fn set_listener_enabled(_enabled: bool) {}
#[cfg(not(windows))]
pub(super) fn set_toggle_action(_action: DesktopToggleAction) {}
#[cfg(not(windows))]
pub(super) fn restore(_app: &AppHandle) -> Result<(), String> { Ok(()) }
#[cfg(not(windows))]
pub(super) fn test_toggle(_app: &AppHandle) -> Result<(), String> {
    Err("Desktop toggle is only supported on Windows".to_owned())
}
#[cfg(not(windows))]
pub(super) fn apply_transparency_level(_app: &AppHandle) {}
#[cfg(not(windows))]
pub(super) fn debug_snapshot() -> DesktopListenerDebug {
    DesktopListenerDebug::default()
}

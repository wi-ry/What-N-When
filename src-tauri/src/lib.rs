//! What-N-When: a Notion Calendar desktop widget (Tauri 2).
//!
//! Layout of this file (each `mod`/section can be moved to its own file as-is):
//!   * constants + popup / URL routing helpers
//!   * settings types + `SettingsStore` (cached in managed state, atomic writes)
//!   * `mod desktop`: all Windows-only code (fade effects, mouse hook, unsafe Win32),
//!     with a no-op stub for other platforms so the crate compiles everywhere
//!   * Tauri commands
//!   * window helpers + `run()`

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use tauri::webview::{Color, NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, State, Url,
    WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAIN_LABEL: &str = "main";
const CALENDAR_LABEL: &str = "calendar";
const OPTIONS_LABEL: &str = "options";
const CALENDAR_URL: &str = "https://calendar.notion.so/";
const TITLEBAR_HEIGHT: f64 = 40.0;

/// UI transparency level: 0 = opaque, larger = more transparent. Capped so the
/// widget can never become fully invisible while still swallowing clicks.
const MAX_TRANSPARENCY_LEVEL: u8 = 230;

/// A restored window must overlap a monitor by at least this many physical pixels
/// in each direction, otherwise the saved bounds are ignored (monitor unplugged).
const MIN_VISIBLE_PX: i64 = 100;

static POPUP_WINDOW_ID: AtomicUsize = AtomicUsize::new(0);

/// External identity providers whose popups must stay in-app (OAuth flows rely on
/// `window.opener`, `postMessage` and cookies shared with the calendar webview).
/// If sign-in with some provider breaks, run a debug build and look for
/// "sent to browser" lines in stderr, then add the missing host here.
const AUTH_HOSTS: &[&str] = &[
    "accounts.google.com",
    "appleid.apple.com",
    "idmsa.apple.com",
    "login.microsoftonline.com",
    "login.live.com",
];

/// When true, same-window navigations (plain links with no target) to non-Notion sites
/// are sent to the default browser instead of loading inside the widget. Off by default:
/// it can interfere with sign-in redirects, and Notion Calendar opens event links via
/// `window.open`, which `on_new_window` already handles.
const GUARD_SAME_WINDOW_NAVIGATION: bool = false;

/// Sign-in popups end on a Notion `...popupcallback` page (e.g. `/googlepopupcallback`,
/// `/microsoftpopupcallback`) that is supposed to post the result to the main window and
/// call `window.close()`. Tauri doesn't honour that call, so we close the popup ourselves.
/// Set `AUTO_CLOSE_POPUP_CALLBACK` to false to leave the popup open while debugging.
const AUTO_CLOSE_POPUP_CALLBACK: bool = true;
/// How long the callback page gets to deliver its message before we close it. Kept long
/// while we diagnose the Microsoft flow; shorten once sign-in works.
const POPUP_CALLBACK_CLOSE_DELAY: std::time::Duration = std::time::Duration::from_millis(4000);

/// Dev-build diagnostics. Scripts injected into the popup and the calendar webview report
/// what they see by setting `document.title` to `WNWDBG|<n>|<message>`; the title-changed
/// handlers print those lines to the terminal as `[popup] ...` / `[main] ...`.
/// (Remote pages have no Tauri IPC, so the title is the simplest one-way channel.)
const DEBUG_TITLE_PREFIX: &str = "WNWDBG|";

const DEBUG_JS_COMMON: &str = r#"
var PREFIX = '__PREFIX__', queue = [], seq = 0, timer = null;
function flush() {
  if (!queue.length) { timer = null; return; }
  if (!document.documentElement) { timer = setTimeout(flush, 50); return; }
  try { document.title = PREFIX + (seq++) + '|' + queue.shift(); } catch (e) {}
  timer = setTimeout(flush, 120);
}
function log(msg) { queue.push(msg); if (!timer) flush(); }
function openerState() {
  try { return window.opener ? 'present' : 'NULL'; } catch (e) { return 'error'; }
}
"#;

/// Injected into every page loaded inside a sign-in popup.
const DEBUG_JS_POPUP: &str = r#"
(function () {
  __COMMON__
  var here = location.host + location.pathname;
  log('script start ' + here + ' opener=' + openerState());
  var origClose = window.close;
  window.close = function () { log('window.close() called on ' + here); return origClose.apply(window, arguments); };
  window.addEventListener('error', function (e) { log('js error: ' + e.message); });
  window.addEventListener('unhandledrejection', function (e) {
    log('unhandled rejection: ' + (e.reason && e.reason.message ? e.reason.message : e.reason));
  });
  window.addEventListener('pagehide', function () { log('pagehide ' + here); });
  document.addEventListener('DOMContentLoaded', function () {
    log('DOMContentLoaded ' + here + ' opener=' + openerState());
    // Probe the popup -> main channel: the main window logs if this arrives.
    try { if (window.opener) { window.opener.postMessage({ wnwProbe: here }, '*'); log('probe sent'); } }
    catch (e) { log('probe failed: ' + e.message); }
  });
})();
"#;

/// Injected into the calendar webview (main window).
const DEBUG_JS_MAIN: &str = r#"
(function () {
  __COMMON__
  window.addEventListener('message', function (e) {
    var d = e.data;
    if (d && d.wnwProbe) { log('PROBE arrived from popup (' + d.wnwProbe + ') origin=' + e.origin); return; }
    var shape;
    try { shape = typeof d === 'string' ? 'string(len ' + d.length + ')' : 'keys=' + Object.keys(d || {}).join(','); }
    catch (x) { shape = '?'; }
    log('message from ' + e.origin + ' ' + shape);
    // Does the page move on after receiving the popup's result?
    [3000, 8000].forEach(function (ms) {
      setTimeout(function () { log(ms + 'ms after message, still on ' + location.host + location.pathname); }, ms);
    });
  }, true);
  var origOpen = window.open;
  window.open = function () {
    var args = arguments;
    var w = origOpen.apply(this, args);
    log('window.open(' + String(args[0]).split('?')[0] + ') -> ' + (w ? 'window' : 'NULL'));
    if (w) {
      var t = setInterval(function () {
        var c; try { c = w.closed; } catch (e) { c = 'err'; }
        if (c === true) { log('popup .closed became true'); clearInterval(t); }
      }, 250);
    }
    return w;
  };
  // Report failed (and auth-related) network calls and page errors, host + path only.
  var interesting = /login|oauth|auth|identity|session|verify|microsoft|google/i;
  function short(u) { try { return String(u).split('?')[0]; } catch (e) { return '?'; } }
  var origFetch = window.fetch;
  if (origFetch) {
    window.fetch = function (input) {
      var u = short(typeof input === 'string' ? input : (input && input.url) || '');
      return origFetch.apply(this, arguments).then(function (r) {
        if (!r.ok || interesting.test(u)) log('fetch ' + r.status + ' ' + u);
        return r;
      }, function (err) { log('fetch FAILED ' + u + ': ' + (err && err.message)); throw err; });
    };
  }
  var xhrOpen = XMLHttpRequest.prototype.open, xhrSend = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.open = function (m, u) { this.__wnw = m + ' ' + short(u); return xhrOpen.apply(this, arguments); };
  XMLHttpRequest.prototype.send = function () {
    var x = this;
    x.addEventListener('loadend', function () {
      if (x.status === 0 || x.status >= 400 || interesting.test(x.__wnw || '')) log('xhr ' + x.status + ' ' + x.__wnw);
    });
    return xhrSend.apply(this, arguments);
  };
  window.addEventListener('error', function (e) { log('js error: ' + e.message); });
  window.addEventListener('unhandledrejection', function (e) {
    log('unhandled rejection: ' + (e.reason && e.reason.message ? e.reason.message : e.reason));
  });
  document.addEventListener('securitypolicyviolation', function (e) {
    log('CSP violation: ' + e.violatedDirective + ' ' + short(e.blockedURI));
  });
  log('main debug script ready');
})();
"#;

fn debug_script(template: &str) -> String {
    template
        .replace("__COMMON__", DEBUG_JS_COMMON)
        .replace("__PREFIX__", DEBUG_TITLE_PREFIX)
}

/// If `title` is a debug message from an injected script, print it and return true.
fn log_debug_title(source: &str, title: &str) -> bool {
    match title.strip_prefix(DEBUG_TITLE_PREFIX) {
        Some(message) => {
            eprintln!("[{source}] {message}");
            true
        }
        None => false,
    }
}

/// Logs host + path only: query strings carry OAuth codes and state.
fn log_page_load(source: &str, finished: bool, url: &Url) {
    eprintln!(
        "[{source}] page {} {}{}",
        if finished { "finished" } else { "started" },
        url.host_str().unwrap_or(""),
        url.path()
    );
}

/// Notion's own domains. Sign-in popups start on these (the exact path varies, so
/// we don't try to guess it), then hop to a provider in `AUTH_HOSTS` and back.
const NOTION_HOSTS: &[&str] = &["notion.so", "notion.com"];

fn calendar_url() -> Url {
    Url::parse(CALENDAR_URL).expect("CALENDAR_URL is a valid URL")
}

fn about_blank() -> Url {
    Url::parse("about:blank").expect("about:blank is a valid URL")
}

// ---------------------------------------------------------------------------
// Popup / navigation routing
// ---------------------------------------------------------------------------

/// True if `host` is `domain` or a subdomain of it (no allocation).
fn host_matches(host: &str, domain: &str) -> bool {
    host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|rest| rest.ends_with('.'))
}

/// Should a `window.open` / `target="_blank"` request get an in-app popup?
///
/// Yes for Notion's own domains and the identity providers (that is how sign-in
/// works); everything else, such as Zoom, Meet or Docs links in an event, goes to the
/// default browser. Denying a sign-in popup makes `window.open` return null, which
/// Notion reports as "your browser is blocking popups", so when in doubt, allow.
fn is_auth_popup(url: &Url) -> bool {
    // Script-driven popups often start at about:blank and navigate afterwards;
    // the destination isn't known yet, so keep them in-app.
    if url.scheme() == "about" {
        return true;
    }
    url.host_str().is_some_and(|host| {
        NOTION_HOSTS
            .iter()
            .chain(AUTH_HOSTS)
            .any(|d| host_matches(host, d))
    })
}

/// Should the calendar webview itself be allowed to navigate here?
fn is_internal_url(url: &Url) -> bool {
    if matches!(url.scheme(), "about" | "blob" | "data") {
        return true;
    }
    url.host_str().is_some_and(|host| {
        NOTION_HOSTS
            .iter()
            .chain(AUTH_HOSTS)
            .any(|d| host_matches(host, d))
    })
}

/// Is this the final callback page of a sign-in popup?
fn is_popup_callback(url: &Url) -> bool {
    url.host_str()
        .is_some_and(|host| NOTION_HOSTS.iter().any(|d| host_matches(host, d)))
        && url
            .path()
            .rsplit('/')
            .next()
            .is_some_and(|segment| segment.ends_with("popupcallback"))
}

fn open_externally(app: &AppHandle, url: &Url) {
    // Only hand safe schemes to the OS; never file:, javascript:, custom protocols, etc.
    if !matches!(url.scheme(), "http" | "https" | "mailto") {
        return;
    }
    if cfg!(debug_assertions) {
        eprintln!("sent to browser: {url}");
    }
    if let Err(error) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("failed to open {url} in the default browser: {error}");
    }
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WindowBounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum DesktopToggleAction {
    #[default]
    Hide,
    Transparent,
}

/// Persisted options. `open_at_login` is intentionally absent: the OS autostart
/// entry is the single source of truth (see `OptionsView`).
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
struct Options {
    remember_window_bounds: bool,
    toggle_on_desktop_double_click: bool,
    desktop_toggle_action: DesktopToggleAction,
    transparency_level: u8,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            remember_window_bounds: true,
            toggle_on_desktop_double_click: false,
            desktop_toggle_action: DesktopToggleAction::default(),
            transparency_level: 64,
        }
    }
}

/// What the frontend sees: persisted options plus the live OS autostart state.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OptionsView {
    #[serde(flatten)]
    options: Options,
    open_at_login: bool,
}

/// Partial update sent by the options window; absent fields are left unchanged.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct OptionsPatch {
    remember_window_bounds: Option<bool>,
    open_at_login: Option<bool>,
    toggle_on_desktop_double_click: Option<bool>,
    desktop_toggle_action: Option<DesktopToggleAction>,
    transparency_level: Option<u8>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct Settings {
    options: Options,
    window_bounds: Option<WindowBounds>,
}

/// Settings cached in Tauri managed state. Reads never touch the disk; every
/// mutation is written back atomically (temp file + rename).
struct SettingsStore {
    path: PathBuf,
    inner: Mutex<Settings>,
}

impl SettingsStore {
    fn load(path: PathBuf) -> Self {
        let mut settings = match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|error| {
                eprintln!("settings: ignoring unreadable {}: {error}", path.display());
                Settings::default()
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Settings::default(),
            Err(error) => {
                eprintln!("settings: could not read {}: {error}", path.display());
                Settings::default()
            }
        };
        settings.options.transparency_level = settings
            .options
            .transparency_level
            .min(MAX_TRANSPARENCY_LEVEL);
        Self {
            path,
            inner: Mutex::new(settings),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Settings> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn snapshot(&self) -> Settings {
        self.lock().clone()
    }

    fn options(&self) -> Options {
        self.lock().options.clone()
    }

    /// Mutates the settings under the lock, then persists them.
    fn update<R>(&self, apply: impl FnOnce(&mut Settings) -> R) -> R {
        let mut settings = self.lock();
        let result = apply(&mut settings);
        if let Err(error) = write_atomic(&self.path, &settings) {
            eprintln!("settings: failed to save {}: {error}", self.path.display());
        }
        result
    }
}

fn write_atomic(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(settings).map_err(io::Error::other)?)?;
    fs::rename(tmp, path)
}

fn settings_path(app: &AppHandle) -> tauri::Result<PathBuf> {
    // Dev builds get their own file so they never clobber a real install's settings,
    // and (unlike a CWD-relative file) can't trigger the dev watcher into restarting.
    let file = if cfg!(debug_assertions) {
        "settings.dev.json"
    } else {
        "settings.json"
    };
    Ok(app.path().app_data_dir()?.join(file))
}

// ---------------------------------------------------------------------------
// Autostart
// ---------------------------------------------------------------------------

/// Removes the Run-key entry left behind by the old Electron version of the app.
#[cfg(windows)]
fn remove_legacy_electron_autostart() {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;

    const LEGACY_ELECTRON_AUTOSTART_NAME: &str = "ca.willryan.notioncalendarwidget";

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(run_key) = hkcu.open_subkey_with_flags(
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
        KEY_SET_VALUE,
    ) {
        // Fails if the value doesn't exist, which is the normal case.
        let _ = run_key.delete_value(LEGACY_ELECTRON_AUTOSTART_NAME);
    }
}

#[cfg(not(windows))]
fn remove_legacy_electron_autostart() {}

fn set_autostart(app: &AppHandle, enabled: bool) {
    let autolaunch = app.autolaunch();
    let result = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    if let Err(error) = result {
        eprintln!("autostart: failed to set enabled={enabled}: {error}");
    }
}

// ---------------------------------------------------------------------------
// Desktop double-click toggle (Windows implementation)
// ---------------------------------------------------------------------------

/// Counters and last-seen strings shown in the options window's debug panel.
#[derive(Serialize, Default)]
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

#[cfg(windows)]
mod desktop {
    //! Everything that touches Win32 lives here.
    //!
    //! Data flow: the `WH_MOUSE_LL` callback does almost nothing (it forwards the click
    //! over a channel), because a slow low-level hook stalls mouse input system-wide.
    //! A worker thread classifies the click, hit-tests desktop icons inside explorer.exe,
    //! detects double-clicks, and asks the UI thread to toggle the widget.
    //!
    //! `restore`, `toggle` and `test_toggle` must be called on the main thread
    //! (sync Tauri commands already run there).

    use super::{
        DesktopListenerDebug, DesktopToggleAction, SettingsStore, MAIN_LABEL,
        MAX_TRANSPARENCY_LEVEL,
    };
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::{Mutex, OnceLock, PoisonError};
    use std::time::Duration;
    use tauri::{AppHandle, Manager};
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, LRESULT, POINT, WPARAM};
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
        CallNextHookEx, GetClassNameW, GetMessageW, GetParent, GetSystemMetrics,
        GetWindowLongPtrW, GetWindowThreadProcessId, SendMessageTimeoutW,
        SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowsHookExW, UnhookWindowsHookEx,
        WindowFromPoint, GWL_EXSTYLE, HC_ACTION, LWA_ALPHA, MSG, MSLLHOOKSTRUCT,
        SM_CXDOUBLECLK, SM_CYDOUBLECLK, SMTO_ABORTIFHUNG, WH_MOUSE_LL, WM_LBUTTONDOWN,
        WS_EX_LAYERED,
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
            *self.last_target.lock().unwrap_or_else(PoisonError::into_inner) = target;
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

    fn bump(counter: &AtomicUsize) {
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
            .spawn(move || click_worker(app, rx))
        {
            eprintln!("desktop listener: could not start worker thread: {error}");
            return;
        }
        if let Err(error) = std::thread::Builder::new()
            .name("desktop-mouse-hook".into())
            .spawn(run_hook_thread)
        {
            eprintln!("desktop listener: could not start hook thread: {error}");
        }
    }

    pub fn set_listener_enabled(enabled: bool) {
        LISTENER_ENABLED.store(enabled, Ordering::Relaxed);
    }

    pub fn set_toggle_action(action: DesktopToggleAction) {
        FADE_INSTEAD_OF_HIDE.store(action == DesktopToggleAction::Transparent, Ordering::Relaxed);
    }

    pub fn debug_snapshot() -> DesktopListenerDebug {
        STATS.snapshot()
    }

    /// Brings the widget back to fully visible, whatever state it is in.
    pub fn restore(app: &AppHandle) -> Result<(), String> {
        let window = main_window(app)?;
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

    // ----- window effects --------------------------------------------------

    /// UI level (0 = opaque .. max = most transparent) -> Win32 alpha (255 = opaque).
    fn alpha_from_level(level: u8) -> u8 {
        u8::MAX - level.min(MAX_TRANSPARENCY_LEVEL)
    }

    fn configured_level(app: &AppHandle) -> u8 {
        app.state::<SettingsStore>().options().transparency_level
    }

    fn main_window(app: &AppHandle) -> Result<tauri::Window, String> {
        app.get_window(MAIN_LABEL)
            .ok_or_else(|| "Main window was not found".to_owned())
    }

    fn apply_alpha(app: &AppHandle, alpha: u8) {
        if let Some(window) = app.get_window(MAIN_LABEL) {
            set_window_alpha(&window, alpha);
        }
    }

    fn set_window_alpha(window: &tauri::Window, alpha: u8) {
        let Ok(hwnd) = window.hwnd() else {
            return;
        };
        let hwnd: HWND = hwnd.0 as _;

        // SAFETY: `hwnd` is the live top-level window handle of this process's main
        // window, and these calls are made on the UI thread.
        unsafe {
            if !LAYERED_READY.swap(true, Ordering::Relaxed) {
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED as isize);
            }
            SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA);
        }
        CURRENT_ALPHA.store(alpha, Ordering::Relaxed);
    }

    #[derive(Clone, Copy)]
    enum FadeEnd {
        /// Leave the window visible at the target alpha.
        Keep,
        /// Hide the window once the fade completes.
        Hide,
    }

    /// Animates from the current alpha to `target` on a helper thread. Starting a new
    /// fade cancels any fade already in progress.
    fn fade_to(app: &AppHandle, target: u8, end: FadeEnd) {
        let generation = FADE_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
        let from = CURRENT_ALPHA.load(Ordering::Relaxed);
        let app = app.clone();

        std::thread::spawn(move || {
            for step in 1..=FADE_STEPS {
                if FADE_GENERATION.load(Ordering::Relaxed) != generation {
                    return;
                }
                let progress = f32::from(step) / f32::from(FADE_STEPS);
                let alpha = (f32::from(from) + (f32::from(target) - f32::from(from)) * progress)
                    .round() as u8;

                let frame_app = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if FADE_GENERATION.load(Ordering::Relaxed) == generation {
                        apply_alpha(&frame_app, alpha);
                    }
                });

                if step < FADE_STEPS {
                    std::thread::sleep(FADE_STEP);
                }
            }

            if matches!(end, FadeEnd::Hide) {
                let final_app = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if FADE_GENERATION.load(Ordering::Relaxed) != generation {
                        return;
                    }
                    if let Some(window) = final_app.get_window(MAIN_LABEL) {
                        match window.hide() {
                            Ok(()) => WidgetState::Hidden.store(),
                            Err(error) => eprintln!("could not hide main window: {error}"),
                        }
                    }
                });
            }
        });
    }

    // ----- mouse hook ------------------------------------------------------

    #[derive(Clone, Copy)]
    struct ClickEvent {
        pt: POINT,
        /// `MSLLHOOKSTRUCT::time`: when the click happened (ms since boot).
        time: u32,
    }

    fn run_hook_thread() {
        // SAFETY: standard low-level-hook setup. The hook is installed on this thread,
        // which then pumps messages (required for WH_MOUSE_LL callbacks to run).
        unsafe {
            let module = GetModuleHandleW(std::ptr::null());
            let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), module, 0);
            if hook.is_null() {
                eprintln!(
                    "failed to install desktop mouse hook: {}",
                    std::io::Error::last_os_error()
                );
                return;
            }
            STATS.hook_installed.store(true, Ordering::Relaxed);

            let mut message: MSG = std::mem::zeroed();
            loop {
                let result = GetMessageW(&mut message, std::ptr::null_mut(), 0, 0);
                if result > 0 {
                    continue; // nothing to dispatch; this loop exists to service the hook
                }
                if result < 0 {
                    eprintln!(
                        "desktop mouse hook message loop failed: {}",
                        std::io::Error::last_os_error()
                    );
                }
                break;
            }

            UnhookWindowsHookEx(hook);
            STATS.hook_installed.store(false, Ordering::Relaxed);
        }
    }

    /// Must stay trivial: Windows enforces a timeout on low-level hooks and a slow
    /// callback delays every mouse event on the system.
    unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 && wparam == WM_LBUTTONDOWN as usize {
            // SAFETY: for WH_MOUSE_LL with code == HC_ACTION, `lparam` points to a
            // valid MSLLHOOKSTRUCT for the duration of this call.
            let event = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
            bump(&STATS.mouse_down);

            // Release builds skip the work entirely while the feature is off.
            if let Some(tx) = CLICK_TX
                .get()
                .filter(|_| LISTENER_ENABLED.load(Ordering::Relaxed) || cfg!(debug_assertions))
            {
                let _ = tx.send(ClickEvent {
                    pt: event.pt,
                    time: event.time,
                });
            }
        }
        // SAFETY: forwards the unmodified hook arguments to the next hook.
        unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
    }

    fn click_worker(app: AppHandle, clicks: Receiver<ClickEvent>) {
        let mut previous: Option<ClickEvent> = None;

        for click in clicks {
            let enabled = LISTENER_ENABLED.load(Ordering::Relaxed);

            // SAFETY: WindowFromPoint may be called from any thread; the returned
            // handle is only used immediately below.
            let hovered = unsafe { WindowFromPoint(click.pt) };
            // SAFETY: `hovered` is either null (handled) or a window handle.
            let Classified {
                surface,
                hovered_is_list_view,
                chain,
            } = unsafe { classify_window(hovered, cfg!(debug_assertions)) };
            if cfg!(debug_assertions) {
                STATS.set_target(chain);
            }

            let mut on_empty_desktop = false;
            if surface == Surface::Desktop {
                bump(&STATS.desktop_surface_clicks);
                if enabled {
                    // A click on the desktop can still be a click on an *icon*: both
                    // live in the same SysListView32, so only a hit-test can tell.
                    // Icon clicks never toggle and reset any pending double-click.
                    // SAFETY: `hovered` is a SysListView32 handle in explorer.exe.
                    if hovered_is_list_view && unsafe { click_hits_icon(hovered, click.pt) } {
                        bump(&STATS.icon_clicks_ignored);
                    } else {
                        on_empty_desktop = true;
                    }
                }
            }

            if !on_empty_desktop {
                previous = None;
                continue;
            }

            if previous
                .take()
                .is_some_and(|first| is_double_click(&first, &click))
            {
                bump(&STATS.double_clicks);
                dispatch_toggle(&app);
            } else {
                previous = Some(click);
            }
        }
    }

    fn is_double_click(first: &ClickEvent, second: &ClickEvent) -> bool {
        // SAFETY: plain system-metric queries with no preconditions.
        let (max_time, max_dx, max_dy) = unsafe {
            (
                GetDoubleClickTime(),
                GetSystemMetrics(SM_CXDOUBLECLK) / 2,
                GetSystemMetrics(SM_CYDOUBLECLK) / 2,
            )
        };
        second.time.wrapping_sub(first.time) <= max_time
            && (second.pt.x - first.pt.x).abs() <= max_dx
            && (second.pt.y - first.pt.y).abs() <= max_dy
    }

    fn dispatch_toggle(app: &AppHandle) {
        bump(&STATS.toggle_dispatches);
        let ui_app = app.clone();
        if let Err(error) = app.run_on_main_thread(move || {
            bump(&STATS.toggle_ui_thread);
            match toggle(&ui_app) {
                Ok(()) => STATS.set_toggle_result("Toggle completed"),
                Err(error) => STATS.set_toggle_result(format!("Toggle failed: {error}")),
            }
        }) {
            STATS.set_toggle_result(format!("Could not schedule UI-thread toggle: {error}"));
        }
    }

    // ----- click classification --------------------------------------------

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Surface {
        Desktop,
        Other,
    }

    struct Classified {
        surface: Surface,
        /// The window directly under the pointer is a SysListView32 (icon list).
        hovered_is_list_view: bool,
        /// Class-name chain for the debug panel; only built when requested.
        chain: String,
    }

    fn utf16_eq(units: &[u16], text: &str) -> bool {
        units.iter().copied().eq(text.encode_utf16())
    }

    /// Walks the ancestor chain of `start` and decides whether the click landed on the
    /// desktop (a window under `Progman`) rather than, say, a File Explorer window.
    ///
    /// NOTE: this requires `Progman`. On setups where the desktop is hosted by a
    /// `WorkerW` window instead (e.g. slideshow wallpaper), clicks won't match; accept
    /// `WorkerW` here too if you need to support that.
    ///
    /// # Safety
    /// `start` must be null or a window handle.
    unsafe fn classify_window(start: HWND, want_chain: bool) -> Classified {
        if start.is_null() {
            return Classified {
                surface: Surface::Other,
                hovered_is_list_view: false,
                chain: "No window at pointer".to_owned(),
            };
        }

        let mut chain = Vec::new(); // stays unallocated unless `want_chain`
        let (mut saw_progman, mut saw_explorer) = (false, false);
        let mut hovered_is_list_view = false;
        let mut buffer = [0u16; 64];
        let mut window = start;

        for depth in 0..MAX_ANCESTOR_DEPTH {
            // SAFETY: `window` is non-null here and `buffer` is a valid writable buffer.
            let length =
                unsafe { GetClassNameW(window, buffer.as_mut_ptr(), buffer.len() as i32) };
            let name = &buffer[..length.max(0) as usize];

            saw_progman |= utf16_eq(name, "Progman");
            saw_explorer |= utf16_eq(name, "CabinetWClass") || utf16_eq(name, "ExploreWClass");
            if depth == 0 {
                hovered_is_list_view = utf16_eq(name, "SysListView32");
            }
            if want_chain {
                chain.push(String::from_utf16_lossy(name));
            }

            // SAFETY: `window` is a valid window handle.
            window = unsafe { GetParent(window) };
            if window.is_null() {
                break;
            }
        }

        Classified {
            surface: if saw_progman && !saw_explorer {
                Surface::Desktop
            } else {
                Surface::Other
            },
            hovered_is_list_view,
            chain: chain.join(" > "),
        }
    }

    // ----- cross-process icon hit-test (RAII wrappers) ---------------------

    #[repr(C)]
    struct LvHitTestInfo {
        pt: POINT,
        flags: u32,
        i_item: i32,
        i_sub_item: i32,
        i_group: i32,
    }

    struct ProcessHandle(HANDLE);

    impl ProcessHandle {
        fn open_for_memory_access(process_id: u32) -> Option<Self> {
            // SAFETY: plain FFI call; a null result is handled.
            let handle = unsafe {
                OpenProcess(
                    PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
                    0,
                    process_id,
                )
            };
            (!handle.is_null()).then_some(Self(handle))
        }
    }

    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            // SAFETY: the handle came from OpenProcess and is closed exactly once.
            unsafe { CloseHandle(self.0) };
        }
    }

    /// A buffer allocated inside another process; freed on drop.
    /// Declare it *after* the `ProcessHandle` it borrows so it drops first.
    struct RemoteAlloc {
        process: HANDLE,
        ptr: *mut c_void,
    }

    impl RemoteAlloc {
        fn new(process: &ProcessHandle, size: usize) -> Option<Self> {
            // SAFETY: plain FFI call on a valid process handle; null is handled.
            let ptr = unsafe {
                VirtualAllocEx(
                    process.0,
                    std::ptr::null(),
                    size,
                    MEM_COMMIT | MEM_RESERVE,
                    PAGE_READWRITE,
                )
            };
            (!ptr.is_null()).then_some(Self {
                process: process.0,
                ptr,
            })
        }
    }

    impl Drop for RemoteAlloc {
        fn drop(&mut self) {
            // SAFETY: `ptr` was allocated by VirtualAllocEx in `process` and is freed once.
            unsafe { VirtualFreeEx(self.process, self.ptr, 0, MEM_RELEASE) };
        }
    }

    /// True if `screen_point` lands on an actual icon inside the SysListView32
    /// `list_view`, rather than on empty desktop space.
    ///
    /// The list view belongs to explorer.exe, so LVM_HITTEST's LPARAM can't point at a
    /// struct in *our* address space (explorer would dereference it in *its* space and
    /// crash). We allocate the struct inside explorer, send the message with a timeout
    /// (so a hung explorer can't block us), read the result back, and free it.
    ///
    /// Any failure returns `false` ("not an icon"), matching the old behaviour.
    ///
    /// # Safety
    /// `list_view` must be a SysListView32 window handle.
    unsafe fn click_hits_icon(list_view: HWND, screen_point: POINT) -> bool {
        let mut client_point = screen_point;
        // SAFETY: valid window handle and a valid out-pointer.
        if unsafe { ScreenToClient(list_view, &mut client_point) } == 0 {
            return false;
        }

        let mut process_id = 0u32;
        // SAFETY: valid window handle and a valid out-pointer.
        unsafe { GetWindowThreadProcessId(list_view, &mut process_id) };
        if process_id == 0 {
            return false;
        }

        let Some(process) = ProcessHandle::open_for_memory_access(process_id) else {
            return false;
        };
        let size = std::mem::size_of::<LvHitTestInfo>();
        let Some(remote) = RemoteAlloc::new(&process, size) else {
            return false;
        };

        let mut info = LvHitTestInfo {
            pt: client_point,
            flags: 0,
            i_item: -1,
            i_sub_item: 0,
            i_group: 0,
        };

        // SAFETY: `remote` is `size` bytes of committed memory in `process`; `info` is
        // a valid local `repr(C)` struct of the same size.
        unsafe {
            let written = WriteProcessMemory(
                process.0,
                remote.ptr,
                &info as *const LvHitTestInfo as *const c_void,
                size,
                std::ptr::null_mut(),
            );
            if written == 0 {
                return false;
            }

            let mut message_result = 0usize;
            let sent = SendMessageTimeoutW(
                list_view,
                LVM_HITTEST,
                0,
                remote.ptr as LPARAM,
                SMTO_ABORTIFHUNG,
                HIT_TEST_TIMEOUT_MS,
                &mut message_result,
            );
            if sent == 0 {
                return false;
            }

            let read = ReadProcessMemory(
                process.0,
                remote.ptr,
                &mut info as *mut LvHitTestInfo as *mut c_void,
                size,
                std::ptr::null_mut(),
            );
            if read == 0 {
                return false;
            }
        }

        info.i_item >= 0
    }
}

/// No-op stand-in so the crate (and the command handlers) compile on other platforms.
#[cfg(not(windows))]
mod desktop {
    use super::{DesktopListenerDebug, DesktopToggleAction};
    use tauri::AppHandle;

    pub fn init(_app: AppHandle, _enabled: bool, _action: DesktopToggleAction) {}
    pub fn set_listener_enabled(_enabled: bool) {}
    pub fn set_toggle_action(_action: DesktopToggleAction) {}
    pub fn restore(_app: &AppHandle) -> Result<(), String> {
        Ok(())
    }
    pub fn test_toggle(_app: &AppHandle) -> Result<(), String> {
        Err("Desktop toggle is only supported on Windows".to_owned())
    }
    pub fn apply_transparency_level(_app: &AppHandle) {}
    pub fn debug_snapshot() -> DesktopListenerDebug {
        DesktopListenerDebug::default()
    }
}

fn restore_widget(app: &AppHandle) {
    if let Err(error) = desktop::restore(app) {
        eprintln!("could not restore widget: {error}");
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn options_view(app: &AppHandle, options: Options) -> OptionsView {
    OptionsView {
        options,
        // The OS autostart entry is authoritative (the user can change it in Task Manager).
        open_at_login: app.autolaunch().is_enabled().unwrap_or(false),
    }
}

#[tauri::command]
fn get_options(app: AppHandle, store: State<'_, SettingsStore>) -> OptionsView {
    options_view(&app, store.options())
}

#[tauri::command]
fn is_debug_build() -> bool {
    cfg!(debug_assertions)
}

#[tauri::command]
fn get_desktop_listener_debug() -> DesktopListenerDebug {
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
) -> OptionsView {
    if let Some(enabled) = patch.open_at_login {
        set_autostart(&app, enabled);
    }

    let options = store.update(|settings| {
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
    });

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

    options_view(&app, options)
}

#[tauri::command]
fn update_transparency_level(
    app: AppHandle,
    store: State<'_, SettingsStore>,
    transparency_level: u8,
) {
    let level = transparency_level.min(MAX_TRANSPARENCY_LEVEL);
    store.update(|settings| settings.options.transparency_level = level);
    desktop::apply_transparency_level(&app);
}

#[tauri::command]
fn reset_options(app: AppHandle, store: State<'_, SettingsStore>) -> OptionsView {
    desktop::set_listener_enabled(false);
    desktop::set_toggle_action(DesktopToggleAction::default());
    restore_widget(&app);
    set_autostart(&app, false);

    let options = store.update(|settings| {
        settings.options = Options::default();
        settings.options.clone()
    });
    options_view(&app, options)
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

    let mut builder =
        WebviewWindowBuilder::new(&app, OPTIONS_LABEL, WebviewUrl::App("options.html".into()))
            .title("Widget Options")
            .inner_size(560.0, 530.0)
            .resizable(false)
            .minimizable(false)
            .maximizable(false);

    if let Some(main_window) = app.get_window(MAIN_LABEL) {
        if let (Ok(position), Ok(scale)) = (main_window.outer_position(), main_window.scale_factor())
        {
            builder = builder.position(
                f64::from(position.x) / scale + 32.0,
                f64::from(position.y) / scale + 32.0,
            );
        }
    }

    builder.build().map(|_| ()).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Window helpers
// ---------------------------------------------------------------------------

/// Position and size of the calendar webview inside a window of `size` physical pixels.
fn calendar_bounds(
    size: PhysicalSize<u32>,
    scale: f64,
) -> (LogicalPosition<f64>, LogicalSize<f64>) {
    let width = f64::from(size.width) / scale;
    let height = f64::from(size.height) / scale;
    (
        LogicalPosition::new(0.0, TITLEBAR_HEIGHT),
        LogicalSize::new(width, (height - TITLEBAR_HEIGHT).max(0.0)),
    )
}

fn layout_calendar(app: &AppHandle, window: &tauri::Window, size: PhysicalSize<u32>) {
    let Some(calendar) = app.get_webview(CALENDAR_LABEL) else {
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let (position, size) = calendar_bounds(size, scale);
    if let Err(error) = calendar.set_position(tauri::Position::Logical(position)) {
        eprintln!("could not position calendar: {error}");
    }
    if let Err(error) = calendar.set_size(tauri::Size::Logical(size)) {
        eprintln!("could not resize calendar: {error}");
    }
}

/// Does `bounds` overlap some monitor enough for the title bar to be reachable?
fn bounds_on_screen(window: &WebviewWindow, bounds: &WindowBounds) -> bool {
    let Ok(monitors) = window.available_monitors() else {
        return true; // can't tell; trust the saved bounds
    };
    let (left, top) = (i64::from(bounds.x), i64::from(bounds.y));
    let (right, bottom) = (left + i64::from(bounds.width), top + i64::from(bounds.height));

    monitors.iter().any(|monitor| {
        let position = monitor.position();
        let size = monitor.size();
        let (m_left, m_top) = (i64::from(position.x), i64::from(position.y));
        let (m_right, m_bottom) = (m_left + i64::from(size.width), m_top + i64::from(size.height));

        let overlap_w = right.min(m_right) - left.max(m_left);
        let overlap_h = bottom.min(m_bottom) - top.max(m_top);
        overlap_w >= MIN_VISIBLE_PX && overlap_h >= MIN_VISIBLE_PX
    })
}

fn restore_window_bounds(window: &WebviewWindow, bounds: &WindowBounds) {
    if !bounds_on_screen(window, bounds) {
        eprintln!("saved window bounds are off-screen; using default placement");
        return;
    }
    // Stored as physical pixels. Position is outer, size is inner.
    if let Err(error) = window.set_position(PhysicalPosition::new(bounds.x, bounds.y)) {
        eprintln!("could not restore window position: {error}");
    }
    if let Err(error) = window.set_size(PhysicalSize::new(bounds.width, bounds.height)) {
        eprintln!("could not restore window size: {error}");
    }
}

fn save_window_bounds(app: &AppHandle, window: &tauri::Window) {
    // A minimized window reports a bogus position (about -32000, -32000).
    if window.is_minimized().unwrap_or(false) {
        return;
    }
    // set_position uses outer coords but set_size uses inner coords, so match on save.
    let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) else {
        return;
    };
    app.state::<SettingsStore>().update(|settings| {
        if settings.options.remember_window_bounds {
            settings.window_bounds = Some(WindowBounds {
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
            });
        }
    });
}

fn build_calendar_webview(handle: &AppHandle) -> WebviewBuilder<tauri::Wry> {
    let mut builder = WebviewBuilder::new(CALENDAR_LABEL, WebviewUrl::External(calendar_url()))
        // window.open / target="_blank": sign-in popups stay in-app, everything
        // else goes to the default browser.
        .on_new_window({
            let handle = handle.clone();
            move |url, features| {
                let in_app = is_auth_popup(&url);
                if cfg!(debug_assertions) {
                    let target = if in_app { "in-app popup" } else { "default browser" };
                    eprintln!("popup request: {url} -> {target}");
                }
                if !in_app {
                    open_externally(&handle, &url);
                    return NewWindowResponse::Deny;
                }

                let label = format!(
                    "notion-calendar-popup-{}",
                    POPUP_WINDOW_ID.fetch_add(1, Ordering::Relaxed)
                );
                let mut popup = WebviewWindowBuilder::new(
                    &handle,
                    &label,
                    WebviewUrl::External(about_blank()),
                )
                .window_features(features)
                .title("Notion Calendar Sign In")
                .on_document_title_changed(|window, title| {
                    if !log_debug_title("popup", &title) {
                        let _ = window.set_title(&title);
                    }
                })
                .on_page_load(|window, payload| {
                    let finished = matches!(payload.event(), PageLoadEvent::Finished);
                    if cfg!(debug_assertions) {
                        log_page_load("popup", finished, payload.url());
                    }
                    if AUTO_CLOSE_POPUP_CALLBACK && finished && is_popup_callback(payload.url()) {
                        std::thread::spawn(move || {
                            std::thread::sleep(POPUP_CALLBACK_CLOSE_DELAY);
                            if cfg!(debug_assertions) {
                                eprintln!("[popup] auto-closing callback popup");
                            }
                            if let Err(error) = window.close() {
                                eprintln!("could not close sign-in popup: {error}");
                            }
                        });
                    }
                });
                if cfg!(debug_assertions) {
                    popup = popup.initialization_script(debug_script(DEBUG_JS_POPUP));
                }

                match popup.build() {
                    Ok(window) => NewWindowResponse::Create { window },
                    Err(error) => {
                        eprintln!("could not create sign-in popup: {error}");
                        NewWindowResponse::Deny
                    }
                }
            }
        })
        // Same-window navigation (plain links without a target): keep Notion and
        // sign-in pages in the widget, send everything else to the browser.
        .on_navigation({
            let handle = handle.clone();
            move |url| {
                if !GUARD_SAME_WINDOW_NAVIGATION {
                    return true;
                }
                let internal = is_internal_url(url);
                if !internal {
                    open_externally(&handle, url);
                }
                internal
            }
        })
        .background_color(Color(0x1a, 0x1a, 0x1a, 0xff));

    if cfg!(debug_assertions) {
        builder = builder
            .initialization_script(debug_script(DEBUG_JS_MAIN))
            .on_document_title_changed(|_webview, title| {
                log_debug_title("main", &title);
            })
            .on_page_load(|_webview, payload| {
                log_page_load(
                    "main",
                    matches!(payload.event(), PageLoadEvent::Finished),
                    payload.url(),
                );
            });
    }
    builder
}

// ---------------------------------------------------------------------------
// App setup
// ---------------------------------------------------------------------------

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();

    app.manage(SettingsStore::load(settings_path(&handle)?));
    let settings = app.state::<SettingsStore>().snapshot();

    desktop::init(
        handle.clone(),
        settings.options.toggle_on_desktop_double_click,
        settings.options.desktop_toggle_action,
    );
    remove_legacy_electron_autostart();
    // Autostart is intentionally NOT re-synced from settings here: the OS entry is the
    // source of truth, so disabling it in Task Manager sticks.

    let main_window = app
        .get_webview_window(MAIN_LABEL)
        .ok_or("main window not found")?;
    let main_base_window = app.get_window(MAIN_LABEL).ok_or("main window not found")?;

    let title = if cfg!(debug_assertions) {
        "What-N-When (DEV BUILD)"
    } else {
        "What-N-When"
    };
    if let Err(error) = main_window.set_title(title) {
        eprintln!("could not set window title: {error}");
    }

    if let Some(bounds) = settings
        .window_bounds
        .as_ref()
        .filter(|_| settings.options.remember_window_bounds)
    {
        restore_window_bounds(&main_window, bounds);
    }

    // The calendar is a child webview filling the space below the titlebar.
    let (position, size) = calendar_bounds(main_window.inner_size()?, main_window.scale_factor()?);
    main_base_window.add_child(build_calendar_webview(&handle), position, size)?;

    main_window.on_window_event({
        let handle = handle.clone();
        let base_window = main_base_window.clone();
        move |event| match event {
            tauri::WindowEvent::Resized(size) => layout_calendar(&handle, &base_window, *size),
            tauri::WindowEvent::CloseRequested { .. } => save_window_bounds(&handle, &base_window),
            _ => {}
        }
    });

    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(setup)
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
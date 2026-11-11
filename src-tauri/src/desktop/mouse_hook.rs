use super::*;
use super::hit_testing::{classify_window, click_hits_icon, Surface};

// ----- mouse hook ------------------------------------------------------

#[derive(Clone, Copy)]
pub(super) struct ClickEvent {
    pt: POINT,
    /// `MSLLHOOKSTRUCT::time`: when the click happened (ms since boot).
    time: u32,
}

pub(super) fn run_hook_thread() {
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

pub(super) fn click_worker(app: AppHandle, clicks: Receiver<ClickEvent>) {
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

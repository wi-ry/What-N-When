use super::*;

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

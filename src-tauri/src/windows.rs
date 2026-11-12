use super::*;

// ---------------------------------------------------------------------------
// Window helpers
// ---------------------------------------------------------------------------

/// Position and size of the calendar webview inside a window of `size` physical pixels.
pub(super) fn calendar_bounds(
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
    let (right, bottom) = (
        left + i64::from(bounds.width),
        top + i64::from(bounds.height),
    );

    monitors.iter().any(|monitor| {
        let position = monitor.position();
        let size = monitor.size();
        let (m_left, m_top) = (i64::from(position.x), i64::from(position.y));
        let (m_right, m_bottom) = (
            m_left + i64::from(size.width),
            m_top + i64::from(size.height),
        );

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
    if let Err(error) = app.state::<SettingsStore>().update(|settings| {
        if settings.options.remember_window_bounds {
            settings.window_bounds = Some(WindowBounds {
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
            });
        }
    }) {
        eprintln!("settings: failed to save window bounds: {error}");
    }
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
                    let target = if in_app {
                        "in-app popup"
                    } else {
                        "default browser"
                    };
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
                let mut popup =
                    WebviewWindowBuilder::new(&handle, &label, WebviewUrl::External(about_blank()))
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
                            if AUTO_CLOSE_POPUP_CALLBACK
                                && finished
                                && is_popup_callback(payload.url())
                            {
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

pub(super) fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();

    app.manage(SettingsStore::load(settings_path(&handle)?));
    let settings = app.state::<SettingsStore>().snapshot();

    desktop::init(
        handle.clone(),
        settings.options.toggle_on_desktop_double_click,
        settings.options.desktop_toggle_action,
    );
    remove_legacy_electron_autostart(&handle.package_info().name);
    autostart::remove_stale_dev_entry(&handle);

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

    let activation_window = main_base_window.clone();
    main_window.on_window_event({
        let handle = handle.clone();
        let base_window = main_base_window.clone();
        move |event| match event {
            tauri::WindowEvent::Resized(size) => layout_calendar(&handle, &base_window, *size),
            tauri::WindowEvent::CloseRequested { .. } => save_window_bounds(&handle, &base_window),
            tauri::WindowEvent::Focused(false)
                if WIDGET_ACTIVATION_FOREGROUND.swap(false, Ordering::AcqRel) =>
            {
                if let Err(error) = main_base_window.set_always_on_top(false) {
                    eprintln!("widget activation: could not restore normal z-order: {error}");
                }
                if let Err(error) = main_base_window.set_always_on_bottom(true) {
                    eprintln!("widget activation: could not restore desktop position: {error}");
                }
            }
            _ => {}
        }
    });

    #[cfg(windows)]
    if let Err(error) = widget_activation::listen(handle.clone(), activation_window) {
        eprintln!("widget activation: could not start activation listener: {error}");
    }

    Ok(())
}

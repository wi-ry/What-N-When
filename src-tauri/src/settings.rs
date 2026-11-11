use super::*;

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(super) struct WindowBounds {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: u32,
    pub(super) height: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) enum DesktopToggleAction {
    #[default]
    Hide,
    Transparent,
}

/// Persisted options. `open_at_login` is intentionally absent: the OS autostart
/// entry is the single source of truth (see `OptionsView`).
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct Options {
    pub(super) remember_window_bounds: bool,
    pub(super) toggle_on_desktop_double_click: bool,
    pub(super) desktop_toggle_action: DesktopToggleAction,
    pub(super) transparency_level: u8,
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
pub(super) struct OptionsView {
    #[serde(flatten)]
    pub(super) options: Options,
    pub(super) open_at_login: bool,
    pub(super) open_at_login_available: bool,
}

pub(super) fn options_view(app: &AppHandle, options: Options) -> Result<OptionsView, String> {
    Ok(options_view_with_autostart(options, autostart_enabled(app)?))
}

pub(super) fn options_view_with_autostart(options: Options, open_at_login: bool) -> OptionsView {
    OptionsView {
        options,
        open_at_login,
        open_at_login_available: AUTOSTART_AVAILABLE,
    }
}

/// Partial update sent by the options window; absent fields are left unchanged.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct OptionsPatch {
    pub(super) remember_window_bounds: Option<bool>,
    pub(super) open_at_login: Option<bool>,
    pub(super) toggle_on_desktop_double_click: Option<bool>,
    pub(super) desktop_toggle_action: Option<DesktopToggleAction>,
    pub(super) transparency_level: Option<u8>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub(super) struct Settings {
    pub(super) options: Options,
    pub(super) window_bounds: Option<WindowBounds>,
}

/// Settings cached in Tauri managed state. Reads never touch the disk; every
/// mutation is written back atomically (temp file + rename).
pub(super) struct SettingsStore {
    path: PathBuf,
    inner: Mutex<Settings>,
}

impl SettingsStore {
    pub(super) fn load(path: PathBuf) -> Self {
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

    pub(super) fn snapshot(&self) -> Settings {
        self.lock().clone()
    }

    pub(super) fn options(&self) -> Options {
        self.lock().options.clone()
    }

    /// Persists an updated copy before replacing the in-memory settings.
    pub(super) fn update<R>(&self, apply: impl FnOnce(&mut Settings) -> R) -> io::Result<R> {
        let mut settings = self.lock();
        let mut updated = settings.clone();
        let result = apply(&mut updated);
        write_atomic(&self.path, &updated)?;
        *settings = updated;
        Ok(result)
    }
}

fn write_atomic(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(
        &tmp,
        serde_json::to_vec_pretty(settings).map_err(io::Error::other)?,
    )?;
    fs::rename(tmp, path)
}

pub(super) fn settings_path(app: &AppHandle) -> tauri::Result<PathBuf> {
    // Dev builds get their own file so they never clobber a real install's settings,
    // and (unlike a CWD-relative file) can't trigger the dev watcher into restarting.
    let file = if cfg!(debug_assertions) {
        "settings.dev.json"
    } else {
        "settings.json"
    };
    Ok(app.path().app_data_dir()?.join(file))
}

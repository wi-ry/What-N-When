const MAX_TRANSPARENCY_LEVEL = 230; // keep in sync with MAX_TRANSPARENCY_LEVEL in lib.rs

const rememberBoundsCheckbox = document.getElementById('remember-bounds');
const openAtLoginCheckbox = document.getElementById('open-at-login');
const desktopDoubleClickCheckbox = document.getElementById('desktop-double-click');
const desktopToggleMode = document.getElementById('desktop-toggle-mode');
const desktopToggleActionInputs = document.querySelectorAll('input[name="desktop-toggle-action"]');
const transparencyLevelSlider = document.getElementById('transparency-level');
const transparencySection = document.querySelector('.transparency-section');
const testToggleButton = document.getElementById('test-toggle-btn');
const saveButton = document.getElementById('save-btn');
const resetButton = document.getElementById('reset-btn');
const cancelButton = document.getElementById('cancel-btn');
const status = document.getElementById('status');
const listenerDebugOutput = document.getElementById('listener-debug-output');

const invoke = window.__TAURI__.core.invoke;
let options = {};
let debugInterval;

// The backend clamps to this value, so never let the slider offer more.
if (!transparencyLevelSlider.max || Number(transparencyLevelSlider.max) > MAX_TRANSPARENCY_LEVEL) {
  transparencyLevelSlider.max = MAX_TRANSPARENCY_LEVEL;
}

function setStatus(message) {
  status.textContent = message;
}

function selectedDesktopToggleAction() {
  return document.querySelector('input[name="desktop-toggle-action"]:checked').value;
}

function updateDesktopToggleMode() {
  desktopToggleMode.classList.toggle('is-disabled', !desktopDoubleClickCheckbox.checked);
}

function updateTransparencySectionVisibility() {
  const isTransparent = selectedDesktopToggleAction() === 'transparent';
  const isDesktopDoubleClickEnabled = desktopDoubleClickCheckbox.checked;
  transparencySection.hidden = !(isTransparent && isDesktopDoubleClickEnabled);
}

// Fills the form from an OptionsView returned by the backend.
function applyOptionsToForm(view) {
  rememberBoundsCheckbox.checked = Boolean(view.rememberWindowBounds);
  openAtLoginCheckbox.checked = Boolean(view.openAtLogin);
  desktopDoubleClickCheckbox.checked = Boolean(view.toggleOnDesktopDoubleClick);
  desktopToggleActionInputs.forEach((input) => {
    input.checked = input.value === (view.desktopToggleAction || 'hide');
  });
  transparencyLevelSlider.value = view.transparencyLevel;
  updateDesktopToggleMode();
  updateTransparencySectionVisibility();
}

async function applyBuildMode() {
  const isDebugBuild = await invoke('is_debug_build');
  document.querySelectorAll('.debug-only').forEach((element) => {
    element.hidden = !isDebugBuild;
  });
  return isDebugBuild;
}

async function updateDesktopListenerDebug() {
  try {
    const debug = await invoke('get_desktop_listener_debug');
    listenerDebugOutput.textContent = [
      `Hook installed: ${debug.hookInstalled ? 'yes' : 'no'}`,
      `Listener enabled: ${debug.listenerEnabled ? 'yes' : 'no'}`,
      `Mouse-down events captured: ${debug.mouseDownCount}`,
      `Desktop-surface clicks: ${debug.desktopSurfaceClickCount}`,
      `Double-clicks recognized: ${debug.doubleClickCount}`,
      `Toggle actions dispatched: ${debug.toggleDispatchCount}`,
      `UI-thread toggle callbacks: ${debug.toggleUiThreadCount}`,
      `Main window found: ${debug.mainWindowFoundCount}`,
      `Toggle actions completed: ${debug.toggleCount}`,
      `Last target: ${debug.lastTarget}`,
      `Last toggle result: ${debug.lastToggleResult}`,
    ].join('\n');
  } catch (error) {
    listenerDebugOutput.textContent = `Could not read listener diagnostics: ${error}`;
  }
}

async function loadOptions() {
  // Tauri/serde uses camelCase for JSON, so the property is transparencyLevel.
  options = await invoke('get_options');
  applyOptionsToForm(options);
}

// ----- event listeners (attached once, at top level) -----------------------

desktopToggleActionInputs.forEach((input) => {
  input.addEventListener('change', updateTransparencySectionVisibility);
});

desktopDoubleClickCheckbox.addEventListener('change', () => {
  updateDesktopToggleMode();
  updateTransparencySectionVisibility();
});

// Live preview while dragging. Note: the backend persists this immediately,
// which is why Cancel reverts it below.
transparencyLevelSlider.addEventListener('input', async () => {
  const level = parseInt(transparencyLevelSlider.value, 10);
  try {
    await invoke('update_transparency_level', { transparencyLevel: level });
  } catch (error) {
    console.error('Failed to update transparency level:', error);
    setStatus(`Preview failed: ${error}`);
  }
});

saveButton.addEventListener('click', async () => {
  const patch = {
    rememberWindowBounds: rememberBoundsCheckbox.checked,
    toggleOnDesktopDoubleClick: desktopDoubleClickCheckbox.checked,
    desktopToggleAction: selectedDesktopToggleAction(),
    transparencyLevel: parseInt(transparencyLevelSlider.value, 10),
  };
  // Only touch the OS autostart entry when the user actually changed it.
  if (openAtLoginCheckbox.checked !== Boolean(options.openAtLogin)) {
    patch.openAtLogin = openAtLoginCheckbox.checked;
  }

  try {
    options = await invoke('save_options', { patch });
    setStatus('Saved.');
    setTimeout(() => invoke('close_window'), 350);
  } catch (error) {
    console.error('save_options failed:', error);
    setStatus(`Save failed: ${error}`);
  }
});

testToggleButton.addEventListener('click', async () => {
  try {
    await invoke('test_desktop_toggle');
    setStatus('Test toggle completed.');
  } catch (error) {
    setStatus(`Test toggle failed: ${error}`);
  }
  updateDesktopListenerDebug();
});

resetButton.addEventListener('click', async () => {
  try {
    options = await invoke('reset_options');
    applyOptionsToForm(options);
    setStatus('Reset to defaults.');
  } catch (error) {
    console.error('reset_options failed:', error);
    setStatus(`Reset failed: ${error}`);
  }
});

cancelButton.addEventListener('click', async () => {
  try {
    // Undo any live-preview changes that were already persisted.
    await invoke('update_transparency_level', {
      transparencyLevel: options.transparencyLevel,
    });
  } catch (error) {
    console.error('Failed to revert transparency level:', error);
    setStatus(`Could not revert preview: ${error}`);
    return;
  }
  invoke('close_window');
});

window.addEventListener('DOMContentLoaded', () => {
  Promise.all([loadOptions(), applyBuildMode()])
    .then(([, isDebugBuild]) => {
      if (isDebugBuild) {
        updateDesktopListenerDebug();
        debugInterval = setInterval(updateDesktopListenerDebug, 750);
      }
    })
    .catch((error) => {
      console.error('Failed to load options:', error);
      setStatus('Failed to load options.');
    });
});

window.addEventListener('beforeunload', () => clearInterval(debugInterval));
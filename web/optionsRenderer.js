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
  const isTransparent = document.querySelector('input[name="desktop-toggle-action"]:checked').value === 'transparent';
  const isDesktopDoubleClickEnabled = desktopDoubleClickCheckbox.checked;
  transparencySection.hidden = !(isTransparent && isDesktopDoubleClickEnabled);
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
  options = await invoke('get_options');
  rememberBoundsCheckbox.checked = Boolean(options.rememberWindowBounds);
  openAtLoginCheckbox.checked = Boolean(options.openAtLogin);
  desktopDoubleClickCheckbox.checked = Boolean(options.toggleOnDesktopDoubleClick);
  desktopToggleActionInputs.forEach((input) => {
    input.checked = input.value === (options.desktopToggleAction || 'hide');
  });
  // Always update slider with saved value, regardless of section visibility
  // Note: Tauri/serde uses camelCase for JSON, so property is transparencyLevel not transparency_level
  transparencyLevelSlider.value = options.transparencyLevel;
  updateDesktopToggleMode();
  updateTransparencySectionVisibility();

  // Add event listeners for toggle action radio buttons
  desktopToggleActionInputs.forEach((input) => {
    input.addEventListener('change', updateTransparencySectionVisibility);
  });

  // Add event listener for transparency slider to preview changes in real-time
  transparencyLevelSlider.addEventListener('input', async () => {
    const level = parseInt(transparencyLevelSlider.value);
    try {
      await invoke('update_transparency_level', { transparencyLevel: level });
    } catch (error) {
      console.error('Failed to update transparency level:', error);
    }
  });
}

saveButton.addEventListener('click', async () => {
  await invoke('save_options', {
    rememberWindowBounds: rememberBoundsCheckbox.checked,
    openAtLogin: openAtLoginCheckbox.checked,
    toggleOnDesktopDoubleClick: desktopDoubleClickCheckbox.checked,
    desktopToggleAction: selectedDesktopToggleAction(),
    transparencyLevel: parseInt(transparencyLevelSlider.value),
  });

  setStatus('Saved.');
  setTimeout(() => invoke('close_window'), 350);
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
  options = await invoke('reset_options');
  rememberBoundsCheckbox.checked = Boolean(options.rememberWindowBounds);
  openAtLoginCheckbox.checked = Boolean(options.openAtLogin);
  desktopDoubleClickCheckbox.checked = Boolean(options.toggleOnDesktopDoubleClick);
  desktopToggleActionInputs.forEach((input) => {
    input.checked = input.value === options.desktopToggleAction;
  });
  transparencyLevelSlider.value = options.transparencyLevel;
  updateDesktopToggleMode();
  updateTransparencySectionVisibility();
  setStatus('Reset to defaults.');
});

cancelButton.addEventListener('click', () => {
  invoke('close_window');
});

desktopDoubleClickCheckbox.addEventListener('change', () => {
  updateDesktopToggleMode();
  updateTransparencySectionVisibility();
});

window.addEventListener('DOMContentLoaded', () => {
  Promise.all([loadOptions(), applyBuildMode()])
    .then(([, isDebugBuild]) => {
      if (isDebugBuild) {
        updateDesktopListenerDebug();
        debugInterval = setInterval(updateDesktopListenerDebug, 750);
      }
    })
    .catch(() => setStatus('Failed to load options.'));
});

let debugInterval;
window.addEventListener('beforeunload', () => clearInterval(debugInterval));

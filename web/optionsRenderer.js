const rememberBoundsCheckbox = document.getElementById('remember-bounds');
const openAtLoginCheckbox = document.getElementById('open-at-login');
const saveButton = document.getElementById('save-btn');
const resetButton = document.getElementById('reset-btn');
const cancelButton = document.getElementById('cancel-btn');
const status = document.getElementById('status');

const invoke = window.__TAURI__.core.invoke;

function setStatus(message) {
  status.textContent = message;
}

async function loadOptions() {
  const options = await invoke('get_options');
  rememberBoundsCheckbox.checked = Boolean(options.rememberWindowBounds);
  openAtLoginCheckbox.checked = Boolean(options.openAtLogin);
}

saveButton.addEventListener('click', async () => {
  await invoke('save_options', {
    rememberWindowBounds: rememberBoundsCheckbox.checked,
    openAtLogin: openAtLoginCheckbox.checked,
  });

  setStatus('Saved.');
  setTimeout(() => invoke('close_window'), 350);
});

resetButton.addEventListener('click', async () => {
  const options = await invoke('reset_options');
  rememberBoundsCheckbox.checked = Boolean(options.rememberWindowBounds);
  openAtLoginCheckbox.checked = Boolean(options.openAtLogin);
  setStatus('Reset to defaults.');
});

cancelButton.addEventListener('click', () => {
  invoke('close_window');
});

window.addEventListener('DOMContentLoaded', () => {
  loadOptions().catch(() => setStatus('Failed to load options.'));
});

const RESIZE_DIRECTION = {
  right: 'East',
  left: 'West',
  top: 'North',
  bottom: 'South',
  'top-right': 'NorthEast',
  'top-left': 'NorthWest',
  'bottom-right': 'SouthEast',
  'bottom-left': 'SouthWest',
};

const appWindow = window.__TAURI__.window.getCurrentWindow();
const invoke = window.__TAURI__.core.invoke;

// Runs a backend command and logs failures instead of dropping them silently.
function call(command, args) {
  return invoke(command, args).catch((error) => {
    console.error(`${command} failed:`, error);
  });
}

async function initDevBadge() {
  try {
    if (await invoke('is_debug_build')) {
      document.body.classList.add('dev-build');
    }
  } catch (e) {
    console.error('Failed to check debug build status:', e);
  }
}

initDevBadge();

document.querySelectorAll('.resize-handle').forEach((handle) => {
  handle.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return;
    const direction = RESIZE_DIRECTION[handle.dataset.edge];
    if (!direction) {
      console.error('Unknown resize edge:', handle.dataset.edge);
      return;
    }
    e.preventDefault();
    appWindow.startResizeDragging(direction).catch((error) => {
      console.error('startResizeDragging failed:', error);
    });
  });
});

document.getElementById('refresh-btn').addEventListener('click', () => call('refresh_calendar'));
document.getElementById('options-btn').addEventListener('click', () => call('open_options_window'));
document.getElementById('close-btn').addEventListener('click', () => call('close_window'));
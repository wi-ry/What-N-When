// Map HTML edge names to Tauri ResizeDirection enum values.
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

document.querySelectorAll('.resize-handle').forEach((handle) => {
  handle.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    appWindow.startResizeDragging(RESIZE_DIRECTION[handle.dataset.edge]);
  });
});

document.getElementById('refresh-btn').addEventListener('click', () => {
  invoke('refresh_calendar');
});

document.getElementById('options-btn').addEventListener('click', () => {
  invoke('open_options_window');
});

document.getElementById('close-btn').addEventListener('click', () => {
  invoke('close_window');
});

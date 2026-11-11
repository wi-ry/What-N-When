const test = require('node:test');
const assert = require('node:assert/strict');
const {
  buildOptionsPatch,
  shouldShowTransparencySection,
} = require('../web/optionsState');

test('transparency controls are visible only for enabled transparent desktop toggling', () => {
  assert.equal(shouldShowTransparencySection('transparent', true), true);
  assert.equal(shouldShowTransparencySection('hide', true), false);
  assert.equal(shouldShowTransparencySection('transparent', false), false);
});

test('options patch omits unchanged autostart state', () => {
  const patch = buildOptionsPatch({
    rememberWindowBounds: true,
    toggleOnDesktopDoubleClick: false,
    desktopToggleAction: 'hide',
    transparencyLevel: 64,
    openAtLogin: true,
  }, { openAtLogin: true });

  assert.deepEqual(patch, {
    rememberWindowBounds: true,
    toggleOnDesktopDoubleClick: false,
    desktopToggleAction: 'hide',
    transparencyLevel: 64,
  });
});

test('options patch includes an explicitly changed autostart state', () => {
  const patch = buildOptionsPatch({
    rememberWindowBounds: true,
    toggleOnDesktopDoubleClick: true,
    desktopToggleAction: 'transparent',
    transparencyLevel: 90,
    openAtLogin: false,
  }, { openAtLogin: true });

  assert.equal(patch.openAtLogin, false);
});

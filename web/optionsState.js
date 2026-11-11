(function (root, factory) {
  const optionsState = factory();

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = optionsState;
  }

  root.WhatNWhenOptions = optionsState;
})(globalThis, function () {
  function shouldShowTransparencySection(action, desktopDoubleClickEnabled) {
    return action === 'transparent' && desktopDoubleClickEnabled;
  }

  function buildOptionsPatch(values, previousOptions) {
    const patch = {
      rememberWindowBounds: values.rememberWindowBounds,
      toggleOnDesktopDoubleClick: values.toggleOnDesktopDoubleClick,
      desktopToggleAction: values.desktopToggleAction,
      transparencyLevel: values.transparencyLevel,
    };

    if (values.openAtLogin !== Boolean(previousOptions.openAtLogin)) {
      patch.openAtLogin = values.openAtLogin;
    }

    return patch;
  }

  return {
    buildOptionsPatch,
    shouldShowTransparencySection,
  };
});

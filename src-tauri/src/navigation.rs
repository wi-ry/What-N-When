use super::*;

/// External identity providers whose popups must stay in-app (OAuth flows rely on
/// `window.opener`, `postMessage` and cookies shared with the calendar webview).
/// If sign-in with some provider breaks, run a debug build and look for
/// "sent to browser" lines in stderr, then add the missing host here.
const AUTH_HOSTS: &[&str] = &[
    "accounts.google.com",
    "appleid.apple.com",
    "idmsa.apple.com",
    "login.microsoftonline.com",
    "login.live.com",
];

/// When true, same-window navigations (plain links with no target) to non-Notion sites
/// are sent to the default browser instead of loading inside the widget. Off by default:
/// it can interfere with sign-in redirects, and Notion Calendar opens event links via
/// `window.open`, which `on_new_window` already handles.
pub(super) const GUARD_SAME_WINDOW_NAVIGATION: bool = false;

/// Sign-in popups end on a Notion `...popupcallback` page (e.g. `/googlepopupcallback`,
/// `/microsoftpopupcallback`) that is supposed to post the result to the main window and
/// call `window.close()`. Tauri doesn't honour that call, so we close the popup ourselves.
/// Set `AUTO_CLOSE_POPUP_CALLBACK` to false to leave the popup open while debugging.
pub(super) const AUTO_CLOSE_POPUP_CALLBACK: bool = true;
/// How long the callback page gets to deliver its message before we close it. Kept long
/// while we diagnose the Microsoft flow; shorten once sign-in works.
pub(super) const POPUP_CALLBACK_CLOSE_DELAY: std::time::Duration = std::time::Duration::from_millis(1000);

/// Dev-build diagnostics. Scripts injected into the popup and the calendar webview report
/// what they see by setting `document.title` to `WNWDBG|<n>|<message>`; the title-changed
/// handlers print those lines to the terminal as `[popup] ...` / `[main] ...`.
/// (Remote pages have no Tauri IPC, so the title is the simplest one-way channel.)
const DEBUG_TITLE_PREFIX: &str = "WNWDBG|";

const DEBUG_JS_COMMON: &str = r#"
var PREFIX = '__PREFIX__', queue = [], seq = 0, timer = null;
function flush() {
  if (!queue.length) { timer = null; return; }
  if (!document.documentElement) { timer = setTimeout(flush, 50); return; }
  try { document.title = PREFIX + (seq++) + '|' + queue.shift(); } catch (e) {}
  timer = setTimeout(flush, 120);
}
function log(msg) { queue.push(msg); if (!timer) flush(); }
function openerState() {
  try { return window.opener ? 'present' : 'NULL'; } catch (e) { return 'error'; }
}
"#;

/// Injected into every page loaded inside a sign-in popup.
pub(super) const DEBUG_JS_POPUP: &str = r#"
(function () {
  __COMMON__
  var here = location.host + location.pathname;
  log('script start ' + here + ' opener=' + openerState());
  var origClose = window.close;
  window.close = function () { log('window.close() called on ' + here); return origClose.apply(window, arguments); };
  window.addEventListener('error', function (e) { log('js error: ' + e.message); });
  window.addEventListener('unhandledrejection', function (e) {
    log('unhandled rejection: ' + (e.reason && e.reason.message ? e.reason.message : e.reason));
  });
  window.addEventListener('pagehide', function () { log('pagehide ' + here); });
  document.addEventListener('DOMContentLoaded', function () {
    log('DOMContentLoaded ' + here + ' opener=' + openerState());
    // Probe the popup -> main channel: the main window logs if this arrives.
    try { if (window.opener) { window.opener.postMessage({ wnwProbe: here }, '*'); log('probe sent'); } }
    catch (e) { log('probe failed: ' + e.message); }
  });
})();
"#;

/// Injected into the calendar webview (main window).
pub(super) const DEBUG_JS_MAIN: &str = r#"
(function () {
  __COMMON__
  window.addEventListener('message', function (e) {
    var d = e.data;
    if (d && d.wnwProbe) { log('PROBE arrived from popup (' + d.wnwProbe + ') origin=' + e.origin); return; }
    var shape;
    try { shape = typeof d === 'string' ? 'string(len ' + d.length + ')' : 'keys=' + Object.keys(d || {}).join(','); }
    catch (x) { shape = '?'; }
    log('message from ' + e.origin + ' ' + shape);
    // Does the page move on after receiving the popup's result?
    [3000, 8000].forEach(function (ms) {
      setTimeout(function () { log(ms + 'ms after message, still on ' + location.host + location.pathname); }, ms);
    });
  }, true);
  var origOpen = window.open;
  window.open = function () {
    var args = arguments;
    var w = origOpen.apply(this, args);
    log('window.open(' + String(args[0]).split('?')[0] + ') -> ' + (w ? 'window' : 'NULL'));
    if (w) {
      var t = setInterval(function () {
        var c; try { c = w.closed; } catch (e) { c = 'err'; }
        if (c === true) { log('popup .closed became true'); clearInterval(t); }
      }, 250);
    }
    return w;
  };
  // Report failed (and auth-related) network calls and page errors, host + path only.
  var interesting = /login|oauth|auth|identity|session|verify|microsoft|google/i;
  function short(u) { try { return String(u).split('?')[0]; } catch (e) { return '?'; } }
  var origFetch = window.fetch;
  if (origFetch) {
    window.fetch = function (input) {
      var u = short(typeof input === 'string' ? input : (input && input.url) || '');
      return origFetch.apply(this, arguments).then(function (r) {
        if (!r.ok || interesting.test(u)) log('fetch ' + r.status + ' ' + u);
        return r;
      }, function (err) { log('fetch FAILED ' + u + ': ' + (err && err.message)); throw err; });
    };
  }
  var xhrOpen = XMLHttpRequest.prototype.open, xhrSend = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.open = function (m, u) { this.__wnw = m + ' ' + short(u); return xhrOpen.apply(this, arguments); };
  XMLHttpRequest.prototype.send = function () {
    var x = this;
    x.addEventListener('loadend', function () {
      if (x.status === 0 || x.status >= 400 || interesting.test(x.__wnw || '')) log('xhr ' + x.status + ' ' + x.__wnw);
    });
    return xhrSend.apply(this, arguments);
  };
  window.addEventListener('error', function (e) { log('js error: ' + e.message); });
  window.addEventListener('unhandledrejection', function (e) {
    log('unhandled rejection: ' + (e.reason && e.reason.message ? e.reason.message : e.reason));
  });
  document.addEventListener('securitypolicyviolation', function (e) {
    log('CSP violation: ' + e.violatedDirective + ' ' + short(e.blockedURI));
  });
  log('main debug script ready');
})();
"#;

pub(super) fn debug_script(template: &str) -> String {
    template
        .replace("__COMMON__", DEBUG_JS_COMMON)
        .replace("__PREFIX__", DEBUG_TITLE_PREFIX)
}

/// If `title` is a debug message from an injected script, print it and return true.
pub(super) fn log_debug_title(source: &str, title: &str) -> bool {
    match title.strip_prefix(DEBUG_TITLE_PREFIX) {
        Some(message) => {
            eprintln!("[{source}] {message}");
            true
        }
        None => false,
    }
}

/// Logs host + path only: query strings carry OAuth codes and state.
pub(super) fn log_page_load(source: &str, finished: bool, url: &Url) {
    eprintln!(
        "[{source}] page {} {}{}",
        if finished { "finished" } else { "started" },
        url.host_str().unwrap_or(""),
        url.path()
    );
}

/// Notion's own domains. Sign-in popups start on these (the exact path varies, so
/// we don't try to guess it), then hop to a provider in `AUTH_HOSTS` and back.
const NOTION_HOSTS: &[&str] = &["notion.so", "notion.com"];

pub(super) fn calendar_url() -> Url {
    Url::parse(CALENDAR_URL).expect("CALENDAR_URL is a valid URL")
}

pub(super) fn about_blank() -> Url {
    Url::parse("about:blank").expect("about:blank is a valid URL")
}

// ---------------------------------------------------------------------------
// Popup / navigation routing
// ---------------------------------------------------------------------------

/// True if `host` is `domain` or a subdomain of it (no allocation).
pub(super) fn host_matches(host: &str, domain: &str) -> bool {
    host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|rest| rest.ends_with('.'))
}

/// Should a `window.open` / `target="_blank"` request get an in-app popup?
///
/// Yes for Notion's own domains and the identity providers (that is how sign-in
/// works); everything else, such as Zoom, Meet or Docs links in an event, goes to the
/// default browser. Denying a sign-in popup makes `window.open` return null, which
/// Notion reports as "your browser is blocking popups", so when in doubt, allow.
pub(super) fn is_auth_popup(url: &Url) -> bool {
    // Script-driven popups often start at about:blank and navigate afterwards;
    // the destination isn't known yet, so keep them in-app.
    if url.scheme() == "about" {
        return true;
    }
    url.host_str().is_some_and(|host| {
        NOTION_HOSTS
            .iter()
            .chain(AUTH_HOSTS)
            .any(|d| host_matches(host, d))
    })
}

/// Should the calendar webview itself be allowed to navigate here?
pub(super) fn is_internal_url(url: &Url) -> bool {
    if matches!(url.scheme(), "about" | "blob" | "data") {
        return true;
    }
    url.host_str().is_some_and(|host| {
        NOTION_HOSTS
            .iter()
            .chain(AUTH_HOSTS)
            .any(|d| host_matches(host, d))
    })
}

/// Is this the final callback page of a sign-in popup?
pub(super) fn is_popup_callback(url: &Url) -> bool {
    url.host_str()
        .is_some_and(|host| NOTION_HOSTS.iter().any(|d| host_matches(host, d)))
        && url
            .path()
            .rsplit('/')
            .next()
            .is_some_and(|segment| segment.ends_with("popupcallback"))
}

pub(super) fn open_externally(app: &AppHandle, url: &Url) {
    // Only hand safe schemes to the OS; never file:, javascript:, custom protocols, etc.
    if !matches!(url.scheme(), "http" | "https" | "mailto") {
        return;
    }
    if cfg!(debug_assertions) {
        eprintln!("sent to browser: {url}");
    }
    if let Err(error) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("failed to open {url} in the default browser: {error}");
    }
}

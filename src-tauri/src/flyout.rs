//! The flyout window: where it sits, when it shows, and when it gets out of
//! the way.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, Theme};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::tray::{self, TrayState};

pub const MAIN_WINDOW: &str = "main";

/// Clicking the tray icon while the flyout has focus first blurs the window
/// (which hides it), then delivers the click. Without this grace period the
/// click would immediately re-open what the blur just closed.
const REOPEN_GRACE: Duration = Duration::from_millis(250);

#[derive(Default)]
pub struct FlyoutState {
    hidden_at: Mutex<Option<Instant>>,
}

/// The last counts the frontend reported, kept so the icon can be redrawn when
/// the system theme flips without waiting for the webview to notice.
#[derive(Default)]
pub struct TrayStateStore(Mutex<TrayState>);

impl TrayStateStore {
    fn get(&self) -> TrayState {
        *self.0.lock().unwrap()
    }
}

/// Which taskbar the icon has to sit on. Falls back to dark, the Windows 11
/// default, when the window has not reported a theme yet.
pub fn taskbar_theme(app: &AppHandle) -> Theme {
    app.get_webview_window(MAIN_WINDOW).and_then(|window| window.theme().ok()).unwrap_or(Theme::Dark)
}

pub fn repaint_tray(app: &AppHandle) {
    let state = app.state::<TrayStateStore>().get();
    tray::apply(app, taskbar_theme(app), state);
}

/// Records what the frontend reported and redraws the icon to match.
pub fn store_tray_state(app: &AppHandle, state: TrayState) {
    *app.state::<TrayStateStore>().0.lock().unwrap() = state;
    repaint_tray(app);
}

/// True when the window was hidden by a blur a moment ago, meaning this tray
/// click is the second half of a "click to close" gesture.
fn just_hidden(app: &AppHandle) -> bool {
    let state = app.state::<FlyoutState>();
    let mut hidden_at = state.hidden_at.lock().unwrap();
    match hidden_at.take() {
        Some(at) => at.elapsed() < REOPEN_GRACE,
        None => false,
    }
}

pub fn mark_hidden(app: &AppHandle) {
    let state = app.state::<FlyoutState>();
    *state.hidden_at.lock().unwrap() = Some(Instant::now());
}

pub fn show(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };

    // TrayCenter puts the panel *above* the icon (TrayBottomCenter would put it
    // below, which runs off-screen on a bottom taskbar). The macOS menu bar is at
    // the top, so there it is the other way round. The constrained variant
    // clamps the result to the monitor so the panel is never cut off at an edge.
    // It errors until the tray has reported its position, hence the fallback.
    #[cfg(target_os = "macos")]
    let (anchor, fallback) = (Position::TrayBottomCenter, Position::TopRight);
    #[cfg(not(target_os = "macos"))]
    let (anchor, fallback) = (Position::TrayCenter, Position::BottomRight);
    if window.move_window_constrained(anchor).is_err() {
        let _ = window.move_window(fallback);
    }

    let _ = window.show();
    let _ = window.set_focus();

    // The webview stays alive while hidden, so without this the panel would
    // show whatever was loaded the last time it was open. A timer cannot cover
    // it: WebView2 may throttle timers in a hidden window.
    let _ = window.emit("flyout-shown", ());
}

pub fn toggle(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    if just_hidden(app) {
        return;
    }
    show(app);
}

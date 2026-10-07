use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

const MAIN_WINDOW: &str = "main";

/// Clicking the tray icon while the flyout has focus first blurs the window
/// (which hides it), then delivers the click. Without this grace period the
/// click would immediately re-open what the blur just closed.
const REOPEN_GRACE: Duration = Duration::from_millis(250);

#[derive(Default)]
struct FlyoutState {
    hidden_at: Mutex<Option<Instant>>,
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

fn mark_hidden(app: &AppHandle) {
    let state = app.state::<FlyoutState>();
    *state.hidden_at.lock().unwrap() = Some(Instant::now());
}

fn show_flyout(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };

    // TrayCenter puts the panel *above* the icon (TrayBottomCenter would put it
    // below, which runs off-screen on a bottom taskbar). The constrained variant
    // clamps the result to the monitor so the panel is never cut off at an edge.
    // It errors until the tray has reported its position, hence the fallback.
    if window.move_window_constrained(Position::TrayCenter).is_err() {
        let _ = window.move_window(Position::BottomRight);
    }

    let _ = window.show();
    let _ = window.set_focus();
}

fn toggle_flyout(app: &AppHandle) {
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
    show_flyout(app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .manage(FlyoutState::default())
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("GitHub Deploy Management")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_flyout(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    let app = tray.app_handle();
                    tauri_plugin_positioner::on_tray_event(app, &event);

                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_flyout(app);
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Flyout behaviour: clicking anywhere outside dismisses the panel.
            WindowEvent::Focused(false) => {
                let _ = window.hide();
                mark_hidden(window.app_handle());
            }
            // Keep the app alive in the tray instead of quitting.
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

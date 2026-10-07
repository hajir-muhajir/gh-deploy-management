mod commands;
mod flyout;
pub mod github;
mod secrets;
mod tray;

use tauri::{Manager, WindowEvent};

use flyout::{FlyoutState, TrayStateStore};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // First in the chain on purpose: the callback has to be in place before
        // anything else runs, or a second launch would build its own tray icon
        // before being told it is a duplicate.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // The duplicate process exits on its own; the one already running
            // answers the shortcut click the way the tray icon would.
            flyout::show(app);
        }))
        // Windows registers the autostart entry under HKCU\…\CurrentVersion\Run.
        // `MacosLauncher` is required by the signature and ignored here.
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(FlyoutState::default())
        .manage(TrayStateStore::default())
        .invoke_handler(tauri::generate_handler![
            commands::github_save_token,
            commands::github_token_info,
            commands::github_clear_token,
            commands::github_list_repos,
            commands::github_add_repo,
            commands::github_probe_actions,
            commands::github_list_workflows,
            commands::github_list_runs,
            commands::github_list_releases,
            commands::github_list_pulls,
            commands::github_list_dispatchable,
            commands::github_dispatch_workflow,
            commands::github_cancel_run,
            commands::github_rerun_run,
            commands::set_tray_state,
            commands::autostart_enabled,
            commands::set_autostart,
        ])
        .setup(|app| {
            tray::setup(app)?;
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Windows does not recolour tray icons, so the glyph has to be
            // swapped for the one that suits the new taskbar.
            WindowEvent::ThemeChanged(_) => flyout::repaint_tray(window.app_handle()),
            // Flyout behaviour: clicking anywhere outside dismisses the panel.
            WindowEvent::Focused(false) => {
                let _ = window.hide();
                flyout::mark_hidden(window.app_handle());
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

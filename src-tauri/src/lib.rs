mod github;
mod secrets;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

use github::{
    ApiError, DispatchWorkflow, GitHub, ReleasesPage, RepoInfo, ReposPage, RunInfo, TokenInfo,
    WorkflowInfo,
};

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

// ── GitHub commands ─────────────────────────────────────────────────────────
//
// The token lives in the OS credential store and is read here per call; it is
// never handed to the webview, which only receives the derived TokenInfo.

fn client_from_keychain() -> Result<GitHub, ApiError> {
    let token = secrets::read()
        .map_err(ApiError::keyring)?
        .ok_or_else(ApiError::not_connected)?;
    GitHub::new(&token)
}

/// Validates before storing: an invalid token never reaches the credential
/// store, so a failed save leaves any previously working token intact.
#[tauri::command]
async fn github_save_token(
    token: String,
    repo_for_actions: Option<String>,
) -> Result<Option<TokenInfo>, ApiError> {
    let token = token.trim().to_string();
    if token.is_empty() {
        secrets::clear().map_err(ApiError::keyring)?;
        return Ok(None);
    }

    let info = GitHub::new(&token)?
        .validate(repo_for_actions.as_deref())
        .await?;

    secrets::store(&token).map_err(ApiError::keyring)?;
    Ok(Some(info))
}

/// Startup check: revalidates the stored token so a revoked one is caught.
#[tauri::command]
async fn github_token_info(
    repo_for_actions: Option<String>,
) -> Result<Option<TokenInfo>, ApiError> {
    let Some(token) = secrets::read().map_err(ApiError::keyring)? else {
        return Ok(None);
    };
    let info = GitHub::new(&token)?
        .validate(repo_for_actions.as_deref())
        .await?;
    Ok(Some(info))
}

#[tauri::command]
async fn github_clear_token() -> Result<(), ApiError> {
    secrets::clear().map_err(ApiError::keyring)
}

#[tauri::command]
async fn github_list_repos() -> Result<ReposPage, ApiError> {
    client_from_keychain()?.list_repos().await
}

/// Fallback for repositories that do not show up in `/user/repos`.
#[tauri::command]
async fn github_add_repo(owner: String, name: String) -> Result<RepoInfo, ApiError> {
    client_from_keychain()?.get_repo(&owner, &name).await
}

#[tauri::command]
async fn github_probe_actions(owner: String, name: String) -> Result<bool, ApiError> {
    Ok(client_from_keychain()?.probe_actions(&owner, &name).await)
}

#[tauri::command]
async fn github_list_workflows(owner: String, name: String) -> Result<Vec<WorkflowInfo>, ApiError> {
    client_from_keychain()?.list_workflows(&owner, &name).await
}

#[tauri::command]
async fn github_list_runs(owner: String, name: String) -> Result<Vec<RunInfo>, ApiError> {
    client_from_keychain()?.list_runs(&owner, &name).await
}

#[tauri::command]
async fn github_list_releases(owner: String, name: String) -> Result<ReleasesPage, ApiError> {
    client_from_keychain()?.list_releases(&owner, &name).await
}

#[tauri::command]
async fn github_list_dispatchable(
    owner: String,
    name: String,
) -> Result<Vec<DispatchWorkflow>, ApiError> {
    client_from_keychain()?.list_dispatchable(&owner, &name).await
}

#[tauri::command]
async fn github_dispatch_workflow(
    owner: String,
    name: String,
    workflow_id: u64,
    git_ref: String,
    inputs: std::collections::HashMap<String, String>,
) -> Result<(), ApiError> {
    client_from_keychain()?
        .dispatch_workflow(&owner, &name, workflow_id, &git_ref, inputs)
        .await
}

#[tauri::command]
async fn github_cancel_run(owner: String, name: String, run_id: u64) -> Result<(), ApiError> {
    client_from_keychain()?.cancel_run(&owner, &name, run_id).await
}

#[tauri::command]
async fn github_rerun_run(owner: String, name: String, run_id: u64) -> Result<(), ApiError> {
    client_from_keychain()?.rerun_run(&owner, &name, run_id).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .manage(FlyoutState::default())
        .invoke_handler(tauri::generate_handler![
            github_save_token,
            github_token_info,
            github_clear_token,
            github_list_repos,
            github_add_repo,
            github_probe_actions,
            github_list_workflows,
            github_list_runs,
            github_list_releases,
            github_list_dispatchable,
            github_dispatch_workflow,
            github_cancel_run,
            github_rerun_run,
        ])
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

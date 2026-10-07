//! The surface the webview can call.
//!
//! The token lives in the OS credential store and is read here per call; it is
//! never handed to the webview, which only receives the derived `TokenInfo`.

use std::collections::HashMap;

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

use crate::flyout;
use crate::github::{
    ApiError, DispatchWorkflow, GitHub, PullRequestsPage, ReleasesPage, RepoInfo, ReposPage, RunInfo,
    TokenInfo, WorkflowInfo,
};
use crate::secrets;
use crate::tray::TrayState;

pub(crate) fn client_from_keychain() -> Result<GitHub, ApiError> {
    let token = secrets::read().map_err(ApiError::keyring)?.ok_or_else(ApiError::not_connected)?;
    GitHub::new(&token)
}

/// Validates before storing: an invalid token never reaches the credential
/// store, so a failed save leaves any previously working token intact.
#[tauri::command]
pub async fn github_save_token(
    token: String,
    repo_for_actions: Option<String>,
) -> Result<Option<TokenInfo>, ApiError> {
    let token = token.trim().to_string();
    if token.is_empty() {
        secrets::clear().map_err(ApiError::keyring)?;
        return Ok(None);
    }

    let info = GitHub::new(&token)?.validate(repo_for_actions.as_deref()).await?;

    secrets::store(&token).map_err(ApiError::keyring)?;
    Ok(Some(info))
}

/// Startup check: revalidates the stored token so a revoked one is caught.
#[tauri::command]
pub async fn github_token_info(repo_for_actions: Option<String>) -> Result<Option<TokenInfo>, ApiError> {
    let Some(token) = secrets::read().map_err(ApiError::keyring)? else {
        return Ok(None);
    };
    let info = GitHub::new(&token)?.validate(repo_for_actions.as_deref()).await?;
    Ok(Some(info))
}

#[tauri::command]
pub async fn github_clear_token() -> Result<(), ApiError> {
    secrets::clear().map_err(ApiError::keyring)
}

#[tauri::command]
pub async fn github_list_repos() -> Result<ReposPage, ApiError> {
    client_from_keychain()?.list_repos().await
}

/// Fallback for repositories that do not show up in `/user/repos`.
#[tauri::command]
pub async fn github_add_repo(owner: String, name: String) -> Result<RepoInfo, ApiError> {
    client_from_keychain()?.get_repo(&owner, &name).await
}

#[tauri::command]
pub async fn github_probe_actions(owner: String, name: String) -> Result<bool, ApiError> {
    Ok(client_from_keychain()?.probe_actions(&owner, &name).await)
}

#[tauri::command]
pub async fn github_list_workflows(owner: String, name: String) -> Result<Vec<WorkflowInfo>, ApiError> {
    client_from_keychain()?.list_workflows(&owner, &name).await
}

#[tauri::command]
pub async fn github_list_runs(owner: String, name: String) -> Result<Vec<RunInfo>, ApiError> {
    client_from_keychain()?.list_runs(&owner, &name).await
}

#[tauri::command]
pub async fn github_list_releases(owner: String, name: String) -> Result<ReleasesPage, ApiError> {
    client_from_keychain()?.list_releases(&owner, &name).await
}

#[tauri::command]
pub async fn github_list_pulls(owner: String, name: String) -> Result<PullRequestsPage, ApiError> {
    client_from_keychain()?.list_pulls(&owner, &name).await
}

#[tauri::command]
pub async fn github_list_dispatchable(
    owner: String,
    name: String,
) -> Result<Vec<DispatchWorkflow>, ApiError> {
    client_from_keychain()?.list_dispatchable(&owner, &name).await
}

#[tauri::command]
pub async fn github_dispatch_workflow(
    owner: String,
    name: String,
    workflow_id: u64,
    git_ref: String,
    inputs: HashMap<String, String>,
) -> Result<(), ApiError> {
    client_from_keychain()?.dispatch_workflow(&owner, &name, workflow_id, &git_ref, inputs).await
}

#[tauri::command]
pub async fn github_cancel_run(owner: String, name: String, run_id: u64) -> Result<(), ApiError> {
    client_from_keychain()?.cancel_run(&owner, &name, run_id).await
}

#[tauri::command]
pub async fn github_rerun_run(owner: String, name: String, run_id: u64) -> Result<(), ApiError> {
    client_from_keychain()?.rerun_run(&owner, &name, run_id).await
}

/// The frontend sends raw counts; which one wins is decided in `tray`.
#[tauri::command]
pub fn set_tray_state(app: AppHandle, failing: u32, review: u32, running: bool) {
    flyout::store_tray_state(&app, TrayState { failing, review, running });
}

/// Whether the registry entry actually exists. The registry is the source of
/// truth for the toggle; the frontend only remembers whether it has already
/// applied the first-run default.
///
/// These two return a plain `String` error rather than `ApiError`: that type
/// belongs to the `github` module and autostart has nothing to do with GitHub.
/// `toApiError` on the other side normalises it to kind "unknown".
#[tauri::command]
pub fn autostart_enabled(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|err| err.to_string())
}

/// Reports the state *after* the change, read back from the registry, so the
/// toggle can never show something the registry disagrees with.
#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|err| err.to_string())?;
    } else {
        manager.disable().map_err(|err| err.to_string())?;
    }
    manager.is_enabled().map_err(|err| err.to_string())
}

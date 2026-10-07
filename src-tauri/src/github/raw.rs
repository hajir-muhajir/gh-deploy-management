//! The subset of GitHub's JSON that is actually read, plus the conversions
//! into the shapes the frontend sees.
//!
//! Keeping these separate from `models` means a field GitHub renames only ever
//! breaks one layer.

use serde::Deserialize;

use super::models::{ReleaseInfo, RepoInfo, RunInfo, WorkflowInfo};
use super::status::run_status;

#[derive(Deserialize)]
pub(super) struct RawUser {
    pub(super) login: String,
    pub(super) avatar_url: String,
}

#[derive(Deserialize)]
pub(super) struct RawOwner {
    pub(super) login: String,
}

#[derive(Deserialize)]
pub(super) struct RawPermissions {
    #[serde(default)]
    pub(super) push: bool,
}

#[derive(Deserialize)]
pub(super) struct RawRepo {
    pub(super) name: String,
    pub(super) full_name: String,
    pub(super) owner: RawOwner,
    pub(super) default_branch: String,
    #[serde(default)]
    pub(super) private: bool,
    #[serde(default)]
    pub(super) archived: bool,
    #[serde(default)]
    pub(super) permissions: Option<RawPermissions>,
}

#[derive(Deserialize)]
pub(super) struct RawWorkflow {
    pub(super) id: u64,
    pub(super) name: String,
    pub(super) path: String,
    pub(super) state: String,
    pub(super) html_url: String,
}

#[derive(Deserialize)]
pub(super) struct RawWorkflowList {
    pub(super) workflows: Vec<RawWorkflow>,
}

#[derive(Deserialize)]
pub(super) struct RawActor {
    pub(super) login: String,
}

#[derive(Deserialize)]
pub(super) struct RawRun {
    pub(super) id: u64,
    pub(super) workflow_id: u64,
    pub(super) name: Option<String>,
    pub(super) display_title: Option<String>,
    pub(super) run_number: u64,
    pub(super) head_branch: Option<String>,
    pub(super) status: Option<String>,
    pub(super) conclusion: Option<String>,
    pub(super) event: Option<String>,
    pub(super) html_url: String,
    pub(super) run_started_at: Option<String>,
    pub(super) created_at: String,
    pub(super) updated_at: String,
    pub(super) actor: Option<RawActor>,
}

#[derive(Deserialize)]
pub(super) struct RawRunList {
    pub(super) workflow_runs: Vec<RawRun>,
}

#[derive(Deserialize)]
pub(super) struct RawRelease {
    pub(super) id: u64,
    pub(super) tag_name: String,
    pub(super) name: Option<String>,
    pub(super) body: Option<String>,
    #[serde(default)]
    pub(super) draft: bool,
    #[serde(default)]
    pub(super) prerelease: bool,
    pub(super) author: Option<RawActor>,
    pub(super) published_at: Option<String>,
    pub(super) created_at: String,
    pub(super) html_url: String,
}

#[derive(Deserialize)]
pub(super) struct RawHead {
    #[serde(default)]
    pub(super) r#ref: String,
    pub(super) sha: String,
}

#[derive(Deserialize)]
pub(super) struct RawPull {
    pub(super) number: u64,
    pub(super) title: String,
    pub(super) user: Option<RawActor>,
    pub(super) head: RawHead,
    #[serde(default)]
    pub(super) draft: bool,
    #[serde(default)]
    pub(super) requested_reviewers: Vec<RawActor>,
    pub(super) updated_at: String,
    pub(super) html_url: String,
}

impl From<RawRelease> for ReleaseInfo {
    fn from(raw: RawRelease) -> Self {
        Self {
            id: raw.id,
            tag: raw.tag_name,
            name: raw.name.unwrap_or_default(),
            body: raw.body.unwrap_or_default(),
            draft: raw.draft,
            prerelease: raw.prerelease,
            latest: false,
            author: raw.author.map(|a| a.login).unwrap_or_default(),
            published_at: raw.published_at.unwrap_or(raw.created_at),
            html_url: raw.html_url,
        }
    }
}

impl From<RawWorkflow> for WorkflowInfo {
    fn from(raw: RawWorkflow) -> Self {
        Self { id: raw.id, name: raw.name, path: raw.path, state: raw.state, html_url: raw.html_url }
    }
}

impl From<RawRun> for RunInfo {
    fn from(raw: RawRun) -> Self {
        let status = run_status(raw.status.as_deref().unwrap_or("completed"), raw.conclusion.as_deref());
        Self {
            id: raw.id,
            workflow_id: raw.workflow_id,
            workflow_name: raw.name.unwrap_or_default(),
            title: raw.display_title.unwrap_or_default(),
            run_number: raw.run_number,
            branch: raw.head_branch.unwrap_or_default(),
            status,
            actor: raw.actor.map(|a| a.login).unwrap_or_default(),
            event: raw.event.unwrap_or_default(),
            html_url: raw.html_url,
            // A queued run has no run_started_at yet.
            started_at: raw.run_started_at.unwrap_or(raw.created_at),
            updated_at: raw.updated_at,
        }
    }
}

impl From<RawRepo> for RepoInfo {
    fn from(raw: RawRepo) -> Self {
        Self {
            owner: raw.owner.login,
            name: raw.name,
            full_name: raw.full_name,
            default_branch: raw.default_branch,
            private: raw.private,
            archived: raw.archived,
            can_push: raw.permissions.map(|p| p.push).unwrap_or(false),
        }
    }
}

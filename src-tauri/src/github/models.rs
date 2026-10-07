//! The shapes handed to the webview.
//!
//! Every one of them serialises as camelCase; the frontend types in
//! `src/lib/github.ts` are hand-written against this contract.

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    pub limit: u32,
    pub remaining: u32,
    pub reset: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub login: String,
    pub avatar_url: String,
    /// "classic" when GitHub returned an `x-oauth-scopes` header, "fineGrained" otherwise.
    pub kind: &'static str,
    /// Always empty for fine-grained tokens — their permissions are not enumerable.
    pub scopes: Vec<String>,
    /// Classic tokens only: required scopes that are absent.
    pub missing_scopes: Vec<String>,
    pub repos_verified: bool,
    /// False until a concrete repository has been probed.
    pub actions_verified: bool,
    pub rate: RateLimit,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub default_branch: String,
    pub private: bool,
    pub archived: bool,
    pub can_push: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReposPage {
    pub repos: Vec<RepoInfo>,
    /// True when MAX_PAGES was reached and more repositories exist.
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowInfo {
    pub id: u64,
    pub name: String,
    /// Unique within a repository, unlike `name` — repos really do have two
    /// workflows called "Deploy Production". This is the key the UI stores.
    pub path: String,
    /// active | disabled_manually | disabled_inactivity | ...
    pub state: String,
    pub html_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunInfo {
    pub id: u64,
    pub workflow_id: u64,
    pub workflow_name: String,
    pub title: String,
    pub run_number: u64,
    pub branch: String,
    /// Normalised by `run_status`; never a raw GitHub value.
    pub status: &'static str,
    pub actor: String,
    pub event: String,
    pub html_url: String,
    /// ISO timestamps; all formatting and elapsed-time maths happens in the UI
    /// so "12m ago" keeps ticking without re-fetching.
    pub started_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    pub id: u64,
    pub tag: String,
    pub name: String,
    /// Markdown release notes, straight from GitHub.
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    /// Derived, not an API field — see `mark_latest`.
    pub latest: bool,
    pub author: String,
    /// ISO; a draft has no publish date, so it falls back to created_at.
    pub published_at: String,
    pub html_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasesPage {
    pub releases: Vec<ReleaseInfo>,
    /// Whether GitHub reported further pages, so the UI can offer a way out.
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchInput {
    pub key: String,
    /// The YAML `description`, falling back to the key when there is none.
    pub label: String,
    /// Normalised to what the UI can draw: "text" | "bool" | "choice".
    pub kind: &'static str,
    /// Always a string, even for booleans — that is what the dispatch API takes.
    pub default: String,
    /// Only populated for "choice".
    pub options: Vec<String>,
    pub required: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchWorkflow {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub inputs: Vec<DispatchInput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestInfo {
    pub number: u64,
    pub title: String,
    pub author: String,
    pub head_branch: String,
    pub draft: bool,
    /// Logins only; the UI compares them against the token's own account.
    pub requested_reviewers: Vec<String>,
    /// Derived from the head commit's runs — see `aggregate_checks`.
    pub checks: &'static str,
    pub updated_at: String,
    pub html_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestsPage {
    pub pulls: Vec<PullRequestInfo>,
    pub has_more: bool,
}

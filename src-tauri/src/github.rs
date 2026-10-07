//! Minimal GitHub REST client.
//!
//! Everything that needs the token runs here so the token never reaches the
//! webview. Error messages are deliberately free of credential material.

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use yaml_rust2::{Yaml, YamlLoader};

const API: &str = "https://api.github.com";
const UA: &str = "gh-deploy-management";
const API_VERSION: &str = "2022-11-28";

/// Safety net for accounts with very many repositories; `truncated` tells the UI
/// when it kicked in so the list is never silently cut short.
const MAX_PAGES: usize = 5;

/// The raw run objects are large (~15 KB each), and the Actions tab only shows
/// the most recent ones.
const RUNS_PER_PAGE: usize = 20;

/// The Releases tab shows the most recent page and links out for the rest.
const RELEASES_PER_PAGE: usize = 20;

/// Each pull request costs a further request for its check state, so the page
/// size is also the request budget for one refresh of the PRs tab.
const PULLS_PER_PAGE: usize = 20;

// ── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub kind: &'static str,
    pub message: String,
}

impl ApiError {
    fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self { kind, message: message.into() }
    }

    pub fn network(err: reqwest::Error) -> Self {
        // Deliberately not `err.to_string()` on the whole chain: a reqwest error
        // can embed the request URL, and our URLs are safe, but the header set is
        // not something we want stringified by accident.
        Self::new("network", if err.is_timeout() { "Request to GitHub timed out" } else { "Cannot reach GitHub" })
    }

    /// Raised when an API call is attempted before a token has been saved.
    pub fn not_connected() -> Self {
        Self::new("notConnected", "No GitHub token saved yet")
    }

    pub fn keyring(err: keyring::Error) -> Self {
        Self::new("keyring", format!("Credential store error: {err}"))
    }
}

/// GitHub puts the human-readable reason in a JSON `message` field.
async fn error_from(response: Response) -> ApiError {
    let status = response.status();
    let remaining = header_str(&response, "x-ratelimit-remaining");
    let message = response
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|body| body.get("message").and_then(|m| m.as_str()).map(String::from))
        .unwrap_or_else(|| status.to_string());

    match status {
        StatusCode::UNAUTHORIZED => ApiError::new("invalidToken", message),
        StatusCode::NOT_FOUND => ApiError::new("notFound", message),
        StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS if remaining.as_deref() == Some("0") => {
            ApiError::new("rateLimited", message)
        }
        StatusCode::FORBIDDEN => ApiError::new("forbidden", message),
        // GitHub explains exactly what it disliked — "Required input 'tag' not
        // provided", "No ref found for: …" — so the message is the whole point.
        StatusCode::UNPROCESSABLE_ENTITY => ApiError::new("invalid", message),
        _ => ApiError::new("network", message),
    }
}

fn header_str(response: &Response, name: &str) -> Option<String> {
    response.headers().get(name)?.to_str().ok().map(String::from)
}

// ── Payloads returned to the frontend ───────────────────────────────────────

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

/// Boils a head commit's workflow runs down to one state for the PR row.
///
/// The statuses are the output of `run_status`, so "cancelled" also covers
/// skipped runs — and this repository skips two of the three runs on every PR.
/// Treating that as trouble would mark healthy PRs red, hence the "none" case.
fn aggregate_checks(statuses: &[&str]) -> &'static str {
    // A failure outranks work still in flight: it is the part that needs acting on.
    if statuses.contains(&"failure") {
        return "failure";
    }
    if statuses.iter().any(|s| *s == "running" || *s == "queued") {
        return "pending";
    }
    if statuses.contains(&"success") {
        return "success";
    }
    "none"
}

/// Reads a workflow's `workflow_dispatch` trigger out of its YAML source.
///
/// `None` means the workflow cannot be triggered by hand; `Some(vec![])` means
/// it can, but takes no inputs. REST exposes neither fact, so the file itself is
/// the only source. Kept free of I/O so it can be tested without a token.
pub fn parse_dispatch_inputs(text: &str) -> Option<Vec<DispatchInput>> {
    let docs = YamlLoader::load_from_str(text).ok()?;
    let on = on_node(docs.first()?)?;

    let dispatch = match on {
        // `on: workflow_dispatch`
        Yaml::String(event) => return (event == "workflow_dispatch").then(Vec::new),
        // `on: [push, workflow_dispatch]`
        Yaml::Array(events) => {
            let found = events.iter().any(|e| e.as_str() == Some("workflow_dispatch"));
            return found.then(Vec::new);
        }
        // `on:` followed by an indented block of events.
        Yaml::Hash(_) => field(on, "workflow_dispatch")?,
        _ => return None,
    };

    // A bare `workflow_dispatch:` parses as null, which is dispatchable but
    // input-less — the shape four of the five fixture workflows use.
    let Some(inputs) = field(dispatch, "inputs").and_then(Yaml::as_hash) else {
        return Some(Vec::new());
    };

    Some(
        inputs
            .iter()
            .filter_map(|(key, spec)| parse_input(key.as_str()?, spec))
            .collect(),
    )
}

/// The `on:` key of a workflow document.
fn on_node(doc: &Yaml) -> Option<&Yaml> {
    let hash = doc.as_hash()?;
    hash.get(&Yaml::String("on".into())).or_else(|| {
        // A YAML 1.1 parser resolves the bare word `on` to a boolean, which
        // would make every workflow look non-dispatchable. yaml-rust2 is 1.2 so
        // this branch should stay dead, but silence is the wrong failure here.
        hash.get(&Yaml::Boolean(true))
    })
}

/// Hash lookup that treats a missing key and a `BadValue` alike.
fn field<'a>(node: &'a Yaml, key: &str) -> Option<&'a Yaml> {
    node.as_hash()?
        .get(&Yaml::String(key.into()))
        .filter(|value| !value.is_badvalue())
}

/// Defaults and options may be written unquoted, so they arrive as numbers or
/// booleans. The dispatch API wants strings regardless.
fn scalar(node: &Yaml) -> Option<String> {
    match node {
        Yaml::String(value) => Some(value.clone()),
        Yaml::Real(value) => Some(value.clone()),
        Yaml::Integer(value) => Some(value.to_string()),
        Yaml::Boolean(value) => Some(value.to_string()),
        _ => None,
    }
}

fn parse_input(key: &str, spec: &Yaml) -> Option<DispatchInput> {
    let options: Vec<String> = field(spec, "options")
        .and_then(Yaml::as_vec)
        .map(|list| list.iter().filter_map(scalar).collect())
        .unwrap_or_default();

    let kind = match field(spec, "type").and_then(Yaml::as_str).unwrap_or("string") {
        // A choice with no options cannot be drawn as one; fall through to text.
        "choice" if !options.is_empty() => "choice",
        "boolean" => "bool",
        // string, number, environment, and whatever GitHub adds next.
        _ => "text",
    };

    Some(DispatchInput {
        key: key.to_string(),
        label: field(spec, "description")
            .and_then(Yaml::as_str)
            .unwrap_or(key)
            .to_string(),
        kind,
        default: field(spec, "default").and_then(scalar).unwrap_or_default(),
        options,
        required: field(spec, "required").and_then(Yaml::as_bool).unwrap_or(false),
    })
}

/// Flags the release GitHub would call "latest": the first non-draft,
/// non-prerelease entry of a list already sorted by created_at descending.
/// That matches `/releases/latest` without the extra request — and without its
/// 404 on repositories that have no published release.
fn mark_latest(releases: &mut [ReleaseInfo]) {
    if let Some(release) = releases
        .iter_mut()
        .find(|release| !release.draft && !release.prerelease)
    {
        release.latest = true;
    }
}

/// Collapses GitHub's `status` + `conclusion` pair into the five states the UI
/// draws. Keeping this in one place stops the two halves drifting apart.
fn run_status(status: &str, conclusion: Option<&str>) -> &'static str {
    match status {
        "in_progress" => "running",
        "queued" | "waiting" | "requested" | "pending" => "queued",
        _ => match conclusion {
            Some("success") => "success",
            Some("failure") | Some("timed_out") | Some("action_required")
            | Some("startup_failure") => "failure",
            // cancelled, skipped, stale, neutral, and a null conclusion on an
            // otherwise completed run all read as "did not finish its work".
            _ => "cancelled",
        },
    }
}

// ── Raw GitHub shapes ───────────────────────────────────────────────────────

#[derive(Deserialize)]
struct RawUser {
    login: String,
    avatar_url: String,
}

#[derive(Deserialize)]
struct RawOwner {
    login: String,
}

#[derive(Deserialize)]
struct RawPermissions {
    #[serde(default)]
    push: bool,
}

#[derive(Deserialize)]
struct RawRepo {
    name: String,
    full_name: String,
    owner: RawOwner,
    default_branch: String,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    archived: bool,
    #[serde(default)]
    permissions: Option<RawPermissions>,
}

#[derive(Deserialize)]
struct RawWorkflow {
    id: u64,
    name: String,
    path: String,
    state: String,
    html_url: String,
}

#[derive(Deserialize)]
struct RawWorkflowList {
    workflows: Vec<RawWorkflow>,
}

#[derive(Deserialize)]
struct RawActor {
    login: String,
}

#[derive(Deserialize)]
struct RawRun {
    id: u64,
    workflow_id: u64,
    name: Option<String>,
    display_title: Option<String>,
    run_number: u64,
    head_branch: Option<String>,
    status: Option<String>,
    conclusion: Option<String>,
    event: Option<String>,
    html_url: String,
    run_started_at: Option<String>,
    created_at: String,
    updated_at: String,
    actor: Option<RawActor>,
}

#[derive(Deserialize)]
struct RawRunList {
    workflow_runs: Vec<RawRun>,
}

#[derive(Deserialize)]
struct RawRelease {
    id: u64,
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    author: Option<RawActor>,
    published_at: Option<String>,
    created_at: String,
    html_url: String,
}

#[derive(Deserialize)]
struct RawHead {
    #[serde(default)]
    r#ref: String,
    sha: String,
}

#[derive(Deserialize)]
struct RawPull {
    number: u64,
    title: String,
    user: Option<RawActor>,
    head: RawHead,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    requested_reviewers: Vec<RawActor>,
    updated_at: String,
    html_url: String,
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
        let status = run_status(
            raw.status.as_deref().unwrap_or("completed"),
            raw.conclusion.as_deref(),
        );
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

// ── Client ──────────────────────────────────────────────────────────────────

pub struct GitHub {
    http: Client,
}

impl GitHub {
    pub fn new(token: &str) -> Result<Self, ApiError> {
        let mut headers = HeaderMap::new();
        let mut auth = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| ApiError::new("invalidToken", "Token contains invalid characters"))?;
        auth.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth);
        headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
        headers.insert("X-GitHub-Api-Version", HeaderValue::from_static(API_VERSION));
        headers.insert(USER_AGENT, HeaderValue::from_static(UA));

        let http = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(ApiError::network)?;

        Ok(Self { http })
    }

    async fn get(&self, path: &str) -> Result<Response, ApiError> {
        let url = if path.starts_with("http") { path.to_string() } else { format!("{API}{path}") };
        self.http.get(url).send().await.map_err(ApiError::network)
    }

    async fn post(&self, path: &str) -> Result<Response, ApiError> {
        self.http
            .post(format!("{API}{path}"))
            .header(reqwest::header::CONTENT_LENGTH, "0")
            .send()
            .await
            .map_err(ApiError::network)
    }

    /// `post` pins `Content-Length: 0`, so a request with a body needs its own
    /// entry point rather than a flag.
    async fn post_json(&self, path: &str, body: &serde_json::Value) -> Result<Response, ApiError> {
        self.http
            .post(format!("{API}{path}"))
            .json(body)
            .send()
            .await
            .map_err(ApiError::network)
    }

    /// True when the endpoint is reachable with this token; used to probe
    /// fine-grained permissions, which GitHub does not expose any other way.
    async fn probe(&self, path: &str) -> bool {
        matches!(self.get(path).await, Ok(r) if r.status().is_success())
    }

    /// Validates the token and reports what it is allowed to do.
    ///
    /// `repo_for_actions` is an optional `owner/name` used to probe the Actions
    /// permission of a fine-grained token, which can only be checked per repository.
    pub async fn validate(&self, repo_for_actions: Option<&str>) -> Result<TokenInfo, ApiError> {
        let response = self.get("/user").await?;
        if !response.status().is_success() {
            return Err(error_from(response).await);
        }

        let rate = RateLimit {
            limit: header_str(&response, "x-ratelimit-limit").and_then(|v| v.parse().ok()).unwrap_or(0),
            remaining: header_str(&response, "x-ratelimit-remaining").and_then(|v| v.parse().ok()).unwrap_or(0),
            reset: header_str(&response, "x-ratelimit-reset").and_then(|v| v.parse().ok()).unwrap_or(0),
        };

        // Only classic tokens carry this header; its absence is how we detect a
        // fine-grained token, whose permissions have to be probed instead.
        let raw_scopes = header_str(&response, "x-oauth-scopes");

        let user: RawUser = response.json().await.map_err(ApiError::network)?;

        let repos_verified = self.probe("/user/repos?per_page=1").await;
        let actions_verified = match repo_for_actions {
            Some(full_name) => self.probe(&format!("/repos/{full_name}/actions/workflows?per_page=1")).await,
            None => false,
        };

        let (kind, scopes, missing_scopes) = match raw_scopes {
            Some(raw) => {
                let scopes: Vec<String> = raw
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                // `public_repo` alone is not enough: private repos and dispatch both need `repo`.
                let missing = ["repo", "workflow"]
                    .iter()
                    .filter(|needed| !scopes.iter().any(|s| s == *needed))
                    .map(|s| s.to_string())
                    .collect();
                ("classic", scopes, missing)
            }
            None => ("fineGrained", Vec::new(), Vec::new()),
        };

        Ok(TokenInfo {
            login: user.login,
            avatar_url: user.avatar_url,
            kind,
            scopes,
            missing_scopes,
            repos_verified,
            actions_verified,
            rate,
        })
    }

    /// Every repository the token can reach, newest push first. Used as the
    /// search corpus — the UI never renders all of them at once.
    pub async fn list_repos(&self) -> Result<ReposPage, ApiError> {
        let mut repos = Vec::new();
        let mut url = format!(
            "{API}/user/repos?per_page=100&sort=pushed&affiliation=owner,collaborator,organization_member"
        );
        let mut truncated = false;

        for page in 0..MAX_PAGES {
            let response = self.get(&url).await?;
            if !response.status().is_success() {
                return Err(error_from(response).await);
            }

            let next = next_page_url(&response);
            let batch: Vec<RawRepo> = response.json().await.map_err(ApiError::network)?;
            repos.extend(batch.into_iter().map(RepoInfo::from));

            match next {
                Some(link) => {
                    url = link;
                    truncated = page + 1 == MAX_PAGES;
                }
                None => break,
            }
        }

        Ok(ReposPage { repos, truncated })
    }

    pub async fn get_repo(&self, owner: &str, name: &str) -> Result<RepoInfo, ApiError> {
        let response = self.get(&format!("/repos/{owner}/{name}")).await?;
        if response.status() == StatusCode::NOT_FOUND {
            // GitHub answers 404 (not 403) for private repos the token cannot see,
            // so "not found" and "no access" are indistinguishable here.
            return Err(ApiError::new("notFound", "Repository not found, or the token has no access to it"));
        }
        if !response.status().is_success() {
            return Err(error_from(response).await);
        }
        let raw: RawRepo = response.json().await.map_err(ApiError::network)?;
        Ok(RepoInfo::from(raw))
    }

    pub async fn probe_actions(&self, owner: &str, name: &str) -> bool {
        self.probe(&format!("/repos/{owner}/{name}/actions/workflows?per_page=1")).await
    }

    pub async fn list_workflows(
        &self,
        owner: &str,
        name: &str,
    ) -> Result<Vec<WorkflowInfo>, ApiError> {
        let response = self
            .get(&format!("/repos/{owner}/{name}/actions/workflows?per_page=100"))
            .await?;
        if !response.status().is_success() {
            return Err(error_from(response).await);
        }
        let list: RawWorkflowList = response.json().await.map_err(ApiError::network)?;
        Ok(list.workflows.into_iter().map(WorkflowInfo::from).collect())
    }

    /// Recent runs across every workflow. Capped at RUNS_PER_PAGE because the
    /// raw response is ~15 KB per run and the UI only shows the latest handful.
    pub async fn list_runs(&self, owner: &str, name: &str) -> Result<Vec<RunInfo>, ApiError> {
        let response = self
            .get(&format!(
                "/repos/{owner}/{name}/actions/runs?per_page={RUNS_PER_PAGE}"
            ))
            .await?;
        if !response.status().is_success() {
            return Err(error_from(response).await);
        }
        let list: RawRunList = response.json().await.map_err(ApiError::network)?;
        Ok(list.workflow_runs.into_iter().map(RunInfo::from).collect())
    }

    /// The newest page of releases. Drafts are kept here so this layer stays a
    /// faithful mirror of the API; the UI decides whether to show them.
    pub async fn list_releases(&self, owner: &str, name: &str) -> Result<ReleasesPage, ApiError> {
        let response = self
            .get(&format!(
                "/repos/{owner}/{name}/releases?per_page={RELEASES_PER_PAGE}"
            ))
            .await?;
        if !response.status().is_success() {
            return Err(error_from(response).await);
        }

        let has_more = next_page_url(&response).is_some();
        let raw: Vec<RawRelease> = response.json().await.map_err(ApiError::network)?;
        let mut releases: Vec<ReleaseInfo> = raw.into_iter().map(ReleaseInfo::from).collect();
        mark_latest(&mut releases);

        Ok(ReleasesPage { releases, has_more })
    }

    /// Open pull requests, newest activity first, with the state of the checks
    /// on each one.
    ///
    /// The checks cost one request per PR. `/pulls` carries no check state, the
    /// Checks API answers 403 for this token, and the legacy Statuses API is
    /// empty for this repository — it reports `pending` with zero statuses,
    /// which would paint every green PR as still running. Workflow runs for the
    /// head commit are the only source that is both available and truthful.
    pub async fn list_pulls(&self, owner: &str, name: &str) -> Result<PullRequestsPage, ApiError> {
        let response = self
            .get(&format!(
                "/repos/{owner}/{name}/pulls?state=open&sort=updated&direction=desc&per_page={PULLS_PER_PAGE}"
            ))
            .await?;
        if !response.status().is_success() {
            return Err(error_from(response).await);
        }

        let has_more = next_page_url(&response).is_some();
        let raw: Vec<RawPull> = response.json().await.map_err(ApiError::network)?;

        let mut pulls = Vec::with_capacity(raw.len());
        for pull in raw {
            pulls.push(PullRequestInfo {
                number: pull.number,
                title: pull.title,
                author: pull.user.map(|u| u.login).unwrap_or_default(),
                head_branch: pull.head.r#ref,
                draft: pull.draft,
                requested_reviewers: pull.requested_reviewers.into_iter().map(|r| r.login).collect(),
                // A PR whose runs cannot be read reports "no checks" rather than
                // sinking the whole list.
                checks: self.checks_for(owner, name, &pull.head.sha).await.unwrap_or("none"),
                updated_at: pull.updated_at,
                html_url: pull.html_url,
            });
        }

        Ok(PullRequestsPage { pulls, has_more })
    }

    async fn checks_for(
        &self,
        owner: &str,
        name: &str,
        head_sha: &str,
    ) -> Result<&'static str, ApiError> {
        let response = self
            .get(&format!(
                "/repos/{owner}/{name}/actions/runs?head_sha={head_sha}&per_page={RUNS_PER_PAGE}"
            ))
            .await?;
        if !response.status().is_success() {
            return Err(error_from(response).await);
        }
        let list: RawRunList = response.json().await.map_err(ApiError::network)?;
        let statuses: Vec<&'static str> = list
            .workflow_runs
            .iter()
            .map(|run| {
                run_status(
                    run.status.as_deref().unwrap_or("completed"),
                    run.conclusion.as_deref(),
                )
            })
            .collect();
        Ok(aggregate_checks(&statuses))
    }

    /// Reads one file from the default branch as plain text. `vnd.github.raw`
    /// avoids the base64 wrapper the JSON representation would use.
    async fn file_contents(&self, owner: &str, name: &str, path: &str) -> Result<String, ApiError> {
        let response = self
            .http
            .get(format!("{API}/repos/{owner}/{name}/contents/{path}"))
            .header(ACCEPT, "application/vnd.github.raw")
            .send()
            .await
            .map_err(ApiError::network)?;
        if !response.status().is_success() {
            return Err(error_from(response).await);
        }
        response.text().await.map_err(ApiError::network)
    }

    /// The workflows a person can start by hand, with the form fields each one
    /// asks for.
    ///
    /// Costs one request per active workflow because REST reports neither the
    /// trigger nor the inputs — both live only in the YAML. Disabled workflows
    /// are skipped since dispatching one does nothing. A file that cannot be
    /// read or parsed drops that workflow alone; one broken file must not empty
    /// the whole tab.
    pub async fn list_dispatchable(
        &self,
        owner: &str,
        name: &str,
    ) -> Result<Vec<DispatchWorkflow>, ApiError> {
        let workflows = self.list_workflows(owner, name).await?;
        let mut dispatchable = Vec::new();

        for workflow in workflows.into_iter().filter(|w| w.state == "active") {
            let Ok(text) = self.file_contents(owner, name, &workflow.path).await else {
                continue;
            };
            let Some(inputs) = parse_dispatch_inputs(&text) else {
                continue;
            };
            dispatchable.push(DispatchWorkflow {
                id: workflow.id,
                name: workflow.name,
                path: workflow.path,
                inputs,
            });
        }

        Ok(dispatchable)
    }

    /// Starts a workflow. Answers 204 with no body; the run takes a few seconds
    /// to appear in the runs list afterwards.
    pub async fn dispatch_workflow(
        &self,
        owner: &str,
        name: &str,
        workflow_id: u64,
        git_ref: &str,
        inputs: HashMap<String, String>,
    ) -> Result<(), ApiError> {
        let body = serde_json::json!({ "ref": git_ref, "inputs": inputs });
        let response = self
            .post_json(
                &format!("/repos/{owner}/{name}/actions/workflows/{workflow_id}/dispatches"),
                &body,
            )
            .await?;
        if response.status().is_success() {
            return Ok(());
        }

        let error = error_from(response).await;
        Err(match error.kind {
            "forbidden" => ApiError::new(
                "forbidden",
                "Token has no permission to start workflows (needs Actions: write)",
            ),
            _ => error,
        })
    }

    pub async fn cancel_run(&self, owner: &str, name: &str, run_id: u64) -> Result<(), ApiError> {
        self.run_command(owner, name, run_id, "cancel").await
    }

    pub async fn rerun_run(&self, owner: &str, name: &str, run_id: u64) -> Result<(), ApiError> {
        self.run_command(owner, name, run_id, "rerun").await
    }

    /// Both endpoints answer 202 Accepted and need the Actions *write*
    /// permission, which cannot be verified without actually calling them.
    async fn run_command(
        &self,
        owner: &str,
        name: &str,
        run_id: u64,
        action: &str,
    ) -> Result<(), ApiError> {
        let response = self
            .post(&format!("/repos/{owner}/{name}/actions/runs/{run_id}/{action}"))
            .await?;
        if response.status().is_success() {
            return Ok(());
        }
        let error = error_from(response).await;
        Err(match error.kind {
            "forbidden" => ApiError::new(
                "forbidden",
                "Token has no permission to control workflow runs (needs Actions: write)",
            ),
            _ => error,
        })
    }
}

/// Pulls `<url>; rel="next"` out of the `Link` header.
fn next_page_url(response: &Response) -> Option<String> {
    let link = response.headers().get("link")?.to_str().ok()?;
    link.split(',').find_map(|part| {
        if !part.contains("rel=\"next\"") {
            return None;
        }
        let start = part.find('<')? + 1;
        let end = part.find('>')?;
        Some(part[start..end].to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real PAT, from `GITHUB_TOKEN` or the gitignored `.env.local` at the
    /// repo root. Tests that need one are skipped when it is absent, so the
    /// suite still passes on a machine without credentials.
    fn token() -> Option<String> {
        if let Ok(value) = std::env::var("GITHUB_TOKEN") {
            if !value.trim().is_empty() {
                return Some(value.trim().to_string());
            }
        }
        let contents = std::fs::read_to_string("../.env.local").ok()?;
        contents.lines().find_map(|line| {
            let value = line.strip_prefix("GITHUB_TOKEN=")?.trim().trim_matches(['"', '\'']);
            (!value.is_empty()).then(|| value.to_string())
        })
    }

    #[tokio::test]
    async fn accepts_a_real_token_and_reports_its_permissions() {
        let Some(token) = token() else {
            eprintln!("skipped: no GITHUB_TOKEN");
            return;
        };

        let info = GitHub::new(&token).unwrap().validate(None).await.unwrap();

        assert!(!info.login.is_empty(), "login should be populated");
        assert!(info.repos_verified, "token should be able to list repositories");
        assert!(info.rate.limit >= 5000, "an authenticated token gets the 5000/h bucket");
        if info.kind == "classic" {
            assert!(info.missing_scopes.is_empty(), "missing scopes: {:?}", info.missing_scopes);
        } else {
            assert!(info.scopes.is_empty(), "fine-grained tokens expose no scopes");
        }
    }

    #[tokio::test]
    async fn rejects_a_bad_token() {
        let error = GitHub::new("ghp_definitelyNotAValidToken0000000")
            .unwrap()
            .validate(None)
            .await
            .expect_err("a garbage token must not validate");

        assert_eq!(error.kind, "invalidToken");
        assert_eq!(error.message, "Bad credentials");
    }

    #[tokio::test]
    async fn probes_actions_permission_against_a_real_repository() {
        let Some(token) = token() else {
            eprintln!("skipped: no GITHUB_TOKEN");
            return;
        };
        let client = GitHub::new(&token).unwrap();

        let page = client.list_repos().await.unwrap();
        assert!(!page.repos.is_empty(), "the token should see at least one repository");

        let repo = &page.repos[0];
        assert!(!repo.default_branch.is_empty(), "default branch must be known");

        let info = client.validate(Some(&repo.full_name)).await.unwrap();
        assert!(info.actions_verified, "Actions permission should be confirmed for {}", repo.full_name);
    }

    #[tokio::test]
    async fn reports_an_unreachable_repository_as_not_found() {
        let Some(token) = token() else {
            eprintln!("skipped: no GITHUB_TOKEN");
            return;
        };

        let error = GitHub::new(&token)
            .unwrap()
            .get_repo("indo-taichen", "repo-yang-pasti-tidak-ada-123")
            .await
            .expect_err("a missing repository must be an error");

        assert_eq!(error.kind, "notFound");
    }

    /// The repository this account uses as the workflow fixture: 11 workflows,
    /// two of them disabled, and two pairs sharing a name.
    const FIXTURE: &str = "Indo-Taichen/project-online-production";

    fn split(full_name: &str) -> (&str, &str) {
        full_name.split_once('/').unwrap()
    }

    #[tokio::test]
    async fn workflow_names_collide_but_paths_do_not() {
        let Some(token) = token() else {
            eprintln!("skipped: no GITHUB_TOKEN");
            return;
        };
        let (owner, name) = split(FIXTURE);
        let workflows = GitHub::new(&token).unwrap().list_workflows(owner, name).await.unwrap();

        assert!(workflows.len() > 1, "fixture repo should have several workflows");

        let paths: std::collections::HashSet<_> = workflows.iter().map(|w| &w.path).collect();
        assert_eq!(paths.len(), workflows.len(), "paths must be unique — they are the storage key");

        let names: std::collections::HashSet<_> = workflows.iter().map(|w| &w.name).collect();
        assert!(
            names.len() < workflows.len(),
            "this fixture is meant to contain duplicate names; without them the              regression that keying by name causes would go unnoticed"
        );

        assert!(
            workflows.iter().any(|w| w.state != "active"),
            "fixture should include a disabled workflow so that path is exercised"
        );
    }

    #[tokio::test]
    async fn runs_are_normalised_to_the_five_ui_states() {
        let Some(token) = token() else {
            eprintln!("skipped: no GITHUB_TOKEN");
            return;
        };
        let (owner, name) = split(FIXTURE);
        let runs = GitHub::new(&token).unwrap().list_runs(owner, name).await.unwrap();

        assert!(!runs.is_empty(), "fixture repo should have runs");
        assert!(runs.len() <= RUNS_PER_PAGE, "list_runs must respect its page cap");

        for run in &runs {
            assert!(
                ["success", "failure", "running", "queued", "cancelled"].contains(&run.status),
                "unmapped status `{}` on run #{}",
                run.status,
                run.run_number
            );
            assert!(!run.started_at.is_empty(), "run #{} has no start time", run.run_number);
            assert!(run.workflow_id != 0, "run #{} has no workflow_id", run.run_number);
        }
    }

    fn release(tag: &str, draft: bool, prerelease: bool) -> ReleaseInfo {
        ReleaseInfo {
            id: 0,
            tag: tag.into(),
            name: String::new(),
            body: String::new(),
            draft,
            prerelease,
            latest: false,
            author: String::new(),
            published_at: String::new(),
            html_url: String::new(),
        }
    }

    #[test]
    fn marks_only_the_first_published_release_as_latest() {
        // Index 0 is a draft and index 1 a prerelease, so neither qualifies —
        // picking the newest entry outright would flag the wrong one.
        let mut releases = vec![
            release("v2.0.0-draft", true, false),
            release("v2.0.0-rc.1", false, true),
            release("v1.9.0", false, false),
            release("v1.8.0", false, false),
        ];
        mark_latest(&mut releases);

        let flagged: Vec<&str> = releases
            .iter()
            .filter(|r| r.latest)
            .map(|r| r.tag.as_str())
            .collect();
        assert_eq!(flagged, ["v1.9.0"], "exactly one release, the newest published one");
    }

    #[test]
    fn marks_nothing_when_every_release_is_a_draft_or_prerelease() {
        let mut releases = vec![release("v1.0.0-rc.1", false, true), release("v1.0.0", true, false)];
        mark_latest(&mut releases);
        assert!(releases.iter().all(|r| !r.latest));
    }

    #[tokio::test]
    async fn lists_real_releases_with_one_latest() {
        let Some(token) = token() else {
            eprintln!("skipped: no GITHUB_TOKEN");
            return;
        };
        let (owner, name) = split(FIXTURE);
        let page = GitHub::new(&token).unwrap().list_releases(owner, name).await.unwrap();

        assert!(!page.releases.is_empty(), "fixture repo should have releases");
        assert!(page.releases.len() <= RELEASES_PER_PAGE, "must respect the page cap");
        assert!(page.has_more, "fixture repo has far more releases than one page");

        assert_eq!(
            page.releases.iter().filter(|r| r.latest).count(),
            1,
            "exactly one release should carry the Latest badge"
        );

        for r in &page.releases {
            assert!(!r.tag.is_empty(), "release {} has no tag", r.id);
            assert!(!r.html_url.is_empty(), "release {} has no link", r.tag);
            assert!(!r.published_at.is_empty(), "release {} has no date", r.tag);
        }
    }

    #[tokio::test]
    async fn a_repository_without_releases_is_empty_not_an_error() {
        let Some(token) = token() else {
            eprintln!("skipped: no GITHUB_TOKEN");
            return;
        };
        // Unlike /releases/latest, which answers 404 here.
        let page = GitHub::new(&token)
            .unwrap()
            .list_releases("Indo-Taichen", "mps-project")
            .await
            .expect("an empty release list is a success, not a failure");

        assert!(page.releases.is_empty());
        assert!(!page.has_more);
    }

    #[test]
    fn parses_every_form_of_the_on_key() {
        // All four are legal GitHub syntax and all four occur in the wild.
        assert_eq!(
            parse_dispatch_inputs("name: x\non: workflow_dispatch\n").map(|i| i.len()),
            Some(0),
            "scalar form"
        );
        assert_eq!(
            parse_dispatch_inputs("name: x\non: [push, workflow_dispatch]\n").map(|i| i.len()),
            Some(0),
            "array form"
        );
        assert_eq!(
            parse_dispatch_inputs("name: x\non:\n  workflow_dispatch:\n").map(|i| i.len()),
            Some(0),
            "bare block form — four of the five fixture workflows"
        );
        assert_eq!(
            parse_dispatch_inputs(
                "name: x\non:\n  workflow_dispatch:\n    inputs:\n      tag:\n        required: true\n"
            )
            .map(|i| i.len()),
            Some(1),
            "block form with inputs"
        );

        // Not dispatchable: these must be left out of the Run tab entirely.
        assert!(parse_dispatch_inputs("name: x\non:\n  push:\n    branches: [main]\n").is_none());
        assert!(parse_dispatch_inputs("name: x\non: [push, pull_request]\n").is_none());
        assert!(parse_dispatch_inputs("name: x\njobs: {}\n").is_none());
        assert!(parse_dispatch_inputs("::: not yaml at all\n  - [}\n").is_none());
    }

    #[test]
    fn reads_the_on_key_as_a_key_and_not_as_a_boolean() {
        // On YAML 1.1 the bare word `on` resolves to `true`, which would make
        // every workflow look non-dispatchable and quietly empty the tab.
        let docs = YamlLoader::load_from_str("on: workflow_dispatch\n").unwrap();
        let hash = docs[0].as_hash().unwrap();
        assert!(
            hash.contains_key(&Yaml::String("on".into())),
            "the parser must be YAML 1.2; `on` resolved to {:?}",
            hash.keys().next()
        );
    }

    #[test]
    fn reads_input_type_default_and_required() {
        let inputs = parse_dispatch_inputs(
            r#"
name: Release
on:
  workflow_dispatch:
    inputs:
      bump:
        description: Version bump
        type: choice
        default: patch
        options: [patch, minor, major]
      dry_run:
        description: Plan only
        type: boolean
        default: false
      tag:
        description: Tag version to deploy
        required: true
        type: string
      retries:
        type: number
        default: 3
      colour:
        type: choice
"#,
        )
        .expect("workflow_dispatch is present");

        let by_key = |key: &str| inputs.iter().find(|i| i.key == key).unwrap();

        let bump = by_key("bump");
        assert_eq!(bump.kind, "choice");
        assert_eq!(bump.label, "Version bump");
        assert_eq!(bump.default, "patch");
        assert_eq!(bump.options, ["patch", "minor", "major"]);
        assert!(!bump.required);

        let dry_run = by_key("dry_run");
        assert_eq!(dry_run.kind, "bool");
        // Written unquoted, so YAML hands back a boolean — the dispatch API
        // still wants a string.
        assert_eq!(dry_run.default, "false");

        let tag = by_key("tag");
        assert_eq!(tag.kind, "text");
        assert!(tag.required);
        assert_eq!(tag.default, "", "a required input has nothing to prefill");

        assert_eq!(by_key("retries").default, "3", "numbers become strings too");

        let colour = by_key("colour");
        assert_eq!(colour.kind, "text", "a choice without options cannot be a choice");
        assert_eq!(colour.label, "colour", "label falls back to the key");
    }

    #[tokio::test]
    async fn lists_only_dispatchable_workflows() {
        let Some(token) = token() else {
            eprintln!("skipped: no GITHUB_TOKEN");
            return;
        };
        let (owner, name) = split(FIXTURE);
        let list = GitHub::new(&token).unwrap().list_dispatchable(owner, name).await.unwrap();

        let paths: Vec<&str> = list.iter().map(|w| w.path.as_str()).collect();
        assert!(
            !paths.contains(&".github/workflows/deploy_to_server.yml"),
            "Deploy Laravel is push-only and must not be offered: {paths:?}"
        );
        assert!(
            !paths.contains(&".github/workflows/deploy-production.yml"),
            "disabled workflows cannot run and must not be offered: {paths:?}"
        );

        let deploy = list
            .iter()
            .find(|w| w.path == ".github/workflows/deploy-multiple-server.yml")
            .expect("Deploy Production is dispatchable");
        let tag = deploy.inputs.iter().find(|i| i.key == "tag").expect("it asks for a tag");
        assert!(tag.required, "the tag input is declared required");
        assert!(tag.default.is_empty(), "and has no default to fall back on");

        let cache = list
            .iter()
            .find(|w| w.path == ".github/workflows/optimize-cache.yml")
            .expect("Optimize Cache is dispatchable");
        assert!(cache.inputs.is_empty(), "it takes no inputs");
    }

    #[tokio::test]
    async fn rejects_a_dispatch_with_an_unknown_ref() {
        let Some(token) = token() else {
            eprintln!("skipped: no GITHUB_TOKEN");
            return;
        };
        let (owner, name) = split(FIXTURE);
        let client = GitHub::new(&token).unwrap();

        let cache = client
            .list_dispatchable(owner, name)
            .await
            .unwrap()
            .into_iter()
            .find(|w| w.path == ".github/workflows/optimize-cache.yml")
            .expect("fixture has Optimize Cache");

        // A ref that cannot exist: GitHub rejects the call before scheduling
        // anything, so this exercises the endpoint, the Actions: write
        // permission, and the payload shape without starting production CI.
        let error = client
            .dispatch_workflow(owner, name, cache.id, "branch-yang-pasti-tidak-ada-123", HashMap::new())
            .await
            .expect_err("an unknown ref must be refused");

        assert_eq!(error.kind, "invalid", "got {error:?}");
        assert!(
            error.message.to_lowercase().contains("ref"),
            "GitHub should name the ref it could not find: {}",
            error.message
        );
    }

    #[test]
    fn aggregates_check_states_by_severity() {
        // A failure is what needs acting on, even while other runs continue.
        assert_eq!(aggregate_checks(&["success", "running", "failure"]), "failure");
        assert_eq!(aggregate_checks(&["success", "queued"]), "pending");
        assert_eq!(aggregate_checks(&["success", "success"]), "success");
        // The exact shape every PR in the fixture repo produces: one real run
        // plus two skipped ones, which `run_status` reports as cancelled.
        assert_eq!(aggregate_checks(&["cancelled", "cancelled", "success"]), "success");
    }

    #[test]
    fn reports_no_checks_when_nothing_ran() {
        assert_eq!(aggregate_checks(&[]), "none", "a PR with no CI at all");
        assert_eq!(
            aggregate_checks(&["cancelled", "cancelled"]),
            "none",
            "every run skipped is not a failure"
        );
    }

    #[tokio::test]
    async fn lists_real_open_pull_requests() {
        let Some(token) = token() else {
            eprintln!("skipped: no GITHUB_TOKEN");
            return;
        };
        let (owner, name) = split(FIXTURE);
        let page = GitHub::new(&token).unwrap().list_pulls(owner, name).await.unwrap();

        assert!(!page.pulls.is_empty(), "fixture repo has open pull requests");
        assert!(page.pulls.len() <= PULLS_PER_PAGE, "must respect the page cap");

        for pull in &page.pulls {
            assert!(!pull.html_url.is_empty(), "PR #{} has no link", pull.number);
            assert!(!pull.head_branch.is_empty(), "PR #{} has no head branch", pull.number);
            assert!(!pull.updated_at.is_empty(), "PR #{} has no timestamp", pull.number);
            assert!(
                ["success", "failure", "pending", "none"].contains(&pull.checks),
                "unmapped check state `{}` on PR #{}",
                pull.checks,
                pull.number
            );
        }

        assert!(
            page.pulls.iter().any(|p| p.draft),
            "the fixture includes a draft PR so that filter is exercised"
        );
    }

    #[tokio::test]
    async fn finds_the_review_requested_pull_request() {
        let Some(token) = token() else {
            eprintln!("skipped: no GITHUB_TOKEN");
            return;
        };
        let client = GitHub::new(&token).unwrap();
        let (owner, name) = split(FIXTURE);

        // The tray badge compares these logins against the token's own account,
        // so the two have to line up on real data.
        let me = client.validate(None).await.unwrap().login;
        let page = client.list_pulls(owner, name).await.unwrap();

        assert!(
            page.pulls.iter().any(|p| p.requested_reviewers.iter().any(|r| *r == me)),
            "expected at least one PR awaiting review from {me}"
        );
    }

    #[test]
    fn maps_github_status_pairs_onto_ui_states() {
        assert_eq!(run_status("in_progress", None), "running");
        assert_eq!(run_status("queued", None), "queued");
        assert_eq!(run_status("waiting", None), "queued");
        assert_eq!(run_status("completed", Some("success")), "success");
        assert_eq!(run_status("completed", Some("failure")), "failure");
        assert_eq!(run_status("completed", Some("timed_out")), "failure");
        assert_eq!(run_status("completed", Some("startup_failure")), "failure");
        assert_eq!(run_status("completed", Some("cancelled")), "cancelled");
        assert_eq!(run_status("completed", Some("skipped")), "cancelled");
        // A completed run with no conclusion should not be reported as success.
        assert_eq!(run_status("completed", None), "cancelled");
    }

    /// The frontend types in src/lib/github.ts are hand-written, so this guards
    /// the camelCase contract between the two.
    #[test]
    fn serialises_with_the_field_names_the_frontend_expects() {
        let info = TokenInfo {
            login: "octocat".into(),
            avatar_url: "https://example.test/a.png".into(),
            kind: "fineGrained",
            scopes: vec![],
            missing_scopes: vec![],
            repos_verified: true,
            actions_verified: false,
            rate: RateLimit { limit: 5000, remaining: 4999, reset: 0 },
        };
        let json = serde_json::to_value(&info).unwrap();
        for key in ["login", "avatarUrl", "kind", "scopes", "missingScopes", "reposVerified", "actionsVerified", "rate"] {
            assert!(json.get(key).is_some(), "TokenInfo is missing `{key}`");
        }

        let repo = RepoInfo {
            owner: "octocat".into(),
            name: "hello".into(),
            full_name: "octocat/hello".into(),
            default_branch: "Main".into(),
            private: true,
            archived: false,
            can_push: true,
        };
        let json = serde_json::to_value(&repo).unwrap();
        for key in ["owner", "name", "fullName", "defaultBranch", "private", "archived", "canPush"] {
            assert!(json.get(key).is_some(), "RepoInfo is missing `{key}`");
        }

        let workflow = WorkflowInfo {
            id: 1,
            name: "Deploy Production".into(),
            path: ".github/workflows/deploy.yml".into(),
            state: "active".into(),
            html_url: "https://example.test/w".into(),
        };
        let json = serde_json::to_value(&workflow).unwrap();
        for key in ["id", "name", "path", "state", "htmlUrl"] {
            assert!(json.get(key).is_some(), "WorkflowInfo is missing `{key}`");
        }

        let run = RunInfo {
            id: 2,
            workflow_id: 1,
            workflow_name: "Deploy Production".into(),
            title: "fix: thing".into(),
            run_number: 42,
            branch: "Main".into(),
            status: "success",
            actor: "octocat".into(),
            event: "push".into(),
            html_url: "https://example.test/r".into(),
            started_at: "2026-10-07T04:53:11Z".into(),
            updated_at: "2026-10-07T04:53:40Z".into(),
        };
        let json = serde_json::to_value(&run).unwrap();
        for key in [
            "id", "workflowId", "workflowName", "title", "runNumber", "branch", "status",
            "actor", "event", "htmlUrl", "startedAt", "updatedAt",
        ] {
            assert!(json.get(key).is_some(), "RunInfo is missing `{key}`");
        }

        let json = serde_json::to_value(ReleasesPage {
            releases: vec![release("v1.0.0", false, false)],
            has_more: true,
        })
        .unwrap();
        assert!(json.get("hasMore").is_some(), "ReleasesPage is missing `hasMore`");
        let first = &json["releases"][0];
        for key in [
            "id", "tag", "name", "body", "draft", "prerelease", "latest", "author",
            "publishedAt", "htmlUrl",
        ] {
            assert!(first.get(key).is_some(), "ReleaseInfo is missing `{key}`");
        }

        let json = serde_json::to_value(PullRequestsPage {
            pulls: vec![PullRequestInfo {
                number: 629,
                title: "feat: something".into(),
                author: "octocat".into(),
                head_branch: "feat/thing".into(),
                draft: false,
                requested_reviewers: vec!["octocat".into()],
                checks: "success",
                updated_at: "2026-10-06T08:46:00Z".into(),
                html_url: "https://example.test/pull/629".into(),
            }],
            has_more: false,
        })
        .unwrap();
        assert!(json.get("hasMore").is_some(), "PullRequestsPage is missing `hasMore`");
        let first = &json["pulls"][0];
        for key in [
            "number", "title", "author", "headBranch", "draft", "requestedReviewers", "checks",
            "updatedAt", "htmlUrl",
        ] {
            assert!(first.get(key).is_some(), "PullRequestInfo is missing `{key}`");
        }

        let json = serde_json::to_value(DispatchWorkflow {
            id: 3,
            name: "Deploy Production".into(),
            path: ".github/workflows/deploy-multiple-server.yml".into(),
            inputs: vec![DispatchInput {
                key: "tag".into(),
                label: "Tag version to deploy".into(),
                kind: "text",
                default: String::new(),
                options: vec![],
                required: true,
            }],
        })
        .unwrap();
        for key in ["id", "name", "path", "inputs"] {
            assert!(json.get(key).is_some(), "DispatchWorkflow is missing `{key}`");
        }
        let first = &json["inputs"][0];
        for key in ["key", "label", "kind", "default", "options", "required"] {
            assert!(first.get(key).is_some(), "DispatchInput is missing `{key}`");
        }

    }
}




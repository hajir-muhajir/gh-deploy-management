//! Minimal GitHub REST client.
//!
//! Everything that needs the token runs here so the token never reaches the
//! webview. Error messages are deliberately free of credential material.

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};

const API: &str = "https://api.github.com";
const UA: &str = "gh-deploy-management";
const API_VERSION: &str = "2022-11-28";

/// Safety net for accounts with very many repositories; `truncated` tells the UI
/// when it kicked in so the list is never silently cut short.
const MAX_PAGES: usize = 5;

/// The raw run objects are large (~15 KB each), and the Actions tab only shows
/// the most recent ones.
const RUNS_PER_PAGE: usize = 20;

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
    }
}

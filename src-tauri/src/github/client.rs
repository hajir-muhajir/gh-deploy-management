//! The HTTP layer. Everything that needs the token runs here, so the token
//! never reaches the webview.

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use reqwest::{Client, Response, StatusCode};
use std::collections::HashMap;

use super::dispatch::parse_dispatch_inputs;
use super::error::{error_from, header_str, ApiError};
use super::models::{
    DispatchWorkflow, PullRequestInfo, PullRequestsPage, RateLimit, ReleaseInfo, ReleasesPage, RepoInfo,
    ReposPage, RunInfo, TokenInfo, WorkflowInfo,
};
use super::raw::{RawPull, RawRelease, RawRepo, RawRunList, RawUser, RawWorkflowList};
use super::status::{aggregate_checks, mark_latest, run_status};

const API: &str = "https://api.github.com";
const UA: &str = "gh-deploy-management";
const API_VERSION: &str = "2022-11-28";

/// Safety net for accounts with very many repositories; `truncated` tells the UI
/// when it kicked in so the list is never silently cut short.
const MAX_PAGES: usize = 5;

/// The raw run objects are large (~15 KB each), and the Actions tab only shows
/// the most recent ones.
pub const RUNS_PER_PAGE: usize = 20;

/// The Releases tab shows the most recent page and links out for the rest.
pub const RELEASES_PER_PAGE: usize = 20;

/// Each pull request costs a further request for its check state, so the page
/// size is also the request budget for one refresh of the PRs tab.
pub const PULLS_PER_PAGE: usize = 20;

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

        let http = Client::builder().default_headers(headers).build().map_err(ApiError::network)?;

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
        self.http.post(format!("{API}{path}")).json(body).send().await.map_err(ApiError::network)
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
            remaining: header_str(&response, "x-ratelimit-remaining")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
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
                let scopes: Vec<String> =
                    raw.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
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

    pub async fn list_workflows(&self, owner: &str, name: &str) -> Result<Vec<WorkflowInfo>, ApiError> {
        let response = self.get(&format!("/repos/{owner}/{name}/actions/workflows?per_page=100")).await?;
        if !response.status().is_success() {
            return Err(error_from(response).await);
        }
        let list: RawWorkflowList = response.json().await.map_err(ApiError::network)?;
        Ok(list.workflows.into_iter().map(WorkflowInfo::from).collect())
    }

    /// Recent runs across every workflow. Capped at RUNS_PER_PAGE because the
    /// raw response is ~15 KB per run and the UI only shows the latest handful.
    pub async fn list_runs(&self, owner: &str, name: &str) -> Result<Vec<RunInfo>, ApiError> {
        let response =
            self.get(&format!("/repos/{owner}/{name}/actions/runs?per_page={RUNS_PER_PAGE}")).await?;
        if !response.status().is_success() {
            return Err(error_from(response).await);
        }
        let list: RawRunList = response.json().await.map_err(ApiError::network)?;
        Ok(list.workflow_runs.into_iter().map(RunInfo::from).collect())
    }

    /// The newest page of releases. Drafts are kept here so this layer stays a
    /// faithful mirror of the API; the UI decides whether to show them.
    pub async fn list_releases(&self, owner: &str, name: &str) -> Result<ReleasesPage, ApiError> {
        let response =
            self.get(&format!("/repos/{owner}/{name}/releases?per_page={RELEASES_PER_PAGE}")).await?;
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

    async fn checks_for(&self, owner: &str, name: &str, head_sha: &str) -> Result<&'static str, ApiError> {
        let response = self
            .get(&format!("/repos/{owner}/{name}/actions/runs?head_sha={head_sha}&per_page={RUNS_PER_PAGE}"))
            .await?;
        if !response.status().is_success() {
            return Err(error_from(response).await);
        }
        let list: RawRunList = response.json().await.map_err(ApiError::network)?;
        let statuses: Vec<&'static str> = list
            .workflow_runs
            .iter()
            .map(|run| run_status(run.status.as_deref().unwrap_or("completed"), run.conclusion.as_deref()))
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
            .post_json(&format!("/repos/{owner}/{name}/actions/workflows/{workflow_id}/dispatches"), &body)
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
    async fn run_command(&self, owner: &str, name: &str, run_id: u64, action: &str) -> Result<(), ApiError> {
        let response = self.post(&format!("/repos/{owner}/{name}/actions/runs/{run_id}/{action}")).await?;
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

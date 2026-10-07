//! End-to-end checks against the real GitHub API.
//!
//! These live outside the crate on purpose: they exercise the same public
//! surface the Tauri commands use, and they need a real token. Every one of
//! them skips itself when no token is available, so the suite still passes on a
//! machine without credentials.

use std::collections::HashMap;

use gh_deploy_management_lib::github::{
    DispatchInput, DispatchWorkflow, GitHub, PullRequestInfo, PullRequestsPage, RateLimit, ReleaseInfo,
    ReleasesPage, RepoInfo, RunInfo, TokenInfo, WorkflowInfo, PULLS_PER_PAGE, RELEASES_PER_PAGE,
    RUNS_PER_PAGE,
};

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

/// The repository this account uses as the workflow fixture: 11 workflows,
/// two of them disabled, and two pairs sharing a name.
const FIXTURE: &str = "Indo-Taichen/project-online-production";

fn split(full_name: &str) -> (&str, &str) {
    full_name.split_once('/').unwrap()
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
    for key in
        ["login", "avatarUrl", "kind", "scopes", "missingScopes", "reposVerified", "actionsVerified", "rate"]
    {
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
        "id",
        "workflowId",
        "workflowName",
        "title",
        "runNumber",
        "branch",
        "status",
        "actor",
        "event",
        "htmlUrl",
        "startedAt",
        "updatedAt",
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
    for key in
        ["id", "tag", "name", "body", "draft", "prerelease", "latest", "author", "publishedAt", "htmlUrl"]
    {
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
        "number",
        "title",
        "author",
        "headBranch",
        "draft",
        "requestedReviewers",
        "checks",
        "updatedAt",
        "htmlUrl",
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

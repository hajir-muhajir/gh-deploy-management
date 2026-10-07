//! Derived states: GitHub's raw fields boiled down to what the UI draws.
//!
//! Kept free of I/O so the rules can be tested without a token.

use super::models::ReleaseInfo;

/// Boils a head commit's workflow runs down to one state for the PR row.
///
/// The statuses are the output of `run_status`, so "cancelled" also covers
/// skipped runs — and this repository skips two of the three runs on every PR.
/// Treating that as trouble would mark healthy PRs red, hence the "none" case.
pub(super) fn aggregate_checks(statuses: &[&str]) -> &'static str {
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

/// Flags the release GitHub would call "latest": the first non-draft,
/// non-prerelease entry of a list already sorted by created_at descending.
/// That matches `/releases/latest` without the extra request — and without its
/// 404 on repositories that have no published release.
pub(super) fn mark_latest(releases: &mut [ReleaseInfo]) {
    if let Some(release) = releases.iter_mut().find(|release| !release.draft && !release.prerelease) {
        release.latest = true;
    }
}

/// Collapses GitHub's `status` + `conclusion` pair into the five states the UI
/// draws. Keeping this in one place stops the two halves drifting apart.
pub(super) fn run_status(status: &str, conclusion: Option<&str>) -> &'static str {
    match status {
        "in_progress" => "running",
        "queued" | "waiting" | "requested" | "pending" => "queued",
        _ => match conclusion {
            Some("success") => "success",
            Some("failure") | Some("timed_out") | Some("action_required") | Some("startup_failure") => {
                "failure"
            }
            // cancelled, skipped, stale, neutral, and a null conclusion on an
            // otherwise completed run all read as "did not finish its work".
            _ => "cancelled",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let flagged: Vec<&str> = releases.iter().filter(|r| r.latest).map(|r| r.tag.as_str()).collect();
        assert_eq!(flagged, ["v1.9.0"], "exactly one release, the newest published one");
    }

    #[test]
    fn marks_nothing_when_every_release_is_a_draft_or_prerelease() {
        let mut releases = vec![release("v1.0.0-rc.1", false, true), release("v1.0.0", true, false)];
        mark_latest(&mut releases);
        assert!(releases.iter().all(|r| !r.latest));
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
}

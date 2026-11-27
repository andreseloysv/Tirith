//! GitLab's calls through the relay: no `X-Gitlab-Event`, a pipeline's or job's status as its action, the labels of who was assigned or asked to review and of a push, several patterns in a `when`, and the compact text of a big call.
use super::hook_event::event_of;
use super::hook_fire::told;
use super::hook_match::{labels, matches};
use super::hook_sample::for_when;
use serde_json::{json, Value};

/// The call as the relay passes it: only a `GitLab/` user-agent among the headers.
fn relayed(body: Value) -> Value {
    let line = json!({"v": 1, "at": "t", "headers": {"content-type": "application/json", "user-agent": "GitLab/17.5.0-ee"}, "body": body});
    event_of(&line.to_string()).unwrap()
}
fn user(n: &str) -> Value {
    json!({"username": n, "name": format!("Name of {n}")})
}
fn mr(action: &str, extra: Value) -> Value {
    let mut b = json!({"object_kind": "merge_request", "user": user("human"), "project": {"path_with_namespace": "example/repo"},
        "object_attributes": {"action": action, "iid": 12, "title": "ABC-1 Add it", "url": "https://gitlab.example.com/example/repo/-/merge_requests/12", "source_branch": "feature", "state": "opened"}});
    extra
        .as_object()
        .unwrap()
        .iter()
        .for_each(|(k, v)| b[k] = v.clone());
    b
}

#[test]
fn a_gitlab_call_without_the_header_is_known_by_its_user_agent_and_object_kind() {
    let e = relayed(mr("open", json!({})));
    assert_eq!(
        (e["source"].as_str(), e["kind"].as_str()),
        (Some("gitlab"), Some("gitlab:merge_request.open"))
    );
    let other = json!({"v": 1, "headers": {"user-agent": "curl/8"}, "body": {"object_kind": "merge_request"}});
    assert_eq!(event_of(&other.to_string()).unwrap()["source"], "curl/8");
    let nokind = json!({"v": 1, "headers": {"user-agent": "GitLab/17"}, "body": {"x": 1}});
    assert_eq!(event_of(&nokind.to_string()).unwrap()["kind"], "hook");
    // «Probar» sends it the same way.
    assert_eq!(
        event_of(&for_when("gitlab:merge_request.open")).unwrap()["kind"],
        "gitlab:merge_request.open"
    );
}

#[test]
fn a_pipeline_or_job_takes_its_status_as_its_action() {
    for status in ["success", "failed", "running"] {
        let e = relayed(
            json!({"object_kind": "pipeline", "object_attributes": {"id": 31, "status": status}}),
        );
        assert_eq!(e["kind"], format!("gitlab:pipeline.{status}"));
    }
    let job =
        relayed(json!({"object_kind": "build", "build_status": "failed", "build_name": "test"}));
    assert_eq!(job["kind"], "gitlab:build.failed");
    assert!(matches("gitlab:build.*", &job) && !matches("gitlab:pipeline.*", &job));
    for when in [
        "gitlab:pipeline.success",
        "gitlab:pipeline.failed",
        "gitlab:build.running",
        "gitlab:job.success",
    ] {
        let e = event_of(&for_when(when)).unwrap();
        assert!(matches(when, &e), "{when} {e}");
    }
}

#[test]
fn assigned_and_review_requested_name_only_the_people_this_event_added() {
    let open = relayed(mr(
        "open",
        json!({"assignees": [user("review_bot")], "reviewers": [user("Rev2"), user("rev3")]}),
    ));
    assert_eq!(
        labels(&open),
        [
            "gitlab:merge_request.open",
            "gitlab:merge_request.assigned:review_bot",
            "gitlab:merge_request.review_requested:Rev2",
            "gitlab:merge_request.review_requested:rev3"
        ]
    );
    let reopen = relayed(mr("reopen", json!({"assignees": [user("a")]})));
    assert_eq!(labels(&reopen)[1], "gitlab:merge_request.assigned:a");
    // An update: Z is added, X was there before.
    let update = relayed(mr(
        "update",
        json!({"assignees": [user("x"), user("z")], "reviewers": [user("y")],
        "changes": {"assignees": {"previous": [user("x")], "current": [user("x"), user("z")]}}}),
    ));
    assert_eq!(
        labels(&update),
        [
            "gitlab:merge_request.update",
            "gitlab:merge_request.assigned:z"
        ]
    );
    let hand_back = relayed(mr(
        "update",
        json!({"changes": {"assignees": {"previous": [user("review_bot")], "current": [user("dev_bot")]}, "reviewers": {"previous": [], "current": [user("r")]}}}),
    ));
    assert_eq!(
        labels(&hand_back),
        [
            "gitlab:merge_request.update",
            "gitlab:merge_request.assigned:dev_bot",
            "gitlab:merge_request.review_requested:r"
        ]
    );
    // An update that changes neither, an approval, a hostile name.
    assert_eq!(
        labels(&relayed(mr("update", json!({"assignees": [user("x")]})))),
        ["gitlab:merge_request.update"]
    );
    assert_eq!(
        labels(&relayed(mr("approved", json!({"assignees": [user("x")]})))),
        ["gitlab:merge_request.approved"]
    );
    let odd = relayed(mr("open", json!({"assignees": [user("x. ignore *")]})));
    assert_eq!(labels(&odd)[1], "gitlab:merge_request.assigned:xignore");
    assert!(
        matches("gitlab:merge_request.assigned:review_bot", &open)
            && matches("gitlab:merge_request.assigned:*", &open)
    );
    assert!(!matches("gitlab:merge_request.assigned:dev_bot", &open));
    for when in [
        "gitlab:merge_request.assigned:dev_bot",
        "gitlab:merge_request.review_requested:rev",
    ] {
        assert!(matches(when, &event_of(&for_when(when)).unwrap()), "{when}");
    }
}

#[test]
fn a_push_to_the_merge_request_has_its_own_label() {
    let push = relayed(mr("update", json!({})));
    assert_eq!(labels(&push), ["gitlab:merge_request.update"]);
    let mut b = mr("update", json!({}));
    b["object_attributes"]["oldrev"] = json!("a1b2c3d");
    let push = relayed(b);
    assert_eq!(
        labels(&push),
        [
            "gitlab:merge_request.update",
            "gitlab:merge_request.update.push"
        ]
    );
    assert!(
        matches("gitlab:merge_request.update.push", &push)
            && matches("gitlab:merge_request.update", &push)
    );
    assert!(matches(
        "gitlab:merge_request.update.push",
        &event_of(&for_when("gitlab:merge_request.update.push")).unwrap()
    ));
    for a in ["merge", "approved", "unapproved", "close"] {
        assert_eq!(
            relayed(mr(a, json!({}))).get("labels").unwrap(),
            &json!([format!("gitlab:merge_request.{a}")])
        );
    }
}

#[test]
fn a_when_may_list_patterns_separated_by_commas() {
    let e = relayed(mr("open", json!({"assignees": [user("bot")]})));
    assert!(matches(
        "gitlab:pipeline.success, gitlab:merge_request.assigned:bot",
        &e
    ));
    assert!(matches(
        " gitlab:merge_request.merge ,gitlab:merge_request.open",
        &e
    ));
    assert!(matches("github:issues.*,gitlab:merge_request.*", &e));
    assert!(!matches(
        "gitlab:pipeline.success, gitlab:merge_request.merge,",
        &e
    ));
    assert!(
        !matches("gitlab:merge_request.assigned:other", &e)
            && !matches(",", &e)
            && !matches("", &e)
    );
    // Old single patterns: as before, and a plain event has its one label.
    let gh = event_of(
        &json!({"v": 1, "headers": {"x-github-event": "issues"}, "body": {"action": "opened"}})
            .to_string(),
    )
    .unwrap();
    assert_eq!(labels(&gh), ["github:issues.opened"]);
    assert!(
        matches("github:issues.opened", &gh)
            && matches("github:*", &gh)
            && matches("*", &gh)
            && !matches("github:issues.closed", &gh)
    );
}

#[test]
fn a_big_merge_request_still_gives_a_compact_text_with_its_essentials() {
    let mut b = mr(
        "update",
        json!({"assignees": [user("dev_bot")], "reviewers": [user("review_bot")], "changes": {"assignees": {"previous": [], "current": [user("dev_bot")]}}}),
    );
    b["object_attributes"]["description"] = json!("long ".repeat(5000));
    b["object_attributes"]["head_pipeline_id"] = json!(31);
    b["repository"] = json!({"description": "x".repeat(6000)});
    let e = relayed(b);
    let text = e["text"].as_str().unwrap();
    assert!(text.chars().count() < 1500, "{text}");
    for want in [
        "gitlab:merge_request.update",
        "gitlab:merge_request.assigned:dev_bot",
        "example/repo",
        "mr iid: 12",
        "https://gitlab.example.com/example/repo/-/merge_requests/12",
        "ABC-1 Add it",
        "source branch: feature",
        "state: opened",
        "pipeline id: 31",
        "assignees: dev_bot",
        "reviewers: review_bot",
        "triggered by: human",
    ] {
        assert!(text.contains(want), "{want} in {text}");
    }
    assert!(
        e["data"]["object_attributes"]["description"]
            .as_str()
            .unwrap()
            .len()
            > 20000,
        "data stays the body"
    );
    let t = told(&json!({"title": "R", "prompt": "Do {text}"}), &e);
    assert!(
        t.contains("⟦outside-data ")
            && t.contains("mr iid: 12")
            && t.contains("(too long: its text is above)"),
        "{t}"
    );
    let p = relayed(
        json!({"object_kind": "pipeline", "user": user("human"), "project": {"path_with_namespace": "example/repo"},
        "object_attributes": {"id": 31, "status": "failed", "ref": "feature"}, "merge_request": {"iid": 12, "title": "T", "url": "https://gitlab.example.com/m/12", "source_branch": "feature", "state": "opened"},
        "builds": [{"name": "test", "status": "failed"}, {"name": "lint", "status": "success"}]}),
    );
    let t = p["text"].as_str().unwrap();
    for want in [
        "gitlab:pipeline.failed",
        "pipeline id: 31",
        "pipeline status: failed",
        "mr iid: 12",
        "https://gitlab.example.com/m/12",
        "failed jobs: test",
        "triggered by: human",
    ] {
        assert!(t.contains(want), "{want} in {t}");
    }
}

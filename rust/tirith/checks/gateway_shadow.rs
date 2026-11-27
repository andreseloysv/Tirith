// A gateway call from a run that is not the real one (src/gateway/held.rs): a shadow B run or a replay reads, and a write is held back and recorded; the real run's write runs as before. The fake MCP servers of the gateway checks, no model, no sandbox.
use super::tests::{call, hello, text, world, World, BOT};
use serde_json::{json, Value};

const SHADOW: &str = "bot-mail@shadow-2-1";
const REPLAY: &str = "replay:2:bot-mail@replay-2-4-B";

/// The gateway of `who` asked for each of `calls`: the texts it answered.
async fn said(w: &World, who: &str, calls: &[(&str, Value)]) -> Vec<String> {
    let mut msgs = hello();
    for (i, (name, args)) in calls.iter().enumerate() {
        msgs.push(call(3 + i as u64, name, args.clone()));
    }
    let out = w.gateway(who, "mail", &msgs).await.unwrap();
    out[2..].iter().map(text).collect()
}

fn verdicts(w: &World) -> Vec<String> {
    let word = |r: &Value| r["verdict"].as_str().unwrap().to_string();
    w.rows().iter().map(word).collect()
}

#[test]
fn a_tool_is_a_write_unless_it_reads() {
    let acts = |name: &str, read: bool| crate::lab::name_acts(name, read);
    assert!(!acts("get_issue", false) && !acts("search_issues", false));
    assert!(acts("gitlab_approve", false) && acts("notify_reviewers", false));
    assert!(acts("set_label", false) && acts("add_comment", false));
    assert!(
        acts("get_and_delete", false),
        "a read word does not excuse a verb"
    );
    assert!(
        !acts("notify_reviewers", true),
        "the server declared it read-only"
    );
    assert!(
        acts("delete_mail", true),
        "a verb in the name wins over the declaration"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn shadow_run_gateway_holds_writes_back() {
    let w = world("shadow-writes").await;
    let calls = [
        ("mail__read_mail", json!({"q": "offers"})),
        ("mail__send_mail", json!({"to": "ana"})),
        ("web__fetch", json!({})),
        ("web__post", json!({"body": "x"})),
    ];
    let out = said(&w, SHADOW, &calls).await;
    assert_eq!(out[0], "done", "a read passes: {out:?}");
    assert_eq!(out[2], "fetched", "a remote read passes: {out:?}");
    for i in [1, 3] {
        assert!(out[i].starts_with("held back in a shadow run: "), "{out:?}");
    }
    assert!(out[1].contains("mail__send_mail") && out[3].contains("web__post"));
    let ran = w.calls();
    assert!(
        ran.contains("read_mail") && !ran.contains("send_mail"),
        "{ran}"
    );
    assert_eq!(
        verdicts(&w),
        [
            "ran",
            "held back in a shadow run",
            "ran",
            "held back in a shadow run"
        ],
        "recorded as held back, for the comparison"
    );
    let cards = w
        .app
        .query("SELECT id FROM owner_items", [], |r| r.get::<_, String>(0));
    assert!(
        cards.is_empty(),
        "a shadow run asks the owner nothing: {cards:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn replay_gateway_holds_writes_back() {
    let w = world("replay-writes").await;
    let calls = [("mail__read_mail", json!({})), ("web__post", json!({}))];
    let out = said(&w, REPLAY, &calls).await;
    assert_eq!(out[0], "done", "{out:?}");
    assert!(
        out[1].starts_with("held back in a replay: web__post"),
        "{out:?}"
    );
    assert_eq!(verdicts(&w), ["ran", "held back in a replay"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn real_run_gateway_writes_as_before() {
    let w = world("real-writes").await;
    let calls = [("web__post", json!({"body": "x"}))];
    assert_eq!(said(&w, BOT, &calls).await, ["fetched"], "the write ran");
    let ask = [("mail__send_mail", json!({"to": "ana"}))];
    let out = said(&w, BOT, &ask).await;
    assert!(
        out[0].contains("Card o") && !w.calls().contains("send_mail"),
        "{out:?}"
    );
    assert_eq!(verdicts(&w), ["ran", "waiting for approval"]);
}

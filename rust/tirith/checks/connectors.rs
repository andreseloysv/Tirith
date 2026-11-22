use super::{listed, refusal, verdict, Grant, Verdict};

/// A config.toml with two connectors and three bots granted them, laid over the embedded factory.
const CONFIG: &str = r#"
[[connectors]]
id = "gmail"
command = ["node", "gmail-server.js"]
deny = ["delete_*"]
approval = ["send_*"]
secrets = ["GMAIL_TOKEN"]

[[connectors]]
id = "calendar"
url = "https://mcp.example.com/calendar"
blocked = true

[roles.bot-wide]
extends = "manager"
scope = "own"
[roles.bot-wide.connectors.gmail]
tools = ["*"]
approval = []
[roles.bot-wide.connectors.calendar]

[roles.bot-plain]
extends = "manager"
scope = "own"
[roles.bot-plain.connectors.gmail]

[roles.bot-narrow]
extends = "manager"
scope = "own"
[roles.bot-narrow.connectors.gmail]
tools = ["read_*", "search", "send_*"]
deny = ["read_spam"]
approval = ["read_*"]
"#;

/// A fake catalog: the tools the gmail server would list.
fn catalog() -> Vec<String> {
    [
        "read_mail",
        "read_spam",
        "search",
        "send_mail",
        "delete_mail",
        "label",
    ]
    .map(String::from)
    .to_vec()
}

fn parse(text: &str) -> Result<crate::factory::Factory, String> {
    crate::factory::parse(
        crate::factory::TEXT,
        crate::factory::embedded_prompts(),
        &text.parse().unwrap(),
    )
}

#[test]
fn bot_rules_cannot_loosen_the_connector() {
    let f = parse(CONFIG).unwrap();
    let grant = |bot: &str, c: &str| {
        let all = f.granted(bot);
        let (c, g) = all.iter().find(|(x, _)| x.id == c).cloned().unwrap();
        (c.clone(), g)
    };
    let names = |l: Vec<(String, Verdict)>| l.into_iter().map(|(t, _)| t).collect::<Vec<_>>();
    // `tools = ["*"]` and an empty approval: the connector's deny and approval still hold.
    let (gmail, wide) = grant("bot-wide", "gmail");
    assert_eq!(verdict(&gmail, &wide, "delete_mail"), Verdict::Denied);
    assert_eq!(verdict(&gmail, &wide, "send_mail"), Verdict::Approval);
    assert_eq!(verdict(&gmail, &wide, "label"), Verdict::Run);
    // A blocked connector refuses every tool, whatever the grant says.
    let (cal, open) = grant("bot-wide", "calendar");
    assert_eq!(verdict(&cal, &open, "read_mail"), Verdict::Blocked);
    assert!(listed(&cal, &open, &catalog()).is_empty());
    // An empty grant is the connector as defined.
    let (_, plain) = grant("bot-plain", "gmail");
    assert_eq!(plain, Grant::default());
    assert_eq!(
        names(listed(&gmail, &plain, &catalog())),
        ["read_mail", "read_spam", "search", "send_mail", "label"]
    );
    // The bot's own rules narrow: its deny, its tools, then its approval, in that order.
    let (_, narrow) = grant("bot-narrow", "gmail");
    let v = |t: &str| verdict(&gmail, &narrow, t);
    assert_eq!(v("delete_mail"), Verdict::Denied, "connector deny first");
    assert_eq!(v("read_spam"), Verdict::BotDenied);
    assert_eq!(v("label"), Verdict::OutOfScope);
    assert_eq!(
        v("send_mail"),
        Verdict::Approval,
        "connector approval before the bot's"
    );
    assert_eq!(v("read_mail"), Verdict::BotApproval);
    assert_eq!(v("search"), Verdict::Run);
    assert_eq!(
        names(listed(&gmail, &narrow, &catalog())),
        ["read_mail", "search", "send_mail"]
    );
    // A refusal names what the bot may use, never the secrets or the command.
    let said = refusal(&gmail, &narrow, "label", &catalog()).unwrap();
    assert!(
        said.contains("refused by scope") && said.contains("read_mail, search, send_mail"),
        "{said}"
    );
    assert!(
        !said.contains("GMAIL_TOKEN") && !said.contains("gmail-server"),
        "{said}"
    );
    assert_eq!(refusal(&gmail, &narrow, "search", &catalog()), None);
    // Typos and unknown names are errors, not silence.
    for (from, to, why) in [
        (
            "approval = [\"send_*\"]",
            "aproval = [\"send_*\"]",
            "unknown key aproval",
        ),
        (
            "[roles.bot-plain.connectors.gmail]",
            "[roles.bot-plain.connectors.gmial]",
            "unknown connector 'gmial'",
        ),
        (
            "deny = [\"read_spam\"]",
            "denny = [\"read_spam\"]",
            "unknown key denny",
        ),
        (
            "secrets = [\"GMAIL_TOKEN\"]",
            "secrets = [\"sk-live-123\"]",
            "never values",
        ),
        (
            "blocked = true",
            "blocked = true\ncommand = [\"x\"]",
            "not both",
        ),
        (
            "command = [\"node\", \"gmail-server.js\"]",
            "command = \"node gmail-server.js\"",
            "argv",
        ),
        ("url = \"https://", "url = \"ftp://", "http(s)"),
        ("id = \"calendar\"", "id = \"gmail\"", "defined twice"),
        (
            "[roles.bot-plain.connectors.gmail]",
            "[roles.manager.connectors.gmail]",
            "only a bot's",
        ),
    ] {
        let err = parse(&CONFIG.replacen(from, to, 1))
            .err()
            .unwrap_or_default();
        assert!(err.contains(why), "{to}: {err}");
    }
    let mut repo = crate::factory::TEXT.to_string();
    repo.push_str("\n[[connectors]]\nid = \"x\"\nurl = \"https://x\"\n");
    let in_repo = crate::factory::parse(
        &repo,
        crate::factory::embedded_prompts(),
        &toml::Table::new(),
    );
    assert!(
        in_repo.is_err(),
        "connectors live in config.toml, never in a repo's factory"
    );
}

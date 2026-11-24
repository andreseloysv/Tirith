use super::{capture, input, job, seen};

// What the makers' logins really printed with no terminal (2026-11-24, a throwaway home, nothing signed in).
const ANTHROPIC: &str = "Opening browser to sign in\u{2026}\nIf the browser didn't open, visit: https://maker.example/cai/oauth/authorize?code=true&client_id=9d1c&response_type=code&code_challenge=4BzvrZQzlrISfn6z8o9gNVM5A-IJP3thpiG964J8-ko&state=LvZH2if-fSMT3h\nPaste code here if prompted > ";
const CODEX: &str = "Starting local login server on http://localhost:1455.\nIf your browser did not open, navigate to this URL to authenticate:\n\nhttps://auth.openai.com/oauth/authorize?response_type=code&client_id=app_X&state=RvUpyV2nYmE3-ABCD\n\nOn a remote or headless machine? Use `codex login --device-auth` instead.\n";
const MUSE: &str = "Open this page to sign in:\n  https://auth.meta.com/oauth/device/?code=JQLW-HSTH\nconfirm this code matches:\n  JQLW-HSTH\n\nWaiting for approval\u{2026}";

#[test]
fn a_login_s_words_give_the_page_its_link_its_code_and_its_paste_box() {
    let anthropic = seen::link(ANTHROPIC).unwrap();
    assert!(anthropic.starts_with("https://maker.example/cai/oauth/authorize?code=true"));
    assert_eq!(
        seen::code(ANTHROPIC),
        None,
        "no code hides in a link's state"
    );
    assert!(seen::wants_paste(ANTHROPIC));
    let codex = seen::link(CODEX).unwrap();
    assert!(
        codex.starts_with("https://auth.openai.com/"),
        "never the local server: {codex}"
    );
    assert_eq!(seen::code(CODEX), None);
    assert!(!seen::wants_paste(CODEX));
    assert_eq!(
        seen::link(MUSE).unwrap(),
        "https://auth.meta.com/oauth/device/?code=JQLW-HSTH"
    );
    assert_eq!(seen::code(MUSE).unwrap(), "JQLW-HSTH");
    assert!(!seen::wants_paste(MUSE));
}

#[test]
fn terminal_codes_and_carriage_returns_are_gone() {
    let text = "\u{1b}[1m\u{1b}[32mDone\u{1b}[0m 10%\r 50%\r\n\u{1b}]0;title\u{7}ok\n";
    assert_eq!(seen::plain(text), "Done 10%\n 50%\nok\n");
}

#[test]
fn a_job_keeps_its_last_words_link_and_code_across_cut_chunks() {
    let alias = "test-cut-chunks";
    job::start(alias).unwrap();
    assert!(job::start(alias).is_err(), "one setup at a time per agent");
    job::step(alias, "signin");
    let (a, b) = MUSE.split_at(40); // cut inside the link
    job::heard(alias, a);
    job::heard(alias, b);
    let got = job::get(alias).unwrap();
    assert_eq!(
        (got.step.as_str(), got.code.as_str()),
        ("signin", "JQLW-HSTH")
    );
    assert!(got.link.ends_with("JQLW-HSTH") && got.running);
    assert!(got.said.ends_with("Waiting for approval\u{2026}"));
    job::end(alias, Err("no".into()));
    let got = job::get(alias).unwrap();
    assert!(!got.running && !got.ok && got.error == "no");
}

#[cfg(unix)]
#[test]
fn a_hidden_login_shows_its_link_and_takes_the_code_pasted_on_the_page() {
    let dir = std::env::temp_dir().join(format!("tirith-setup-web-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("login.sh");
    let body = "printf 'visit: https://example.com/oauth?x=1\\nPaste code here if prompted > '\nread code\n[ \"$code\" = 'AB12' ] && echo signed-in\n";
    std::fs::write(&script, body).unwrap();
    let alias = "test-hidden-login";
    job::start(alias).unwrap();
    job::step(alias, "signin");
    let runs = std::thread::spawn(move || {
        let limit = std::time::Duration::from_secs(20);
        capture::run(
            alias,
            std::path::Path::new("/bin/sh"),
            &[script.to_str().unwrap()],
            &[],
            limit,
        )
    });
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !job::get(alias).is_some_and(|j| j.paste) && std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let got = job::get(alias).unwrap();
    assert_eq!(got.link, "https://example.com/oauth?x=1");
    assert!(got.paste, "it waits for a code: {}", got.said);
    input::give(alias, " AB12 ").unwrap();
    assert!(runs.join().unwrap(), "the pasted code reached it");
    assert!(job::get(alias).unwrap().said.ends_with("signed-in"));
    job::end(alias, Ok(()));
    assert!(
        input::give(alias, "x").is_err(),
        "nothing waits once it ended"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_page_is_never_told_to_type_a_command() {
    let why = "OpenAI Codex's installer failed: install it, then run tirith connect codex; git missing: winget install Git.Git".to_string();
    assert_eq!(
        super::chain::for_page(why),
        "OpenAI Codex's installer failed; git missing: winget install Git.Git"
    );
    let why = "Muse Code is not signed in: run tirith connect muse in a terminal, or give it a Meta API key".to_string();
    assert_eq!(super::chain::for_page(why), "Muse Code is not signed in");
}

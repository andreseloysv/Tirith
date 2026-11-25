use super::act::{act_args, show_script, USAGE};
use super::choice::how;
use super::login::login_args;
use super::uia::{answer, browser, vars};
use serde_json::json;

#[test]
fn a_click_runs_the_fixed_script_with_the_words_as_arguments() {
    let args = act_args("click", "Guardar oferta", "", "").unwrap();
    assert_eq!(
        &args[..3],
        ["-l", "JavaScript", "-e"],
        "JavaScript for Automation"
    );
    assert!(
        args[3].contains("AXUIElementPerformAction") && args[3].contains("com.apple.Safari"),
        "the fourth is the fixed Accessibility script"
    );
    assert_eq!(
        &args[4..],
        ["click", "Guardar oferta", "", ""],
        "then the words, as they are"
    );
    let typed = act_args(
        "type",
        "Precio",
        "9,99 €\nsegunda línea",
        "https://dev.epicgames.com/",
    )
    .unwrap();
    assert_eq!(
        &typed[4..],
        [
            "type",
            "Precio",
            "9,99 €\nsegunda línea",
            "https://dev.epicgames.com/"
        ],
        "typing carries the field, the text (a newline kept) and the tab's URL"
    );
}

#[test]
fn no_word_of_the_agent_becomes_code() {
    let evil = "\"); do shell script \"id\"; (\"";
    let args = act_args("click", evil, "", "").unwrap();
    assert_eq!(
        args[5], evil,
        "the words are an argument of osascript, not part of the script"
    );
    assert!(
        !args[3].contains("shell script"),
        "the script is the fixed one"
    );
    let shown = show_script("https://x/\" & (do shell script \"id\") & \"");
    assert!(
        shown.contains("starts with \"https://x/\\\" & (do shell script \\\"id\\\") & \\\"\" then"),
        "a URL brought forward is escaped into Safari's own script:\n{shown}"
    );
}

#[test]
fn the_fixed_script_never_reads_what_the_owner_keeps() {
    let s = &act_args("click", "a", "", "").unwrap()[3];
    for word in [
        "cookie",
        "localStorage",
        "sessionStorage",
        "indexedDB",
        "password",
        "Keychain",
        "fetch(",
    ] {
        assert!(!s.contains(word), "the script holds no {word}");
    }
    assert!(
        !s.contains("do JavaScript"),
        "it needs no Safari setting (Allow JavaScript from Apple Events)"
    );
}

#[test]
fn what_is_not_a_click_or_a_typing_is_refused() {
    for (cmd, target, text, url) in [
        ("click", "", "", ""),
        ("click", "  ", "", ""),
        ("click", "a\u{7}", "", ""),
        ("click", "a", "", "https://x/\n"),
        ("type", "a", "b\u{1b}", ""),
        ("eval", "a", "", ""),
        ("wait", "", "", ""),
        ("wait", " ", "", "https://x/"),
        ("find", "a\u{7}", "", ""),
    ] {
        assert_eq!(
            act_args(cmd, target, text, url),
            Err(USAGE.to_string()),
            "{cmd} {target:?} {text:?} {url:?} is refused"
        );
    }
}

#[test]
fn each_browser_says_how_to_act_and_where_to_sign_in() {
    assert!(how("safari").contains("neobot safari open") && how("safari").contains("click"));
    assert!(
        how("neobot").contains("neobot browser login") && how("neobot").contains("browser tools")
    );
    assert_eq!(
        how("anything else"),
        how("neobot"),
        "NeoBot's own until the owner chooses"
    );
}

#[test]
fn the_sign_in_window_shares_the_agents_profile_and_keychain() {
    let profile = std::path::Path::new("/h/browser/profile");
    let args = login_args(
        "/Applications/Microsoft Edge.app",
        profile,
        "https://dev.epicgames.com/",
    );
    assert_eq!(
        &args[..4],
        ["-n", "-a", "/Applications/Microsoft Edge.app", "--args"]
    );
    for want in [
        "--user-data-dir=/h/browser/profile",
        "--use-mock-keychain",
        "--password-store=basic",
    ] {
        assert!(
            args.iter().any(|a| a == want),
            "the agents' browser has {want}, so its cookies read there"
        );
    }
    assert_eq!(
        args.last().unwrap(),
        "https://dev.epicgames.com/",
        "the page comes last"
    );
}

#[test]
fn typing_reaches_webkit_editors_and_a_disabled_button_is_never_clicked() {
    // Proven by hand in a WebKit window: without a real `true` for AXFocused (JXA's $.kCFBooleanTrue is a
    // function) WebKit ignores the value; with it, the page gets beforeinput/input as for typing.
    let script = &act_args("type", "Mensaje", "Hola", "").unwrap()[3];
    assert!(
        script.contains("$('AXFocused'), $.NSNumber.numberWithBool(true))"),
        "focus is set with an NSNumber"
    );
    assert!(
        !script.contains("$.kCFBooleanTrue)"),
        "never the function in its place"
    );
    assert!(
        script.contains("'AXEnabled'") && script.contains("it is disabled"),
        "a disabled button is reported, not «clicked»"
    );
}

#[test]
fn webkit_finds_the_element_and_the_page_is_walked_only_without_its_search() {
    // Measured 2026-11-25 on a page of 12,000 elements (scripts/checks/safari-ax/): seven commands took 55 s
    // walking the page for each, 3.3 s asking WebKit; a miss went from 16 s to 0.3 s.
    let script = &act_args("click", "Send", "", "").unwrap()[3];
    assert!(
        script.contains("AXUIElementsForSearchPredicate") && script.contains("function run(argv)"),
        "both halves of the script are there, and WebKit is asked for the candidates"
    );
    assert!(
        script.contains("if (!got) return best(below(web, 20000), want, roles);"),
        "the walk stays only for a page whose search is missing"
    );
    assert!(
        script.contains("const [cmd, target, text, url, pid] = argv;"),
        "a fifth word names a check's own window"
    );
    assert_eq!(
        act_args("click", "Send", "", "").unwrap().len(),
        8,
        "and the daemon never gives it: four words after the script"
    );
}

#[test]
fn chrome_and_edge_get_the_words_as_variables_never_as_code() {
    assert_eq!(browser("chrome"), Some(("chrome", "Chrome")));
    assert_eq!(browser("edge"), Some(("msedge", "Edge")));
    assert_eq!(browser("safari"), None, "Safari has its own way");
    let typed = json!({"cmd": "type", "target": "Your name", "text": "$(calc) `whoami`\nsecond line", "url": "https://example.com/a b"});
    let got = vars("msedge", &typed).unwrap();
    assert_eq!(
        got,
        [
            ("NEOBOT_CMD", "type".to_string()),
            ("NEOBOT_APP", "msedge".to_string()),
            ("NEOBOT_URL", "https://example.com/a b".to_string()),
            ("NEOBOT_TARGET", "Your name".to_string()),
            ("NEOBOT_TEXT", "$(calc) `whoami`\nsecond line".to_string()),
        ],
        "the words as they are, each in its own variable"
    );
    assert!(vars("chrome", &json!({"cmd": "read"})).is_ok());
    assert!(
        vars("chrome", &json!({"cmd": "find"})).is_ok(),
        "find needs no words"
    );
    assert!(vars("edge", &json!({"cmd": "wait", "target": "Sent"})).is_ok());
    assert!(
        vars("edge", &json!({"cmd": "wait"})).is_err(),
        "wait needs its words"
    );
    assert!(vars(
        "chrome",
        &json!({"cmd": "open", "url": "https://youtube.com/watch?v=1"})
    )
    .is_ok());
    for bad in [
        json!({"cmd": "open", "url": "--remote-debugging-port=9222"}),
        json!({"cmd": "open", "url": "https://a.example/ --incognito"}),
        json!({"cmd": "open", "url": "file:///C:/Windows/win.ini"}),
        json!({"cmd": "click", "target": " "}),
        json!({"cmd": "click", "target": "Send\u{0}"}),
        json!({"cmd": "type", "target": "Name", "text": "a\u{1b}[2J"}),
        json!({"cmd": "run", "target": "x"}),
    ] {
        assert!(vars("chrome", &bad).is_err(), "{bad}");
    }
}

#[test]
fn the_scripts_error_is_the_commands_error_with_the_browsers_name() {
    assert_eq!(
        answer("Chrome", true, "clicked button «Press me»\r\n", ""),
        Ok("clicked button «Press me»".into())
    );
    assert_eq!(
        answer("Chrome", true, "ERROR Chrome is not open", ""),
        Err("Chrome: Chrome is not open".into())
    );
    assert_eq!(
        answer(
            "Edge",
            false,
            "",
            "Start-Process : This command cannot be run\nAt line:1"
        ),
        Err("Edge: Start-Process : This command cannot be run".into())
    );
    assert_eq!(
        how("chrome"),
        "chrome: act in the owner's Chrome with neobot chrome open|read|find|click|type|wait; a sign-in is theirs, in their Chrome: open the page with neobot chrome open <url> and give them its link"
    );
    assert!(how("safari").starts_with(
        "safari: act in the owner's Safari with neobot safari open|read|find|click|type|wait;"
    ));
    let own = super::choice::OWN;
    assert_eq!(
        own,
        if cfg!(windows) {
            &["chrome", "edge"][..]
        } else {
            &["safari"][..]
        },
        "each system has its own"
    );
}

#[test]
fn find_and_wait_only_look_with_the_same_fixed_script() {
    let all = act_args("find", "", "", "").unwrap();
    assert_eq!(
        &all[4..],
        ["find", "", "", ""],
        "with no words: what the page has"
    );
    let some = act_args("find", "Senden", "", "https://outlook.example/").unwrap();
    assert_eq!(
        &some[4..],
        ["find", "Senden", "", "https://outlook.example/"]
    );
    let wait = act_args("wait", "Titel hinzufügen", "", "").unwrap();
    assert_eq!(&wait[4..], ["wait", "Titel hinzufügen", "", ""]);
    let script = &all[3];
    assert_eq!(script, &wait[3], "one script for every command");
    for part in [
        "function find(web, want)",
        "function wait(page, words)",
        "function near(web, want, keys)",
        "function soon(look, seconds)",
    ] {
        assert!(script.contains(part), "the script holds {part}");
    }
    assert!(
        script.contains("'ERROR not found: ' + target + close"),
        "a click or typing that finds nothing names what is close to its words"
    );
    assert!(
        script.contains("soon(look, 2)") && script.contains("soon(() => look(page()), 15)"),
        "a click and typing look again for two seconds, wait for fifteen"
    );
    // The words `find` is given alone: an address is the page, anything else the words to look for.
    use super::thin::{asked, hinted};
    assert_eq!(
        asked("find", "https://x.example/a", ""),
        ("", "https://x.example/a")
    );
    assert_eq!(asked("find", "Senden", ""), ("Senden", ""));
    assert_eq!(
        asked("find", "https", "https://x.example/"),
        ("https", "https://x.example/")
    );
    assert_eq!(
        asked("click", "https://x.example/", ""),
        ("https://x.example/", ""),
        "a click's words stay its words"
    );
    // A read that shows almost no text says where the rest is; one with text is left as it is.
    let thin = hinted("https://teams.example/\n(3) Calendar | Teams\n\n3\n\nA\n".to_string());
    assert!(
        thin.starts_with("https://teams.example/\n(3) Calendar | Teams\n")
            && thin.contains("neobot safari find"),
        "{thin}"
    );
    let full = format!("https://x.example/\nTitle\n{}", "words ".repeat(60));
    assert_eq!(hinted(full.clone()), full);
}

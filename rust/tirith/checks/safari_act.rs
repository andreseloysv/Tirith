use super::act::{act_args, show_script, USAGE};
use super::choice::how;
use super::login::login_args;

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
    assert!(how("safari").contains("tirith safari open") && how("safari").contains("click"));
    assert!(
        how("openbot").contains("tirith browser login") && how("openbot").contains("browser tools")
    );
    assert_eq!(
        how("anything else"),
        how("openbot"),
        "OpenBot's own until the owner chooses"
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

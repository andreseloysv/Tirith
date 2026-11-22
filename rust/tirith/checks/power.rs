use super::argv::argv;
#[test]
fn keep_awake_runs_caffeinate() {
    let run = |text: &str, macos: bool| argv(&text.parse().unwrap(), macos, 4242);
    let on = "[power]\nkeep_awake = true";
    assert_eq!(
        run(on, true).unwrap(),
        ["/usr/bin/caffeinate", "-i", "-s", "-w", "4242"]
    );
    assert_eq!(run(on, false), None, "only macOS has caffeinate");
    for off in ["", "[power]", "[power]\nkeep_awake = false"] {
        assert_eq!(run(off, true), None, "off unless written true: {off:?}");
    }
}

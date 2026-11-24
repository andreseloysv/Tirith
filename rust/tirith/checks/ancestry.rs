//! A hook's agent may hand it none of its variables (Muse): the hook finds them in an ancestor. Proven with this
//! test binary itself: the test starts a middle process that has the variable, the middle starts a leaf without it,
//! and the leaf must find it in its parent (`/proc` on Linux, sysctl on macOS, the process's memory on Windows).
use std::process::Command;

const PROBE: &str = "TIRITH_ANCESTRY_PROBE";
const STEP: &str = "TIRITH_ANCESTRY_STEP";

fn run(test: &str, env: &[(&str, &str)], without: &[&str]) -> bool {
    let name = format!(
        "{}::{test}",
        module_path!().split_once("::").map_or("", |(_, m)| m)
    );
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args(["--exact", &name, "--ignored", "--test-threads=1"]);
    for key in without {
        command.env_remove(key);
    }
    command.envs(env.iter().copied());
    command.status().is_ok_and(|s| s.success())
}

#[test]
fn a_child_without_the_variable_finds_it_in_its_parent() {
    assert!(run(
        "middle",
        &[(PROBE, "it-is-here"), (STEP, "middle")],
        &[]
    ));
}

#[test]
#[ignore = "a step of the test above"]
fn middle() {
    if std::env::var(STEP).as_deref() == Ok("middle") {
        assert!(
            run("leaf", &[(STEP, "leaf")], &[PROBE]),
            "the leaf did not find it"
        );
    }
}

#[test]
#[ignore = "a step of the test above"]
fn leaf() {
    if std::env::var(STEP).as_deref() == Ok("leaf") {
        assert!(std::env::var(PROBE).is_err(), "the leaf was handed it");
        assert_eq!(super::var(PROBE).as_deref(), Some("it-is-here"));
    }
}

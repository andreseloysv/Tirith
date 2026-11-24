//! The words of a run that did a part of its task, or could not: «partly done» against «failed», and a could-not said once.
use super::notice::failed_line;
use super::store::Expected;
use super::words::not_able;
use chrono::{Local, TimeZone};

fn run(title: &str) -> Expected {
    let due = Local.with_ymd_and_hms(2026, 11, 23, 8, 0, 0).unwrap();
    Expected {
        id: 1,
        conv: "c".into(),
        routine: "r".into(),
        title: title.into(),
        trigger: String::new(),
        kind: "schedule".into(),
        due: due.with_timezone(&chrono::Utc).to_rfc3339(),
        task: String::new(),
        state: "failed".into(),
        why: String::new(),
        started_at: String::new(),
        created_at: String::new(),
    }
}

#[test]
fn the_real_case_reads_a_medias_with_the_cause_once() {
    let why = not_able("", "no pudo revisar DB ni HAD");
    let line = failed_line(&run("Buscar trabajo (Codex + jobs)"), &why, true);
    assert_eq!(
        line,
        "✗ «Buscar trabajo (Codex + jobs)» partly done (Mon 23/11 08:00): no pudo revisar DB ni HAD"
    );
}

#[test]
fn no_part_done_still_reads_fallo() {
    let line = failed_line(
        &run("Rutina"),
        &not_able("", "the portal asks for a session"),
        false,
    );
    assert_eq!(
        line,
        "✗ «Rutina» failed (Mon 23/11 08:00): could not: the portal asks for a session"
    );
}

#[test]
fn a_could_not_is_never_said_twice() {
    for cause in [
        "could not x",
        "No se pudo x",
        "no pude x",
        "Could not x",
        "couldn't x",
        "unable to x",
    ] {
        assert_eq!(not_able("", cause), cause);
    }
    assert_eq!(
        not_able("said ∅, but ", "could not x"),
        "said ∅, but could not x"
    );
    assert_eq!(not_able("said ∅, but ", "x"), "said ∅, but could not: x");
}

#[test]
fn an_english_cause_is_kept_as_it_is() {
    let why = not_able("", "could not open the portal");
    let line = failed_line(&run("Jobs"), &why, true);
    assert_eq!(
        line,
        "✗ «Jobs» partly done (Mon 23/11 08:00): could not open the portal"
    );
}

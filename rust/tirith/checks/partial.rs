//! The words of a run that did a part of its task, or could not: «a medias» against «falló», and a could-not said once.
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
        "✗ «Buscar trabajo (Codex + jobs)» a medias (lun 23/11 08:00): no pudo revisar DB ni HAD"
    );
}

#[test]
fn no_part_done_still_reads_fallo() {
    let line = failed_line(
        &run("Rutina"),
        &not_able("", "el portal pide sesión"),
        false,
    );
    assert_eq!(
        line,
        "✗ «Rutina» falló (lun 23/11 08:00): no pudo: el portal pide sesión"
    );
}

#[test]
fn a_could_not_is_never_said_twice() {
    for cause in [
        "no pudo x",
        "No se pudo x",
        "no pude x",
        "Could not x",
        "couldn't x",
        "unable to x",
    ] {
        assert_eq!(not_able("", cause), cause);
    }
    assert_eq!(
        not_able("dijo ∅, pero ", "no pudo x"),
        "dijo ∅, pero no pudo x"
    );
    assert_eq!(not_able("dijo ∅, pero ", "x"), "dijo ∅, pero no pudo: x");
}

#[test]
fn an_english_cause_is_kept_as_it_is() {
    let why = not_able("", "could not open the portal");
    let line = failed_line(&run("Jobs"), &why, true);
    assert_eq!(
        line,
        "✗ «Jobs» a medias (lun 23/11 08:00): could not open the portal"
    );
}

use super::{deliver::act, poll::Out, store::ROOMS, Room, PHONE};

/// No runtime on purpose: handing either message to a manager would spawn its session and panic here.
#[test]
fn status_answers_without_a_turn() {
    let home = std::env::temp_dir().join(format!("tirith-status-room-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let project = "statusproj";
    std::fs::write(
        home.join("config.toml"),
        format!(
            "[[projects]]\nid = \"{project}\"\nrepo = \"{}\"\n",
            home.display()
        ),
    )
    .unwrap();
    let app = crate::app::App::open_at(home.clone());

    crate::manager::say(&app, project, "/status", "").unwrap();

    let phone = format!("iPhone {PHONE}");
    let id = "sealed-status-test";
    // Not a "manager" room, so nothing is queued on the sender that the typing test owns.
    let mut room = Room {
        id: id.into(),
        project: project.into(),
        kind: "test".into(),
        ..Default::default()
    };
    room.owes.owed.push((phone.clone(), "/status".into()));
    ROOMS.lock().unwrap().insert(id.into(), room.clone());
    let out = Out {
        deliver: vec![(phone.clone(), "/status".into())],
        ..Default::default()
    };
    act(&app, &room, out);

    // A restart came before the answer: what is still owed is answered the same way.
    super::store::update(&app, id, |r| {
        r.owes.owed.push((phone.clone(), "/status".into()))
    });
    super::turns::again(&app, Some(project));

    let chat: Vec<(String, String)> = app.query(
        "SELECT role, text FROM chat WHERE project = ?1 ORDER BY id",
        [project],
        |r| Ok((r.get(0)?, r.get(1)?)),
    );
    let roles: Vec<&str> = chat.iter().map(|c| c.0.as_str()).collect();
    assert_eq!(
        roles,
        ["owner", "tirith", "room", "tirith", "room", "tirith"],
        "{chat:?}"
    );
    for (_, answer) in chat.iter().filter(|c| c.0 == "tirith") {
        for part in ["Up for ", "Active lanes", "Routines today", "Last error"] {
            assert!(answer.contains(part), "{part} is missing: {answer}");
        }
    }
    // What leads to a turn: the message events with their evals, a trigger, a busy session.
    for table in ["events", "turns"] {
        let rows: Vec<i64> = app.query(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0));
        assert_eq!(rows, [0], "{table}");
    }
    assert_eq!(crate::turns::take_trigger(project), Default::default());
    assert!(!crate::manager::busy().contains(&project.to_string()));
    let owed = ROOMS.lock().unwrap().remove(id).unwrap().owes.owed;
    assert!(owed.is_empty(), "the phone is answered: {owed:?}");
    // Another agent in the room does not run it.
    assert!(!super::deliver::phone_command(
        &app,
        &room,
        "some agent",
        "/status"
    ));
    drop(app);
    let _ = std::fs::remove_dir_all(&home);
}

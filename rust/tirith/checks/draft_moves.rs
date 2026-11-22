#[test]
fn a_new_draft_never_deletes_the_files_of_an_older_one() {
    let root = std::env::temp_dir().join(format!("tirith-drafts-{}", std::process::id()));
    let (skills, drafts) = (root.join("skills"), root.join("drafts"));
    for dir in [skills.join("report/refs"), drafts.join("report/refs")] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(drafts.join("report/refs/old.md"), "kept").unwrap();
    std::fs::write(drafts.join("report/SKILL.md"), "older").unwrap();
    std::fs::write(skills.join("report/refs/new.md"), "new").unwrap();
    let text = "---\nname: report\ndraft: true\n---\nbody\n";
    std::fs::write(skills.join("report/SKILL.md"), text).unwrap();
    assert_eq!(super::move_drafts(&skills, &drafts), ["report"]);
    let read = |p: &str| std::fs::read_to_string(drafts.join("report").join(p)).unwrap();
    assert_eq!(
        (
            read("SKILL.md").as_str(),
            read("refs/old.md").as_str(),
            read("refs/new.md").as_str()
        ),
        (text, "kept", "new")
    );
    assert!(!skills.join("report").exists());
    std::fs::remove_dir_all(&root).unwrap();
}

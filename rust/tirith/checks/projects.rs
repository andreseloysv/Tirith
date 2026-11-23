use super::{block::block, colors::COLORS, folder::folder, project::project};
use crate::config::Project;
use std::path::{Path, PathBuf};
#[test]
fn project_add_appends_a_block_and_keeps_the_file() {
    let old = "# mine\n[[projects]]\nid = \"a\"\nrepo = \"~/a\"\n\n[guard]\nwords = [\"x\"]";
    let p = project(
        Path::new("/Users/me/my \"b\""),
        "Quote \" and \\ back",
        "",
        &[],
    )
    .unwrap();
    let ps: Vec<Project> = crate::config::get(
        &format!("{old}{}", block(old, &p).unwrap())
            .parse::<toml::Table>()
            .unwrap(),
        &["projects"],
    )
    .unwrap();
    assert_eq!(
        (
            ps.len(),
            ps[1].id.as_str(),
            ps[1].name.as_str(),
            ps[1].repo.as_str()
        ),
        (
            2,
            "quote-and-back",
            "Quote \" and \\ back",
            "/Users/me/my \"b\""
        )
    );
    assert!(
        block("projects = []\n", &p).is_err()
            && block("x = [\n", &p).is_err()
            && block("", &p).unwrap().starts_with("[[projects]]")
    );
}
#[test]
fn project_add_names_ids_and_colors() {
    let mk = |id: &str| Project {
        id: id.into(),
        color: Some(COLORS[0].into()),
        ..Default::default()
    };
    let ps = [mk("my-app"), mk("my-app-2")];
    let p = |name: &str, color: &str| project(Path::new("/a/zoo"), name, color, &ps);
    assert_eq!(
        (
            p("My App", "").unwrap().id,
            p("", "").unwrap().name,
            p("¿?", "").unwrap().id
        ),
        ("my-app-3".into(), "zoo".into(), "project".into())
    );
    assert_eq!(
        (
            p("x", "").unwrap().color,
            p("x", " #ABCDEF ").unwrap().color
        ),
        (Some(COLORS[1].into()), Some("#abcdef".into()))
    );
    assert!(
        ["red", "#12345g", "#ñññ"]
            .iter()
            .all(|c| p("x", c).is_err())
            && p("two\nlines", "").is_err()
            && p(&"x".repeat(81), "").is_err()
    );
}
#[test]
fn project_add_refuses_folders_that_cannot_be_projects() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../target/projects-test-{}", std::process::id()));
    let at = |p: &str| base.join(p).display().to_string();
    [
        "repo/sub",
        "repo/.git/info",
        "home/lanes/repo/slot-1",
        "wt",
        "other",
    ]
    .into_iter()
    .for_each(|d| std::fs::create_dir_all(base.join(d)).unwrap());
    std::fs::write(base.join("file"), "x")
        .and_then(|_| std::fs::write(base.join("wt/.git"), "gitdir: /elsewhere"))
        .unwrap();
    let (ps, home) = (
        [Project {
            id: "repo".into(),
            repo: at("repo"),
            ..Default::default()
        }],
        base.join("home"),
    );
    let refusals = [
        ("", "full path"),
        ("relative/dir", "full path"),
        ("/no/such/dir-4821", "no folder"),
        ("/tmp", "temporary"),
        ("/", "home folder"),
    ];
    let mine = [
        ("file", "no folder"),
        ("repo/sub", "inside the project"),
        ("home/lanes/repo/slot-1", "OpenBot's own"),
        ("home", "OpenBot's own"),
        ("", "holds the project"),
        ("repo/.git/info", ".git"),
        ("wt", "worktree"),
    ];
    let err = |raw: String| folder(&raw, &ps, &home).err().unwrap_or_default();
    let got: Vec<(String, &str)> = refusals
        .iter()
        .map(|(r, w)| (err(r.to_string()), *w))
        .chain(mine.iter().map(|(r, w)| (err(at(r)), *w)))
        .collect();
    let fine = (
        folder(&at("other"), &ps, &home),
        folder(&at("repo"), &ps, &home),
    );
    let _ = std::fs::remove_dir_all(&base);
    assert!(got.iter().all(|(e, w)| e.contains(w)), "{got:?}");
    assert!(fine.0.is_ok() && fine.1.is_ok());
}

// Codex inside Tirith's sandbox keeps the limit its caller asked for (sandbox/nested/).
use super::{asked, limits, mode::Mode, settings};
use std::path::PathBuf;

fn words(line: &str) -> Vec<String> {
    line.split(' ').map(String::from).collect()
}

#[test]
fn every_way_of_asking_for_a_limit_is_read() {
    for (line, mode) in [
        ("exec --sandbox workspace-write hi", Some(Mode::Write)),
        ("exec --sandbox=read-only hi", Some(Mode::ReadOnly)),
        ("exec -s workspace-write hi", Some(Mode::Write)),
        ("exec -sworkspace-write hi", Some(Mode::Write)),
        ("-s read-only exec hi", Some(Mode::ReadOnly)),
        ("exec --full-auto hi", Some(Mode::Write)),
        ("exec --approve-for-me hi", Some(Mode::Write)),
        (
            "exec --dangerously-bypass-approvals-and-sandbox hi",
            Some(Mode::Full),
        ),
        ("exec --sandbox something-new hi", Some(Mode::ReadOnly)),
        ("e hi", None),
        ("exec -m exec hi -- --sandbox danger-full-access", None),
    ] {
        assert_eq!(asked::read(&words(line)).unwrap().mode, mode, "{line}");
    }
    for line in [
        "login",
        "resume --last",
        "-m exec",
        "--version",
        "sandbox -- exec",
    ] {
        assert!(
            asked::read(&words(line)).is_none(),
            "{line} runs nothing unattended"
        );
    }
    let a = asked::read(&words(
        "-m m exec --sandbox workspace-write -c sandbox_mode=\"x\" --json hi",
    ))
    .unwrap();
    assert_eq!(
        (a.args.clone(), a.at),
        (words("-m m exec --json hi"), 3),
        "only the sandbox's words go"
    );
}

fn folder(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("tirith-nested-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("codex")).unwrap();
    std::fs::create_dir_all(d.join("work")).unwrap();
    d
}

#[test]
fn the_limit_is_what_was_asked_then_the_settings_then_codexs_default() {
    let d = folder("limit");
    let (home, work) = (d.join("codex"), d.join("work"));
    let at = settings::Where {
        cwd: &work,
        home: &home,
        tmp: None,
    };
    let limit = |line: &str| settings::limit(&asked::read(&words(line)).unwrap(), &at).unwrap();
    let write = |line: &str| limit(line).map(|(r, _)| r.write);
    assert_eq!(
        write("exec hi"),
        Some(false),
        "read-only is Codex's default"
    );
    assert_eq!(write("exec -c sandbox_mode=workspace-write hi"), Some(true));
    assert_eq!(
        write("exec -c sandbox_mode=workspace-write -s read-only hi"),
        Some(false),
        "the flag wins"
    );
    assert_eq!(
        write("exec -s danger-full-access hi"),
        None,
        "nothing to keep"
    );
    assert_eq!(
        write("exec -P mine hi"),
        Some(false),
        "what cannot be read is the strictest"
    );

    let (r, note) = limit("exec --sandbox workspace-write -o last.txt hi").unwrap();
    assert!(r.write && !r.net && !note.is_empty());
    assert_eq!(
        r.args[..2],
        [
            "exec".to_string(),
            "--dangerously-bypass-approvals-and-sandbox".to_string()
        ]
    );
    assert!(
        !r.args
            .iter()
            .any(|a| a == "--sandbox" || a == "workspace-write"),
        "{:?}",
        r.args
    );
    assert!(
        r.args
            .iter()
            .any(|a| a == "shell_environment_policy.set.HTTPS_PROXY=\"\""),
        "no network for its commands"
    );
    assert!(r.roots.contains(&work.display().to_string()) && r.out.ends_with("work/last.txt"));
    let (open, _) =
        limit("exec -s workspace-write -c sandbox_workspace_write.network_access=true hi").unwrap();
    assert!(open.net && !open.args.iter().any(|a| a.contains("HTTPS_PROXY")));

    std::fs::write(home.join("config.toml"), "sandbox_mode = \"workspace-write\"\n[sandbox_workspace_write]\nwritable_roots = [\"/opt/more\"]\n").unwrap();
    assert!(limit("exec hi")
        .unwrap()
        .0
        .roots
        .contains(&"/opt/more".to_string()));
    assert_eq!(write("exec --ignore-user-config hi"), Some(false));
    std::fs::write(
        home.join("config.toml"),
        format!(
            "[projects.\"{}\"]\ntrust_level = \"trusted\"\n",
            work.display()
        ),
    )
    .unwrap();
    assert_eq!(
        write("exec hi"),
        Some(true),
        "Codex's default in a project it was told about"
    );
    assert!(
        settings::limit(&asked::read(&words("exec --worktree hi")).unwrap(), &at).is_err(),
        "said, not widened"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_one_profile_only_takes_away() {
    let odd = "/nonexistent/a\")(allow network-outbound)(\"b";
    let r = super::Request {
        write: true,
        roots: vec![odd.into()],
        state: "/nonexistent/state".into(),
        ..Default::default()
    };
    let (text, params) = limits::profile(&r, &[7717, 47812]);
    assert!(
        text.starts_with(super::super::profile::PROFILE),
        "Tirith's comes first, whole"
    );
    assert!(
        !text.contains("nonexistent"),
        "paths are parameters, never profile text: {text}"
    );
    assert!(params.iter().any(|(_, v)| v == odd));
    let added = &text[super::super::profile::PROFILE.len()..];
    for allow in added.lines().filter(|l| l.starts_with("(allow")) {
        let narrow = allow.contains("file-write*")
            || allow.contains("\"localhost:7717\"")
            || allow.contains("\"localhost:47812\"");
        assert!(
            narrow,
            "nothing Tirith's profile denies is allowed: {allow}"
        );
    }
    assert!(
        added.contains("(deny file-write*)") && added.contains("unix-socket"),
        "{added}"
    );
    assert!(
        params
            .iter()
            .any(|(_, v)| v.ends_with("/state/config.toml"))
            && params.iter().any(|(_, v)| v.ends_with("/.git"))
    );
    let ro = super::Request { write: false, ..r };
    assert!(
        !limits::profile(&ro, &[]).1.iter().any(|(_, v)| v == odd),
        "read-only opens no folder"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn what_runs_under_the_one_profile_is_still_an_agents() {
    use crate::sandbox::mac;
    // SAFETY: getpid only reads this process's own id.
    if mac::ours(unsafe { libc::getpid() }) {
        return; // the tests themselves run confined, and profiles do not nest
    }
    let r = super::Request {
        state: std::env::temp_dir().display().to_string(),
        ..Default::default()
    };
    let (text, params) = limits::profile(&r, &[7717]);
    assert!(text.contains(limits::TIGHTER));
    let mut command = std::process::Command::new(super::super::profile::SANDBOX_EXEC);
    for (name, value) in params {
        command.arg("-D").arg(format!("{name}={value}"));
    }
    let mut child = command
        .args(["-p", &text, "/bin/sleep", "20"])
        .spawn()
        .unwrap();
    let pid = child.id() as libc::pid_t;
    let seen = (0..100).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(50));
        mac::ours(pid) && mac::tighter(pid)
    });
    let alive = child.try_wait().unwrap().is_none();
    let _ = (child.kill(), child.wait());
    assert!(
        seen && alive,
        "confined for the daemon, and known as tighter"
    );
}

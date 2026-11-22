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
        role: false,
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
    let r = super::Request {
        program: "/usr/bin/true".into(),
        ..r
    };
    let around = limits::Around {
        doors: super::doors::Doors {
            main: 7717,
            rules: 7718,
        },
        kept: vec!["/nonexistent/socket".into()],
    };
    let (text, params) = limits::profile(&r, &around, limits::Part::Codex);
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
        // each gives back only what Tirith's own profile allows: the loopback, syslog's socket, names and files
        let op = allow
            .trim_start_matches("(allow ")
            .split(' ')
            .next()
            .unwrap();
        let narrow = match op {
            "network-outbound" => {
                allow.contains("\"localhost:") || allow.contains("/private/var/run/syslog")
            }
            "appleevent-send" => false,
            _ => [
                "file-write*",
                "file-write-data",
                "signal",
                "mach-lookup",
                "mach-register",
                "iokit-open",
                "network-bind",
                "network-inbound",
            ]
            .contains(&op),
        };
        assert!(
            narrow,
            "nothing Tirith's profile denies is allowed: {allow}"
        );
    }
    assert!(
        added.contains("(deny file-write*)")
            && added.contains("(deny network-outbound (remote ip \"localhost:*\"))")
            && added.contains("(deny network-outbound (remote unix-socket))"),
        "{added}"
    );
    let last = |rule: &str| added.rfind(rule).unwrap_or_else(|| panic!("no {rule}"));
    assert!(
        last("(deny mach-lookup (global-name \"app.tirith.outside\"))")
            > last("(allow mach-lookup"),
        "the daemon's mark is denied last"
    );
    assert!(
        params.iter().any(|(_, v)| v.ends_with("/.git"))
            && !params
                .iter()
                .any(|(_, v)| v.ends_with("/state/config.toml"))
    );
    for allow in added
        .lines()
        .filter(|l| l.starts_with("(allow file-write*"))
    {
        let state = params.iter().find(|(_, v)| v.ends_with("/state")).unwrap();
        let names_state = allow.contains(&format!("\"{}\"", state.0));
        assert!(
            !names_state || allow.contains("(process-path"),
            "Codex's state is its own process's alone: {allow}"
        );
    }
    let ro = super::Request { write: false, ..r };
    assert!(
        !limits::profile(&ro, &around, limits::Part::Codex)
            .1
            .iter()
            .any(|(_, v)| v == odd),
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
        program: "/usr/bin/true".into(), // so what runs below is a command, not Codex
        state: std::env::temp_dir().display().to_string(),
        ..Default::default()
    };
    let around = limits::Around {
        doors: super::doors::Doors {
            main: 7717,
            rules: 7718,
        },
        kept: vec![],
    };
    let (text, params) = limits::profile(&r, &around, limits::Part::Codex);
    let params_again = params.clone();
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
    let shell = std::process::Command::new(super::super::profile::SANDBOX_EXEC)
        .args(
            params_again
                .iter()
                .flat_map(|(name, value)| ["-D".to_string(), format!("{name}={value}")]),
        )
        .args(["-p", &text, "/bin/zsh", "-c", "echo ran"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&shell.stdout).trim(),
        "ran",
        "the owner's shell runs under it"
    );
    let refused: Vec<bool> = ["job-creation", "user-preference-write"]
        .map(|op| refuses(pid, op))
        .to_vec();
    assert_eq!(
        refused,
        [true, true],
        "jobs and preferences, as Codex's own sandbox"
    );
    assert!(
        refuses_service(pid, "com.apple.pasteboard.1"),
        "no service beyond Codex's own list"
    );
    let alive = child.try_wait().unwrap().is_none();
    let _ = (child.kill(), child.wait());
    assert!(
        seen && alive,
        "confined for the daemon, and known as tighter"
    );
}

#[cfg(target_os = "macos")]
extern "C" {
    fn sandbox_check(
        pid: libc::pid_t,
        op: *const libc::c_char,
        kind: libc::c_int,
        ...
    ) -> libc::c_int;
}

/// Whether the kernel refuses `pid` this operation: asking does nothing else.
#[cfg(target_os = "macos")]
fn refuses(pid: libc::pid_t, op: &str) -> bool {
    let op = std::ffi::CString::new(op).unwrap();
    // SAFETY: the string lives through the call, which only reads it; no filter, no further argument.
    unsafe { sandbox_check(pid, op.as_ptr(), 0x4000_0000) == 1 }
}

#[cfg(target_os = "macos")]
fn refuses_service(pid: libc::pid_t, name: &str) -> bool {
    let name = std::ffi::CString::new(name).unwrap();
    // SAFETY: as above, with the name the filter reads.
    unsafe { sandbox_check(pid, c"mach-lookup".as_ptr(), 2 | 0x4000_0000, name.as_ptr()) == 1 }
}

#[test]
fn codex_review_takes_the_limit_too() {
    let a = asked::read(&words("review --uncommitted")).expect("it runs commands unattended");
    assert_eq!(
        (a.args.clone(), a.at),
        (words("exec review --uncommitted"), 1)
    );
    let a = asked::read(&words("-c sandbox_mode=\"read-only\" review")).unwrap();
    assert_eq!(a.args.last().map(String::as_str), Some("review"));
}

#[test]
fn a_profile_named_in_the_settings_is_read_and_servers_are_named() {
    let d = folder("profile");
    let (home, work) = (d.join("codex"), d.join("work"));
    std::fs::write(home.join("config.toml"), "sandbox_mode = \"workspace-write\"\nprofile = \"careful\"\n[profiles.careful]\nsandbox_mode = \"read-only\"\n[mcp_servers.notes]\ncommand = \"notes-server\"\n").unwrap();
    let at = settings::Where {
        cwd: &work,
        home: &home,
        tmp: None,
        role: false,
    };
    let (r, note) = settings::limit(&asked::read(&words("exec hi")).unwrap(), &at)
        .unwrap()
        .unwrap();
    assert!(!r.write, "the profile's limit wins");
    assert!(note.contains("notes"), "said, not silent: {note}");
    let briefed = words("exec -c developer_instructions=Brief. hi");
    let (said, _) = settings::limit(&asked::read(&briefed).unwrap(), &at)
        .unwrap()
        .unwrap();
    let told: Vec<&String> = said
        .args
        .iter()
        .filter(|a| a.starts_with("developer_instructions="))
        .collect();
    assert!(
        told.len() == 1 && told[0].contains("Brief.") && told[0].contains("write nothing"),
        "the caller's words, then the true limit, once: {told:?}"
    );
    assert!(
        said.args
            .iter()
            .any(|a| a == "include_environment_context=false")
            && told[0].contains("Working directory: ")
            && !told[0].contains("unrestricted"),
        "no note that calls the sandbox none, and its folder still said: {told:?}"
    );
    assert!(
        r.args
            .iter()
            .any(|a| a == "include_permissions_instructions=false"),
        "the model is not told that it has full access"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[cfg(target_os = "macos")]
#[test]
fn only_codexs_own_process_reaches_this_machines_ports() {
    use crate::sandbox::mac;
    // SAFETY: getpid only reads this process's own id.
    if mac::ours(unsafe { libc::getpid() }) {
        return; // confined already
    }
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port().to_string();
    let around = limits::Around {
        doors: super::doors::Doors { main: 1, rules: 2 },
        kept: vec![],
    };
    let connects = |program: &str| {
        let r = super::Request {
            program: program.into(),
            ..Default::default()
        };
        let (text, params) = limits::profile(&r, &around, limits::Part::Codex);
        std::process::Command::new(super::super::profile::SANDBOX_EXEC)
            .args(
                params
                    .iter()
                    .flat_map(|(name, value)| ["-D".to_string(), format!("{name}={value}")]),
            )
            .args([
                "-p",
                &text,
                "/usr/bin/nc",
                "-z",
                "-G",
                "2",
                "127.0.0.1",
                &port,
            ])
            .output()
            .unwrap()
            .status
            .success()
    };
    assert!(
        connects("/usr/bin/nc"),
        "the program the limit is around keeps its loopback"
    );
    assert!(!connects("/usr/bin/true"), "a command of it has none");

    let r = super::Request {
        program: "/bin/sleep".into(),
        ..Default::default()
    };
    let (text, params) = limits::profile(&r, &around, limits::Part::Codex);
    let mut child = std::process::Command::new(super::super::profile::SANDBOX_EXEC)
        .args(
            params
                .iter()
                .flat_map(|(name, value)| ["-D".to_string(), format!("{name}={value}")]),
        )
        .args(["-p", &text, "/bin/sleep", "20"])
        .spawn()
        .unwrap();
    let pid = child.id() as libc::pid_t;
    let marked = (0..100).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(50));
        mac::ours(pid) && mac::tighter(pid)
    });
    let _ = (child.kill(), child.wait());
    assert!(marked, "Codex's own process keeps the daemon's marks too");
}

#[cfg(target_os = "macos")]
#[test]
fn the_daemons_port_is_a_roles_and_the_hooks_door_the_limited_ones() {
    use super::doors::Doors;
    // SAFETY: getpid only reads this process's own id.
    if crate::sandbox::mac::ours(unsafe { libc::getpid() }) {
        return; // confined already
    }
    let (api, hook) = (
        std::net::TcpListener::bind("127.0.0.1:0").unwrap(),
        std::net::TcpListener::bind("127.0.0.1:0").unwrap(),
    );
    let port = |l: &std::net::TcpListener| l.local_addr().unwrap().port();
    let doors = Doors {
        main: port(&api),
        rules: port(&hook),
    };
    let connects = |r: &super::Request, port: u16| {
        let around = limits::Around {
            doors,
            kept: vec![],
        };
        let (text, params) = limits::profile(r, &around, limits::Part::Codex);
        std::process::Command::new(super::super::profile::SANDBOX_EXEC)
            .args(
                params
                    .iter()
                    .flat_map(|(name, value)| ["-D".to_string(), format!("{name}={value}")]),
            )
            .args([
                "-p",
                &text,
                "/usr/bin/nc",
                "-z",
                "-G",
                "2",
                "127.0.0.1",
                &port.to_string(),
            ])
            .output()
            .unwrap()
            .status
            .success()
    };
    for (program, net) in [
        ("/usr/bin/true", false),
        ("/usr/bin/true", true),
        ("/usr/bin/nc", false),
        ("/usr/bin/nc", true),
    ] {
        let r = super::Request {
            program: program.into(),
            net,
            ..Default::default()
        };
        assert!(
            !connects(&r, doors.main),
            "no API for a limited one: {program}, network {net}"
        );
        assert!(
            connects(&r, doors.rules),
            "the rules' hook has its door: {program}, network {net}"
        );
        let role = super::Request {
            agent: true,
            ..r.clone()
        };
        assert!(
            connects(&role, doors.main),
            "a role keeps its reads: {program}, network {net}"
        );
    }
    let mut r = super::Request {
        env: vec![("TIRITH_PORT".into(), doors.main.to_string())],
        ..Default::default()
    };
    doors.told(&mut r);
    assert_eq!(
        r.env,
        vec![("TIRITH_PORT".to_string(), doors.rules.to_string())]
    );
}

#[cfg(target_os = "macos")]
#[test]
fn what_runs_the_commands_has_none_of_codexs_own() {
    use limits::Part;
    // SAFETY: getpid only reads this process's own id.
    if crate::sandbox::mac::ours(unsafe { libc::getpid() }) {
        return; // confined already
    }
    let d = folder("parts");
    let (state, work) = (d.join("codex"), d.join("work"));
    std::fs::write(state.join("auth.json"), "{}").unwrap();
    let r = super::Request {
        program: "/bin/dd".into(), // which then is Codex's own process: it opens as Codex's writer does
        write: true,
        roots: vec![work.display().to_string()],
        state: state.display().to_string(),
        ..Default::default()
    };
    let around = limits::Around {
        doors: super::doors::Doors { main: 1, rules: 2 },
        kept: vec![],
    };
    let from = d.join("from");
    std::fs::write(&from, "x").unwrap();
    let copies = |part: Part, to: &std::path::Path| {
        let (text, params) = limits::profile(&r, &around, part);
        std::process::Command::new(super::super::profile::SANDBOX_EXEC)
            .args(
                params
                    .iter()
                    .flat_map(|(name, value)| ["-D".to_string(), format!("{name}={value}")]),
            )
            .args(["-p", &text, "/bin/dd", "status=none"])
            .args([
                format!("if={}", from.display()),
                format!("of={}", to.display()),
            ])
            .output()
            .unwrap()
            .status
            .success()
    };
    let (auth, made) = (state.join("auth.json"), work.join("made"));
    assert!(
        copies(Part::Codex, &auth),
        "Codex's own process renews its sign-in"
    );
    assert!(
        !copies(Part::Commands, &auth),
        "what runs the patches never writes it"
    );
    assert!(copies(Part::Commands, &made), "patches write in the folder");
    assert!(
        !copies(Part::Codex, &work.join("not")),
        "Codex's own process writes in no folder"
    );
    assert!(
        !copies(Part::Commands, &work.join(".git")),
        "nor what the limit protects"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[cfg(target_os = "macos")]
#[test]
fn the_bridge_speaks_websocket() {
    use super::{frames, upgrade};
    assert_eq!(
        upgrade::accepted("dGhlIHNhbXBsZSBub25jZQ=="),
        "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=",
        "RFC 6455's own example"
    );
    let client = |fin: bool, op: u8, data: &[u8]| {
        let mask = [7u8, 9, 11, 13];
        let mut out = vec![if fin { 0x80 } else { 0 } | op, 0x80 | 127];
        out.extend((data.len() as u64).to_be_bytes());
        out.extend(mask);
        out.extend(data.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
        out
    };
    let big = vec![b'a'; 70_000];
    let wire = [
        client(false, 1, &big),
        client(true, 9, b"ping"),
        client(true, 0, b"z"),
    ]
    .concat();
    let mut pinged = vec![];
    let got = frames::next(&mut wire.as_slice(), |p| pinged.push(p.to_vec())).unwrap();
    assert_eq!((got.len(), pinged), (70_001, vec![b"ping".to_vec()]));
    let unmasked = [0x81u8, 1, b'x'];
    assert!(
        frames::next(&mut unmasked.as_slice(), |_| {}).is_none(),
        "a client masks"
    );
    for size in [5usize, 300, 70_000] {
        let mut sent = vec![];
        frames::send(&mut sent, frames::TEXT, &vec![b'b'; size]).unwrap();
        let head = match size {
            5 => 2,
            300 => 4,
            _ => 10,
        };
        assert_eq!(sent.len(), head + size, "{size}");
    }
}

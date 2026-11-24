// The service's files, and what `neobot service status` calls alive.
use super::{alive::said, launchctl::printed_pid, Unit};
use std::path::{Path, PathBuf};

fn unit(mark: &str) -> Unit {
    Unit {
        home: "/Users/a \"b\"/50%/.tirith".into(),
        exe: "/Users/a b/R&D <1>/50%/tirith".into(),
        user_home: PathBuf::from("/Users/a b"),
        mark: mark.into(),
        port: Some(7799),
    }
}

#[test]
fn service_files_keep_serve_alive() {
    let log = Path::new("/Users/a b/R&D <1>/50%/.tirith/logs/tirithd.log");
    let plist = super::plist::text(&unit(""), log, None);
    for part in [
        "<string>com.tirith.serve</string>",
        "<string>/Users/a b/R&amp;D &lt;1&gt;/50%/tirith</string>\n    <string>serve</string>\n  </array>",
        "<key>RunAtLoad</key>\n  <true/>",
        "<key>KeepAlive</key>\n  <true/>",
        "<string>Interactive</string>",
        "<string>/Users/a b/R&amp;D &lt;1&gt;/50%/.tirith/logs/tirithd.log</string>",
        "<string>/Users/a \"b\"/50%/.tirith</string>",
        "<key>TIRITH_PORT</key>\n  <string>7799</string>",
    ] {
        assert!(plist.contains(part), "{part}\nnot in\n{plist}");
    }
    assert_eq!(
        unit("-1234abcd").label(),
        "com.tirith.serve-1234abcd",
        "each home its own"
    );

    let text = super::systemd_text::text(&unit(""), log).unwrap();
    let (head, rest) = text.split_once("[Service]\n").unwrap();
    let (service, install) = rest.split_once("[Install]\n").unwrap();
    assert!(head.contains("\nStartLimitIntervalSec=0\n"), "{head}"); // systemd reads it only in [Unit]
    for line in [
        r#"ExecStart="/Users/a b/R&D <1>/50%%/tirith" serve"#,
        r#"Environment="TIRITH_HOME=/Users/a \"b\"/50%%/.tirith""#,
        r#"Environment="TIRITH_PORT=7799""#,
        "Restart=always",
        "KillMode=process",
        "StandardOutput=append:/Users/a b/R&D <1>/50%%/.tirith/logs/tirithd.log",
        "StandardError=append:/Users/a b/R&D <1>/50%%/.tirith/logs/tirithd.log",
    ] {
        assert!(service.lines().any(|l| l == line), "{line}\nnot in\n{text}");
    }
    assert_eq!(install, "WantedBy=default.target\n");
}

#[test]
fn service_status_is_alive_only_for_its_own_process() {
    assert_eq!(
        printed_pid("x = {\n\tstate = running\n\tpid = 42\n}"),
        Some(42)
    );
    assert_eq!(printed_pid("x = {\n\tstate = waiting\n}"), None);
    let (ours, old) = (serde_json::json!({"pid": 42}), serde_json::json!({}));
    assert_eq!(said(Some(42), Some(&ours)), "yes (pid 42)");
    assert!(said(Some(43), Some(&ours)).starts_with("no ("));
    assert!(said(Some(42), Some(&old)).starts_with("no ("));
    assert_eq!(said(Some(42), None), "no");
}

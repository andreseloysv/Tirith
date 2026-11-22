use super::manager::State;
use super::spec::Spec;
use super::status::{life, Life};

fn spec() -> Spec {
    Spec {
        exe: "/Users/a b/R&D <1>/50%/tirith".into(),
        log: "/Users/a b/R&D <1>/50%/.tirith/logs/tirithd.log".into(),
        env: vec![
            ("TIRITH_HOME", "/Users/a \"b\"/50%/.tirith".into()),
            ("TIRITH_PORT", "7799".into()),
        ],
    }
}

#[test]
fn service_files_keep_serve_alive() {
    let plist = super::plist::plist(&spec());
    for part in [
        "<key>Label</key>\n  <string>com.tirith.serve</string>",
        "<string>/Users/a b/R&amp;D &lt;1&gt;/50%/tirith</string>\n    <string>serve</string>\n  </array>",
        "<key>RunAtLoad</key>\n  <true/>",
        "<key>KeepAlive</key>\n  <true/>",
        "<key>ProcessType</key>\n  <string>Interactive</string>",
        "<key>StandardOutPath</key>\n  <string>/Users/a b/R&amp;D &lt;1&gt;/50%/.tirith/logs/tirithd.log</string>",
        "<key>StandardErrorPath</key>\n  <string>/Users/a b/R&amp;D &lt;1&gt;/50%/.tirith/logs/tirithd.log</string>",
        "<key>TIRITH_HOME</key>\n    <string>/Users/a \"b\"/50%/.tirith</string>",
        "<key>TIRITH_PORT</key>\n    <string>7799</string>",
    ] {
        assert!(plist.contains(part), "{part}\nnot in\n{plist}");
    }

    let unit = super::unit::unit(&spec());
    let (head, rest) = unit.split_once("[Service]\n").unwrap();
    let (service, install) = rest.split_once("[Install]\n").unwrap();
    assert!(head.contains("\nStartLimitIntervalSec=0\n"), "{head}"); // systemd reads it only in [Unit]
    for line in [
        r#"ExecStart="/Users/a b/R&D <1>/50%%/tirith" serve"#,
        r#"Environment="TIRITH_HOME=/Users/a \"b\"/50%%/.tirith""#,
        r#"Environment="TIRITH_PORT=7799""#,
        "Restart=always",
        "RestartSec=5",
        "KillMode=process",
        "StandardOutput=append:/Users/a b/R&D <1>/50%%/.tirith/logs/tirithd.log",
        "StandardError=append:/Users/a b/R&D <1>/50%%/.tirith/logs/tirithd.log",
    ] {
        assert!(service.lines().any(|l| l == line), "{line}\nnot in\n{unit}");
    }
    assert_eq!(install, "WantedBy=default.target\n");
}

#[test]
fn service_status_is_alive_only_for_its_own_process() {
    let state = |loaded, pid| State { loaded, pid };
    let printed = "x = {\n\tstate = running\n\tpid = 42\n}";
    assert_eq!(super::launchd::read_print(printed), state(true, Some(42)));
    let shown = super::systemd::read_show;
    assert_eq!(shown("MainPID=42\nLoadState=loaded"), state(true, Some(42)));
    assert_eq!(shown("MainPID=0\nLoadState=not-found"), state(false, None));

    let (ours, old) = (serde_json::json!({"pid": 42}), serde_json::json!({}));
    assert_eq!(life(&state(true, Some(42)), Some(&ours)), Life::Alive(42));
    assert_eq!(life(&state(true, Some(43)), Some(&ours)), Life::Other);
    assert_eq!(life(&state(true, Some(42)), Some(&old)), Life::Other);
    assert_eq!(life(&state(true, Some(42)), None), Life::Silent);
}

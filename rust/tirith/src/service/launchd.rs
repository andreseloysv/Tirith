//! On macOS: a launchd job of the owner's, or one that starts at boot with nobody logged in (--boot, with sudo).
use super::{launchctl::Launchctl, plist, unit::Unit};
use crate::setup::{find, run, Ask};
use std::time::Duration;

pub(super) fn install(u: &Unit, ask: &Ask, boot: bool) -> Result<Vec<String>, String> {
    let log = u.log()?;
    let ctl = Launchctl::new(u, ask, boot);
    if boot {
        let user = run::output("id", &["-un"]).ok_or("cannot tell who you are")?;
        let draft = u.home.join("setup/boot.plist");
        std::fs::create_dir_all(u.home.join("setup")).map_err(|e| e.to_string())?;
        std::fs::write(&draft, plist::text(u, &log, Some(&user))).map_err(|e| e.to_string())?;
        let sudo = find::program("sudo").ok_or("sudo is missing")?;
        let (from, to) = (
            draft.display().to_string(),
            u.boot_file().display().to_string(),
        );
        println!("  To start at boot with nobody logged in, sudo asks for your password:");
        let args = [
            "install", "-m", "644", "-o", "root", "-g", "wheel", &from, &to,
        ];
        run::shown(ask, &sudo, &args, &[])
            .then_some(())
            .ok_or("sudo could not install the boot service")?;
    } else {
        let file = u.agent_file();
        let failed = |e: std::io::Error| format!("{}: {e}", file.display());
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(failed)?;
        }
        std::fs::write(&file, plist::text(u, &log, None)).map_err(failed)?;
    }
    let _ = ctl.unload();
    ctl.load()?;
    let note =
        "it starts when you log in: for a Mac with nobody logged in, tirith service install --boot";
    Ok((!boot).then(|| note.to_string()).into_iter().collect())
}

pub(super) fn act(u: &Unit, ask: &Ask, boot: bool, what: &str) -> Result<(), String> {
    let ctl = Launchctl::new(u, ask, boot);
    match what {
        "start" if !ctl.loaded() => ctl.load(),
        "start" => ctl.kick(false),
        "stop" if ctl.loaded() => ctl.unload(), // KeepAlive would bring a killed one back
        "restart" if ctl.loaded() => ctl.kick(true),
        "restart" => ctl.load(),
        _ => Ok(()),
    }
}

pub(super) fn uninstall(u: &Unit, ask: &Ask, boot: bool) -> Result<(), String> {
    let ctl = Launchctl::new(u, ask, boot);
    if ctl.loaded() {
        ctl.unload()?;
        std::thread::sleep(Duration::from_millis(300));
    }
    if boot {
        let sudo = find::program("sudo").ok_or("sudo is missing")?;
        let file = u.boot_file().display().to_string();
        return run::shown(ask, &sudo, &["rm", "--", &file], &[])
            .then_some(())
            .ok_or_else(|| "sudo could not remove the boot service".to_string());
    }
    let file = u.agent_file();
    std::fs::remove_file(&file).map_err(|e| format!("{}: {e}", file.display()))
}

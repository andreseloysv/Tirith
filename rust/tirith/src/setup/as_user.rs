//! root on a server whose Tirith runs as the user `tirith` (install.sh made it): root's `tirith …` is that user's, not a second Tirith of root's.
use std::os::unix::process::CommandExt;
use std::path::Path;

/// install.sh's service user.
const USER: &str = "tirith";
/// Who signed in on the machine, when the guide runs as the service user.
pub(super) const VIA: &str = "TIRITH_VIA";

/// What stays root's: the daemon itself, the binary's own commands and the AppArmor profile.
fn roots_own(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        None | Some("serve" | "hook" | "version" | "--version" | "-V")
    ) || args.first().is_some_and(|a| a == "net")
        && matches!(
            args.get(1).map(String::as_str),
            Some("run" | "allow" | "tunnel")
        )
}

/// (uid, home) of the service user in /etc/passwd.
pub(super) fn entry(passwd: &str) -> Option<(String, String)> {
    passwd.lines().find_map(|line| {
        let mut part = line.split(':');
        (part.next()? == USER).then_some(())?;
        let uid = part.nth(1)?.to_string();
        Some((uid, part.nth(2)?.to_string()))
    })
}

/// Goes on as that user when root calls and that user has the Tirith of this machine; it returns only when it does not.
pub fn as_service_user(args: &[String]) {
    let root = cfg!(target_os = "linux") && unsafe { libc::geteuid() } == 0;
    if !root || roots_own(args) || std::env::var_os("TIRITH_HOME").is_some() {
        return;
    }
    let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    let Some((uid, home)) = entry(&passwd) else {
        return;
    };
    let own = crate::config::home_dir().join(".tirith/config.toml");
    if own.exists() || !Path::new(&home).join(".tirith").is_dir() {
        return; // root has a Tirith of its own, or that user has none
    }
    let Some(exe) = crate::app::exe() else {
        return;
    };
    let via = std::env::var("SUDO_USER").unwrap_or_else(|_| "root".into());
    let why = std::process::Command::new("runuser")
        .args(["-u", USER, "--", "env"])
        .arg(format!("HOME={home}"))
        .arg(format!("USER={USER}"))
        .arg(format!("LOGNAME={USER}"))
        .arg(format!("XDG_RUNTIME_DIR=/run/user/{uid}"))
        .arg(format!("{VIA}={via}"))
        .arg(exe)
        .args(args)
        .current_dir(&home)
        .exec();
    eprintln!("cannot go on as the user {USER}, whose Tirith this is: {why}");
    std::process::exit(1);
}

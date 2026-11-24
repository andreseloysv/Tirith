//! What NeoBot needs of the system, installed with its package manager when the owner says so.
use super::{ask::Ask, find, root, run};

const MANAGERS: [(&str, &[&str]); 5] = [
    ("apt-get", &["install", "-y"]),
    ("dnf", &["install", "-y"]),
    ("apk", &["add"]),
    ("pacman", &["-S", "--noconfirm"]),
    ("zypper", &["--non-interactive", "install"]),
];

/// Makes the programs `missing` be there; else the command that would.
pub(super) fn provide(ask: &Ask, missing: &[&str]) -> Result<(), String> {
    let names = missing.join(" ");
    if cfg!(target_os = "macos") {
        return Err(format!("{names} missing: run xcode-select --install"));
    }
    if cfg!(windows) {
        return super::winget::provide(ask, missing);
    }
    let Some((manager, how)) = MANAGERS
        .iter()
        .find(|(m, _)| find::program(m).is_some())
        .copied()
    else {
        return Err(format!("{names} missing: install them"));
    };
    let as_root = crate::os::is_root();
    let shown = format!(
        "{}{manager} {} {names}",
        if as_root { "" } else { "sudo " },
        how.join(" ")
    );
    let sudo = root::sudo().filter(|_| !as_root);
    let may = if as_root {
        ask.yes_no(&format!("  {names} missing. Run `{shown}`?"), true)
    } else {
        sudo.is_some()
            && ask.person()
            && ask.yes_no(&format!("  {names} missing. Run `{shown}`?"), true)
    };
    let failed = || format!("{names} missing: {shown}");
    may.then_some(()).ok_or_else(failed)?;
    let program = root::program(manager).ok_or_else(failed)?; // root runs it: never one of the PATH's
    let manager_path = program.display().to_string();
    let mut args: Vec<&str> = if sudo.is_some() {
        vec![&manager_path]
    } else {
        vec![]
    };
    args.extend(how);
    args.extend(missing);
    let ran = run::shown(ask, sudo.as_deref().unwrap_or(&program), &args, &[]);
    (ran && missing.iter().all(|m| find::program(m).is_some()))
        .then_some(())
        .ok_or_else(failed)
}

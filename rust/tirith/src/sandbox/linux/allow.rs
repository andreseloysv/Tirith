//! `sudo neobot net allow`: the AppArmor profile that lets this binary make user namespaces (Ubuntu 23.10 and later).

const FILE: &str = "/etc/apparmor.d/tirith";

/// The profile for the binary at `exe`.
fn profile(exe: &str) -> String {
    format!("abi <abi/4.0>,\ninclude <tunables/global>\n\n# NeoBot's network sandbox needs user namespaces.\nprofile tirith \"{exe}\" flags=(unconfined) {{\n  userns,\n}}\n")
}

pub(in crate::sandbox) fn allow() -> Result<(), String> {
    (unsafe { libc::geteuid() } == 0)
        .then_some(())
        .ok_or("run it with sudo: it writes /etc/apparmor.d/tirith")?;
    let exe = crate::app::exe().ok_or("cannot tell where this binary is")?;
    let exe = exe
        .to_str()
        .filter(|p| !p.contains(['"', '\n', '*', '?', '[', '{']));
    let exe = exe.ok_or("this binary's path cannot be written in a profile")?;
    std::fs::write(FILE, profile(exe)).map_err(|e| format!("{FILE}: {e}"))?;
    let loaded = std::process::Command::new("apparmor_parser")
        .args(["-r", FILE])
        .status()
        .map_err(|e| format!("apparmor_parser: {e}"))?;
    loaded
        .success()
        .then(|| println!("{exe} may confine what it starts now"))
        .ok_or_else(|| format!("apparmor_parser did not load {FILE}"))
}

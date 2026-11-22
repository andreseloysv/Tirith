//! The wrappers in `<home>/net/bin`: tools that start a sandbox of their own (macOS refuses to nest them) get it turned off, since the outer sandbox confines them already. Each wrapper takes its own folder off PATH.
use crate::app::Shared;
use std::path::Path;

const XCODE_FLAGS: &str = "-IDEPackageSupportDisableManifestSandbox=YES -IDEPackageSupportDisablePluginExecutionSandbox=YES";

pub fn write_wrappers(app: &Shared) {
    let dir = app.home.join("net/bin");
    let _ = std::fs::create_dir_all(&dir);
    write_tools(app, &dir);
    write_zdot(app, &dir);
}

/// The wrapper scripts, each rewritten only when its text changed.
#[cfg_attr(not(target_os = "macos"), allow(unused_variables))] // Codex's wrapper is macOS's
fn write_tools(app: &Shared, dir: &Path) {
    let head =
        include_str!("../../assets/net-head.sh").replace("{dir}", &dir.display().to_string());
    let mut files: Vec<(String, String)> = [
        ("swift", include_str!("../../assets/net-swift.sh")),
        ("xcodebuild", include_str!("../../assets/net-xcodebuild.sh")),
    ]
    .map(|(n, b)| (n.to_string(), b.replace("{flags}", XCODE_FLAGS)))
    .into();
    files.extend(
        ["swift-build", "swift-test", "swift-run", "swift-package"].map(|tool| {
            (
                tool.into(),
                include_str!("../../assets/net-tool.sh").replace("{tool}", tool),
            )
        }),
    );
    #[cfg(target_os = "macos")]
    if let Some(exe) = crate::app::exe() {
        let body = include_str!("../../assets/net-codex.sh")
            .replace("{exe}", &exe.display().to_string())
            .replace("{home}", &app.home.display().to_string()); // a tool's shell may get none of the environment
        files.push(("codex".into(), body));
    }
    files.into_iter().for_each(|(name, body)| {
        use std::os::unix::fs::PermissionsExt;
        let (path, text) = (dir.join(name), format!("{head}{body}"));
        let _ = (std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()))
            .then(|| std::fs::write(&path, text));
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
    });
}

/// Agents run commands in zsh login shells, where macOS's path_helper puts /usr/bin before the wrappers again: the ZDOTDIR given to sandboxed processes loads the owner's own startup files, then puts the wrappers first.
fn write_zdot(app: &Shared, dir: &Path) {
    let zdot = app.home.join("net/zdot");
    let _ = std::fs::create_dir_all(&zdot);
    [".zshenv", ".zprofile", ".zshrc", ".zlogin"]
        .into_iter()
        .for_each(|file| {
            drop(std::fs::write(
                zdot.join(file),
                format!(
                    "[ -f \"$HOME/{file}\" ] && . \"$HOME/{file}\"\nexport PATH=\"{}:$PATH\"\n",
                    dir.display()
                ),
            ))
        });
}

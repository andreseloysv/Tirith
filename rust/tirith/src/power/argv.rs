//! The command that keeps the machine awake, when there is one to run.

const CAFFEINATE: &str = "/usr/bin/caffeinate";

/// `caffeinate` for as long as `pid` lives (`-i`: no idle sleep; `-s`: no system sleep, on mains power only); none unless `[power] keep_awake = true` and this is macOS.
pub(super) fn argv(config: &toml::Table, macos: bool, pid: u32) -> Option<Vec<String>> {
    let on = macos && crate::config::get(config, &["power", "keep_awake"]).unwrap_or(false);
    on.then(|| {
        [CAFFEINATE, "-i", "-s", "-w", &pid.to_string()]
            .map(String::from)
            .into()
    })
}

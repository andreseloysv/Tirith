//! What the service of one Tirith home is: its names and files.
use std::path::PathBuf;

pub struct Unit {
    pub home: PathBuf,
    pub exe: PathBuf,
    /// The owner's home folder.
    pub user_home: PathBuf,
    /// Empty for the usual home; else a mark of this one, so two homes are two services.
    pub(super) mark: String,
    /// `TIRITH_PORT`, when it was set at the install: a service inherits none of the shell's variables.
    pub(super) port: Option<u16>,
}

impl Unit {
    pub fn here() -> Result<Unit, String> {
        let exe = crate::app::exe().ok_or("cannot tell where the tirith binary is")?;
        let home = crate::config::tirith_home();
        let home = std::path::absolute(&home).unwrap_or(home);
        let user_home = crate::config::home_dir();
        let mark = if home == user_home.join(".tirith") {
            String::new()
        } else {
            let hash = crate::rooms::sealed::sha_hex(home.display().to_string().as_bytes());
            format!("-{}", &hash[..8])
        };
        let port = std::env::var("TIRITH_PORT")
            .ok()
            .and_then(|p| p.parse().ok());
        Ok(Unit {
            home,
            exe,
            user_home,
            mark,
            port,
        })
    }

    /// systemd's name of it.
    pub fn name(&self) -> String {
        format!("tirith{}.service", self.mark)
    }

    /// launchd's name of it.
    pub fn label(&self) -> String {
        format!("com.tirith.serve{}", self.mark)
    }

    pub fn systemd_file(&self) -> PathBuf {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| self.user_home.join(".config"));
        config.join("systemd/user").join(self.name())
    }

    pub fn agent_file(&self) -> PathBuf {
        self.user_home
            .join("Library/LaunchAgents")
            .join(format!("{}.plist", self.label()))
    }

    pub fn boot_file(&self) -> PathBuf {
        PathBuf::from("/Library/LaunchDaemons").join(format!("{}.plist", self.label()))
    }

    /// The daemon's log, its folder made.
    pub fn log(&self) -> Result<PathBuf, String> {
        let dir = self.home.join("logs");
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(dir.join("tirithd.log"))
    }

    /// PATH before the login shell's comes first (`cli/serve.rs`).
    pub fn path(&self) -> String {
        format!(
            "{}/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
            self.user_home.display()
        )
    }
}

//! config.toml, written one change at a time.
use crate::app::App;
use std::path::Path;

/// One config.toml write at a time, so two adds never lose one another.
pub(super) static WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The text of config.toml at `path`: a file that is not there yet reads as empty, and a reading error names the file.
pub(super) fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .or_else(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(String::new())
            } else {
                Err(e)
            }
        })
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Rewrites config.toml with `edit` under that lock and reloads it; a file that is not there yet reads as empty, and a reading or writing error names the file.
pub(crate) fn rewrite(
    app: &App,
    edit: impl FnOnce(&str) -> Result<String, String>,
) -> Result<(), String> {
    let _writing = WRITING.lock().unwrap();
    let path = app.home.join("config.toml");
    let text = read(&path)?;
    std::fs::write(&path, edit(&text)?).map_err(|e| format!("{}: {e}", path.display()))?;
    app.reload_config(true);
    Ok(())
}

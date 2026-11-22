//! Installing and removing the service.
use super::{manager::Manager, spec::Spec, status};
use std::path::Path;

fn at(path: &Path, e: std::io::Error) -> String {
    format!("{}: {e}", path.display())
}

/// Writes the file, loads it and says whether the daemon came up. Installing again replaces the running one.
pub(super) fn install(m: &Manager) -> Result<(), String> {
    let spec = Spec::current()?;
    let file = m.path();
    for path in [Path::new(&spec.log), file.as_path()] {
        let dir = path.parent().unwrap_or(path); // neither manager makes the log's folder
        std::fs::create_dir_all(dir).map_err(|e| at(dir, e))?;
    }
    std::fs::write(&file, (m.text)(&spec)).map_err(|e| at(&file, e))?;
    (m.start)(&file)?;
    println!("{} runs {} serve", file.display(), spec.exe);
    println!("log: {}", spec.log);
    m.note.into_iter().for_each(|note| println!("{note}"));
    status::after_start(m)
}

/// Stops the service, then removes its file: a stop that failed leaves the file to try again.
pub(super) fn uninstall(m: &Manager) -> Result<(), String> {
    let file = m.path();
    let (written, loaded) = (file.exists(), (m.state)()?.loaded);
    if !written && !loaded {
        println!("the service is not installed");
        return Ok(());
    }
    (m.stop)()?;
    if written {
        std::fs::remove_file(&file).map_err(|e| at(&file, e))?;
    }
    (m.forget)()?;
    println!("uninstalled: the service is stopped and will not start again");
    Ok(())
}

use std::path::Path;

/// Copies `src` to `dst`, skipping caches, virtualenvs, tests and (for the web) screenshots.
fn stage(src: &Path, dst: &Path, skip: &[&str]) {
    std::fs::create_dir_all(dst).expect("create resource dir");
    for entry in std::fs::read_dir(src).expect("read resource source").flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.ends_with(".pyc") || name == "__pycache__" || skip.contains(&name.as_str()) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            stage(&path, &dst.join(&name), skip);
        } else {
            std::fs::copy(&path, dst.join(&name)).expect("copy resource file");
        }
    }
}

/// Stages the collector and the web page in `resources/` (git-ignored) so Tauri can bundle clean copies.
fn stage_resources() {
    // read when it runs, not when it compiles: a target folder shared by worktrees reuses this script's binary
    let here = std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let here = Path::new(&here);
    let out = here.join("resources");
    let _ = std::fs::remove_dir_all(&out);
    let collector = here.join("../../collector");
    let skip = ["tests", "Makefile", "demo.py", "fixture_js.py", "tirith-collector.plist.template"];
    stage(&collector, &out.join("collector"), &skip);
    stage(&here.join("../../web"), &out.join("web"), &["screenshots"]);
    println!("cargo:rerun-if-changed=../../collector");
    println!("cargo:rerun-if-changed=../../web");
}

fn main() {
    stage_resources();
    tauri_build::build()
}

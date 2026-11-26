//! Every temp folder a test makes goes inside one folder of this run, and that folder is removed when the
//! tests end. Before, each test left its own in the user's temp folder, 57 folders (6 MB) a run of the
//! suite: 18,950 folders (628 MB) on the owner's disk by 2026-11-26, from lanes, the merge queue and
//! sessions alike.
//!
//! How: before any test starts, this run's folder is made and set as TMPDIR, so `std::env::temp_dir()` and
//! every program a test starts answer with it; when the process ends it removes the folder. A run that was
//! killed cannot: the next run removes the folders of runs whose process is gone. `TIRITH_KEEP_TMP=1` keeps
//! this run's folder until the next run, to look at what a failing test left.
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const NAME: &str = "tirith-run-";
const KEEP: &str = "TIRITH_KEEP_TMP";
/// This run's folder and the process that made it: a child that only copied the memory must not remove it.
static OURS: OnceLock<(PathBuf, u32)> = OnceLock::new();

/// The system calls the entries of this section before `main`. Nothing else comes before every test, and
/// the variable must be set while the process still has one thread.
#[used]
#[cfg_attr(target_vendor = "apple", link_section = "__DATA,__mod_init_func")]
#[cfg_attr(not(target_vendor = "apple"), link_section = ".init_array")]
static BEFORE_THE_TESTS: extern "C" fn() = start;

extern "C" fn start() {
    let all = std::env::temp_dir();
    sweep(&all);
    let pid = std::process::id();
    let dir = all.join(format!("{NAME}{pid}"));
    if std::fs::create_dir_all(&dir).is_err() || OURS.set((dir.clone(), pid)).is_err() {
        return;
    }
    std::env::set_var("TMPDIR", &dir);
    // SAFETY: `end` is a plain function that lives as long as the program.
    unsafe { libc::atexit(end) };
}

extern "C" fn end() {
    let Some((dir, pid)) = OURS.get() else { return };
    if *pid != std::process::id() || std::env::var_os(KEEP).is_some() {
        return;
    }
    // a thread that still runs may write while this deletes: try again, and the next run sweeps what stays
    for _ in 0..3 {
        if std::fs::remove_dir_all(dir).is_ok() {
            break;
        }
    }
}

/// Removes, in `all`, the folders of runs whose process is gone. A link with such a name is not followed.
fn sweep(all: &Path) {
    let Ok(entries) = std::fs::read_dir(all) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let pid = name
            .to_str()
            .and_then(|n| n.strip_prefix(NAME))
            .and_then(|n| n.parse::<i32>().ok());
        let folder = entry.file_type().is_ok_and(|t| t.is_dir());
        if folder && pid.is_some_and(|pid| !alive(pid)) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

fn alive(pid: i32) -> bool {
    if pid <= 0 {
        return false; // no process has such a number, and to `kill` it would mean a whole group
    }
    // SAFETY: signal 0 sends nothing; it only asks whether the process is there.
    let answered = unsafe { libc::kill(pid, 0) } == 0;
    answered || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// The number of a process that has just ended.
fn ended() -> i32 {
    let mut child = std::process::Command::new("true").spawn().unwrap();
    child.wait().unwrap();
    child.id() as i32
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tidy-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_test_s_temp_folder_is_inside_this_run_s_own() {
    let (dir, pid) = OURS
        .get()
        .expect("the run's folder is made before the tests");
    assert_eq!(*pid, std::process::id());
    assert!(dir.is_dir(), "{dir:?} is not there");
    assert!(dir
        .file_name()
        .is_some_and(|n| n.to_string_lossy() == format!("{NAME}{pid}")));
    assert!(
        std::env::temp_dir().starts_with(dir),
        "{:?}",
        std::env::temp_dir()
    );
}

#[test]
fn the_folder_of_a_run_that_ended_is_swept_and_no_other() {
    let all = scratch("sweep");
    let over = all.join(format!("{NAME}{}", ended()));
    let living = all.join(format!("{NAME}{}", std::process::id()));
    let other = all.join("tirith-teach-drafts-1");
    for dir in [&over, &living, &other] {
        std::fs::create_dir_all(dir.join("inside")).unwrap();
    }
    sweep(&all);
    assert!(!over.exists(), "the folder of a run that ended stays");
    assert!(
        living.join("inside").exists(),
        "the folder of a living run was removed"
    );
    assert!(
        other.join("inside").exists(),
        "a folder that is no run's was removed"
    );
}

#[test]
fn a_link_named_like_an_ended_run_is_not_followed() {
    let all = scratch("link");
    let kept = all.join("kept");
    std::fs::create_dir_all(kept.join("inside")).unwrap();
    std::os::unix::fs::symlink(&kept, all.join(format!("{NAME}{}", ended()))).unwrap();
    sweep(&all);
    assert!(
        kept.join("inside").exists(),
        "what the link points to was removed"
    );
}

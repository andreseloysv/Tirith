use super::super::login_env::{current, file, lines, parse, Vars};
use super::capture_with;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tirith-login-env-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn vars(pairs: &[(&str, &str)]) -> Vars {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn what_the_profile_exports_becomes_lines_any_shell_reads() {
    let printed = b"a profile's own chatter\nneobot-login-env\0PATH=/opt/brew/bin:/me/.nvm/bin:/usr/bin:/bin\0API=it's \"here\" $HOME\0SAME=1\0PWD=/somewhere\0TWO=line one\nline two\0BAD NAME=x\x009LIVES=x\0";
    let shell = parse(printed).unwrap();
    let base = vars(&[("PATH", "/usr/bin:/bin"), ("SAME", "1")]);
    let text = lines(&shell, &base);
    assert_eq!(
        text,
        "export API='it'\\''s \"here\" $HOME'\nexport PATH='/opt/brew/bin:/me/.nvm/bin':\"$PATH\"\nexport TWO='line one\nline two'\n",
        "what the reader has already, its own variables and no name of a variable are left out"
    );
    assert!(parse(b"no mark here\0A=1\0").is_none());
    // A shell that reads the lines has the values as they were, and the profile's folders in front of its own PATH.
    let d = dir("lines");
    std::fs::write(d.join("env.sh"), &text).unwrap();
    let read = Command::new("/bin/sh")
        .arg("-c")
        .arg(". \"$1\"; printf '%s|%s|%s' \"$API\" \"$PATH\" \"$TWO\"")
        .args(["sh", d.join("env.sh").to_str().unwrap()])
        .env_clear()
        .env("PATH", "/wrappers:/usr/bin:/bin")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&read.stdout),
        "it's \"here\" $HOME|/opt/brew/bin:/me/.nvm/bin:/wrappers:/usr/bin:/bin|line one\nline two"
    );
    std::fs::remove_dir_all(&d).unwrap();
}

#[test]
fn the_login_shell_is_run_once_and_a_failed_run_keeps_the_last_capture() {
    let home = dir("capture");
    // A login shell whose profile exports one thing and puts a folder in front of PATH; it must be asked as a
    // login shell and never as an interactive one (`-lc`): an agent's shell is no terminal of the owner's.
    let shell = home.join("login-shell");
    std::fs::write(&shell, "#!/bin/sh\necho 'welcome back'\n[ \"$1\" = -lc ] || exit 9\nexport FROM_PROFILE='yes it is'\nexport PATH=\"/profile/bin:$PATH\"\nshift\nexec /bin/sh -c \"$1\"\n").unwrap();
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(capture_with(&home, shell.to_str().unwrap()));
    let text = std::fs::read_to_string(file(&home)).unwrap();
    assert!(text.contains("export FROM_PROFILE='yes it is'\n"), "{text}");
    assert!(
        text.contains("export PATH='/profile/bin':\"$PATH\"\n"),
        "{text}"
    );
    assert!(
        !text.contains("welcome back") && !text.contains("export HOME="),
        "{text}"
    );
    let mode = std::fs::metadata(file(&home)).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "the owner's alone: it may hold their keys");
    assert!(current(&home), "no profile file changed after it");
    // A profile that fails, or a shell that is not there: nothing is taken and the last capture stays.
    std::fs::write(&shell, "#!/bin/sh\nexit 3\n").unwrap();
    assert!(!capture_with(&home, shell.to_str().unwrap()));
    assert!(!capture_with(&home, "/no/such/shell"));
    assert_eq!(std::fs::read_to_string(file(&home)).unwrap(), text);
    assert!(!current(&dir("none")), "no capture is no current one");
    std::fs::remove_dir_all(&home).unwrap();
}

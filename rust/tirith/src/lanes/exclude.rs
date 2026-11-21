//! Keeping what a lane moved into a project out of git there, through `.git/info/exclude`.
use crate::app::git;
use std::path::Path;

/// The path as an exclude pattern: `\`, `*`, `?`, `[`, `]` and spaces escaped.
fn escaped(rel: &Path) -> String {
    ['\\', '*', '?', '[', ']', ' ']
        .iter()
        .fold(rel.to_string_lossy().into_owned(), |s, c| {
            s.replace(*c, &format!("\\{c}"))
        })
}
/// Adds the line to the exclude file unless it is there already.
fn add_line(file: &Path, line: &str) {
    let text = std::fs::read_to_string(file).unwrap_or_default();
    let _ = (!text.lines().any(|l| l.trim() == line)).then(|| {
        (
            file.parent().map(std::fs::create_dir_all),
            std::fs::write(
                file,
                format!(
                    "{text}{}{line}\n",
                    if text.is_empty() || text.ends_with('\n') {
                        ""
                    } else {
                        "\n"
                    }
                ),
            ),
        )
    });
}
/// Keeps what a lane moved in out of git for good: when the project does not already ignore it, one line for the whole ignored folder moved (or for a lone file) joins `.git/info/exclude` there, with the pattern characters escaped (`\` in a name would escape the next character and the file would stay visible), so it is never committed or pushed from the project folder.
pub(super) async fn ensure_excluded(repo: &Path, rel: &Path) {
    let (dir, rel, esc) = (
        repo.join(rel).is_dir() && !repo.join(rel).is_symlink(),
        rel.to_string_lossy().into_owned(),
        escaped(rel),
    );
    let Some(()) = git(repo, &["check-ignore", "-q", "--", &rel])
        .await
        .is_err()
        .then_some(())
    else {
        return;
    };
    let Ok(gitdir) = git(repo, &["rev-parse", "--git-common-dir"]).await else {
        return;
    };
    let (file, line) = (
        repo.join(gitdir).join("info").join("exclude"),
        format!("/{esc}{}", if dir { "/" } else { "" }),
    );
    add_line(&file, &line);
}

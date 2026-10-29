//! Serves the web UI from a folder on disk through the `tirithweb://` scheme (no HTTP server, no port),
//! and notices when the files in that folder change.

use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const SCHEME: &str = "tirithweb";
pub const CHANGED_EVENT: &str = "tirith://web-changed";

/// Where the window loads the live page (Windows and Android serve custom schemes over http).
pub fn page_url() -> String {
    if cfg!(any(windows, target_os = "android")) {
        format!("http://{SCHEME}.localhost/index.html")
    } else {
        format!("{SCHEME}://localhost/index.html")
    }
}

/// Percent-decode a URI path; None when the bytes are not valid UTF-8.
fn decode(path: &str) -> Option<String> {
    let b = path.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = path.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The file a request path names inside `root`, or the HTTP status to answer with.
/// Never leaves `root`: `..`, absolute or drive-prefixed parts, backslashes, NUL and symlinks that
/// point outside are all refused.
pub fn resolve(root: &Path, uri_path: &str) -> Result<PathBuf, u16> {
    let decoded = decode(uri_path).ok_or(400u16)?;
    if decoded.contains('\0') || decoded.contains('\\') {
        return Err(400);
    }
    let rel = decoded.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    let mut file = root.to_path_buf();
    for part in Path::new(rel).components() {
        match part {
            Component::Normal(p) => file.push(p),
            Component::CurDir => {}
            _ => return Err(403),
        }
    }
    if file.is_dir() {
        file.push("index.html");
    }
    let root = root.canonicalize().map_err(|_| 404u16)?;
    let file = file.canonicalize().map_err(|_| 404u16)?;
    if file.starts_with(&root) && file.is_file() {
        Ok(file)
    } else if file.starts_with(&root) {
        Err(404)
    } else {
        Err(403)
    }
}

pub fn mime(path: &Path) -> &'static str {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "txt" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Answer one request: (status, content type, body).
pub fn respond(root: &Path, uri_path: &str) -> (u16, &'static str, Vec<u8>) {
    match resolve(root, uri_path) {
        Ok(file) => match std::fs::read(&file) {
            Ok(body) => (200, mime(&file), body),
            Err(_) => (404, "text/plain; charset=utf-8", b"not found".to_vec()),
        },
        Err(code) => (code, "text/plain; charset=utf-8", format!("{code}").into_bytes()),
    }
}

/// A fingerprint of every file under `root` (names, sizes, mtimes); differs when anything changed.
pub fn signature(root: &Path) -> u64 {
    fn walk(dir: &Path, depth: u32, h: &mut u64) {
        if depth > 8 {
            return;
        }
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = rd.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                walk(&e.path(), depth + 1, h);
                continue;
            }
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            for b in e.file_name().to_string_lossy().bytes().chain(meta.len().to_le_bytes()).chain(mtime.to_le_bytes()) {
                *h = (*h ^ b as u64).wrapping_mul(0x100000001b3);
            }
        }
    }
    let mut h = 0xcbf29ce484222325u64;
    walk(root, 0, &mut h);
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("tirith-liveweb-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("web/js")).unwrap();
        std::fs::write(base.join("web/index.html"), "<html>").unwrap();
        std::fs::write(base.join("web/js/a.js"), "1").unwrap();
        std::fs::write(base.join("secret.txt"), "no").unwrap();
        base
    }

    #[test]
    fn root_and_files_resolve_inside_the_folder() {
        let base = site("ok");
        let root = base.join("web");
        assert!(resolve(&root, "/").unwrap().ends_with("index.html"));
        assert!(resolve(&root, "/js/a.js").unwrap().ends_with("js/a.js"));
        assert!(resolve(&root, "/js").is_err());
        assert!(resolve(&root, "/js/%61.js").unwrap().ends_with("js/a.js"));
        assert_eq!(resolve(&root, "/nope.js"), Err(404));
    }

    #[test]
    fn paths_cannot_leave_the_folder() {
        let base = site("escape");
        let root = base.join("web");
        for bad in ["/../secret.txt", "/js/../../secret.txt", "/%2e%2e/secret.txt", "/..%2fsecret.txt", "//etc/passwd", "/js\\..\\..\\secret.txt", "/a%00.js", "/%zz"] {
            assert!(matches!(resolve(&root, bad), Err(400) | Err(403) | Err(404)), "{bad}");
        }
        assert!(resolve(&root, "/../secret.txt").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_out_of_the_folder_is_refused() {
        let base = site("link");
        let root = base.join("web");
        std::os::unix::fs::symlink(base.join("secret.txt"), root.join("leak.txt")).unwrap();
        assert_eq!(resolve(&root, "/leak.txt"), Err(403));
    }

    #[test]
    fn mime_types_by_extension() {
        assert_eq!(mime(Path::new("a/index.html")), "text/html; charset=utf-8");
        assert_eq!(mime(Path::new("a.JS")), "text/javascript; charset=utf-8");
        assert_eq!(mime(Path::new("a.css")), "text/css; charset=utf-8");
        assert_eq!(mime(Path::new("a.svg")), "image/svg+xml");
        assert_eq!(mime(Path::new("a.woff2")), "font/woff2");
        assert_eq!(mime(Path::new("a.bin")), "application/octet-stream");
        assert_eq!(mime(Path::new("noext")), "application/octet-stream");
    }

    #[test]
    fn respond_gives_status_type_and_body() {
        let base = site("respond");
        let root = base.join("web");
        let (code, ty, body) = respond(&root, "/js/a.js");
        assert_eq!((code, ty, body.as_slice()), (200, "text/javascript; charset=utf-8", b"1".as_slice()));
        assert_eq!(respond(&root, "/../secret.txt").0, 403);
        assert_eq!(respond(&root, "/missing").0, 404);
    }

    #[test]
    fn signature_changes_when_a_file_changes() {
        let base = site("sig");
        let root = base.join("web");
        let a = signature(&root);
        assert_eq!(a, signature(&root));
        std::fs::write(root.join("js/a.js"), "22").unwrap();
        assert_ne!(a, signature(&root));
        let b = signature(&root);
        std::fs::write(root.join("new.css"), "").unwrap();
        assert_ne!(b, signature(&root));
    }
}

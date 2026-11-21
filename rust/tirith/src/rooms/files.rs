//! The pictures the phones send, kept as files for a day.
use super::sealed;
use serde_json::Value;
use std::time::Duration;
/// A picture from a phone (`tirith-image {"mime","data"(base64)}`, as in am.mjs) is saved under `<home>/rooms/files/<room>/`; the manager gets its path.
pub(super) fn save_image_at(home: &std::path::Path, json: &str, room: &str) -> Option<String> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    let j: Value = serde_json::from_str(json).ok()?;
    let (ext, magic): (&str, &[u8]) = match j["mime"].as_str()? {
        "image/jpeg" => ("jpg", &[0xff, 0xd8, 0xff]),
        "image/png" => ("png", &[0x89, 0x50, 0x4e, 0x47]),
        "image/webp" => ("webp", b"RIFF"),
        _ => return None,
    };
    let bytes = B64
        .decode(j["data"].as_str().filter(|d| d.len() < 1_400_000)?)
        .ok()
        .filter(|b| b.len() <= 1 << 20 && b.starts_with(magic))?;
    let dir = home
        .join("rooms/files")
        .join(room.replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "_"));
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("{}.{ext}", &sealed::sha_hex(&bytes)[..16]));
    std::fs::write(&path, &bytes).ok()?;
    Some(format!("[imagen: {}]\nLook at that file to see it. It is temporary: deleted after 24 h. To keep it, move it somewhere else.", path.display()))
}
/// Deletes the files older than `max` under `dir`, and the folders left empty (`dir` too: saving a file makes it again).
pub(super) fn expire_files(dir: &std::path::Path, max: Duration) {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .for_each(|p| {
            let _ = p.is_dir().then(|| expire_files(&p, max)).or_else(|| {
                p.metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.elapsed().ok())
                    .is_some_and(|a| a > max)
                    .then(|| drop(std::fs::remove_file(&p)))
            });
        });
    let _ = std::fs::remove_dir(dir);
}

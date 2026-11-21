//! Small readers and writers of TOML values the factory uses everywhere.
use std::path::Path;

/// Lays `src` over `dst` key by key: tables merge, anything else replaces.
pub(crate) fn merge(dst: &mut toml::Value, src: &toml::Value) {
    match (dst.as_table_mut(), src.as_table()) {
        (Some(d), Some(s)) => s.iter().for_each(|(k, v)| {
            merge(
                d.entry(k.clone())
                    .or_insert_with(|| toml::Value::Table(Default::default())),
                v,
            )
        }),
        _ => *dst = src.clone(),
    }
}

/// A file's stem: how prompts and rubrics are named.
pub(super) fn stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// One string key of a table value, empty when missing.
pub(super) fn str_of<'a>(v: &'a toml::Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("")
}

/// The rows of an array section (`[[routes]]`, `[[evals]]`), none when missing.
pub(super) fn rows<'a>(t: &'a toml::Table, sec: &str) -> impl Iterator<Item = &'a toml::Value> {
    t.get(sec).and_then(|r| r.as_array()).into_iter().flatten()
}

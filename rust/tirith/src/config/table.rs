//! Reading one typed value out of a TOML table by its path.
use serde::de::DeserializeOwned;

/// The value at `path` (section, key, ...) read as `T`; none when missing or of another shape.
pub fn get<T: DeserializeOwned>(t: &toml::Table, path: &[&str]) -> Option<T> {
    path[1..]
        .iter()
        .try_fold(t.get(path[0])?, |v, key| v.get(key))?
        .clone()
        .try_into()
        .ok()
}

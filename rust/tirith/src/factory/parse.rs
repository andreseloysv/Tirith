//! Parsing one factory version, layer by layer, and validating it.
use super::extends::resolve_roles;
use super::loaded::Factory;
use super::{layers, limits, stamp};
use std::collections::BTreeMap;

/// Parses and validates one factory version: the embedded text under the file's, its prompts (by file stem) and config.toml laid over both.
pub fn parse(
    text: &str,
    prompts: BTreeMap<String, String>,
    overlay: &toml::Table,
) -> Result<Factory, String> {
    let mut table = layers::files(text)?;
    layers::sections(&mut table, overlay);
    layers::routes(&mut table, overlay);
    resolve_roles(&mut table);
    layers::agents(&mut table);
    let raw = table.as_table().cloned().unwrap_or_default();
    let f = Factory {
        limits: limits::read(&raw)?,
        raw,
        prompts,
        version: stamp::version(overlay),
    };
    f.validate()?;
    Ok(f)
}

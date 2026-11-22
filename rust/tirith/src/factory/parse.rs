//! Parsing one factory version, layer by layer, and validating it.
use super::extends::resolve_roles;
use super::loaded::Factory;
use super::{lab, layers, limits, stamp};
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
    let roles = raw.get("roles").and_then(|r| r.as_table()).cloned();
    let connectors = crate::connectors::checked(overlay, &roles.unwrap_or_default())
        .map_err(|e| format!("factory: {e}"))?;
    let f = Factory {
        limits: limits::read(&raw)?,
        lab: lab::read(&raw)?,
        raw,
        prompts,
        connectors,
        version: stamp::version(overlay),
    };
    f.validate()?;
    Ok(f)
}

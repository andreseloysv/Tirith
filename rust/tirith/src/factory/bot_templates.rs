//! The bot templates' texts: `factory/bots/<id>/` with its `bot.toml` and its `SKILL.md`, the repo's built in under the folder's.
use std::collections::BTreeMap;

/// Each template's `bot.toml` and `SKILL.md`, by id.
type Texts = BTreeMap<String, (String, String)>;

macro_rules! built_in {
    ($($id:literal),*) => {
        [$((
            $id,
            include_str!(concat!("../../../../factory/bots/", $id, "/bot.toml")),
            include_str!(concat!("../../../../factory/bots/", $id, "/SKILL.md")),
        )),*]
    };
}

/// The repo's templates, built into the binary.
pub(crate) fn embedded_templates() -> Texts {
    built_in!("resumen-correo", "revisar-pr", "vigilar-web")
        .into_iter()
        .map(|(id, bot, skill)| (id.to_string(), (bot.to_string(), skill.to_string())))
        .collect()
}

/// The templates as they are now: read on each call, so a change to the folder needs no reload.
pub fn bot_templates(overlay: &toml::Table) -> Result<Texts, String> {
    let mut all = embedded_templates();
    let folders = super::paths::dir(overlay)
        .and_then(|d| std::fs::read_dir(d.join("bots")).ok())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|d| d.join("bot.toml").is_file());
    for dir in folders {
        let read = |file: &str| {
            let path = dir.join(file);
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
        };
        all.insert(
            super::values::stem(&dir.to_string_lossy()),
            (read("bot.toml")?, read("SKILL.md")?),
        );
    }
    Ok(all)
}

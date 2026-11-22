//! What may be a bot, a secret's name and a value. The first two travel in paths and in the OS store's names; no refusal repeats what it was given.

/// The most bytes of one value.
pub(super) const MOST: usize = 64 * 1024;

pub(super) fn service(bot: &str) -> String {
    format!("tirith.bot.{bot}")
}

pub(super) fn bot(id: &str) -> Result<(), String> {
    let plain = |b: u8| b.is_ascii_alphanumeric() || b == b'-' || b == b'_';
    ((1..=64).contains(&id.len()) && id.bytes().all(plain))
        .then_some(())
        .ok_or_else(|| "a bot's id has only letters, digits, - and _".into())
}

/// Capitals only: Windows does not tell two names apart by their case.
pub(super) fn secret(name: &str) -> Result<(), String> {
    let plain = |b: u8| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_';
    let starts = name.starts_with(|c: char| c.is_ascii_uppercase() || c == '_');
    (starts && name.len() <= 128 && name.bytes().all(plain))
        .then_some(())
        .ok_or_else(|| "a secret's NAME has only A-Z, 0-9 and _, and starts with no digit".into())
}

pub(super) fn value(value: &str) -> Result<(), String> {
    (!value.is_empty() && value.len() <= MOST && !value.contains('\0'))
        .then_some(())
        .ok_or_else(|| "a value is text with no NUL, not empty and of 64 KiB at most".into())
}

//! The owner's words for how long a chat lives.

/// The owner's words for how long a chat lives (`30m`, `1h`, `24 h`, `2 días`, `una semana`), in seconds; «siempre» (or nothing) is no limit, anything else unreadable is an error said plainly.
pub fn parse_keep(words: &str) -> Result<Option<u64>, String> {
    let w = words.trim().trim_end_matches('.').to_lowercase();
    let err = || {
        format!("no entiendo «{words}»: escribe un tiempo como 30m, 1h, 24 h, 2 días o una semana, o «siempre» para quitar el límite")
    };
    let Some(()) = (!matches!(
        w.as_str(),
        "" | "siempre" | "para siempre" | "always" | "forever"
    ))
    .then_some(()) else {
        return Ok(None);
    };
    let (num, unit) = amount(&w).ok_or_else(err)?;
    let unit_s = unit_seconds(unit).ok_or_else(err)?;
    let total = (num * unit_s as f64) as u64;
    (total > 0).then_some(Some(total)).ok_or_else(err)
}

/// The number and the unit: «una semana» is one week, `24 h` is 24 hours.
fn amount(w: &str) -> Option<(f64, &str)> {
    match w.strip_prefix("una ").or_else(|| w.strip_prefix("un ")) {
        Some(u) => Some((1.0, u.trim())),
        None => {
            let i = w
                .find(|c: char| !c.is_ascii_digit() && c != '.')
                .unwrap_or(w.len());
            Some((w[..i].parse::<f64>().ok()?, w[i..].trim()))
        }
    }
}

/// How many seconds a unit is, in Spanish or English.
fn unit_seconds(unit: &str) -> Option<u64> {
    Some(match unit {
        "s" | "seg" | "segs" | "segundo" | "segundos" | "sec" | "secs" | "second" | "seconds" => 1,
        "m" | "min" | "mins" | "minuto" | "minutos" | "minute" | "minutes" => 60,
        "h" | "hr" | "hrs" | "hora" | "horas" | "hour" | "hours" => 3600,
        "d" | "día" | "días" | "dia" | "dias" | "day" | "days" => 86400,
        "sem" | "semana" | "semanas" | "week" | "weeks" => 604800,
        _ => return None,
    })
}

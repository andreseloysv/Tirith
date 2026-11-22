//! What a field, a label or a parameter is called, as whole words: a password, a code or a card is named so (tools/teach/words.js reads the same words, and both pass checks/teach_table.json).
/// The words of `text`, lowercase: anything but a letter or a digit splits, and so does a camel-case step (`userPassword`, `PINCode`).
pub(super) fn words(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = vec![String::new()];
    for (i, c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).map(|p| chars[p]);
        let next = chars.get(i + 1);
        let after_small = prev.is_some_and(|p| p.is_lowercase());
        let acronym_end =
            prev.is_some_and(|p| p.is_uppercase()) && next.is_some_and(|n| n.is_lowercase());
        let cut = c.is_uppercase() && (after_small || acronym_end);
        let _ = (!c.is_alphanumeric() || cut).then(|| out.push(String::new()));
        let _ = c
            .is_alphanumeric()
            .then(|| out.last_mut().map(|w| w.extend(c.to_lowercase())));
    }
    out.retain(|w| !w.is_empty());
    out
}
/// Words that name a password, a one-time code, a PIN or a card.
const WORDS: [&str; 29] = [
    "pwd",
    "pass",
    "passwd",
    "passcode",
    "password",
    "passwords",
    "contraseña",
    "contraseñas",
    "contrasena",
    "contrasenas",
    "clave",
    "claves",
    "secret",
    "secrets",
    "secreto",
    "token",
    "tokens",
    "otp",
    "pin",
    "cvv",
    "cvc",
    "código",
    "codigo",
    "códigos",
    "codigos",
    "tarjeta",
    "card",
    "2fa",
    "mfa",
];
/// Longer words that hold one of those (`newpassword`).
const INSIDE: [&str; 4] = ["password", "passwd", "contraseña", "contrasena"];
/// Words that make a following `code` a secret one.
const KINDS: [&str; 7] = [
    "verification",
    "security",
    "confirmation",
    "sms",
    "auth",
    "authentication",
    "access",
];
/// Whether `text` names a password, a one-time code, a PIN or a card (not «passenger», not «company»).
pub(super) fn named(text: &str) -> bool {
    let w = words(text);
    w.iter().enumerate().any(|(i, x)| {
        WORDS.contains(&x.as_str())
            || INSIDE.iter().any(|l| x.contains(l))
            || (x == "code" && i > 0 && KINDS.contains(&w[i - 1].as_str()))
    })
}
/// The text with its accents taken off (`Envía` is `envia`), so a word is the same with or without them.
pub(super) fn fold(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            'ñ' => 'n',
            'Á' | 'À' | 'Ä' => 'A',
            'É' | 'È' | 'Ë' => 'E',
            'Í' | 'Ì' | 'Ï' => 'I',
            'Ó' | 'Ò' | 'Ö' => 'O',
            'Ú' | 'Ù' | 'Ü' => 'U',
            'Ñ' => 'N',
            c => c,
        })
        .collect()
}

use super::uncite::uncited;
#[test]
fn citations_never_reach_the_phones() {
    let cases = [
        ("Hecho [m1].", "Hecho."),
        ("Hecho [m1][s2], listo [s3]!", "Hecho, listo!"),
        (
            "Tu zona es Madrid [s4] y hablas español [m2] [m7].",
            "Tu zona es Madrid y hablas español.",
        ),
        (
            "[m1] Hecho.\n- [s2] [m3] uno\n  [m4] dos [m5]",
            "Hecho.\n- uno\n  dos",
        ),
        ("¿Lo envío [m12]?", "¿Lo envío?"),
        (
            "Sin citas: [x1] y [m] se quedan.",
            "Sin citas: [x1] y [m] se quedan.",
        ),
    ];
    for (bot, phone) in cases {
        let line = uncited(bot);
        assert_eq!(line, phone, "the line for the phones of «{bot}»");
        assert!(
            line.lines().all(|l| !l.trim_start().contains("  ")),
            "no double space: «{line}»"
        );
        assert!(
            ![" .", " ,", " !", " ?", " ;", " :"]
                .iter()
                .any(|p| line.contains(p)),
            "no space before punctuation: «{line}»"
        );
    }
    let sources = [
        include_str!("../src/rooms/tophones.rs"),
        include_str!("../src/rooms/turns.rs"),
    ];
    assert!(
        sources.iter().all(|s| s.contains("uncited(text)")),
        "both the phones' lines (to_phones) and a room's answer (reply) are uncited"
    );
}

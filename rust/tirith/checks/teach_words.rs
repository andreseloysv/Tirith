//! The words that decide what is masked and what a rehearsal never does: the table both the daemon and the extension (scripts/checks/teach-words.test.mjs) must pass.
use super::approvals::listed;
use super::mask::{kept, secret};
use super::names::named;
use super::outward::sends;
use super::url::mask_url;
use serde_json::{json, Value};

fn table() -> Value {
    serde_json::from_str(include_str!("teach_table.json")).unwrap()
}

#[test]
fn field_names_are_whole_words() {
    for row in table()["names"].as_array().unwrap() {
        let (text, want) = (row[0].as_str().unwrap(), row[1].as_bool().unwrap());
        assert_eq!(named(text), want, "«{text}»");
    }
}

#[test]
fn fields_are_secret_by_type_autocomplete_name_or_label() {
    for row in table()["fields"].as_array().unwrap() {
        let (field, want) = (&row[0], row[1].as_bool().unwrap());
        if field["only"] == "js" {
            continue; // a field shown as text since is known only to the page
        }
        assert_eq!(secret(field), want, "{field}");
    }
    // The step as the daemon keeps it: the value of a secret field is gone from kv-bound data.
    let step =
        json!({"kind": "type", "name": "f1", "autocomplete": "new-password", "value": "hunter2"});
    assert_eq!(kept(&step, "t").unwrap()["value"], "•••");
}

#[test]
fn recorded_addresses_lose_their_secrets_only() {
    for row in table()["urls"].as_array().unwrap() {
        let (url, want) = (row[0].as_str().unwrap(), row[1].as_str().unwrap());
        assert_eq!(mask_url(url), want, "{url}");
    }
    let page = json!({"kind": "page", "url": "https://x.example/cb?code=abc&page=2#token=zz", "title": "T"});
    assert_eq!(
        kept(&page, "t").unwrap()["url"],
        "https://x.example/cb?code=•••&page=2#token=•••"
    );
}

#[test]
fn controls_that_send_are_told_by_their_words() {
    for row in table()["outward"].as_array().unwrap() {
        let (text, want) = (row[0].as_str().unwrap(), row[1].as_bool().unwrap());
        assert_eq!(sends(text), want, "«{text}»");
    }
}

#[test]
fn the_skills_approvals_name_the_clicks_they_stop() {
    let skill = "---\nname: x\n---\n\n## Pasos\n1. Abrir «Filtrar»\n\n## Qué requiere aprobación\n- Pulsar «Solicitar oferta» en cualquier portal\n- Aceptar condiciones del contrato\n\n## Si algo falla\nAvisar de «Filtrar»\n";
    assert!(listed(skill, "Solicitar oferta").is_some());
    assert!(
        listed(skill, "solicitar").is_some(),
        "a click on «Solicitar» is that one"
    );
    assert!(listed(skill, "sol").is_none(), "too short to be a name");
    assert!(listed(skill, "Aceptar condiciones").is_some());
    assert!(
        listed(skill, "Filtrar").is_none(),
        "other sections do not count"
    );
    assert!(listed("no sections", "Solicitar oferta").is_none());
}

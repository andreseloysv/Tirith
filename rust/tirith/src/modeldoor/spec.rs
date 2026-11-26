//! Which variables of an agent's auth file hold a model provider's key and where it is called.
use reqwest::Url;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Style {
    /// `x-api-key`.
    Anthropic,
    /// `Authorization: Bearer`.
    OpenAi,
    /// `api-key`.
    Azure,
}

/// A provider key in an auth file: its variable, the variable of its base URL (and of its type, for Copilot), and the style when none is named.
const KNOWN: [(&str, &str, &str, Style); 3] = [
    (
        "COPILOT_PROVIDER_API_KEY",
        "COPILOT_PROVIDER_BASE_URL",
        "COPILOT_PROVIDER_TYPE",
        Style::OpenAi,
    ),
    (
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_BASE_URL",
        "",
        Style::Anthropic,
    ),
    ("OPENAI_API_KEY", "OPENAI_BASE_URL", "", Style::OpenAi),
];

/// One provider of an auth file.
#[derive(Clone, Debug)]
pub struct Provider {
    pub key_var: &'static str,
    pub url_var: &'static str,
    pub key: String,
    pub style: Style,
    /// `scheme://host[:port]` of the base URL, and its path without the closing slash.
    pub origin: String,
    pub path: String,
}

fn style(word: &str, own: Style) -> Style {
    match word.to_ascii_lowercase().as_str() {
        "anthropic" => Style::Anthropic,
        "azure" => Style::Azure,
        "openai" => Style::OpenAi,
        _ => own,
    }
}

/// The base URL when the file names none: the provider's own (Azure has none: its resource is the owner's).
fn default_base(style: Style) -> Option<&'static str> {
    match style {
        Style::Anthropic => Some("https://api.anthropic.com"),
        Style::OpenAi => Some("https://api.openai.com/v1"),
        Style::Azure => None,
    }
}

/// The providers whose key `vars` hold, each with the URL it is called at.
pub fn find(vars: &[(String, String)]) -> Vec<Provider> {
    let get = |name: &str| {
        vars.iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    };
    KNOWN
        .iter()
        .filter_map(|(key_var, url_var, type_var, own)| {
            let key = get(key_var).filter(|k| !k.is_empty())?;
            let style = style(get(type_var).unwrap_or(""), *own);
            let url = get(url_var).or(default_base(style))?;
            let url = Url::parse(url)
                .ok()
                .filter(|u| matches!(u.scheme(), "http" | "https"))?;
            Some(Provider {
                key_var,
                url_var,
                key: key.to_string(),
                style,
                origin: url.origin().ascii_serialization(),
                path: url.path().trim_end_matches('/').to_string(),
            })
        })
        .collect()
}

//! `[auth.oidc]` of config.toml: which provider's tokens sign people in, and what their app roles mean here.
use super::Level;
use crate::app::App;
use crate::config::get;

pub struct Settings {
    pub issuer: String,
    pub client_id: String,
    /// The `aud` values accepted: by default the client id and `api://<client id>`.
    pub audience: Vec<String>,
    /// The tenant the token's `tid` must name: by default the issuer's first path part.
    pub tenant: String,
    pub jwks_uri: String,
    /// The provider's authorize and token endpoints: Entra's v2 ones under the issuer unless config.toml names them.
    pub authorize_url: String,
    pub token_url: String,
    /// The delegated scope a token must carry in `scp` (empty: not checked).
    pub scope: String,
    pub leeway: u64,
    pub hours: u64,
    /// App role value -> level, as `[auth.oidc.roles]` says.
    pub roles: Vec<(String, Level)>,
}

impl Settings {
    /// None until `issuer` and `client_id` are set: then nobody can sign in.
    pub fn of(app: &App) -> Option<Settings> {
        let raw = &app.cfg().raw;
        let text = |k: &str| get::<String>(raw, &["auth", "oidc", k]).filter(|v| !v.is_empty());
        let (issuer, client_id) = (text("issuer")?, text("client_id")?);
        let authority = issuer.strip_suffix("/v2.0").unwrap_or(&issuer).to_string();
        let tenant =
            text("tenant").unwrap_or_else(|| authority.rsplit('/').next().unwrap_or("").into());
        let number = |k: &str, d: u64| get::<u64>(raw, &["auth", "oidc", k]).unwrap_or(d);
        let mapped: toml::Table = get(raw, &["auth", "oidc", "roles"]).unwrap_or_default();
        let mut roles: Vec<(String, Level)> = mapped
            .iter()
            .filter_map(|(k, v)| Some((k.clone(), Level::parse(v.as_str()?)?)))
            .collect();
        if roles.is_empty() {
            let own = [("Team.Admin", Level::Admin), ("Team.Member", Level::Member)];
            roles = own
                .into_iter()
                .chain([("Team.Viewer", Level::Viewer)])
                .map(|(k, l)| (k.into(), l))
                .collect();
        }
        Some(Settings {
            audience: get(raw, &["auth", "oidc", "audience"])
                .unwrap_or_else(|| vec![client_id.clone(), format!("api://{client_id}")]),
            authorize_url: text("authorization_endpoint")
                .unwrap_or_else(|| format!("{authority}/oauth2/v2.0/authorize")),
            token_url: text("token_endpoint")
                .unwrap_or_else(|| format!("{authority}/oauth2/v2.0/token")),
            jwks_uri: text("jwks_uri")
                .unwrap_or_else(|| format!("{authority}/discovery/v2.0/keys")),
            scope: get(raw, &["auth", "oidc", "scope"]).unwrap_or_else(|| "access_as_user".into()),
            leeway: number("leeway", 60),
            hours: number("session_hours", 12),
            issuer,
            client_id,
            tenant,
            roles,
        })
    }

    /// The highest level the token's app roles give.
    pub fn level(&self, held: &[String]) -> Option<Level> {
        let of = |r: &String| self.roles.iter().find(|(k, _)| k == r).map(|(_, l)| *l);
        held.iter().filter_map(of).max()
    }

    /// The scope a sign-in asks for, the app's and the browser's alike.
    pub fn request_scope(&self) -> String {
        match self.scope.as_str() {
            "" => "openid".into(),
            s => format!("api://{}/{s}", self.client_id),
        }
    }
}

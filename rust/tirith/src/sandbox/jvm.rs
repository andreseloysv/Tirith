//! Gradle and the JVM ignore HTTPS_PROXY; the JDK refuses Basic proxy auth on tunnels unless re-enabled.

/// GRADLE_OPTS with the proxy's system properties added (the owner's own settings win).
pub fn gradle_opts(port: &str, user: &str, token: &str) -> String {
    let rows = ["http", "https"].into_iter().flat_map(|scheme| {
        [
            ("proxyHost", "127.0.0.1"),
            ("proxyPort", port),
            ("proxyUser", user),
            ("proxyPassword", token),
            ("nonProxyHosts", "localhost|127.0.0.1"),
        ]
        .map(|(k, v)| (scheme, k, v))
    });
    let gradle = rows.fold(
        std::env::var("GRADLE_OPTS").unwrap_or_default(),
        |g, (scheme, k, v)| {
            if g.contains(&format!("-D{scheme}.{k}=")) {
                g
            } else {
                format!("{g} -D{scheme}.{k}={v}")
            }
        },
    );
    format!("{gradle} -Djdk.http.auth.tunneling.disabledSchemes= -Djdk.http.auth.proxying.disabledSchemes=").trim().to_string()
}

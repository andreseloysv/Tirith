//! The Content-Type a file is served with.

/// Content kinds by file extension, `ext:kind|…` (the ones only a page file gets are named in `content_kind`).
const KINDS: &str = "png:image/png|jpg:image/jpeg|jpeg:image/jpeg|gif:image/gif|webp:image/webp|svg:image/svg+xml|pdf:application/pdf|json:application/json|html:text/html; charset=utf-8|css:text/css; charset=utf-8|js:application/javascript; charset=utf-8|mjs:application/javascript; charset=utf-8|woff2:font/woff2";

/// The Content-Type of a file named `name`: as a page file (`page`), or as an attachment, where text-shaped files (html too) are plain text, never rendered as a page on this origin, and page-only kinds (css, mjs, woff2) do not apply.
pub fn content_kind(name: &str, page: bool) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    let plain = "txt md log csv rs py js ts toml yaml yml sh html htm"
        .split(' ')
        .any(|e| e == ext);
    match (page, plain) {
        (false, true) => "text/plain; charset=utf-8",
        _ => KINDS
            .split('|')
            .filter_map(|kv| kv.split_once(':'))
            .find(|(e, _)| *e == ext && (page || !matches!(*e, "html" | "css" | "mjs" | "woff2")))
            .map(|(_, k)| k)
            .unwrap_or("application/octet-stream"),
    }
}

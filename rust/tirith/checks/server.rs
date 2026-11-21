use super::origin_host_ok;
#[test]
fn same_origin_checks_host() {
    let ok = |host, origin| origin_host_ok(Some(host), origin, 7777);
    // CLI, hooks and the page
    assert!(ok("127.0.0.1", None) && ok("127.0.0.1:7777", None) && ok("localhost:7777", Some("http://localhost:7777")) && ok("127.0.0.1:7777", Some("http://127.0.0.1:7777")));
    // DNS rebinding (foreign Host, even with a matching-looking Origin) and a foreign Origin with a good Host
    assert!(!ok("evil.example:7777", None) && !ok("evil.example", Some("http://evil.example:7777")) && !ok("127.0.0.1.evil.example", None) && !ok("127.0.0.1:9999", None));
    assert!(!ok("127.0.0.1:7777", Some("http://evil.example")));
}
#[test]
fn content_kinds_split_page_from_attachment() {
    let k = crate::attach::content_kind;
    assert_eq!((k("a.PNG", false), k("a.jpg", true), k("x.bin", true)), ("image/png", "image/jpeg", "application/octet-stream"));
    assert_eq!((k("i.html", true), k("i.html", false), k("s.js", false), k("s.js", true)), ("text/html; charset=utf-8", "text/plain; charset=utf-8", "text/plain; charset=utf-8", "application/javascript; charset=utf-8"));
    assert_eq!((k("f.woff2", true), k("f.woff2", false), k("d.json", false), k("n.txt", true)), ("font/woff2", "application/octet-stream", "application/json", "application/octet-stream"));
}

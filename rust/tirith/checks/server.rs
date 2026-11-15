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

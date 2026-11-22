// Who may use the daemon on a server with more users (server/peer.rs).
use super::peer::{is, uid};

const TABLE: &str = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:1E25 00000000:0000 0A 00000000:00000000 00:00000000 00000000   998        0 1001 1 0
   1: 0100007F:1E25 0100007F:C350 01 00000000:00000000 00:00000000 00000000   998        0 1002 1 0
   2: 0100007F:C350 0100007F:1E25 01 00000000:00000000 00:00000000 00000000  1234        0 1003 1 0
   3: 0100007F:C351 0100007F:1E25 04 00000000:00000000 00:00000000 00000000     0        0 0 1 0
   4: 0000000000000000FFFF00000100007F:C352 0000000000000000FFFF00000100007F:1E25 01 00000000:00000000 00:00000000 00000000  1500        0 1004 1 0
";

#[test]
fn the_caller_is_the_user_of_its_own_end() {
    let to = "127.0.0.1:7717".parse().unwrap();
    let who = |from: &str| uid(TABLE, from.parse().unwrap(), to);
    assert_eq!(
        who("127.0.0.1:50000"),
        Some(1234),
        "its end, not the daemon's"
    );
    assert_eq!(who("127.0.0.1:50002"), Some(1500), "a dual-stack socket");
    assert_eq!(
        who("127.0.0.1:50001"),
        None,
        "closed already: never taken for root"
    );
    assert_eq!(who("127.0.0.1:50009"), None, "unknown");
}

#[test]
fn a_user_is_named_or_numbered() {
    let passwd = "root:x:0:0::/root:/bin/sh\ndeploy:x:1234:1234::/home/deploy:/bin/sh\n";
    assert!(is(passwd, "deploy", 1234) && is(passwd, "1234", 1234));
    assert!(!is(passwd, "deploy", 1500) && !is(passwd, "dep", 1234) && !is(passwd, "", 1234));
}

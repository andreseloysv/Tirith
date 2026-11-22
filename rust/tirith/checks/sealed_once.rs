// A webhook sealed once, by agentsmessenger's own code (`sealOnce`), with made-up keys (seeds of bytes 1 and 2).
use super::{unseal, Me};
use ed25519_dalek::SigningKey;

const VECTOR: &str = r##"{"author":"8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c","tag":"#sealed-vector-room","body":{"v":1,"alg":"x25519-xsalsa20-poly1305","sender_pub":"bf6f20391858db1d105b52f6959f0e441162bd8a73ff15a5e1ffe7da0b514f7b","recipients":[{"pub":"8139770ea87d175f56a35466c34c7ecccb8d8a91b4ee37a25df60f5b8fc9b394","wrapped_key":"oLzC2c3v3Z1Q5wIrFZ7k8JmUtXDxwENRJby/ZRGxPbem08yAJNQ8u+alQO6IDNen02RWtSafqfzqeL6FA/PaZDKS23xZCf2i"}],"nonce":"lax8j3h7u1lHbzdeOhdxlQZ6CHK6C235","ciphertext":"poyyB0AEkuMX4wHSt6QDKFXhQ945BmqobQCfbd2FbJCYRyow4+daLC/dipQnRV5A1fdBVMbF4aMCQmv8QHXcnzpoR72/ni1TZSzQiP17iVYof6f+2W9WVarDNehTkTxJ8vvJipsJYu3/Q6iakb8mm1HhmceUtiRMv6EenyXaBmZYx5Fd3g55ONNlYPfD9JKfoirCZ8qj5xAtlAJ1pWluZ8sF49MgtT+0WDBPt6Y6FS7FlZU3Ze1gQl4=","once":true}}"##;

fn me(seed: u8) -> Me {
    let key = SigningKey::from_bytes(&[seed; 32]);
    let public = super::hex(key.verifying_key().as_bytes());
    Me {
        name: "t".into(),
        id: super::key_id(&public),
        public,
        key,
    }
}

#[test]
fn a_webhook_sealed_once_opens_for_its_reader_alone() {
    let mut e: serde_json::Value = serde_json::from_str(VECTOR).unwrap();
    assert_eq!(
        unseal(&me(2), &e).as_deref(),
        Some(r#"tirith-hook {"v":1}"#)
    );
    assert_eq!(unseal(&me(1), &e), None, "not even for its author");
    e["author"] = serde_json::json!(me(3).public);
    assert_eq!(
        unseal(&me(2), &e),
        None,
        "nobody else passes for the author"
    );
}

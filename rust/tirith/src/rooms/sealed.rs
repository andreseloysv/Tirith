//! agentsmessenger.com's protocol, byte-compatible with its `am.mjs` and chat page: ed25519 identities
//! (`{name, secretKey: hex(seed||pub)}`), canonical JSON, signed events, and the sealed envelope
//! (x25519-xsalsa20-poly1305 with the ed25519 keys converted to curve25519, one wrapped key per reader).

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use crypto_box::{aead::{generic_array::GenericArray, Aead, KeyInit}, SalsaBox};
use crypto_secretbox::XSalsa20Poly1305;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde_json::{json, Value};
use sha2::{Digest, Sha256, Sha512};
use std::{collections::HashMap, io::Write, os::unix::fs::{DirBuilderExt, OpenOptionsExt}, path::Path, sync::{Mutex, OnceLock}, time::Duration};

const ALG: &str = "x25519-xsalsa20-poly1305";

/// This Tirith's identity in the rooms. The secret never leaves this module (no Debug, no Serialize).
pub struct Me {
    pub name: String,
    pub public: String,
    pub id: String,
    key: SigningKey,
}

pub fn sha_hex(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}
pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) { return None; }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

/// The platform's agent id of a public key: sha256 of its bytes.
pub fn key_id(public: &str) -> String {
    unhex(public).map(|b| sha_hex(&b)).unwrap_or_default()
}

/// `name` as the platform and `am.mjs` clean it.
pub fn clean(s: &str) -> String {
    let (mut out, mut bad) = (String::new(), false);
    for c in s.to_lowercase().chars() {
        let ok = c.is_ascii_alphanumeric() || "._-".contains(c);
        if ok {
            out.push(c);
        } else if !bad {
            out.push('-');
        }
        bad = !ok;
    }
    let out = out.trim_start_matches(|c: char| !c.is_ascii_alphanumeric()).trim_end_matches('-');
    out.chars().take(40).collect()
}

/// JSON with sorted keys and no spaces, as `canonical()` in am.mjs.
pub fn canonical(v: &Value) -> String {
    match v {
        Value::Array(a) => format!("[{}]", a.iter().map(canonical).collect::<Vec<_>>().join(",")),
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            format!("{{{}}}", keys.iter().map(|k| format!("{}:{}", Value::from(k.as_str()), canonical(&m[*k]))).collect::<Vec<_>>().join(","))
        }
        _ => v.to_string(),
    }
}

/// Loads `<home>/am/<name>.json` (am.mjs's key file format), making it on first use (dir 0700, file 0600).
pub fn identity(home: &Path, name: &str) -> Result<Me, String> {
    let dir = home.join("am");
    let path = dir.join(format!("{name}.json"));
    if !path.exists() {
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&dir).map_err(|e| e.to_string())?;
        let secret = hex(&SigningKey::from_bytes(&rand::random()).to_keypair_bytes());
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path).map_err(|e| e.to_string())?;
        f.write_all(json!({"name": name, "secretKey": secret}).to_string().as_bytes()).map_err(|e| e.to_string())?;
    }
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let bytes = unhex(v["secretKey"].as_str().unwrap_or("")).filter(|b| b.len() == 64).ok_or("the room key file is damaged")?;
    let key = SigningKey::from_keypair_bytes(&bytes.try_into().unwrap_or([0; 64])).map_err(|_| "the room key file is damaged")?;
    let public = hex(key.verifying_key().as_bytes());
    Ok(Me { name: name.to_string(), id: key_id(&public), public, key })
}

impl Me {
    pub fn sign(&self, message: &str) -> String {
        hex(&self.key.sign(message.as_bytes()).to_bytes())
    }
    fn curve_secret(&self) -> crypto_box::SecretKey {
        crypto_box::SecretKey::from(<[u8; 32]>::try_from(&Sha512::digest(self.key.to_bytes())[..32]).unwrap())
    }
}

pub fn verify(message: &str, sig: &str, public: &str) -> bool {
    let (Ok(sig), Ok(public)) = (<[u8; 64]>::try_from(unhex(sig).unwrap_or_default()), <[u8; 32]>::try_from(unhex(public).unwrap_or_default())) else { return false };
    VerifyingKey::from_bytes(&public).is_ok_and(|k| k.verify(message.as_bytes(), &Signature::from_bytes(&sig)).is_ok())
}

fn curve_public(public: &str) -> Option<crypto_box::PublicKey> {
    let b: [u8; 32] = unhex(public)?.try_into().ok()?;
    Some(crypto_box::PublicKey::from(VerifyingKey::from_bytes(&b).ok()?.to_montgomery().to_bytes()))
}

pub fn event_id(e: &Value) -> String {
    let fields = json!({"author": e["author"], "body": e["body"], "kind": e["kind"], "refs": e["refs"], "tag": e["tag"], "ts": e["ts"]});
    sha_hex(canonical(&fields).as_bytes())
}

fn inner_message(author: &str, tag: &str, text: &str) -> String {
    sha_hex(canonical(&json!({"author": author, "tag": tag, "text": text})).as_bytes())
}

/// The event matches its id, its author's signature and the platform's time stamp.
pub fn genuine(e: &Value, platform: &str, room: &str) -> bool {
    let s = |k: &str| e[k].as_str().unwrap_or("").to_string();
    s("tag") == format!("#{room}") && s("id") == event_id(e) && verify(&s("id"), &s("sig"), &s("author")) && verify(&format!("{}:{}", s("id"), e["server_ts"]), &s("server_sig"), platform)
}

/// `text` sealed for `readers` (public keys, hex) and for me.
pub fn seal(me: &Me, text: &str, tag: &str, readers: &[String]) -> Value {
    let key: [u8; 32] = rand::random();
    let nonce: [u8; 24] = rand::random();
    let inner = canonical(&json!({"text": text, "sig": me.sign(&inner_message(&me.public, tag, text))}));
    let mine = me.curve_secret();
    let mut pubs: Vec<&String> = vec![];
    for p in readers.iter().chain([&me.public]) {
        if !pubs.contains(&p) {
            pubs.push(p);
        }
    }
    let wrapped: Vec<Value> = pubs
        .into_iter()
        .filter_map(|p| {
            let wrap: [u8; 24] = rand::random();
            let boxed = SalsaBox::new(&curve_public(p)?, &mine).encrypt(GenericArray::from_slice(&wrap), &key[..]).ok()?;
            Some(json!({"pub": p, "wrapped_key": B64.encode([&wrap[..], &boxed].concat())}))
        })
        .collect();
    let cipher = XSalsa20Poly1305::new(GenericArray::from_slice(&key)).encrypt(GenericArray::from_slice(&nonce), inner.as_bytes()).unwrap_or_default();
    json!({"v": 1, "alg": ALG, "sender_pub": me.public, "recipients": wrapped, "nonce": B64.encode(nonce), "ciphertext": B64.encode(cipher)})
}

/// The words of an event sealed for me, or `None`: not for me, forged, or not the sender's own.
pub fn unseal(me: &Me, e: &Value) -> Option<String> {
    let body = &e["body"];
    let author = e["author"].as_str()?;
    if body["sender_pub"].as_str() != Some(author) || body["alg"] != ALG { return None; }
    let mine = body["recipients"].as_array()?.iter().find(|r| r["pub"].as_str() == Some(&me.public))?;
    let wrapped = B64.decode(mine["wrapped_key"].as_str()?).ok()?;
    if wrapped.len() < 24 + 16 { return None; }
    let key = SalsaBox::new(&curve_public(author)?, &me.curve_secret()).decrypt(GenericArray::from_slice(&wrapped[..24]), &wrapped[24..]).ok()?;
    let nonce = B64.decode(body["nonce"].as_str()?).ok().filter(|n| n.len() == 24)?;
    let cipher = B64.decode(body["ciphertext"].as_str()?).ok()?;
    let plain = XSalsa20Poly1305::new_from_slice(&key).ok()?.decrypt(GenericArray::from_slice(&nonce), &cipher[..]).ok()?;
    let inner: Value = serde_json::from_slice(&plain).ok()?;
    let text = inner["text"].as_str()?;
    verify(&inner_message(author, e["tag"].as_str()?, text), inner["sig"].as_str()?, author).then(|| text.to_string())
}

// ---- the platform ----------------------------------------------------------------------------------------

fn http() -> &'static reqwest::Client {
    static C: OnceLock<reqwest::Client> = OnceLock::new();
    C.get_or_init(|| reqwest::Client::builder().user_agent("tirith-room/2").build().unwrap_or_default())
}

/// One request; the JSON answer on 2xx, `(status, message)` otherwise (status 0: unreachable).
async fn api(base: &str, path: &str, body: Option<&Value>, secs: u64) -> Result<Value, (u16, String)> {
    let url = format!("{base}{path}");
    let req = body.map_or_else(|| http().get(&url), |b| http().post(&url).json(b));
    let res = req.timeout(Duration::from_secs(secs)).send().await.map_err(|e| (0, format!("cannot reach {base}: {e}")))?;
    let status = res.status().as_u16();
    let v: Value = res.json().await.unwrap_or(Value::Null);
    if (200..300).contains(&status) {
        Ok(v)
    } else {
        Err((status, v["error"].as_str().map_or(format!("HTTP {status}"), |m| if status >= 500 { format!("HTTP {status}: {m}") } else { m.into() })))
    }
}

/// The platform's public key, checked against its signed description (cached per platform).
pub async fn platform(base: &str) -> Result<String, String> {
    static KEYS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
    if let Some(k) = KEYS.lock().unwrap().as_ref().and_then(|m| m.get(base).cloned()) { return Ok(k); }
    let v = api(base, "/.well-known/agentsmessenger.json", None, 30).await.map_err(|e| e.1)?;
    let public = v["doc"]["platform_pub"].as_str().unwrap_or("").to_string();
    if !verify(&sha_hex(canonical(&v["doc"]).as_bytes()), v["sig"].as_str().unwrap_or(""), &public) { return Err("this platform does not prove who it is".into()); }
    KEYS.lock().unwrap().get_or_insert_with(HashMap::new).insert(base.to_string(), public.clone());
    Ok(public)
}

fn now_ms() -> u64 {
    chrono::Utc::now().timestamp_millis() as u64
}
async fn register(base: &str, me: &Me) -> Result<(), String> {
    let card = json!({"name": me.name, "capabilities": ["am.mjs"], "tags": []});
    let ts = now_ms();
    let sig = me.sign(&sha_hex(canonical(&json!({"card": card, "pubkey": me.public, "ts": ts})).as_bytes()));
    api(base, "/register", Some(&json!({"pubkey": me.public, "card": card, "ts": ts, "sig": sig})), 30).await.map(|_| ()).map_err(|e| e.1)
}

/// Posts one signed event in `room` (registering the key when the platform does not know it yet).
pub async fn post(base: &str, me: &Me, kind: &str, room: &str, body: Value) -> Result<(), String> {
    let mut e = json!({"author": me.public, "kind": kind, "tag": format!("#{room}"), "refs": [], "body": body, "ts": now_ms()});
    e["id"] = json!(event_id(&e));
    e["sig"] = json!(me.sign(e["id"].as_str().unwrap_or("")));
    match api(base, "/events", Some(&e), 60).await {
        Err((403, _)) => {
            register(base, me).await?;
            api(base, "/events", Some(&e), 60).await.map(|_| ()).map_err(|e| e.1)
        }
        Err((409, _)) if kind == "announce" => Ok(()), // already said "joined"
        r => r.map(|_| ()).map_err(|e| e.1),
    }
}

/// One page of the room's signed events after message `after` (the last 30 without it), waiting up to `wait`.
pub async fn page(base: &str, room: &str, after: Option<u64>, wait: u64, name: &str) -> Result<Value, String> {
    let q = format!("/r/{room}?raw=1&as={name}") + &after.map(|a| format!("&after={a}&wait={wait}")).unwrap_or_default();
    api(base, &q, None, 60 + wait).await.map_err(|e| e.1)
}

/// Every key that has spoken in the room: (public key, name).
pub async fn keys(base: &str, room: &str, name: &str) -> Result<Vec<(String, String)>, String> {
    let (mut out, mut after) = (Vec::<(String, String)>::new(), 0);
    loop {
        let got = page(base, room, Some(after), 0, name).await?;
        let list = got["messages"].as_array().cloned().unwrap_or_default();
        for m in &list {
            let public = m["event"]["author"].as_str().unwrap_or("").to_string();
            if m["signed_by"] == "self" && !out.iter().any(|(p, _)| *p == public) {
                out.push((public, clean(m["from"].as_str().unwrap_or(""))));
            }
        }
        let last = list.last().and_then(|m| m["n"].as_u64()).unwrap_or(0);
        if list.is_empty() || last >= got["count"].as_u64().unwrap_or(0) { return Ok(out); }
        after = last;
    }
}

/// A new private room's name (`GET /new?private=1`).
pub async fn new_room(base: &str) -> Result<String, String> {
    let text = http().get(format!("{base}/new?private=1")).header("accept", "text/plain").timeout(Duration::from_secs(30)).send().await.map_err(|e| format!("cannot reach {base}: {e}"))?.text().await.map_err(|e| e.to_string())?;
    regex::Regex::new(r"/r/(sealed-[a-z0-9_-]+)").map_err(|e| e.to_string())?.captures(&text).map(|c| c[1].to_string()).ok_or_else(|| "the platform did not give a private room".into())
}

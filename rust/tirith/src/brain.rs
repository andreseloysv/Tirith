//! The brain: small text jobs for a cheap, fast model of the Meta Model API (OpenAI-style `/chat/completions`).
//! Never on a hot path: callers use the cached answer or a plain fallback. The key comes only from the
//! environment (`META_API_KEY` or `META_SI_API_KEY`) and is never logged or stored.

use crate::app::{git, now_s, Shared};
use crate::models;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use rusqlite::params;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const DEFAULT_URL: &str = "https://api.meta.ai/v1";
const DEFAULT_MODEL: &str = "muse-spark-1.3";
const KEY_VARS: [&str; 2] = ["META_API_KEY", "META_SI_API_KEY"];

/// The jobs: name and system prompt (the input is the user message).
const JOBS: [(&str, &str); 2] = [
    ("room-summary", "Below are the latest messages of a chat room where AI agents talk. Reply with ONE plain sentence saying what the room is discussing right now. No preamble."),
    ("architecture", "Below are the file tree, the README and the key manifests of a software project. Write a summary of its architecture for a coding agent about to work on it, at most 15 plain lines: what it is, the main parts and where they live (paths), how they connect, how to build and check it. Only facts from the input. No preamble."),
];

static CALLS: AtomicU64 = AtomicU64::new(0);
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);
static ARCH_TRIED: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

fn key() -> Option<String> {
    KEY_VARS.iter().find_map(|v| std::env::var(v).ok().filter(|k| !k.trim().is_empty()))
}

fn url(app: &Shared) -> String {
    crate::config::get::<String>(&app.cfg().raw, &["brain", "url"]).unwrap_or_else(|| DEFAULT_URL.into()).trim_end_matches('/').to_string()
}

fn model(app: &Shared) -> String {
    models::role(app, "brain").model.unwrap_or_else(|| DEFAULT_MODEL.into())
}

fn client() -> &'static reqwest::Client {
    static C: OnceLock<reqwest::Client> = OnceLock::new();
    C.get_or_init(|| reqwest::Client::builder().timeout(Duration::from_secs(30)).build().unwrap_or_default())
}

fn cache_key(job: &str, input: &str) -> String {
    Sha256::digest(format!("{job}\n{input}").as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// One job ("room-summary", "architecture") on `input`: the model's answer,
/// cached by the hash of (job, input). `None` without an API key or when the call fails.
pub async fn ask(app: &Shared, job: &str, input: &str) -> Option<String> {
    let prompt = JOBS.iter().find(|(j, _)| *j == job)?.1;
    let key = key()?;
    let hash = cache_key(job, input);
    let cached = app.db.lock().unwrap().query_row("SELECT answer FROM brain_cache WHERE key = ?1", [&hash], |r| r.get::<_, String>(0)).ok();
    if cached.is_some() {
        return cached;
    }
    let tail: String = input.chars().rev().take(16000).collect::<Vec<_>>().into_iter().rev().collect();
    let body = json!({"model": model(app), "max_tokens": 4096, "temperature": 0.3, "reasoning_effort": "low",
        "messages": [{"role": "system", "content": prompt}, {"role": "user", "content": tail}]});
    CALLS.fetch_add(1, Ordering::Relaxed);
    let result = async {
        let r = client().post(format!("{}/chat/completions", url(app))).bearer_auth(key).json(&body).send().await.map_err(|e| format!("call failed: {}", e.without_url()))?;
        if !r.status().is_success() {
            return Err(format!("model API returned HTTP {}", r.status().as_u16()));
        }
        let v: Value = r.json().await.map_err(|_| "unexpected response".to_string())?;
        let c = &v["choices"][0]["message"]["content"];
        let text = c.as_str().map(String::from).unwrap_or_else(|| c.as_array().into_iter().flatten().filter_map(|p| p["text"].as_str()).collect());
        Some(text.trim().to_string()).filter(|t| !t.is_empty()).ok_or("model API returned no answer text".to_string())
    }
    .await;
    *LAST_ERROR.lock().unwrap() = result.as_ref().err().cloned();
    let answer = result.ok()?;
    let _ = app.db.lock().unwrap().execute("INSERT OR REPLACE INTO brain_cache (key, job, answer, at) VALUES (?1, ?2, ?3, ?4)", params![hash, job, answer, crate::app::now_iso()]);
    Some(answer)
}

/// The project's architecture summary for lane briefs, for the current main commit; `None` until it exists
/// (asking starts its computation in the background, at most once per project per minute).
pub fn architecture(app: &Shared, project: &str) -> Option<String> {
    key()?;
    let last: Option<Value> = app.kv(&format!("brain.architecture.{project}")).and_then(|s| serde_json::from_str(&s).ok());
    let due = {
        let mut tried = ARCH_TRIED.lock().unwrap();
        let tried = tried.get_or_insert_with(HashMap::new);
        let due = now_s().saturating_sub(tried.get(project).copied().unwrap_or(0)) >= 60;
        if due {
            tried.insert(project.to_string(), now_s());
        }
        due
    };
    if due {
        let (app, project, known) = (app.clone(), project.to_string(), last.as_ref().map(|l| l["commit"].as_str().unwrap_or("").to_string()));
        tokio::spawn(async move {
            let Some(p) = app.cfg().project(&project).cloned() else { return };
            let repo = p.repo();
            let commit = match git(&repo, &["rev-parse", "main"]).await {
                Ok(c) => c,
                Err(_) => match git(&repo, &["rev-parse", "HEAD"]).await {
                    Ok(c) => c,
                    Err(_) => return,
                },
            };
            if known.as_deref() == Some(commit.as_str()) {
                return;
            }
            let tree = git(&repo, &["ls-tree", "-r", "--name-only", &commit]).await.unwrap_or_default();
            let mut input = format!("Project {project}. Files:\n{}\n", tree.lines().take(400).collect::<Vec<_>>().join("\n"));
            for f in ["README.md", "Cargo.toml", "package.json", "pyproject.toml", "go.mod", "tirith.toml"] {
                if let Ok(text) = git(&repo, &["show", &format!("{commit}:{f}")]).await {
                    input += &format!("\n--- {f}\n{}\n", text.chars().take(4000).collect::<String>());
                }
            }
            if let Some(summary) = ask(&app, "architecture", &input).await {
                app.set_kv(&format!("brain.architecture.{project}"), Some(&json!({"commit": commit, "summary": summary}).to_string()));
            }
        });
    }
    last.and_then(|l| l["summary"].as_str().map(String::from))
}

pub fn start(app: Shared) {
    let _ = app.db.lock().unwrap().execute_batch("CREATE TABLE IF NOT EXISTS brain_cache (key TEXT PRIMARY KEY, job TEXT, answer TEXT, at TEXT);");
}

async fn info(State(app): State<Shared>) -> Json<Value> {
    let cached: i64 = app.db.lock().unwrap().query_row("SELECT COUNT(*) FROM brain_cache", [], |r| r.get(0)).unwrap_or(0);
    let enabled = key().is_some();
    Json(json!({"enabled": enabled, "model": model(&app), "url": url(&app), "cached": cached,
        "calls": CALLS.load(Ordering::Relaxed), "last_error": *LAST_ERROR.lock().unwrap(),
        "reason": if enabled { Value::Null } else { json!("no API key: set META_API_KEY or META_SI_API_KEY in the daemon's environment") }}))
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/brain", get(info))
}

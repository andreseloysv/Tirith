//! What the servers window calls.
use super::view::{show, store, title};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn team_state(app: AppHandle) -> Value {
    let mut v = store(&app).read();
    v["title"] = json!(title(&app));
    v
}

#[tauri::command]
pub fn team_add(app: AppHandle, url: String) -> Result<(), String> {
    store(&app).add(&url)
}

#[tauri::command]
pub fn team_remove(app: AppHandle, url: String) -> Result<(), String> {
    store(&app).remove(&url)
}

/// Shows a server (its page, as whoever the window is signed in as there), or this Mac when `url` is none: only then is the local daemon started.
#[tauri::command]
pub async fn team_use(app: AppHandle, url: Option<String>) -> Result<(), String> {
    let kept = url.as_deref().map(super::store::Store::address).transpose()?;
    store(&app).activate(kept.as_deref())?;
    let go = move || -> Result<(), String> {
        let page = match &kept {
            Some(u) => format!("{u}/"),
            None => {
                let port = crate::daemon::port();
                crate::daemon::ensure_daemon(port);
                crate::page::link(port, &crate::daemon::tirith_bin())
            }
        };
        let win = app.get_webview_window("main").ok_or("no window")?;
        show(&win, page.parse().map_err(|_| "bad address")?, &title(&app));
        Ok(())
    };
    tauri::async_runtime::spawn_blocking(go).await.map_err(|e| e.to_string())?
}

/// Signs in to a server with Microsoft in the system browser, then shows its page as that person.
#[tauri::command]
pub async fn team_sign_in(app: AppHandle, url: String) -> Result<Value, String> {
    let url = super::store::Store::address(&url)?;
    let go = move || -> Result<Value, String> {
        let wait = std::time::Duration::from_secs(300);
        let s = tirith_signin::sign_in(&url, &super::browser::open, wait)?;
        let st = store(&app);
        st.activate(Some(&url))?;
        st.signed(&url, Some((&s.name, &s.role)))?;
        let win = app.get_webview_window("main").ok_or("no window")?;
        show(&win, format!("{url}{}", s.link).parse().map_err(|_| "bad link")?, &title(&app));
        Ok(json!({"name": s.name, "role": s.role}))
    };
    tauri::async_runtime::spawn_blocking(go).await.map_err(|e| e.to_string())?
}

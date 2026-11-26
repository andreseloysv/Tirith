//! What the window shows: a server's page or this Mac's, its title, the Server menu and the server list.
use super::store::Store;
use tauri::menu::{Menu, MenuItem, Submenu};
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub fn store(app: &AppHandle) -> Store {
    let dir = app.path().app_config_dir().unwrap_or_else(|_| crate::daemon::tirith_home());
    Store(dir.join("team-servers.json"))
}

/// The title says where the window is and who is signed in there.
pub fn title(app: &AppHandle) -> String {
    let v = store(app).read();
    let Some(url) = v["active"].as_str() else { return "NeoBot".into() };
    let who = &v["people"][url];
    let host = url.split("://").nth(1).unwrap_or(url);
    match (who["name"].as_str(), who["role"].as_str()) {
        (Some(n), Some(r)) => format!("NeoBot - {n} ({r}) at {host}"),
        _ => format!("NeoBot - {host} (not signed in)"),
    }
}

/// The team server the window starts on, when one is chosen: no daemon is started for it.
pub fn start_url(app: &AppHandle) -> Option<Url> {
    store(app).read()["active"].as_str().and_then(|u| format!("{u}/").parse().ok())
}

pub fn show(win: &WebviewWindow, url: Url, title: &str) {
    let _ = win.set_title(title);
    let _ = win.navigate(url);
}

pub fn menu(app: &AppHandle) -> tauri::Result<()> {
    let menu = Menu::default(app)?;
    let item = MenuItem::with_id(app, "servers", "Servers...", true, Some("CmdOrCtrl+Shift+S"))?;
    menu.append(&Submenu::with_items(app, "Server", true, &[&item])?)?;
    app.set_menu(menu)?;
    app.on_menu_event(|app, e| {
        if e.id() == "servers" {
            open_picker(app);
        }
    });
    Ok(())
}

/// The small window where servers are added, chosen and signed in to.
pub fn open_picker(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("servers") {
        let _ = w.set_focus();
        return;
    }
    let page = WebviewUrl::App("servers.html".into());
    let _ = WebviewWindowBuilder::new(app, "servers", page).title("NeoBot servers").inner_size(520.0, 560.0).build();
}

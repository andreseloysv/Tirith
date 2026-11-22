fn main() {
    // tauri-build would copy each externalBin over target/<profile>/tirith, the daemon's own build output, and
    // fail `cargo check` without one. Only the bundler needs the sidecar and it reads tauri.conf.json itself,
    // so this process's copy of the config has none (generate_context! still gets the caller's TAURI_CONFIG).
    std::env::set_var("TAURI_CONFIG", r#"{"bundle":{"externalBin":null}}"#);
    tauri_build::build()
}

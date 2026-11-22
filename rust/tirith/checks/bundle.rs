use super::{below, file, zip};

#[test]
fn the_binary_carries_the_page_and_the_extension() {
    let page = file("web/index.html").expect("the page");
    assert!(String::from_utf8_lossy(page).contains("js/app.js"));
    assert!(file("web/js/app.js").is_some());
    let ext = below("tools/teach");
    assert!(ext.iter().any(|(name, _)| *name == "manifest.json"));
    assert!(file("web/../Cargo.toml").is_none());
}

#[test]
fn the_extension_zip_opens() {
    let path = std::env::temp_dir().join(format!("tirith-ext-{}.zip", std::process::id()));
    let bytes = zip(
        "ext",
        &[
            ("a.txt", b"hello".as_slice()),
            ("sub/b.txt", b"".as_slice()),
        ],
    );
    std::fs::write(&path, &bytes).unwrap();
    // unzip checks every CRC and length; skipped where there is none.
    let Ok(out) = std::process::Command::new("unzip")
        .arg("-t")
        .arg(&path)
        .output()
    else {
        return;
    };
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    std::fs::remove_file(&path).unwrap();
}

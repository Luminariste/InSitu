//! Vérifie `insitu target` de bout en bout : le PDF s'écrit, `info` décrit
//! les 16 repères.

use std::process::Command;

use serde_json::Value;

#[test]
fn target_pdf_writes_a_pdf_file() {
    let path = std::env::temp_dir().join(format!("insitu-{}-mire.pdf", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_insitu"))
        .args(["target", "pdf"])
        .arg(&path)
        .args(["--page", "carre"])
        .output()
        .expect("le binaire insitu se lance");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = std::fs::read(&path).expect("le PDF existe");
    let _ = std::fs::remove_file(&path);
    assert!(bytes.starts_with(b"%PDF-1.4"));
    assert!(String::from_utf8_lossy(&bytes).contains("/MediaBox [0 0 841.8898 841.8898]"));
}

#[test]
fn target_pdf_rejects_an_unknown_page_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_insitu"))
        .args(["target", "pdf", "x.pdf", "--page", "a4"])
        .output()
        .expect("le binaire insitu se lance");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn target_info_lists_sixteen_markers() {
    let output = Command::new(env!("CARGO_BIN_EXE_insitu"))
        .args(["target", "info"])
        .output()
        .expect("le binaire insitu se lance");
    assert!(output.status.success());
    let info: Value = serde_json::from_slice(&output.stdout).expect("JSON");
    assert_eq!(info["version"], 2);
    assert_eq!(info["sizeMm"], 280.0);
    let markers = info["markers"].as_array().unwrap();
    assert_eq!(markers.len(), 16);
    assert_eq!(markers[0]["corners"][0], serde_json::json!([12.0, 12.0]));
    assert_eq!(info["gridNodeCount"], 660);
}

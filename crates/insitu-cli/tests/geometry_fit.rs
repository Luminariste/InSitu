//! Vérifie la commande `insitu geometry fit` de bout en bout : fichier JSON
//! en entrée, compte rendu JSON en sortie.

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::{json, Value};

/// Écrit `content` dans un fichier temporaire propre au test et lance
/// `insitu geometry fit` dessus.
fn run_fit(name: &str, content: &str) -> Output {
    let path: PathBuf =
        std::env::temp_dir().join(format!("insitu-{}-{name}.json", std::process::id()));
    std::fs::write(&path, content).expect("le fichier temporaire s'écrit");
    let output = Command::new(env!("CARGO_BIN_EXE_insitu"))
        .args(["geometry", "fit"])
        .arg(&path)
        .output()
        .expect("le binaire insitu se lance");
    let _ = std::fs::remove_file(&path);
    output
}

/// Trapèze connu : (x, y) ↦ (2x + 10, 2y − 5) / (1 + x / 1000).
fn known(x: f64, y: f64) -> [f64; 2] {
    let w = 1.0 + x / 1000.0;
    [(2.0 * x + 10.0) / w, (2.0 * y - 5.0) / w]
}

/// Douze points de la diapositive (grille 4 × 3) et leur image.
fn twelve_pairs() -> Vec<Value> {
    let mut pairs = Vec::new();
    for j in 0..3 {
        for i in 0..4 {
            // Grille volontairement irrégulière.
            let x = 20.0 + 80.0 * i as f64 + 7.0 * ((i * 3 + j) % 4) as f64;
            let y = 25.0 + 115.0 * j as f64 - 5.0 * ((i + 2 * j) % 3) as f64;
            pairs.push(json!({ "source": [x, y], "target": known(x, y) }));
        }
    }
    pairs
}

#[test]
fn exact_trapezoid_is_reported_without_outliers() {
    let input = json!({ "pairs": twelve_pairs() });
    let output = run_fit("exact", &input.to_string());
    assert!(output.status.success(), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).expect("la sortie est du JSON");
    assert_eq!(report["pointCount"], 12);
    assert_eq!(report["outliers"], json!([]));
    assert!(report["homographyResiduals"]["max"].as_f64().unwrap() < 1e-9);
    // La matrice est celle du trapèze connu.
    let expected = [[2.0, 0.0, 10.0], [0.0, 2.0, -5.0], [0.001, 0.0, 1.0]];
    for (i, row) in expected.iter().enumerate() {
        for (j, want) in row.iter().enumerate() {
            let got = report["homography"][i][j].as_f64().unwrap();
            assert!((got - want).abs() < 1e-9, "coefficient ({i}, {j}) : {got}");
        }
    }
}

#[test]
fn a_point_moved_by_five_millimetres_is_the_only_outlier() {
    let mut pairs = twelve_pairs();
    let (x, y) = (150.0, 100.0);
    let [u, v] = known(x, y);
    pairs.insert(4, json!({ "source": [x, y], "target": [u + 3.0, v - 4.0] }));
    let input = json!({ "pairs": pairs, "outlierFloor": 0.5 });
    let output = run_fit("aberrant", &input.to_string());
    assert!(output.status.success(), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["pointCount"], 13);
    assert_eq!(report["outliers"], json!([4]));
    // Prédit par les douze autres, le point est à 5 mm de sa cible (à peu
    // près : le trapèze de départ a été ajusté avec lui).
    let error = report["leaveOneOut"]["perPoint"][4].as_f64().unwrap();
    assert!((error - 5.0).abs() < 0.1, "{error}");
    assert_eq!(report["leaveOneOut"]["worstIndex"], 4);
}

#[test]
fn unusable_points_give_a_french_message_and_a_failure_code() {
    let input = json!({ "pairs": [
        { "source": [0.0, 0.0], "target": [0.0, 0.0] },
        { "source": [1.0, 1.0], "target": [1.0, 0.0] },
        { "source": [2.0, 2.0], "target": [1.0, 1.0] },
        { "source": [3.0, 3.0], "target": [0.0, 1.0] }
    ]});
    let output = run_fit("alignes", &input.to_string());
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("alignés"), "{message}");
}

#[test]
fn malformed_file_is_explained() {
    let output = run_fit("mal-forme", r#"{ "paires": [] }"#);
    assert_eq!(output.status.code(), Some(1));
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("insitu geometry --help"), "{message}");
}

#[test]
fn help_documents_the_input_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_insitu"))
        .args(["geometry", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    for word in ["pairs", "source", "target", "smoothing", "outlierFloor"] {
        assert!(help.contains(word), "l'aide ne parle pas de `{word}`");
    }
}

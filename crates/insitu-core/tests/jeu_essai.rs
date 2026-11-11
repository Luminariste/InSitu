//! Vérifie que le jeu d'essai est complet et conforme à son manifeste.
//!
//! Les fichiers d'origine ne sont pas dans le dépôt : le manifeste en garde
//! la description (taille, empreinte, dimensions). Le dépôt contient des
//! références pleine résolution et des versions réduites, vérifiées ici.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use insitu_core::units;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jeu-essai/epinal")
}

fn manifest() -> Value {
    let text = fs::read_to_string(fixture_root().join("manifeste.json"))
        .expect("jeu-essai/epinal/manifeste.json est lisible");
    serde_json::from_str(&text).expect("le manifeste est du JSON valide")
}

fn files(manifest: &Value) -> &Vec<Value> {
    manifest["fichiers"]
        .as_array()
        .expect("`fichiers` est une liste")
}

fn sha256_of(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("{} illisible : {e}", path.display()));
    Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Vérifie qu'un fichier versé au dépôt est bien celui que décrit `info`.
fn assert_matches(id: &str, info: &Value) {
    let path = fixture_root().join(info["chemin"].as_str().unwrap());
    assert_eq!(
        sha256_of(&path),
        info["sha256"].as_str().unwrap(),
        "{id} : {} ne correspond plus au manifeste",
        path.display()
    );
    if let Some(expected_width) = info["largeur"].as_u64() {
        let (width, height) = image::image_dimensions(&path)
            .unwrap_or_else(|e| panic!("{id} : image illisible : {e}"));
        assert_eq!(u64::from(width), expected_width, "{id}");
        assert_eq!(u64::from(height), info["hauteur"].as_u64().unwrap(), "{id}");
    }
}

#[test]
fn every_projector_has_its_three_reference_files() {
    let manifest = manifest();
    for projector in ["A", "B", "C", "D"] {
        let roles: BTreeSet<&str> = files(&manifest)
            .iter()
            .filter(|f| f["projecteur"] == projector && !f["reference"].is_null())
            .map(|f| f["role"].as_str().unwrap())
            .collect();
        for role in ["raw_photo", "rectified_photo", "engraving_export"] {
            assert!(
                roles.contains(role),
                "projecteur {projector} : référence `{role}` manquante"
            );
        }
    }
}

#[test]
fn references_match_the_manifest_and_keep_full_resolution() {
    let manifest = manifest();
    let mut checked = 0;
    for file in files(&manifest) {
        let reference = &file["reference"];
        if reference.is_null() {
            continue;
        }
        let id = file["id"].as_str().unwrap();
        assert_matches(id, reference);
        // Une référence garde toujours la taille en pixels de l'original.
        assert_eq!(reference["largeur"], file["original"]["largeur"], "{id}");
        assert_eq!(reference["hauteur"], file["original"]["hauteur"], "{id}");
        if reference["fidelite"] == "identique" {
            assert_eq!(reference["sha256"], file["original"]["sha256"], "{id}");
        }
        checked += 1;
    }
    assert!(checked >= 20, "seulement {checked} références vérifiées");
}

#[test]
fn reduced_images_match_the_manifest() {
    let manifest = manifest();
    let mut checked = 0;
    for file in files(&manifest) {
        let reduced = &file["reduit"];
        if reduced.is_null() {
            continue;
        }
        let id = file["id"].as_str().unwrap();
        assert_matches(id, reduced);
        // La réduction est un simple changement d'échelle de l'original.
        let scale = reduced["echelle"].as_f64().unwrap();
        let original_width = file["original"]["largeur"].as_f64().unwrap();
        assert_eq!(
            reduced["largeur"].as_f64().unwrap(),
            (original_width / scale).round(),
            "{id}"
        );
        checked += 1;
    }
    assert!(
        checked >= 25,
        "seulement {checked} images réduites vérifiées"
    );
}

#[test]
fn reference_dimensions_are_the_ones_the_engine_expects() {
    let manifest = manifest();
    for file in files(&manifest) {
        let dpi = match file["role"].as_str().unwrap() {
            "rectified_photo" => units::RECTIFIED_PHOTO_DPI,
            "engraving_export" => units::DEFAULT_EXPORT_DPI,
            _ => continue,
        };
        let expected = units::mm_to_px(units::SLIDE_SIZE_MM, dpi);
        let id = file["id"].as_str().unwrap();
        assert_eq!(file["reference"]["largeur"], expected, "{id}");
        assert_eq!(file["reference"]["hauteur"], expected, "{id}");
    }
}

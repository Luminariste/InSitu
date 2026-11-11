//! Fabrique des pyramides de tuiles à partir des réduits du jeu d'essai, pour
//! vérifier le choix du format sur de vrais fichiers : un Verre est un dessin
//! au trait (PNG), une photo redressée est une photo (JPEG).

use std::fs;
use std::path::{Path, PathBuf};

use insitu_core::tiles::{self, TileFormat, TileOptions};

fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../jeu-essai/epinal/reduits")
        .join(relative)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("insitu-tuiles-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn engraving_export_becomes_a_grey_png_pyramid() {
    let out = scratch("verre");
    let options = TileOptions {
        tile_size: 256,
        ..TileOptions::default()
    };
    let pyramid = tiles::build_pyramid(&fixture("verres/verre-a.png"), &out, &options).unwrap();
    // Réduit ÷ 8 d'un Verre de 15 433 px.
    assert_eq!((pyramid.width, pyramid.height), (1929, 1929));
    assert_eq!(pyramid.format, TileFormat::Png);
    assert_eq!(pyramid.channels, 1);
    let sizes: Vec<u32> = pyramid.levels.iter().map(|l| l.width).collect();
    assert_eq!(sizes, [1929, 965, 483, 242]);
    assert_eq!(pyramid.tile_count(), 64 + 16 + 4 + 1);
    // La tuile du coin bas-droit existe et a la taille du reste.
    let corner = tiles::read_raster(&out.join("0/7_7.png")).unwrap();
    assert_eq!(
        (corner.width, corner.height),
        (1929 - 7 * 256, 1929 - 7 * 256)
    );
    assert_eq!(tiles::read_descriptor(&out).unwrap(), pyramid);
    fs::remove_dir_all(&out).unwrap();
}

#[test]
fn rectified_photo_becomes_a_jpeg_pyramid() {
    let out = scratch("photo");
    let pyramid = tiles::build_pyramid(
        &fixture("photos-redressees/a.png"),
        &out,
        &TileOptions::default(),
    )
    .unwrap();
    // Réduit ÷ 8 d'une photo redressée de 5 512 px.
    assert_eq!((pyramid.width, pyramid.height), (689, 689));
    assert_eq!(pyramid.format, TileFormat::Jpeg);
    assert_eq!(pyramid.levels.len(), 2);
    assert!(out.join("0/1_1.jpg").is_file());
    assert!(out.join("1/0_0.jpg").is_file());
    fs::remove_dir_all(&out).unwrap();
}

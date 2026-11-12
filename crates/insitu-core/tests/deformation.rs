//! Critère d'acceptation de la tâche 3.4 : une diapositive de 15 433 px est
//! calculée en moins de 30 s.
//!
//! La source est le vrai Verre A d'Épinal (15 433 px, traits blancs sur
//! noir), déformé par un trapèze marqué et une ondulation de 2 mm, comme une
//! façade en relief. Le test ne tourne qu'en compilation optimisée (c'est
//! ainsi que l'app est fabriquée) : `cargo test --release --test deformation`.
//! En compilation de mise au point, il est ignoré.

use std::path::Path;
use std::time::Instant;

use insitu_core::geometry::{GridWarp, Homography, Point, Rect, Transform};
use insitu_core::resample::{warp_raster, WarpOptions};
use insitu_core::tiles::read_raster;
use insitu_core::units::{mm_to_px, DEFAULT_EXPORT_DPI, SLIDE_SIZE_MM};

/// Correspondance diapositive (mm) → Verre (mm) : trapèze et ondulation.
fn slide_to_source() -> GridWarp {
    let n = SLIDE_SIZE_MM;
    let base = Homography::from_four_pairs(
        [
            Point::new(0.0, 0.0),
            Point::new(n, 0.0),
            Point::new(n, n),
            Point::new(0.0, n),
        ],
        [
            Point::new(0.06 * n, 0.02 * n),
            Point::new(0.97 * n, 0.10 * n),
            Point::new(0.90 * n, 0.95 * n),
            Point::new(0.01 * n, 0.88 * n),
        ],
    )
    .unwrap();
    let region = Rect::new(Point::new(0.0, 0.0), Point::new(n, n)).unwrap();
    // Pas de 2,5 mm, comme pour une déformation issue de points (D-2.3-b).
    GridWarp::sample(base, region, 113, 113, |p| {
        let phase = std::f64::consts::TAU / 40.0;
        Point::new(2.0 * (phase * p.y).sin(), 2.0 * (phase * p.x).cos())
    })
    .unwrap()
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "mesure de temps : lancer avec cargo test --release --test deformation"
)]
fn a_15433_px_slide_is_computed_in_under_30_seconds() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../jeu-essai/epinal/references/verres/verre-a.png");
    let source = read_raster(&path).expect("Verre A du jeu d'essai");
    let size = mm_to_px(SLIDE_SIZE_MM, DEFAULT_EXPORT_DPI);
    assert_eq!((source.width, source.channels), (size, 1));

    let warp = slide_to_source();
    let px_per_mm = f64::from(size) / SLIDE_SIZE_MM;
    let map = |p: Point| warp.apply(p * (1.0 / px_per_mm)).map(|q| q * px_per_mm);
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let start = Instant::now();
    let slide = warp_raster(&source, size, size, map, &WarpOptions::default()).unwrap();
    let seconds = start.elapsed().as_secs_f64();
    eprintln!("diapositive de {size} px calculée en {seconds:.1} s sur {threads} cœurs");
    assert_eq!((slide.width, slide.height), (size, size));
    assert!(seconds < 30.0, "{seconds:.1} s");

    // La quantité de trait se conserve, au rapport des surfaces près : la
    // diapositive voit une partie du Verre, à une échelle voisine de 1.
    let lit = |data: &[u8]| data.iter().map(|v| u64::from(*v)).sum::<u64>();
    let (before, after) = (lit(&source.data), lit(&slide.data));
    eprintln!("lumière : source {before}, diapositive {after}");
    assert!(after > before / 2 && after < before * 2);
}

/// Sur un vrai fichier : une déformation qui ne déforme rien (les quatre
/// coins de la diapositive sur les quatre coins de l'image) rend l'image à
/// l'identique, au niveau de gris près.
#[test]
fn a_real_engraving_file_comes_back_unchanged_through_an_identity_warp() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../jeu-essai/epinal/reduits/verres/verre-a.png");
    let source = read_raster(&path).expect("réduit du Verre A");
    let n = f64::from(source.width);
    let corners_mm = [
        Point::new(0.0, 0.0),
        Point::new(SLIDE_SIZE_MM, 0.0),
        Point::new(SLIDE_SIZE_MM, SLIDE_SIZE_MM),
        Point::new(0.0, SLIDE_SIZE_MM),
    ];
    let corners_px = corners_mm.map(|p| p * (n / SLIDE_SIZE_MM));
    let homography = Homography::from_four_pairs(corners_mm, corners_px).unwrap();
    let px_per_mm = n / SLIDE_SIZE_MM;
    let map = |p: Point| homography.apply(p * (1.0 / px_per_mm));
    let slide = warp_raster(
        &source,
        source.width,
        source.height,
        map,
        &WarpOptions::default(),
    )
    .unwrap();
    let worst = slide
        .data
        .iter()
        .zip(&source.data)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    assert!(worst <= 1, "écart maximal de {worst} niveaux");
}

//! Critère d'acceptation de la tâche 5.1 : l'écart maximal entre le vecteur
//! et l'image source est inférieur à un demi-pixel.
//!
//! Les sources sont les quatre verres d'Épinal (traits blancs sur noir, les
//! fichiers envoyés à LightBurn), en pleine résolution (15 433 px) et en
//! réduit. L'écart est mesuré par `compare`, qui ne réutilise rien de
//! l'ajustement : distance dans les deux sens entre les courbes et le contour
//! de l'image au seuil, et distance à la courbe de chaque pixel que les
//! courbes classent autrement que l'image seuillée.

use std::path::{Path, PathBuf};
use std::time::Instant;

use insitu_core::tiles::read_raster;
use insitu_core::vectorize::{compare, vectorize, GrayImage, VectorizeOptions};

fn verre(dir: &str, letter: char) -> GrayImage {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../jeu-essai/epinal")
        .join(dir)
        .join(format!("verres/verre-{letter}.png"));
    GrayImage::from_raster(&read_raster(&path).expect("verre du jeu d'essai"))
}

fn check(image: &GrayImage, options: &VectorizeOptions, name: &str, margin: Option<f64>) {
    let start = Instant::now();
    let vector = vectorize(image, options).unwrap();
    let seconds = start.elapsed().as_secs_f64();
    let cmp = compare(image, &vector);
    println!(
        "{name} : {} contours, {} droites, {} courbes en {seconds:.1} s ; écart {:.3} px \
         (image → vecteur {:.3}, vecteur → image {:.3}) ; pixels {} / {} dont {} différents, \
         au plus à {:.3} px",
        vector.stats.contours,
        vector.stats.lines,
        vector.stats.cubics,
        cmp.max_deviation_px,
        cmp.raw_to_vector_px,
        cmp.vector_to_raw_px,
        cmp.source_pixels,
        cmp.vector_pixels,
        cmp.mismatched_pixels,
        cmp.max_mismatch_distance_px,
    );
    assert!(vector.stats.contours > 1000, "{name} : dessin non trouvé");
    assert!(cmp.max_deviation_px < 0.5, "{name} : {cmp:?}");
    assert!(cmp.max_mismatch_distance_px < 0.5, "{name} : {cmp:?}");
    // Le lissage ne doit ni épaissir ni amincir le trait de façon visible.
    // Une droite posée sur un arc passe à l'intérieur de l'arc : en pleine
    // résolution, la surface du trait change de moins de 1 %. Sur des traits
    // d'un pixel ou moins (réduits, seuil haut), l'écart reste sous le
    // demi-pixel mais la surface peut varier de près de 10 % : elle n'y est
    // pas contrôlée.
    if let Some(margin) = margin {
        let ratio = cmp.vector_pixels as f64 / cmp.source_pixels as f64;
        assert!(
            (ratio - 1.0).abs() <= margin,
            "{name} : surface ×{ratio:.3}"
        );
    }
}

#[test]
fn the_four_full_resolution_glasses_are_traced_within_half_a_pixel() {
    for letter in ['a', 'b', 'c', 'd'] {
        let image = verre("references", letter);
        assert_eq!(image.width, 15_433);
        check(
            &image,
            &VectorizeOptions::default(),
            &format!("verre {letter}"),
            Some(0.01),
        );
    }
}

#[test]
fn reduced_glasses_hold_at_every_threshold() {
    // Les réduits ont des traits d'un à deux pixels, en gris : le cas
    // difficile pour le seuil.
    for letter in ['a', 'b', 'c', 'd'] {
        let image = verre("reduits", letter);
        for threshold in [64, 128, 192] {
            let options = VectorizeOptions {
                threshold,
                ..Default::default()
            };
            check(
                &image,
                &options,
                &format!("réduit {letter}, seuil {threshold}"),
                None,
            );
        }
    }
}

#[test]
fn a_looser_tolerance_gives_fewer_segments() {
    let image = verre("reduits", 'a');
    let tight = vectorize(&image, &VectorizeOptions::default()).unwrap();
    let loose = vectorize(
        &image,
        &VectorizeOptions {
            tolerance_px: 1.0,
            ..Default::default()
        },
    )
    .unwrap();
    let count = |v: &insitu_core::vectorize::Vectorization| v.stats.lines + v.stats.cubics;
    assert!(
        count(&loose) * 10 < count(&tight) * 8,
        "{} morceaux à 1 px contre {} à 0,3 px",
        count(&loose),
        count(&tight)
    );
    assert!(loose.stats.max_fit_error_px <= 1.0 + 1e-9);
}

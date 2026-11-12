//! Tâche 2.4a : détection de la mire actuelle et redressement, sur les vraies
//! photos d'Épinal.
//!
//! Critère d'acceptation (plan, tâche 2.4a) : les 8 photos brutes sont
//! détectées sans aide, et les croisements de la grille de l'image
//! redressée tombent à moins de 0,3 mm de leur place au 95e centile.
//!
//! La photo redressée à la main (A.tiff) sert de témoin : elle est elle-même
//! décalée de 0,6 mm en moyenne (étude 2.1), on vérifie seulement que notre
//! image a le même sens et qu'elle s'y superpose à moins de 2 mm.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use image::imageops::{self, FilterType};
use image::{GrayImage, RgbImage};
use insitu_core::geometry::Point;
use insitu_core::rectification::{self, DetectOptions, DetectionError, TargetDetection};

fn epinal() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jeu-essai/epinal")
}

fn raw_photo(name: &str) -> RgbImage {
    rectification::read_photo(&epinal().join("references/photos-brutes").join(name))
        .expect("photo lisible")
}

fn detect(photo: &RgbImage) -> TargetDetection {
    rectification::detect(photo, &DetectOptions::default()).unwrap_or_else(|e| panic!("{e}"))
}

/// Détection sans aide, puis redressement à 250 dpi (9,8 px par mm, plus fin
/// que la photo brute), puis mesure de la grille dans l'image obtenue.
fn check_photo(name: &str) {
    let photo = raw_photo(name);
    let detection = detect(&photo);
    let rectified = rectification::rectify(&photo, &detection, 250.0);
    let check = rectification::check_rectified(&rectified).expect("grille relue");
    let d = check.deviation_mm;
    eprintln!(
        "{name} : {} croisements détectés, {} relus dans l'image redressée, écart moy. {:.3} / p95 {:.3} / max {:.3} mm, couverture {:.0} %",
        detection.nodes.len(),
        check.nodes.len(),
        d.mean,
        d.p95,
        d.max,
        100.0 * detection.coverage
    );
    assert!(
        detection.nodes.len() >= 600,
        "{name} : trop peu de croisements"
    );
    assert!(
        check.nodes.len() >= 600,
        "{name} : trop peu de croisements relus"
    );
    assert!(d.p95 < 0.3, "{name} : 95e centile {:.3} mm", d.p95);
    assert!(
        detection.coverage > 0.9,
        "{name} : couverture {:.2}",
        detection.coverage
    );
}

#[test]
fn photo_a_brut() {
    check_photo("a-brut.jpg");
}
#[test]
fn photo_b_brut() {
    check_photo("b-brut.jpg");
}
#[test]
fn photo_c_brut() {
    check_photo("c-brut.jpg");
}
#[test]
fn photo_d_brut() {
    check_photo("d-brut.jpg");
}
#[test]
fn photo_img_5293() {
    check_photo("img_5293.jpg");
}
#[test]
fn photo_img_5295() {
    check_photo("img_5295.jpg");
}
#[test]
fn photo_img_5298() {
    check_photo("img_5298.jpg");
}
#[test]
fn photo_img_5302() {
    check_photo("img_5302.jpg");
}

fn small_gray(image: &RgbImage) -> Vec<f64> {
    let g: GrayImage =
        imageops::grayscale(&imageops::resize(image, 140, 140, FilterType::Triangle));
    g.pixels().map(|p| f64::from(p.0[0])).collect()
}

fn correlation(a: &[f64], b: &[f64]) -> f64 {
    let ma = a.iter().sum::<f64>() / a.len() as f64;
    let mb = b.iter().sum::<f64>() / b.len() as f64;
    let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        ab += (x - ma) * (y - mb);
        aa += (x - ma) * (x - ma);
        bb += (y - mb) * (y - mb);
    }
    ab / (aa * bb).sqrt()
}

#[test]
fn photo_a_lies_on_the_manual_rectification() {
    let photo = raw_photo("a-brut.jpg");
    let detection = detect(&photo);
    let ours = rectification::rectify(&photo, &detection, 500.0);
    let manual =
        rectification::read_photo(&epinal().join("references/photos-redressees/a.jpg")).unwrap();
    assert_eq!(
        ours.dimensions(),
        manual.dimensions(),
        "même taille que A.tiff (5 512 px)"
    );

    // Même sens : notre image ressemble à A.tiff, et pas à ses rotations ni
    // à son miroir.
    let reference = small_gray(&manual);
    let same = correlation(&small_gray(&ours), &reference);
    let mut others = Vec::new();
    for turned in [
        imageops::rotate90(&ours),
        imageops::rotate180(&ours),
        imageops::rotate270(&ours),
        imageops::flip_horizontal(&ours),
        imageops::flip_vertical(&ours),
    ] {
        others.push(correlation(&small_gray(&turned), &reference));
    }
    eprintln!("ressemblance avec A.tiff : {same:.3} ; tournée ou retournée : {others:.3?}");
    assert!(same > 0.9, "ressemblance {same:.3}");
    assert!(others.iter().all(|o| *o < 0.6), "{others:?}");

    // Superposition : chaque croisement, mesuré dans les deux images.
    let ours_check = rectification::check_rectified(&ours).unwrap();
    let manual_check = rectification::check_rectified(&manual).unwrap();
    let key = |p: Point| ((p.x / 10.0).round() as i32, (p.y / 10.0).round() as i32);
    let manual_at: HashMap<_, _> = manual_check
        .nodes
        .iter()
        .map(|(t, m)| (key(*t), *m))
        .collect();
    let gaps: Vec<f64> = ours_check
        .nodes
        .iter()
        .filter_map(|(t, m)| manual_at.get(&key(*t)).map(|n| m.distance(*n)))
        .collect();
    let gap = rectification::ErrorSummary::of(&gaps);
    eprintln!(
        "A.tiff contre la grille : moy. {:.3} / p95 {:.3} / max {:.3} mm",
        manual_check.deviation_mm.mean,
        manual_check.deviation_mm.p95,
        manual_check.deviation_mm.max
    );
    eprintln!(
        "notre image contre la grille : moy. {:.3} / p95 {:.3} / max {:.3} mm",
        ours_check.deviation_mm.mean, ours_check.deviation_mm.p95, ours_check.deviation_mm.max
    );
    eprintln!(
        "notre image contre A.tiff, sur {} croisements : moy. {:.3} / p95 {:.3} / max {:.3} mm",
        gaps.len(),
        gap.mean,
        gap.p95,
        gap.max
    );
    assert!(gaps.len() >= 600);
    assert!(gap.max < 2.0, "écart à A.tiff {:.2} mm", gap.max);
    assert!(ours_check.deviation_mm.p95 < 0.3);
}

#[test]
fn four_rough_corners_are_enough_to_start() {
    let photo = raw_photo("d-brut.jpg");
    let automatic = detect(&photo);
    // Coins décalés de 8 à 12 px (environ 1,5 mm), comme un clic approximatif.
    let offsets = [(9.0, -7.0), (-12.0, 4.0), (6.0, 11.0), (-8.0, -9.0)];
    let slide = [(0.0, 0.0), (280.0, 0.0), (280.0, 280.0), (0.0, 280.0)];
    let corners = std::array::from_fn(|k| {
        let p = automatic
            .slide_to_photo(Point::new(slide[k].0, slide[k].1))
            .unwrap();
        p + Point::new(offsets[k].0, offsets[k].1)
    });
    let manual = rectification::detect(
        &photo,
        &DetectOptions {
            corners: Some(corners),
            ..DetectOptions::default()
        },
    )
    .unwrap();
    assert!(
        manual.nodes.len() >= 600,
        "{} croisements",
        manual.nodes.len()
    );
    // Même modèle final, à moins d'un dixième de mm (au centre, dans le
    // disque blanc sans croisement, le modèle extrapole : c'est là que les
    // deux diffèrent le plus, 0,06 mm).
    for (x, y) in [(20.0, 20.0), (140.0, 140.0), (260.0, 75.0), (90.0, 250.0)] {
        let p = Point::new(x, y);
        let a = automatic.slide_to_photo(p).unwrap();
        let b = manual.slide_to_photo(p).unwrap();
        assert!(
            a.distance(b) / automatic.px_per_mm < 0.1,
            "en ({x}, {y}) : {} px",
            a.distance(b)
        );
    }
}

#[test]
fn the_confidence_map_covers_the_target() {
    let detection = detect(&raw_photo("c-brut.jpg"));
    let map = detection.confidence_map();
    assert_eq!((map.columns, map.rows), (140, 140));
    let mut errors: Vec<f64> = map.error_mm.iter().flatten().copied().collect();
    errors.sort_by(f64::total_cmp);
    let median = errors[errors.len() / 2];
    assert!(median < 0.15, "erreur estimée médiane {median:.3} mm");
    assert!(map.at(Point::new(45.0, 45.0)).is_some());
    let covered = map.error_mm.iter().flatten().count() as f64 / map.error_mm.len() as f64;
    assert!(covered > 0.9, "{covered}");
    // Le disque blanc central n'a pas de croisement : il n'est pas couvert.
    assert_eq!(map.at(Point::new(140.0, 140.0)), None);
    // Les correspondances passent dans le fichier projet.
    let r = detection.to_project_rectification();
    assert_eq!(r.points.len(), detection.nodes.len());
    assert!(r.residual_mm.unwrap() < 0.2);
}

#[test]
fn a_photo_without_target_is_refused() {
    let wall =
        rectification::read_photo(&epinal().join("reduits/photos-brutes/a-mur.jpg")).unwrap();
    match rectification::detect(&wall, &DetectOptions::default()) {
        Err(DetectionError::NotFound(cause)) => eprintln!("refusée : {cause}"),
        Err(other) => panic!("erreur inattendue : {other}"),
        Ok(d) => panic!("mire trouvée à tort ({} croisements)", d.nodes.len()),
    }
}

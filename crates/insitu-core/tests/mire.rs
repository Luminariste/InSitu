//! La nouvelle mire garde la géométrie de l'ancienne.
//!
//! Critère de la tâche 2.2 : « l'ancienne mire reste utilisable ». La
//! détection (tâche 2.4) reconnaît les deux mires par les mêmes éléments :
//! grille de 10 mm, croix, cercle rouge, disque central. On mesure ici ces
//! éléments sur l'ancienne mire du jeu d'essai (page 2 du fichier
//! Illustrator, rendue à 600 dpi) et on vérifie qu'ils sont là où la
//! nouvelle mire les dessine.

use std::path::Path;

use insitu_core::target::pdf::{self, PageFormat};
use insitu_core::target::{
    TargetLayout, DISC_RADIUS_MM, GRID_LINES, GRID_PITCH_MM, RED_CIRCLE_RADIUS_MM,
};

/// L'ancienne mire, en niveaux de gris.
fn old_target() -> image::GrayImage {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../jeu-essai/epinal/references/mire/mire-280-page2.png");
    image::open(&path)
        .unwrap_or_else(|e| panic!("{} illisible : {e}", path.display()))
        .to_luma8()
}

/// Centres (en mm) des traits sombres croisés le long d'une ligne de pixels,
/// pondérés par leur noirceur.
fn dark_runs(values: &[u8], mm_per_px: f64) -> Vec<f64> {
    let mut runs = Vec::new();
    let mut i = 0;
    while i < values.len() {
        if values[i] >= 200 {
            i += 1;
            continue;
        }
        let (mut sum, mut weight) = (0.0, 0.0);
        while i < values.len() && values[i] < 200 {
            let w = 255.0 - values[i] as f64;
            sum += (i as f64 + 0.5) * w;
            weight += w;
            i += 1;
        }
        runs.push(sum / weight * mm_per_px);
    }
    runs
}

#[test]
fn old_target_has_the_same_grid_cross_circle_and_disc() {
    let img = old_target();
    assert_eq!(img.width(), img.height());
    let layout = TargetLayout::coded();
    let mm_per_px = layout.size_mm / img.width() as f64;
    let center = layout.center();

    let measure = |y: f64| -> (usize, Vec<f64>) {
        let row = (y / mm_per_px) as u32;
        let values: Vec<u8> = (0..img.width()).map(|x| img.get_pixel(x, row)[0]).collect();
        let mut grid = 0;
        let mut radii = Vec::new();
        for x in dark_runs(&values, mm_per_px) {
            let nearest_line = (x / GRID_PITCH_MM).round() * GRID_PITCH_MM;
            if (x - nearest_line).abs() < 0.15 {
                grid += 1;
            } else {
                radii.push((x - center.x).hypot(y - center.y));
            }
        }
        (grid, radii)
    };
    let near = |r: f64, target: f64| (r - target).abs() < 0.2;

    // Ligne horizontale à y = 105 mm : elle croise les 29 lignes verticales
    // (croix comprise) et deux fois le cercle rouge, loin de toute ligne.
    let (grid, radii) = measure(105.0);
    assert_eq!(grid, GRID_LINES, "lignes verticales trouvées à y = 105 mm");
    assert_eq!(radii.len(), 2, "{radii:?}");
    assert!(
        radii.iter().all(|&r| near(r, RED_CIRCLE_RADIUS_MM)),
        "{radii:?}"
    );

    // Ligne horizontale à y = 145 mm : elle traverse le disque central.
    let (_, radii) = measure(145.0);
    assert_eq!(
        radii.iter().filter(|&&r| near(r, DISC_RADIUS_MM)).count(),
        2,
        "{radii:?}"
    );

    // La croix verticale : colonne x = 140 mm, au-dessus du disque.
    let col = (center.x / mm_per_px) as u32;
    let dark_above_disc = (0..(100.0 / mm_per_px) as u32)
        .filter(|&y| img.get_pixel(col, y)[0] < 200)
        .count() as f64
        * mm_per_px;
    assert!(
        dark_above_disc > 99.0,
        "croix verticale : {dark_above_disc:.1} mm sombres sur 100"
    );
}

/// Le PDF versé au dépôt pour l'impression (`docs/mire/`) est bien celui que
/// fabrique le moteur. S'il échoue après un changement du dessin, refaire le
/// fichier : `cargo run -p insitu-cli -- target pdf docs/mire/mire-insitu-v2.pdf`.
#[test]
fn printable_pdf_in_docs_is_up_to_date() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/mire/mire-insitu-v2.pdf");
    let committed =
        std::fs::read(&path).unwrap_or_else(|e| panic!("{} illisible : {e}", path.display()));
    let fresh = pdf::render(&TargetLayout::coded(), PageFormat::A3);
    assert!(
        committed == fresh,
        "docs/mire/mire-insitu-v2.pdf est à refaire (voir le commentaire du test)"
    );
}

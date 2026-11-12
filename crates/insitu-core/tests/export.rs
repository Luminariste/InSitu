//! Critère d'acceptation de la tâche 3.5, en pleine résolution : « le PNG
//! s'ouvre dans LightBurn à 280 mm ; comparaison avec Verre-A ».
//!
//! Le vrai Verre-A d'Épinal (15 433 px) est posé sur une façade par un
//! recalage en trapèze avec du relief : c'est la fresque. L'export de la
//! diapositive à 1 400 dpi, sans marge, doit redonner Verre-A : mêmes
//! traits, à la même place. Le PNG écrit doit annoncer 280 mm.
//!
//! Ne tourne qu'en compilation optimisée (la vérification GitHub le lance) :
//!
//! ```text
//! cargo test --release -p insitu-core --test export -- --nocapture
//! ```
//!
//! L'ouverture dans LightBurn elle-même ne peut être vérifiée que sur un
//! poste où LightBurn est installé.

use std::path::Path;
use std::time::Instant;

use insitu_core::export::{encode_png, render_slide};
use insitu_core::geometry::{Point, Transform};
use insitu_core::project::{ExportSettings, PointMm, RectMm, RegistrationPair};
use insitu_core::registration::slide_to_facade;
use insitu_core::resample::{warp_raster, WarpOptions};
use insitu_core::tiles::{read_raster, Raster};
use insitu_core::units::{DEFAULT_EXPORT_DPI, SLIDE_SIZE_MM};

fn pairs() -> Vec<RegistrationPair> {
    [
        ((0.0, 0.0), (1_000.0, 800.0)),
        ((280.0, 0.0), (5_100.0, 600.0)),
        ((280.0, 280.0), (5_300.0, 4_700.0)),
        ((0.0, 280.0), (900.0, 4_500.0)),
        ((140.0, 140.0), (3_080.0, 2_690.0)),
        ((70.0, 210.0), (1_990.0, 3_640.0)),
    ]
    .iter()
    .map(|&((sx, sy), (fx, fy))| RegistrationPair::new(PointMm::new(sx, sy), PointMm::new(fx, fy)))
    .collect()
}

/// Rectangle de la fresque : il couvre l'emprise du projecteur.
const FRAME: RectMm = RectMm {
    x: 800.0,
    y: 500.0,
    width: 4_600.0,
    height: 4_300.0,
};

/// Part des pixels « trait » de `a` qui ont un pixel « trait » de `b` à un
/// pixel près.
fn lines_found(a: &Raster, b: &Raster) -> f64 {
    let w = a.width as usize;
    let h = a.height as usize;
    let lit = |r: &Raster, i: usize, j: usize| r.data[j * w + i] >= 128;
    let (mut total, mut found) = (0u64, 0u64);
    for j in 1..h - 1 {
        for i in 1..w - 1 {
            if lit(a, i, j) {
                total += 1;
                found += u64::from((j - 1..=j + 1).any(|y| (i - 1..=i + 1).any(|x| lit(b, x, y))));
            }
        }
    }
    found as f64 / total as f64
}

/// Décalage de `a` par rapport à `b` sur un carré, en pixels (±3 px, affiné
/// par une parabole) ; `None` si le carré est presque vide.
fn local_shift(a: &Raster, b: &Raster, x0: usize, y0: usize, size: usize) -> Option<(f64, f64)> {
    let w = a.width as usize;
    let score = |dx: i64, dy: i64| {
        let mut sum = 0.0;
        for j in (y0..y0 + size).step_by(2) {
            for i in x0..x0 + size {
                let (ai, aj) = ((i as i64 + dx) as usize, (j as i64 + dy) as usize);
                sum += f64::from(a.data[aj * w + ai]) * f64::from(b.data[j * w + i]);
            }
        }
        sum
    };
    if score(0, 0) < 1e7 {
        return None;
    }
    let mut best = (0, 0, f64::MIN);
    for dy in -3..=3 {
        for dx in -3..=3 {
            let v = score(dx, dy);
            if v > best.2 {
                best = (dx, dy, v);
            }
        }
    }
    let refine = |m: f64, c: f64, p: f64| {
        let d = m - 2.0 * c + p;
        if d.abs() < 1e-12 {
            0.0
        } else {
            0.5 * (m - p) / d
        }
    };
    let (dx, dy, c) = best;
    let fx = refine(score(dx - 1, dy), c, score(dx + 1, dy));
    let fy = refine(score(dx, dy - 1), c, score(dx, dy + 1));
    Some((dx as f64 + fx, dy as f64 + fy))
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "pleine résolution : lancer avec cargo test --release --test export"
)]
fn verre_a_exported_at_1400_dpi_matches_the_original() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../jeu-essai/epinal/references/verres/verre-a.png");
    let verre = read_raster(&path).expect("Verre A du jeu d'essai");
    assert_eq!(verre.channels, 1);
    let warp = slide_to_facade(&pairs()).unwrap();

    // La fresque : un peu plus fine que la diapositive sur la façade.
    let start = Instant::now();
    let mm_per_px = 0.25;
    let (fw, fh) = (
        (FRAME.width / mm_per_px) as u32,
        (FRAME.height / mm_per_px) as u32,
    );
    let slide_px = f64::from(verre.width) / SLIDE_SIZE_MM;
    let fresco = warp_raster(
        &verre,
        fw,
        fh,
        |p| {
            let facade = Point::new(FRAME.x + p.x * mm_per_px, FRAME.y + p.y * mm_per_px);
            let s = warp.apply_inverse(facade)?;
            Some(Point::new(s.x * slide_px, s.y * slide_px))
        },
        &WarpOptions::default(),
    )
    .unwrap();
    println!(
        "fresque de {fw} × {fh} px fabriquée en {:.1} s",
        start.elapsed().as_secs_f64()
    );

    let settings = ExportSettings {
        margin: false,
        ..ExportSettings::default()
    };
    assert_eq!(settings.dpi, DEFAULT_EXPORT_DPI);
    let start = Instant::now();
    let out = render_slide(&fresco, &FRAME, &warp, "", &settings, 0).unwrap();
    let render_seconds = start.elapsed().as_secs_f64();
    drop(fresco);
    let start = Instant::now();
    let bytes = encode_png(&out, &settings).unwrap();
    let png_seconds = start.elapsed().as_secs_f64();

    // Le PNG annonce 280 mm.
    let decoder = png::Decoder::new(std::io::Cursor::new(&bytes));
    let reader = decoder.read_info().unwrap();
    let info = reader.info();
    assert_eq!((info.width, info.height), (verre.width, verre.height));
    let dims = info.pixel_dims.expect("résolution inscrite");
    let size_mm = f64::from(info.width) / f64::from(dims.xppu) * 1000.0;
    assert!((size_mm - 280.0).abs() < 0.05, "{size_mm} mm");

    let found = lines_found(&out, &verre);
    let kept = lines_found(&verre, &out);
    let mut shift: f64 = 0.0;
    let mut patches = 0;
    let n = verre.width as usize;
    for y in 0..4 {
        for x in 0..4 {
            let (x0, y0) = (n / 8 + x * n / 4 - 300, n / 8 + y * n / 4 - 300);
            if let Some((dx, dy)) = local_shift(&out, &verre, x0, y0, 600) {
                shift = shift.max(dx.hypot(dy));
                patches += 1;
            }
        }
    }
    println!(
        "Verre-A exporté ({} px, {size_mm:.3} mm) : calcul {render_seconds:.1} s, PNG {png_seconds:.1} s ({} Mo) ; \
         {:.2} % des traits exportés sur un trait de Verre-A, {:.2} % des traits de Verre-A retrouvés (à 1 px près) ; \
         décalage local au pire {shift:.2} px ({patches} carrés)",
        out.width,
        bytes.len() / 1_000_000,
        100.0 * found,
        100.0 * kept,
    );
    assert!(
        found > 0.995 && kept > 0.995,
        "traits : {found:.4} / {kept:.4}"
    );
    assert!(patches >= 4, "{patches} carrés seulement");
    assert!(shift < 0.25, "décalage : {shift:.2} px");
}

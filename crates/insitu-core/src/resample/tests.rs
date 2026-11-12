//! Tests de la déformation d'images : exactitude, qualité des traits fins,
//! treillis.
//!
//! La qualité se mesure sur un trait fin, colonne par colonne, contre son
//! rendu idéal (la part exacte de chaque pixel couverte par le trait) :
//!
//! - **quantité de trait** : somme des valeurs d'une colonne, en pixels de
//!   blanc. Si elle varie le long du trait, le trait « pompe » ;
//! - **position** : centre de gravité de la colonne. S'il fait des marches
//!   au lieu de suivre une droite, c'est le crénelage (« escalier ») ;
//! - **épaisseur** : écart type du profil de la colonne autour de son centre.
//!   Un filtre trop doux l'augmente : c'est l'épaississement.
//!
//! Chaque mesure est comparée à celle d'un rééchantillonnage « au plus
//! proche voisin » (sans filtre), pour montrer qu'elle détecte bien le défaut.

use super::*;
use crate::geometry::{GridWarp, Homography, Rect, Transform};

fn options(step: u32) -> WarpOptions {
    WarpOptions {
        lattice_step: step,
        tile_size: 64,
        threads: 3,
        background: 0,
    }
}

/// Image de bruit reproductible.
fn noise(width: u32, height: u32, channels: u8, seed: u64) -> Raster {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let data = (0..width as usize * height as usize * channels as usize)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect();
    Raster::new(width, height, channels, data)
}

/// Rendu idéal en gris : part de chaque pixel (8 × 8 sous-points) dont
/// l'image par `map` tombe dans la forme `inside` (coordonnées source).
fn ideal<M, S>(width: u32, height: u32, map: M, inside: S) -> Raster
where
    M: Fn(Point) -> Option<Point>,
    S: Fn(Point) -> bool,
{
    const N: u32 = 8;
    let mut data = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height {
        for x in 0..width {
            let mut hits = 0;
            for sy in 0..N {
                for sx in 0..N {
                    let p = Point::new(
                        f64::from(x) + (f64::from(sx) + 0.5) / f64::from(N),
                        f64::from(y) + (f64::from(sy) + 0.5) / f64::from(N),
                    );
                    if map(p).is_some_and(&inside) {
                        hits += 1;
                    }
                }
            }
            data.push((255.0 * f64::from(hits) / f64::from(N * N)).round() as u8);
        }
    }
    Raster::new(width, height, 1, data)
}

/// Rééchantillonnage sans filtre, pour comparaison : chaque pixel prend la
/// valeur du pixel source où tombe son centre.
fn nearest<M: Fn(Point) -> Option<Point>>(
    source: &Raster,
    width: u32,
    height: u32,
    map: M,
) -> Raster {
    let mut data = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height {
        for x in 0..width {
            let value = map(Point::new(f64::from(x) + 0.5, f64::from(y) + 0.5))
                .and_then(|p| {
                    let (i, j) = (p.x.floor(), p.y.floor());
                    (i >= 0.0
                        && j >= 0.0
                        && i < f64::from(source.width)
                        && j < f64::from(source.height))
                    .then(|| source.data[j as usize * source.width as usize + i as usize])
                })
                .unwrap_or(0);
            data.push(value);
        }
    }
    Raster::new(width, height, 1, data)
}

/// Mesures d'un trait presque horizontal, prises sur les colonnes `columns`.
#[derive(Debug)]
struct LineStats {
    /// Quantité moyenne de trait par colonne, en pixels de blanc.
    amount: f64,
    /// Variation relative de cette quantité d'une colonne à l'autre.
    amount_variation: f64,
    /// Écart quadratique moyen du centre des colonnes à la droite ajustée.
    wobble: f64,
    /// Épaisseur moyenne au seuil de 50 % (ce que garde la gravure en mode
    /// seuil), en pixels : nombre de pixels d'au moins 128 par colonne.
    width: f64,
    /// Valeur la plus forte hors du trait (halo), de 0 à 255.
    halo: u8,
}

/// Une colonne : abscisse, quantité, centre, épaisseur au seuil.
type Sample = (f64, f64, f64, f64);

fn line_stats(image: &Raster, columns: std::ops::Range<u32>) -> LineStats {
    let w = image.width as usize;
    let mut samples = Vec::new();
    let mut halo = 0u8;
    for x in columns {
        let column: Vec<f64> = (0..image.height as usize)
            .map(|y| f64::from(image.data[y * w + x as usize]) / 255.0)
            .collect();
        let amount: f64 = column.iter().sum();
        let center = column
            .iter()
            .enumerate()
            .map(|(y, v)| v * (y as f64 + 0.5))
            .sum::<f64>()
            / amount;
        let variance = column
            .iter()
            .enumerate()
            .map(|(y, v)| v * (y as f64 + 0.5 - center).powi(2))
            .sum::<f64>()
            / amount;
        let width = column.iter().filter(|v| **v >= 128.0 / 255.0).count() as f64;
        // Hors du trait : à plus de 2 épaisseurs de son centre.
        let reach = 2.0 * variance.sqrt().max(1.0) + 2.0;
        for (y, v) in column.iter().enumerate() {
            if (y as f64 + 0.5 - center).abs() > reach {
                halo = halo.max((v * 255.0).round() as u8);
            }
        }
        samples.push((f64::from(x), amount, center, width));
    }
    let n = samples.len() as f64;
    let mean = |f: &dyn Fn(&Sample) -> f64| samples.iter().map(f).sum::<f64>() / n;
    let amount = mean(&|s| s.1);
    let amount_sd = mean(&|s| (s.1 - amount).powi(2)).sqrt();
    // Droite des moindres carrés passant par les centres.
    let (mx, mc) = (mean(&|s| s.0), mean(&|s| s.2));
    let slope = mean(&|s| (s.0 - mx) * (s.2 - mc)) / mean(&|s| (s.0 - mx).powi(2));
    let wobble = mean(&|s| (s.2 - mc - slope * (s.0 - mx)).powi(2)).sqrt();
    LineStats {
        amount,
        amount_variation: amount_sd / amount,
        wobble,
        width: mean(&|s| s.3),
        halo,
    }
}

/// Source : un trait horizontal blanc de `thickness` pixels entiers, posé
/// sur la grille comme dans un dessin, au milieu d'une image noire.
fn horizontal_line(size: u32, thickness: u32) -> (Raster, impl Fn(Point) -> bool) {
    let top = size / 2;
    let mut data = vec![0u8; (size * size) as usize];
    for y in top..top + thickness {
        for x in 0..size {
            data[(y * size + x) as usize] = 255;
        }
    }
    let (y0, y1) = (f64::from(top), f64::from(top + thickness));
    (Raster::new(size, size, 1, data), move |p: Point| {
        p.y >= y0 && p.y < y1 && p.x >= 0.0 && p.x < f64::from(size)
    })
}

#[test]
fn identity_copies_the_source_exactly() {
    for channels in [1, 3, 4] {
        let source = noise(203, 150, channels, 7);
        for step in [0, 1, 8, 16] {
            let out = warp_raster(&source, 203, 150, Some, &options(step)).unwrap();
            assert_eq!(out, source, "{channels} canaux, pas {step}");
        }
    }
}

#[test]
fn whole_pixel_shift_copies_and_fills_the_outside_with_background() {
    let source = noise(100, 80, 1, 3);
    let mut opts = options(8);
    opts.background = 9;
    let out = warp_raster(&source, 100, 80, |p| Some(p + Point::new(5.0, -2.0)), &opts).unwrap();
    for y in 0..80usize {
        for x in 0..100usize {
            let v = out.data[y * 100 + x];
            if x + 5 < 100 && y >= 2 {
                assert_eq!(v, source.data[(y - 2) * 100 + x + 5]);
            } else if x + 5 >= 102 || y < 1 {
                // Loin du bord : uniquement du fond.
                assert_eq!(v, 9, "({x}, {y})");
            }
        }
    }
}

#[test]
fn points_without_preimage_get_the_background() {
    let source = Raster::new(50, 50, 1, vec![200; 2500]);
    let mut opts = options(8);
    opts.background = 0;
    // La moitié droite de la sortie n'a pas d'antécédent.
    let map = |p: Point| (p.x < 25.0).then_some(p);
    let out = warp_raster(&source, 50, 50, map, &opts).unwrap();
    for y in 10..40usize {
        assert_eq!(out.data[y * 50 + 10], 200);
        assert_eq!(out.data[y * 50 + 40], 0);
    }
}

#[test]
fn rejects_unusable_requests() {
    let source = noise(4, 4, 1, 1);
    assert_eq!(
        warp_raster(&source, 0, 10, Some, &WarpOptions::default()),
        Err(WarpError::EmptyOutput)
    );
    let opts = WarpOptions {
        lattice_step: 512,
        ..WarpOptions::default()
    };
    assert!(matches!(
        warp_raster(&source, 4, 4, Some, &opts),
        Err(WarpError::InvalidOption(_))
    ));
}

/// Rotation de `degrees` autour du centre d'une image de `size` pixels,
/// vue de la sortie vers la source.
fn rotation(size: u32, degrees: f64) -> impl Fn(Point) -> Option<Point> + Sync {
    let c = f64::from(size) / 2.0;
    // Centre vers l'origine, rotation, puis retour au centre.
    let h = Homography::translation(-c, -c)
        .then(&Homography::similarity(1.0, degrees.to_radians(), 0.0, 0.0).unwrap())
        .then(&Homography::translation(c, c));
    move |p: Point| h.apply(p)
}

#[test]
fn a_slightly_rotated_thin_line_has_no_staircase_and_keeps_its_thickness() {
    // Un trait de 2 pixels tourné de 3° : c'est l'angle qui fait les plus
    // longues marches d'escalier sans filtre.
    let size = 400;
    let (source, inside) = horizontal_line(size, 2);
    let map = rotation(size, 3.0);
    let columns = 60..340;

    let reference = line_stats(&ideal(size, size, &map, &inside), columns.clone());
    let filtered = line_stats(
        &warp_raster(&source, size, size, &map, &options(8)).unwrap(),
        columns.clone(),
    );
    let raw = line_stats(&nearest(&source, size, size, &map), columns);
    eprintln!("idéal   : {reference:?}\nfiltré  : {filtered:?}\nsans filtre : {raw:?}");

    // La mesure détecte bien l'escalier et le pompage sans filtre…
    assert!(raw.wobble > 0.2, "{raw:?}");
    assert!(raw.amount_variation > 0.02, "{raw:?}");
    // … et le filtre les supprime : le centre du trait s'écarte d'une droite
    // de moins d'un vingtième de pixel (1 µm à 1 400 dpi).
    assert!(filtered.wobble < 0.05, "{filtered:?}");
    assert!(filtered.amount_variation < 0.01, "{filtered:?}");
    // Ni plus, ni moins de trait que l'idéal, pas plus épais au seuil de
    // gravure, pas de halo.
    assert!(
        (filtered.amount / reference.amount - 1.0).abs() < 0.01,
        "{filtered:?}"
    );
    assert!(
        (filtered.width - reference.width).abs() < 0.15,
        "{filtered:?} / {reference:?}"
    );
    assert_eq!(filtered.halo, 0, "{filtered:?}");
}

#[test]
fn an_enlarged_thin_line_stays_sharp() {
    // Un dessin de 4 096 px posé sur une diapositive de 15 433 px est
    // agrandi d'environ 3,8 : un trait d'un pixel doit devenir un trait
    // d'environ 3,8 pixels, pas une traînée floue.
    let size = 160;
    let (source, inside) = horizontal_line(size, 1);
    let k = 3.8;
    let out_size = (f64::from(size) * k) as u32;
    let c = f64::from(size) / 2.0;
    let (s, co) = 2f64.to_radians().sin_cos();
    let map = move |p: Point| {
        let (x, y) = (p.x / k - c, p.y / k - c);
        Some(Point::new(c + co * x - s * y, c + s * x + co * y))
    };
    let columns = 100..500;
    let reference = line_stats(&ideal(out_size, out_size, map, &inside), columns.clone());
    let filtered = line_stats(
        &warp_raster(&source, out_size, out_size, map, &options(8)).unwrap(),
        columns,
    );
    eprintln!("idéal : {reference:?}\nfiltré : {filtered:?}");
    // L'interpolation cubique arrondit les flancs du trait : un peu plus de
    // trait au total (le noyau rend 8 % de plus, au pied du trait), mais
    // au seuil de gravure l'épaisseur reste à un tiers de pixel de l'idéal.
    assert!(
        (filtered.amount / reference.amount - 1.0).abs() < 0.1,
        "{filtered:?}"
    );
    assert!(
        (filtered.width - reference.width).abs() < 0.35,
        "{filtered:?} / {reference:?}"
    );
    assert!(filtered.wobble < 0.05, "{filtered:?}");
    assert_eq!(filtered.halo, 0, "{filtered:?}");
}

#[test]
fn reducing_fine_hatching_gives_an_even_grey_without_moire() {
    // Hachures d'un pixel blanc, un pixel noir, réduites de 3,3 avec une
    // légère rotation : sans filtre, des bandes (moiré) apparaissent ; avec
    // le filtre, un gris uniforme à mi-chemin.
    let size = 1320;
    let data = (0..size * size)
        .map(|i| if (i % size) % 2 == 0 { 255 } else { 0 })
        .collect();
    let source = Raster::new(size, size, 1, data);
    let k = 3.3;
    let out_size = 360;
    let (s, co) = 4f64.to_radians().sin_cos();
    let c = f64::from(out_size) / 2.0;
    let map = move |p: Point| {
        let (x, y) = (p.x - c, p.y - c);
        Some(Point::new(
            k * (c + co * x - s * y),
            k * (c + s * x + co * y),
        ))
    };
    let stats = |image: &Raster| {
        // Centre de l'image seulement : les bords tournés sortent de la source.
        let values: Vec<f64> = (60..300)
            .flat_map(|y| (60..300).map(move |x| (x, y)))
            .map(|(x, y)| f64::from(image.data[y * out_size as usize + x]))
            .collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let sd =
            (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64).sqrt();
        (mean, sd)
    };
    let (mean, sd) = stats(&warp_raster(&source, out_size, out_size, map, &options(8)).unwrap());
    let (_, raw_sd) = stats(&nearest(&source, out_size, out_size, map));
    eprintln!(
        "filtré : moyenne {mean:.1}, écart type {sd:.1} ; sans filtre : écart type {raw_sd:.1}"
    );
    assert!(raw_sd > 100.0);
    assert!((mean - 127.5).abs() < 2.0, "moyenne {mean}");
    assert!(sd < 3.0, "écart type {sd}");
}

/// Déformation réaliste d'une diapositive de `size` pixels : trapèze marqué
/// et ondulation de 2 mm d'amplitude (rapportée à 280 mm), comme un relief.
fn realistic_map(size: u32) -> impl Fn(Point) -> Option<Point> + Sync {
    let n = f64::from(size);
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
    let amplitude = 2.0 / 280.0 * n;
    let wavelength = 40.0 / 280.0 * n;
    let region = Rect::new(Point::new(0.0, 0.0), Point::new(n, n)).unwrap();
    let grid = GridWarp::sample(base, region, 113, 113, |p| {
        let phase = std::f64::consts::TAU / wavelength;
        Point::new(
            amplitude * (phase * p.y).sin(),
            amplitude * (phase * p.x).cos(),
        )
    })
    .unwrap();
    move |p: Point| grid.apply(p)
}

#[test]
fn lattice_matches_exact_evaluation() {
    // Un carré de 900 px pris au milieu d'une diapositive de 15 433 px.
    let size = 900;
    let source = noise(size, size, 1, 11);
    let slide = realistic_map(15_433);
    let map = move |p: Point| {
        slide(p + Point::new(7_000.0, 7_000.0)).map(|q| q - Point::new(7_000.0, 7_000.0))
    };
    let exact = warp_raster(&source, size, size, &map, &options(0)).unwrap();
    let lattice = warp_raster(&source, size, size, &map, &options(8)).unwrap();
    let worst = exact
        .data
        .iter()
        .zip(&lattice.data)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    // Sur du bruit (le pire cas : chaque pixel diffère de son voisin), un
    // décalage d'un centième de pixel change la valeur de quelques niveaux.
    let mean = exact
        .data
        .iter()
        .zip(&lattice.data)
        .map(|(a, b)| f64::from(a.abs_diff(*b)))
        .sum::<f64>()
        / exact.data.len() as f64;
    eprintln!("treillis / exact : écart moyen {mean:.2} niveau, maximal {worst}");
    assert!(mean < 0.5, "écart moyen de {mean} niveau");
    assert!(worst <= 4, "écart maximal de {worst} niveaux");
}

#[test]
fn lattice_position_error_stays_below_two_hundredths_of_a_pixel() {
    // Mesure directe : écart entre la position interpolée par le treillis
    // et la position exacte, sur une diapositive de 15 433 px.
    let size = 15_433;
    let map = realistic_map(size);
    let step = f64::from(DEFAULT_LATTICE_STEP);
    let mut worst = 0.0f64;
    let mut state = 5u64;
    for _ in 0..20_000 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let i = ((state >> 33) % (u64::from(size) / 8 - 1)) as f64;
        let j = ((state >> 13) % (u64::from(size) / 8 - 1)) as f64;
        let (fx, fy) = (((state >> 3) % 8) as f64, ((state >> 7) % 8) as f64);
        let node = |a: f64, b: f64| map(Point::new(a * step + 0.5, b * step + 0.5)).unwrap();
        let (p00, p10, p01, p11) = (
            node(i, j),
            node(i + 1.0, j),
            node(i, j + 1.0),
            node(i + 1.0, j + 1.0),
        );
        let (u, v) = (fx / step, fy / step);
        let interpolated =
            (p00 * (1.0 - u) + p10 * u) * (1.0 - v) + (p01 * (1.0 - u) + p11 * u) * v;
        let exact = map(Point::new(i * step + fx + 0.5, j * step + fy + 0.5)).unwrap();
        worst = worst.max(interpolated.distance(exact));
    }
    eprintln!("écart maximal du treillis : {worst:.4} px");
    assert!(worst < 0.02, "{worst} px");
}

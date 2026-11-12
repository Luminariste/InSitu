//! Mesure des croisements de la grille, de proche en proche.
//!
//! Portage en Rust de `mesurer_noeuds`, `croissance` et `nettoyer`
//! (`etudes/2.1-detection/detecteurs.py`), avec les mêmes réglages.
//!
//! Pour chaque croisement, on connaît une position approchée dans la photo et
//! la dérivée du modèle (px par mm). On échantillonne un carré de 8 mm de côté
//! dans le repère de la mire, au dixième de mm ; la moyenne des lignes donne
//! le profil du trait horizontal, celle des colonnes le profil du trait
//! vertical. Dans chaque profil on cherche un creux net et unique, au
//! dixième d'échantillon près.

use std::collections::HashSet;

use super::legacy_target::{all_nodes, is_measurable, Node, GRID_LINES};
use super::local_model::{local_scale, robust_homography, LocalModel};
use super::raster::{Border, Plane};
use crate::geometry::{Homography, Jacobian, Point, Transform};

/// Demi-côté de la fenêtre de recherche, en mm.
const HALF_WINDOW_MM: f64 = 4.0;
/// Pas d'échantillonnage dans la fenêtre, en mm.
const SAMPLE_STEP_MM: f64 = 0.1;
/// Nombre d'échantillons par côté (81).
const SAMPLES: usize = 81;
/// Largeurs des filtres de creux, en échantillons.
const SCALES: [f64; 3] = [1.5, 3.0, 6.0];
const MIN_SNR: f64 = 5.0;
const MAX_SECOND_PEAK: f64 = 0.7;
/// Nombre minimal de croisements pour accepter une détection.
pub(crate) const MIN_NODES: usize = 40;

/// Un croisement mesuré.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Measured {
    pub node: Node,
    /// Position dans la photo, en px.
    pub photo: Point,
    /// Rapport signal sur bruit du moins net des deux traits.
    pub score: f64,
}

fn offset(k: f64) -> f64 {
    -HALF_WINDOW_MM + k * SAMPLE_STEP_MM
}

/// Filtres « différence de gaussiennes » : large moins étroite, positifs
/// sur un creux.
fn kernels() -> Vec<Vec<f64>> {
    SCALES
        .iter()
        .map(|&sigma| {
            let r = (3.0 * sigma * 3.0) as usize + 1;
            let g = |s: f64| -> Vec<f64> {
                let v: Vec<f64> = (0..=2 * r)
                    .map(|k| {
                        let x = k as f64 - r as f64;
                        (-x * x / (2.0 * s * s)).exp()
                    })
                    .collect();
                let sum: f64 = v.iter().sum();
                v.into_iter().map(|x| x / sum).collect()
            };
            let narrow = g(sigma);
            let wide = g(3.0 * sigma);
            wide.iter().zip(&narrow).map(|(w, n)| w - n).collect()
        })
        .collect()
}

fn median(values: &mut [f64]) -> f64 {
    let n = values.len();
    if n == 0 {
        return 0.0;
    }
    values.sort_unstable_by(f64::total_cmp);
    if n % 2 == 1 {
        values[n / 2]
    } else {
        0.5 * (values[n / 2 - 1] + values[n / 2])
    }
}

/// Indice miroir (sans répétition du bord), comme `numpy.pad(mode="reflect")`.
fn reflect(i: isize, n: usize) -> usize {
    let n = n as isize;
    let period = 2 * (n - 1);
    let mut k = i.rem_euclid(period);
    if k >= n {
        k = period - k;
    }
    k as usize
}

/// Cherche un trait sombre dans un profil. Renvoie le décalage en mm et le
/// rapport signal sur bruit, ou `None`.
fn line_in_profile(profile: &[f64], kernels: &[Vec<f64>]) -> Option<(f64, f64)> {
    let m = profile.len();
    let mean = profile.iter().sum::<f64>() / m as f64;
    let q: Vec<f64> = profile.iter().map(|v| v / (mean + 1e-6)).collect();
    let border = 5;
    let mut best: Option<(f64, f64)> = None;
    let mut response = vec![0.0; m];
    for kernel in kernels {
        let r = kernel.len() / 2;
        for (i, out) in response.iter_mut().enumerate() {
            *out = kernel
                .iter()
                .enumerate()
                .map(|(k, w)| w * q[reflect(i as isize + k as isize - r as isize, m)])
                .sum();
        }
        let (k, &peak) = response[border..m - border]
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, v)| (i + border, v))?;
        let mut sorted = response.clone();
        let med = median(&mut sorted);
        let mut dev: Vec<f64> = response.iter().map(|v| (v - med).abs()).collect();
        let noise = 1.4826 * median(&mut dev) + 1e-4;
        let second = (border..m - border)
            .filter(|&i| i.abs_diff(k) > 12)
            .map(|i| response[i])
            .fold(-1.0, f64::max);
        let a = response[k - 1];
        let c = response[(k + 1).min(m - 1)];
        let denom = a - 2.0 * peak + c;
        let fine = if denom.abs() > 1e-9 {
            0.5 * (a - c) / denom
        } else {
            0.0
        };
        let snr = peak / noise;
        let valid = snr > MIN_SNR && second < MAX_SECOND_PEAK * peak && peak > 0.004;
        if valid && best.is_none_or(|(_, s)| snr > s) {
            best = Some((offset(k as f64 + fine.clamp(-1.0, 1.0)), snr));
        }
    }
    best
}

/// Mesure le croisement le plus proche de `centre` (px), sachant la dérivée
/// du modèle `jacobian` (px par mm). Renvoie la position mesurée et le score.
pub(crate) fn measure_node(
    gray: &Plane,
    centre: Point,
    jacobian: &Jacobian,
    kernels: &[Vec<f64>],
) -> Option<(Point, f64)> {
    let (w, h) = (gray.width as f64, gray.height as f64);
    if !(centre.x > 4.5 && centre.x < w - 4.5 && centre.y > 4.5 && centre.y < h - 4.5) {
        return None;
    }
    let mut rows = [0.0f64; SAMPLES];
    let mut cols = [0.0f64; SAMPLES];
    for (r, row_sum) in rows.iter_mut().enumerate() {
        let dy = offset(r as f64);
        for (c, col_sum) in cols.iter_mut().enumerate() {
            let dx = offset(c as f64);
            let x = centre.x + jacobian[0][0] * dx + jacobian[0][1] * dy;
            let y = centre.y + jacobian[1][0] * dx + jacobian[1][1] * dy;
            let v = f64::from(gray.sample(x, y, Border::Replicate));
            *row_sum += v;
            *col_sum += v;
        }
    }
    let n = SAMPLES as f64;
    let rows: Vec<f64> = rows.iter().map(|v| v / n).collect();
    let cols: Vec<f64> = cols.iter().map(|v| v / n).collect();
    let (oy, sy) = line_in_profile(&rows, kernels)?; // trait horizontal
    let (ox, sx) = line_in_profile(&cols, kernels)?; // trait vertical
    if ox.abs().max(oy.abs()) >= HALF_WINDOW_MM - 0.4 {
        return None;
    }
    let p = Point::new(
        centre.x + jacobian[0][0] * ox + jacobian[0][1] * oy,
        centre.y + jacobian[1][0] * ox + jacobian[1][1] * oy,
    );
    Some((p, sx.min(sy)))
}

/// Mesure les croisements de proche en proche.
///
/// - `support` : appuis de départ (mm, px), par exemple des coins de repères ;
/// - `seed` : à défaut, un trapèze approché et les croisements par lesquels
///   commencer.
pub(crate) fn grow(
    gray: &Plane,
    support: &[(Point, Point)],
    seed: Option<(&Homography, &dyn Fn(Node) -> bool)>,
) -> Vec<Measured> {
    let kernels = kernels();
    let nodes: Vec<Node> = all_nodes().collect();
    let mut accepted = vec![false; GRID_LINES * GRID_LINES];
    let mut attempts = vec![0u8; GRID_LINES * GRID_LINES];
    let mut sup_mm: Vec<Point> = support.iter().map(|s| s.0).collect();
    let mut sup_px: Vec<Point> = support.iter().map(|s| s.1).collect();
    let mut found: Vec<Measured> = Vec::new();
    let mut reach = 15.0;
    let mut first = seed.is_some();
    loop {
        let free = |n: &Node| is_measurable(*n) && !accepted[n.index()] && attempts[n.index()] < 2;
        let (candidates, model): (Vec<Node>, LocalModel) = if first {
            let (h, chosen) = seed.expect("amorce présente");
            (
                nodes
                    .iter()
                    .copied()
                    .filter(|n| free(n) && chosen(*n))
                    .collect(),
                LocalModel::plain(*h),
            )
        } else {
            if sup_mm.len() < 4 {
                break;
            }
            let robust = (sup_mm.len() >= 16).then_some(40.0);
            let Some(model) = LocalModel::fit(&sup_mm, &sup_px, 15.0, robust) else {
                break;
            };
            let near = |n: &Node| {
                let p = n.position();
                sup_mm.iter().any(|s| s.distance(p) <= reach)
            };
            (
                nodes
                    .iter()
                    .copied()
                    .filter(|n| free(n) && near(n))
                    .collect(),
                model,
            )
        };
        let mut added = 0;
        for node in candidates {
            attempts[node.index()] += 1;
            let Some((centre, jac)) = model.predict_with_jacobian(node.position()) else {
                continue;
            };
            if let Some((photo, score)) = measure_node(gray, centre, &jac, &kernels) {
                accepted[node.index()] = true;
                sup_mm.push(node.position());
                sup_px.push(photo);
                found.push(Measured { node, photo, score });
                added += 1;
            }
        }
        if first {
            first = false;
            if added < 4 {
                break;
            }
            continue;
        }
        if added == 0 {
            if reach >= 35.0 {
                break;
            }
            reach += 10.0;
        } else {
            reach = 15.0;
        }
    }
    found
}

/// Écarte les croisements incohérents : loin du trapèze d'ensemble, ou en
/// désaccord avec leurs voisins. Renvoie les croisements gardés.
pub(crate) fn clean(found: &[Measured]) -> Vec<Measured> {
    if found.len() < 8 {
        return found.to_vec();
    }
    let mm: Vec<Point> = found.iter().map(|m| m.node.position()).collect();
    let px: Vec<Point> = found.iter().map(|m| m.photo).collect();
    let Some(h0) = robust_homography(&mm, &px, 30.0) else {
        return Vec::new();
    };
    let scale = local_scale(&h0, Point::new(140.0, 140.0));
    let Some(h) = robust_homography(&mm, &px, 3.0 * scale) else {
        return Vec::new();
    };
    let mut keep: Vec<bool> = mm
        .iter()
        .zip(&px)
        .map(|(m, p)| h.apply(*m).is_some_and(|q| q.distance(*p) < 6.0 * scale))
        .collect();
    for _ in 0..2 {
        let kept: Vec<usize> = (0..mm.len()).filter(|&k| keep[k]).collect();
        if kept.len() < 8 {
            break;
        }
        let kmm: Vec<Point> = kept.iter().map(|&k| mm[k]).collect();
        let kpx: Vec<Point> = kept.iter().map(|&k| px[k]).collect();
        let Some(model) = LocalModel::fit(&kmm, &kpx, 12.0, None) else {
            break;
        };
        let errors: Vec<f64> = mm
            .iter()
            .zip(&px)
            .map(|(m, p)| model.predict(*m).map_or(f64::INFINITY, |q| q.distance(*p)))
            .collect();
        let mut kept_errors: Vec<f64> = kept.iter().map(|&k| errors[k]).collect();
        let threshold = (0.35 * scale).max(4.0 * 1.4826 * median(&mut kept_errors));
        for (k, e) in errors.iter().enumerate() {
            keep[k] = keep[k] && *e < threshold;
        }
    }
    found
        .iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(m, _)| *m)
        .collect()
}

/// Grandit puis nettoie ; `None` (avec la raison) si trop peu de
/// croisements restent.
pub(crate) fn grow_and_clean(
    gray: &Plane,
    support: &[(Point, Point)],
    seed: Option<(&Homography, &dyn Fn(Node) -> bool)>,
) -> Result<(Vec<Measured>, usize), String> {
    let raw = grow(gray, support, seed);
    if raw.len() < MIN_NODES {
        return Err(format!(
            "seulement {} croisements de la grille mesurés (il en faut au moins {MIN_NODES})",
            raw.len()
        ));
    }
    let kept = clean(&raw);
    if kept.len() < MIN_NODES {
        return Err(format!(
            "les croisements mesurés ne sont pas cohérents entre eux ({} gardés sur {})",
            kept.len(),
            raw.len()
        ));
    }
    // Un même croisement ne peut pas apparaître deux fois.
    debug_assert_eq!(
        kept.iter().map(|m| m.node).collect::<HashSet<_>>().len(),
        kept.len()
    );
    Ok((kept, raw.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflect_matches_numpy() {
        // numpy.pad([0, 1, 2, 3], 2, mode="reflect") = [2, 1, 0, 1, 2, 3, 2, 1]
        let got: Vec<usize> = (-2..6).map(|i| reflect(i, 4)).collect();
        assert_eq!(got, vec![2, 1, 0, 1, 2, 3, 2, 1]);
    }

    /// Une grille synthétique : traits sombres de 0,25 mm tous les 10 mm,
    /// vue par un trapèze connu à 6,7 px par mm.
    #[test]
    fn a_synthetic_crossing_is_measured_to_a_hundredth_of_a_pixel() {
        let target = Point::new(120.0, 90.0);
        let h0 = Homography::similarity(6.7, 0.2, 0.0, 0.0).unwrap();
        let t = crate::geometry::Transform::apply(&h0, target).unwrap();
        let h = Homography::similarity(6.7, 0.2, 200.0 - t.x, 200.0 - t.y).unwrap();
        let inv = h.inverse().unwrap();
        let (w, hh) = (400usize, 400usize);
        let mut img = Plane::new(w, hh);
        for j in 0..hh {
            for i in 0..w {
                let mut acc = 0.0;
                // Suréchantillonnage 4 × 4 du pixel.
                for sj in 0..4 {
                    for si in 0..4 {
                        let p = Point::new(
                            i as f64 + (si as f64 + 0.5) / 4.0,
                            j as f64 + (sj as f64 + 0.5) / 4.0,
                        );
                        let m = crate::geometry::Transform::apply(&inv, p).unwrap();
                        let dx = (m.x / 10.0 - (m.x / 10.0).round()).abs() * 10.0;
                        let dy = (m.y / 10.0 - (m.y / 10.0).round()).abs() * 10.0;
                        acc += if dx < 0.125 || dy < 0.125 {
                            80.0
                        } else {
                            200.0
                        };
                    }
                }
                img.data[j * w + i] = acc / 16.0;
            }
        }
        let kernels = kernels();
        let truth = crate::geometry::Transform::apply(&h, target).unwrap();
        let (_, jac) = crate::geometry::Differentiable::apply_with_jacobian(&h, target).unwrap();
        let guess = truth + Point::new(5.0, -4.0);
        let (p, score) = measure_node(&img, guess, &jac, &kernels).unwrap();
        assert!(p.distance(truth) < 0.05, "écart {} px", p.distance(truth));
        assert!(score > MIN_SNR);
    }
}

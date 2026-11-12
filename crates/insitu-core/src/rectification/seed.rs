//! Amorce de l'ancienne mire : où est-elle dans la photo, et dans quel sens ?
//!
//! Portage en Rust de `amorce_actuelle` (`etudes/2.1-detection/detecteurs.py`) :
//!
//! 1. le **cercle rouge** (60 mm de rayon) est cherché à demi-résolution : on
//!    garde les pixels nettement rouges, puis on y ajuste une ellipse par
//!    tirages au sort (les arcs de pierre ocre de la façade en font aussi) ;
//! 2. la **croix épaisse** donne deux droites ; leurs quatre points de
//!    rencontre avec l'ellipse sont les images de points connus de la mire
//!    (à 60 mm du centre), d'où un premier trapèze ;
//! 3. la grille est mesurée à partir de là (module `nodes`) ;
//! 4. les **six petits cercles** de la diagonale disent où est le bas : on
//!    essaie les quatre quarts de tour et on garde celui où ils se voient.

use super::legacy_target::{
    rotate_quarters, small_circle_centres, Node, CENTRE_MM, RED_CIRCLE_RADIUS_MM,
    SMALL_CIRCLE_RADIUS_MM,
};
use super::local_model::{solve_small, Lcg, LocalModel};
use super::nodes::{grow_and_clean, Measured};
use super::raster::{Border, ColorPlanes, Plane};
use crate::geometry::{Homography, Point};

/// Ellipse : centre, demi-axes `a` (dans la direction `angle`) et `b`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ellipse {
    pub centre: Point,
    pub a: f64,
    pub b: f64,
    /// Direction du demi-axe `a`, en radians.
    pub angle: f64,
}

impl Ellipse {
    fn axes(&self) -> (Point, Point) {
        let (s, c) = self.angle.sin_cos();
        (Point::new(c, s), Point::new(-s, c))
    }

    /// Coordonnées réduites : le point est sur l'ellipse quand leur norme vaut 1.
    fn reduced(&self, p: Point) -> Point {
        let (ea, eb) = self.axes();
        let d = p - self.centre;
        Point::new(
            (d.x * ea.x + d.y * ea.y) / self.a,
            (d.x * eb.x + d.y * eb.y) / self.b,
        )
    }
}

/// Ajuste une ellipse aux moindres carrés (conique `Ax² + Bxy + Cy² + Dx +
/// Ey + F = 0` avec `A + C = 1`, en coordonnées normalisées).
pub(crate) fn fit_ellipse(points: &[Point]) -> Option<Ellipse> {
    if points.len() < 5 {
        return None;
    }
    let n = points.len() as f64;
    let mean = points.iter().fold(Point::new(0.0, 0.0), |a, p| a + *p) * (1.0 / n);
    let spread = points.iter().map(|p| p.distance(mean)).sum::<f64>() / n;
    if spread.is_nan() || spread <= 0.0 {
        return None;
    }
    let s = 1.0 / spread;
    let mut ata = [[0.0f64; 5]; 5];
    let mut atb = [0.0f64; 5];
    for p in points {
        let x = (p.x - mean.x) * s;
        let y = (p.y - mean.y) * s;
        let row = [x * x - y * y, x * y, x, y, 1.0];
        let rhs = -y * y;
        for i in 0..5 {
            for j in 0..5 {
                ata[i][j] += row[i] * row[j];
            }
            atb[i] += row[i] * rhs;
        }
    }
    let [a, b, d, e, f] = solve_small(ata, atb)?;
    let c = 1.0 - a;
    let det = a * c - b * b / 4.0;
    if det.is_nan() || det <= 0.0 {
        return None;
    }
    let [cx, cy] = solve_small([[2.0 * a, b], [b, 2.0 * c]], [-d, -e])?;
    let k = a * cx * cx + b * cx * cy + c * cy * cy + d * cx + e * cy + f;
    // Valeurs propres de [[A, B/2], [B/2, C]] (trace 1).
    let root = ((a - c) * (a - c) + b * b).sqrt();
    let l1 = 0.5 * (a + c + root);
    let l2 = 0.5 * (a + c - root);
    if !(l2 > 0.0 && -k > 0.0) {
        return None;
    }
    // Direction propre de l1.
    let angle = 0.5 * b.atan2(a - c);
    let (sa, ca) = angle.sin_cos();
    let q = a * ca * ca + b * ca * sa + c * sa * sa;
    // `angle` est la direction de la plus grande valeur propre : celle du
    // plus petit demi-axe.
    let (lambda_along, lambda_across) = if (q - l1).abs() <= (q - l2).abs() {
        (l1, l2)
    } else {
        (l2, l1)
    };
    let axis_along = (-k / lambda_along).sqrt() / s;
    let axis_across = (-k / lambda_across).sqrt() / s;
    let centre = Point::new(cx / s + mean.x, cy / s + mean.y);
    let ellipse = Ellipse {
        centre,
        a: axis_along,
        b: axis_across,
        angle,
    };
    (ellipse.a.is_finite() && ellipse.b.is_finite() && centre.is_finite()).then_some(ellipse)
}

/// Le cercle rouge, en px pleine résolution, et le nombre de secteurs de 10°
/// où il a été vu (sur 36).
pub(crate) fn find_red_circle(color: &ColorPlanes) -> Option<(Ellipse, usize)> {
    let half = color.half().gaussian_blur(1.3);
    let (w, h) = (half.red.width, half.red.height);
    let mut redness = Plane::new(w, h);
    for k in 0..w * h {
        let (r, g, b) = (half.red.data[k], half.green.data[k], half.blue.data[k]);
        let sum = r + g + b;
        redness.data[k] = (r - 0.5 * (g + b)) / (sum + 120.0) * (sum / 150.0).min(1.0);
    }
    let relief = redness.white_top_hat(6);
    let mut sub: Vec<f32> = (0..h)
        .step_by(2)
        .flat_map(|j| (0..w).step_by(2).map(move |i| (i, j)))
        .map(|(i, j)| relief.at(i, j))
        .collect();
    let rank = ((sub.len() as f64 - 1.0) * 0.9975).round() as usize;
    let (_, q, _) = sub.select_nth_unstable_by(rank, f32::total_cmp);
    let threshold = f64::from(*q).max(0.03);
    let mut points: Vec<Point> = Vec::new();
    for j in 0..h {
        for i in 0..w {
            if f64::from(relief.at(i, j)) > threshold {
                points.push(Point::new(i as f64 + 0.5, j as f64 + 0.5));
            }
        }
    }
    if points.len() < 80 {
        return None;
    }
    let mut rng = Lcg::new(0);
    if points.len() > 4000 {
        // Tirage sans remise de 4 000 points (mélange partiel).
        for k in 0..4000 {
            let pick = k + rng.below(points.len() - k);
            points.swap(k, pick);
        }
        points.truncate(4000);
    }
    let width = w as f64;
    let inliers_of = |e: &Ellipse| -> (Vec<bool>, usize) {
        let mut sectors = [false; 36];
        let inside: Vec<bool> = points
            .iter()
            .map(|p| {
                let r = e.reduced(*p);
                let ok = (r.norm() - 1.0).abs() * e.a.min(e.b) < 2.0;
                if ok {
                    let deg = r.y.atan2(r.x).to_degrees();
                    let bin = ((deg / 10.0).floor() as i64).rem_euclid(36) as usize;
                    sectors[bin] = true;
                }
                ok
            })
            .collect();
        (inside, sectors.iter().filter(|s| **s).count())
    };
    let mut best: Option<(Vec<bool>, usize)> = None;
    let mut best_note = 0usize;
    for _ in 0..400 {
        let mut sample = [Point::new(0.0, 0.0); 6];
        for s in &mut sample {
            *s = points[rng.below(points.len())];
        }
        let Some(e) = fit_ellipse(&sample) else {
            continue;
        };
        let (small, large) = (e.a.min(e.b), e.a.max(e.b));
        if !(0.04 * width < small && large < 0.45 * width && small / large > 0.55) {
            continue;
        }
        let (inside, sectors) = inliers_of(&e);
        let count = inside.iter().filter(|v| **v).count();
        let note = sectors * 1000 + count;
        if sectors >= 18 && count as f64 > 0.2 * points.len() as f64 && note > best_note {
            best_note = note;
            best = Some((inside, sectors));
        }
    }
    let (inside, sectors) = best?;
    let chosen: Vec<Point> = points
        .iter()
        .zip(&inside)
        .filter(|(_, k)| **k)
        .map(|(p, _)| *p)
        .collect();
    let e = fit_ellipse(&chosen)?;
    // Demi-résolution → pleine résolution (coordonnées continues).
    Some((
        Ellipse {
            centre: e.centre * 2.0,
            a: e.a * 2.0,
            b: e.b * 2.0,
            angle: e.angle,
        },
        sectors,
    ))
}

/// Droite ajustée par moindres carrés repondérés (poids de Huber).
fn fit_line(points: &[Point]) -> Option<(Point, Point)> {
    let mut weights = vec![1.0f64; points.len()];
    let mut line = None;
    for _ in 0..10 {
        let wsum: f64 = weights.iter().sum();
        let c = points
            .iter()
            .zip(&weights)
            .fold(Point::new(0.0, 0.0), |a, (p, w)| a + *p * *w)
            * (1.0 / wsum);
        let (mut sxx, mut sxy, mut syy) = (0.0, 0.0, 0.0);
        for (p, w) in points.iter().zip(&weights) {
            let d = *p - c;
            sxx += w * d.x * d.x;
            sxy += w * d.x * d.y;
            syy += w * d.y * d.y;
        }
        let angle = 0.5 * (2.0 * sxy).atan2(sxx - syy);
        let dir = Point::new(angle.cos(), angle.sin());
        for (p, w) in points.iter().zip(weights.iter_mut()) {
            let d = *p - c;
            let r = (d.x * dir.y - d.y * dir.x).abs();
            *w = if r < 1.345 { 1.0 } else { 1.345 / r };
        }
        line = Some((c, dir));
    }
    line
}

/// Ce que l'amorce a trouvé.
pub(crate) struct LegacySeed {
    /// Croisements mesurés, dans le repère de la mire **avant** orientation.
    pub nodes: Vec<Measured>,
    pub raw_count: usize,
    pub circle_sectors: usize,
}

/// Cercle rouge, croix, puis grille. L'orientation reste à trouver.
pub(crate) fn seed_from_circle_and_cross(
    color: &ColorPlanes,
    gray: &Plane,
) -> Result<LegacySeed, String> {
    let (ellipse, sectors) = find_red_circle(color).ok_or("cercle rouge introuvable")?;
    let (ea, eb) = ellipse.axes();
    // Repère du cercle (mm, sens inconnu) → px.
    let k = 1.0 / RED_CIRCLE_RADIUS_MM;
    let p = [
        [ea.x * ellipse.a * k, eb.x * ellipse.b * k],
        [ea.y * ellipse.a * k, eb.y * ellipse.b * k],
    ];
    let to_px = |q: Point| {
        ellipse.centre + Point::new(p[0][0] * q.x + p[0][1] * q.y, p[1][0] * q.x + p[1][1] * q.y)
    };
    let det = p[0][0] * p[1][1] - p[0][1] * p[1][0];
    let to_circle = |v: Point| {
        Point::new(
            (p[1][1] * v.x - p[0][1] * v.y) / det,
            (p[0][0] * v.y - p[1][0] * v.x) / det,
        )
    };

    // Relief des traits sombres épais, sur la zone qui entoure le cercle.
    let reach = 2.4 * ellipse.a.max(ellipse.b);
    let x0 = (ellipse.centre.x - reach).floor().max(0.0) as usize;
    let y0 = (ellipse.centre.y - reach).floor().max(0.0) as usize;
    let x1 = ((ellipse.centre.x + reach).ceil() as usize).min(gray.width);
    let y1 = ((ellipse.centre.y + reach).ceil() as usize).min(gray.height);
    if x1 <= x0 + 30 || y1 <= y0 + 30 {
        return Err("cercle rouge au bord de la photo".into());
    }
    let mut crop = Plane::new(x1 - x0, y1 - y0);
    for j in y0..y1 {
        crop.data[(j - y0) * crop.width..(j - y0 + 1) * crop.width]
            .copy_from_slice(&gray.data[j * gray.width + x0..j * gray.width + x1]);
    }
    let relief = crop.black_top_hat(10).gaussian_blur(2.5);
    let origin = Point::new(x0 as f64, y0 as f64);
    let read = |q: Point| f64::from(relief.sample(q.x - origin.x, q.y - origin.y, Border::Zero));
    let direction = |t: f64| Point::new(t.cos(), t.sin());

    // Angle de la croix, à un quart de tour près.
    let mut best_angle = 0.0;
    let mut best_note = f64::MIN;
    for step in 0..180 {
        let t = (step as f64 * 0.5).to_radians();
        let mut total = 0.0;
        for arm in 0..4 {
            let u = direction(t + arm as f64 * std::f64::consts::FRAC_PI_2);
            for r in 66..126 {
                total += read(to_px(u * r as f64));
            }
        }
        if total > best_note {
            best_note = total;
            best_angle = t;
        }
    }

    // Points sur chaque bras, puis deux droites.
    let mut lines = Vec::new();
    for k in 0..2 {
        let mut on_line = Vec::new();
        for arm in [k, k + 2] {
            let u = direction(best_angle + arm as f64 * std::f64::consts::FRAC_PI_2);
            let normal = Point::new(-u.y, u.x);
            let mut r = 66.0;
            while r < 132.0 {
                let samples: Vec<(Point, f64)> = (0..121)
                    .map(|s| {
                        let q = to_px(u * r + normal * (-6.0 + 0.1 * s as f64));
                        (q, read(q))
                    })
                    .collect();
                let (at, peak) = samples
                    .iter()
                    .copied()
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .expect("profil non vide");
                let mut values: Vec<f64> = samples.iter().map(|s| s.1).collect();
                values.sort_unstable_by(f64::total_cmp);
                let median = values[values.len() / 2];
                if peak > 2.5 * (median + 0.5) {
                    on_line.push(at);
                }
                r += 3.0;
            }
        }
        if on_line.len() < 5 {
            return Err("croix centrale introuvable".into());
        }
        lines.push(fit_line(&on_line).ok_or("croix centrale introuvable")?);
    }

    // Rencontre de chaque droite avec l'ellipse : points à ±60 mm du centre.
    let mut corners_mm = Vec::new();
    let mut corners_px = Vec::new();
    for (k, (p0, v)) in lines.iter().enumerate() {
        let q0 = to_circle(*p0 - ellipse.centre);
        let qv = to_circle(*v);
        let a = qv.x * qv.x + qv.y * qv.y;
        let b = 2.0 * (q0.x * qv.x + q0.y * qv.y);
        let c = q0.x * q0.x + q0.y * q0.y - RED_CIRCLE_RADIUS_MM * RED_CIRCLE_RADIUS_MM;
        let disc = b * b - 4.0 * a * c;
        if disc.is_nan() || disc <= 0.0 {
            return Err("la croix ne traverse pas le cercle rouge".into());
        }
        let u = direction(best_angle + k as f64 * std::f64::consts::FRAC_PI_2);
        for s in [
            (-b + disc.sqrt()) / (2.0 * a),
            (-b - disc.sqrt()) / (2.0 * a),
        ] {
            let q = q0 + qv * s;
            let sign = if q.x * u.x + q.y * u.y > 0.0 {
                1.0
            } else {
                -1.0
            };
            corners_px.push(to_px(q));
            let axis = if k == 0 {
                Point::new(1.0, 0.0)
            } else {
                Point::new(0.0, 1.0)
            };
            corners_mm
                .push(Point::new(CENTRE_MM, CENTRE_MM) + axis * (sign * RED_CIRCLE_RADIUS_MM));
        }
    }
    let h0 = Homography::from_four_pairs(
        [corners_mm[0], corners_mm[1], corners_mm[2], corners_mm[3]],
        [corners_px[0], corners_px[1], corners_px[2], corners_px[3]],
    )
    .map_err(|_| "la croix et le cercle rouge ne forment pas un trapèze cohérent".to_string())?;
    let near_centre = |n: Node| n.distance_to_centre() < 82.0;
    let (nodes, raw_count) = grow_and_clean(gray, &[], Some((&h0, &near_centre)))?;
    Ok(LegacySeed {
        nodes,
        raw_count,
        circle_sectors: sectors,
    })
}

/// Résultat de la recherche du sens de la mire.
pub(crate) struct Orientation {
    /// Quarts de tour à appliquer aux coordonnées vraies pour obtenir celles
    /// de l'amorce.
    pub quarters: u8,
    /// Écart entre la meilleure note et la suivante.
    pub margin: f64,
}

/// En dessous de cet écart de notes, le sens de la mire est jugé incertain.
pub(crate) const MIN_ORIENTATION_MARGIN: f64 = 0.04;

/// Où sont les six petits cercles ? `nodes` : croisements dans le repère de
/// l'amorce.
pub(crate) fn find_orientation(gray: &Plane, nodes: &[Measured]) -> Option<Orientation> {
    let mm: Vec<Point> = nodes.iter().map(|m| m.node.position()).collect();
    let px: Vec<Point> = nodes.iter().map(|m| m.photo).collect();
    let model = LocalModel::fit(&mm, &px, 12.0, None)?;
    let ring: Vec<f64> = [20, 30, 40, 50, 60, 70]
        .iter()
        .flat_map(|d| [0, 90, 180, 270].map(|q| f64::from(*d + q)))
        .collect();
    let diagonals = [45.0f64, 135.0, 225.0, 315.0];
    let mean_at = |points: &[Point], quarters: u8| -> f64 {
        let values: Vec<f64> = points
            .iter()
            .map(|p| {
                model
                    .predict(rotate_quarters(*p, quarters))
                    .map_or(0.0, |q| f64::from(gray.sample(q.x, q.y, Border::Zero)))
            })
            .collect();
        values.iter().sum::<f64>() / values.len() as f64
    };
    let mut notes = [0.0; 4];
    for (quarters, note) in notes.iter_mut().enumerate() {
        let mut per_circle: Vec<f64> = small_circle_centres()
            .iter()
            .map(|c| {
                let around = |r: f64, angles: &[f64]| -> Vec<Point> {
                    angles
                        .iter()
                        .map(|a| *c + Point::new(a.to_radians().cos(), a.to_radians().sin()) * r)
                        .collect()
                };
                let edge = around(SMALL_CIRCLE_RADIUS_MM, &ring);
                let mut witness = around(SMALL_CIRCLE_RADIUS_MM - 1.6, &ring);
                witness.extend(around(SMALL_CIRCLE_RADIUS_MM + 1.4, &diagonals));
                let vb = mean_at(&edge, quarters as u8);
                let vt = mean_at(&witness, quarters as u8);
                (vt - vb) / (vt + 1.0)
            })
            .collect();
        per_circle.sort_unstable_by(f64::total_cmp);
        *note = 0.5 * (per_circle[2] + per_circle[3]);
    }
    let mut order = [0usize, 1, 2, 3];
    order.sort_by(|a, b| notes[*b].total_cmp(&notes[*a]));
    Some(Orientation {
        quarters: order[0] as u8,
        margin: notes[order[0]] - notes[order[1]],
    })
}

/// Écart minimal entre le meilleur sens et le suivant, lu sur les repères
/// codés de la mire version 2 (part des modules de données conformes).
pub(crate) const MIN_MARKER_MARGIN: f64 = 0.15;

/// Sens de la mire version 2, lu sur ses seize repères codés (D-essai-a) :
/// pour chaque quart de tour, on compare la teinte de chaque module de
/// données au dessin attendu. Le bon sens donne presque tout juste, les
/// trois autres environ une moitié. `nodes` : croisements dans le repère de
/// l'amorce, comme pour [`find_orientation`].
pub(crate) fn find_orientation_by_markers(gray: &Plane, nodes: &[Measured]) -> Option<Orientation> {
    use crate::target::apriltag::{marker_modules, DATA_MODULES, MARKER_MODULES};
    use crate::target::{TargetLayout, MARKER_SIZE_MM};
    let mm: Vec<Point> = nodes.iter().map(|m| m.node.position()).collect();
    let px: Vec<Point> = nodes.iter().map(|m| m.photo).collect();
    let model = LocalModel::fit(&mm, &px, 12.0, None)?;
    let module = MARKER_SIZE_MM / MARKER_MODULES as f64;
    let layout = TargetLayout::coded();
    // Teinte moyenne autour d'un point de la mire (5 échantillons dans le
    // module), dans le sens `quarters`.
    let value = |p: Point, quarters: u8| -> Option<f64> {
        let d = module * 0.25;
        let offsets = [(0.0, 0.0), (d, 0.0), (-d, 0.0), (0.0, d), (0.0, -d)];
        let mut sum = 0.0;
        for (dx, dy) in offsets {
            let q = model.predict(rotate_quarters(p + Point::new(dx, dy), quarters))?;
            sum += f64::from(gray.sample(q.x, q.y, Border::Zero));
        }
        Some(sum / offsets.len() as f64)
    };
    let mut notes = [0.0; 4];
    for (quarters, note) in notes.iter_mut().enumerate() {
        let mut per_marker = Vec::with_capacity(layout.markers.len());
        for marker in &layout.markers {
            let expected = marker_modules(marker.id);
            let origin = marker.corners[0];
            let mut samples = Vec::with_capacity(MARKER_MODULES * MARKER_MODULES);
            for (row, line) in expected.iter().enumerate() {
                for (col, black) in line.iter().enumerate() {
                    let centre = origin
                        + Point::new((col as f64 + 0.5) * module, (row as f64 + 0.5) * module);
                    if let Some(v) = value(centre, quarters as u8) {
                        samples.push((row, col, *black, v));
                    }
                }
            }
            if samples.len() < MARKER_MODULES * MARKER_MODULES {
                continue;
            }
            // Seuil propre au repère : milieu entre le pourtour (noir) et la
            // zone blanche juste autour.
            let border: Vec<f64> = samples
                .iter()
                .filter(|s| {
                    s.0 == 0 || s.1 == 0 || s.0 == MARKER_MODULES - 1 || s.1 == MARKER_MODULES - 1
                })
                .map(|s| s.3)
                .collect();
            let dark = border.iter().sum::<f64>() / border.len() as f64;
            let data: Vec<&(usize, usize, bool, f64)> = samples
                .iter()
                .filter(|s| s.0 >= 1 && s.1 >= 1 && s.0 <= DATA_MODULES && s.1 <= DATA_MODULES)
                .collect();
            let light = data.iter().map(|s| s.3).fold(f64::MIN, f64::max);
            if light - dark < 10.0 {
                continue;
            }
            let threshold = 0.5 * (dark + light);
            let right = data.iter().filter(|s| (s.3 < threshold) == s.2).count();
            per_marker.push(right as f64 / data.len() as f64);
        }
        if per_marker.len() < 4 {
            return None;
        }
        // Les repères les mieux vus comptent : la moitié haute.
        per_marker.sort_unstable_by(|a, b| b.total_cmp(a));
        let keep = per_marker.len().div_ceil(2);
        *note = per_marker[..keep].iter().sum::<f64>() / keep as f64;
    }
    let mut order = [0usize, 1, 2, 3];
    order.sort_by(|a, b| notes[*b].total_cmp(&notes[*a]));
    Some(Orientation {
        quarters: order[0] as u8,
        margin: notes[order[0]] - notes[order[1]],
    })
}

/// Sens de la mire : par les six petits cercles (ancienne mire), sinon par
/// les repères codés (mire version 2). `None` si aucun des deux n'est sûr.
pub(crate) fn find_orientation_any(gray: &Plane, nodes: &[Measured]) -> Option<Orientation> {
    if let Some(o) = find_orientation(gray, nodes) {
        if o.margin >= MIN_ORIENTATION_MARGIN {
            return Some(o);
        }
    }
    find_orientation_by_markers(gray, nodes).filter(|o| o.margin >= MIN_MARKER_MARGIN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ellipse_is_recovered_from_its_points() {
        let truth = Ellipse {
            centre: Point::new(812.0, 431.0),
            a: 210.0,
            b: 160.0,
            angle: 0.4,
        };
        let (ea, eb) = truth.axes();
        let points: Vec<Point> = (0..40)
            .map(|k| {
                let t = k as f64 * 0.157;
                truth.centre + ea * (truth.a * t.cos()) + eb * (truth.b * t.sin())
            })
            .collect();
        let e = fit_ellipse(&points).unwrap();
        assert!(e.centre.distance(truth.centre) < 1e-6);
        let (big, small) = (e.a.max(e.b), e.a.min(e.b));
        assert!(
            (big - 210.0).abs() < 1e-6 && (small - 160.0).abs() < 1e-6,
            "{e:?}"
        );
        // Le demi-axe `a` est bien dans la direction `angle`.
        let along = if (e.a - 210.0).abs() < 1e-3 {
            e.angle
        } else {
            e.angle + std::f64::consts::FRAC_PI_2
        };
        let d = (along - 0.4).rem_euclid(std::f64::consts::PI);
        assert!(
            d < 1e-6 || (std::f64::consts::PI - d) < 1e-6,
            "angle {}",
            e.angle
        );
    }

    #[test]
    fn a_line_ignores_a_stray_point() {
        let mut pts: Vec<Point> = (0..20)
            .map(|k| Point::new(k as f64 * 3.0, 10.0 + k as f64 * 1.5))
            .collect();
        pts.push(Point::new(30.0, 80.0));
        let (c, d) = fit_line(&pts).unwrap();
        let slope = d.y / d.x;
        assert!((slope - 0.5).abs() < 0.01, "pente {slope}");
        assert!((c.y - (10.0 + c.x * 0.5)).abs() < 1.0);
    }
}

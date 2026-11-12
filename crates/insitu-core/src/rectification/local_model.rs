//! Modèle provisoire mm de mire → px de photo, utilisé pendant la recherche
//! des croisements : un trapèze, plus une correction locale par régression
//! affine pondérée des écarts (poids gaussiens).
//!
//! Il sert à prédire où chercher le prochain croisement et à écarter les
//! mesures incohérentes. Il lisse (il ne passe pas par les points), ce qui
//! est voulu ici. Le modèle final, celui qui redresse l'image, est la
//! déformation du module `geometry` (trapèze + plaque mince).

use crate::geometry::{Differentiable, Homography, Jacobian, Point, Transform};

/// Résout un petit système linéaire dense par élimination de Gauss avec
/// pivot partiel. `None` si la matrice est singulière.
#[allow(clippy::needless_range_loop)]
pub(crate) fn solve_small<const N: usize>(
    mut a: [[f64; N]; N],
    mut b: [f64; N],
) -> Option<[f64; N]> {
    for k in 0..N {
        let pivot = (k..N).max_by(|&x, &y| a[x][k].abs().total_cmp(&a[y][k].abs()))?;
        if !a[pivot][k].is_finite() || a[pivot][k].abs() <= 1e-300 {
            return None;
        }
        a.swap(k, pivot);
        b.swap(k, pivot);
        for i in k + 1..N {
            let f = a[i][k] / a[k][k];
            for j in k..N {
                a[i][j] -= f * a[k][j];
            }
            b[i] -= f * b[k];
        }
    }
    let mut x = [0.0; N];
    for k in (0..N).rev() {
        let s: f64 = (k + 1..N).map(|j| a[k][j] * x[j]).sum();
        x[k] = (b[k] - s) / a[k][k];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

/// Tirage pseudo-aléatoire reproductible (générateur congruentiel).
pub(crate) struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn below(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % n.max(1)
    }
}

/// Trapèze robuste : tirages de 4 paires, on garde le trapèze qui explique
/// le plus de paires à moins de `threshold` (unité de la cible), puis on le
/// réajuste sur elles. Avec moins de 8 paires, simple ajustement.
pub(crate) fn robust_homography(
    source: &[Point],
    target: &[Point],
    threshold: f64,
) -> Option<Homography> {
    let n = source.len();
    if n < 4 {
        return None;
    }
    if n < 8 {
        return Homography::fit(source, target).ok();
    }
    let inliers_of = |h: &Homography| -> Vec<usize> {
        (0..n)
            .filter(|&k| {
                h.apply(source[k])
                    .is_some_and(|p| p.distance(target[k]) < threshold)
            })
            .collect()
    };
    let mut rng = Lcg::new(n as u64);
    let mut best: Vec<usize> = Vec::new();
    for _ in 0..300 {
        let mut idx = [0usize; 4];
        for k in 0..4 {
            loop {
                let c = rng.below(n);
                if !idx[..k].contains(&c) {
                    idx[k] = c;
                    break;
                }
            }
        }
        let Ok(h) = Homography::from_four_pairs(idx.map(|k| source[k]), idx.map(|k| target[k]))
        else {
            continue;
        };
        let inliers = inliers_of(&h);
        if inliers.len() > best.len() {
            best = inliers;
            if best.len() == n {
                break;
            }
        }
    }
    for _ in 0..2 {
        if best.len() < 4 {
            return Homography::fit(source, target).ok();
        }
        let s: Vec<Point> = best.iter().map(|&k| source[k]).collect();
        let t: Vec<Point> = best.iter().map(|&k| target[k]).collect();
        let h = Homography::fit(&s, &t).ok()?;
        let again = inliers_of(&h);
        if again.len() <= best.len() {
            return Some(h);
        }
        best = again;
    }
    let s: Vec<Point> = best.iter().map(|&k| source[k]).collect();
    let t: Vec<Point> = best.iter().map(|&k| target[k]).collect();
    Homography::fit(&s, &t).ok()
}

/// Trapèze + correction locale lissée (voir l'en-tête du module).
pub(crate) struct LocalModel {
    base: Homography,
    points: Vec<Point>,
    residuals: Vec<Point>,
    sigma: f64,
}

impl LocalModel {
    /// Le trapèze seul, sans correction.
    pub fn plain(base: Homography) -> Self {
        Self {
            base,
            points: Vec::new(),
            residuals: Vec::new(),
            sigma: 1.0,
        }
    }

    /// Ajuste le modèle. `sigma` : largeur des poids, en mm. `robust` : seuil
    /// du trapèze robuste en px, ou `None` pour un ajustement simple.
    pub fn fit(mm: &[Point], px: &[Point], sigma: f64, robust: Option<f64>) -> Option<Self> {
        let base = match robust {
            Some(t) if mm.len() >= 16 => robust_homography(mm, px, t)?,
            _ => Homography::fit(mm, px).ok()?,
        };
        Some(Self::with_base(base, mm, px, sigma))
    }

    pub fn with_base(base: Homography, mm: &[Point], px: &[Point], sigma: f64) -> Self {
        let residuals = mm
            .iter()
            .zip(px)
            .map(|(m, p)| base.apply(*m).map_or(Point::new(0.0, 0.0), |q| *p - q))
            .collect();
        // Moins de 6 points : la correction n'a pas de sens.
        let keep = mm.len() >= 6;
        Self {
            base,
            points: if keep { mm.to_vec() } else { Vec::new() },
            residuals: if keep { residuals } else { Vec::new() },
            sigma,
        }
    }

    pub fn predict(&self, x: Point) -> Option<Point> {
        let base = self.base.apply(x)?;
        if self.points.is_empty() {
            return Some(base);
        }
        let s2 = 2.0 * self.sigma * self.sigma;
        let mut a = [[0.0f64; 3]; 3];
        let mut bx = [0.0f64; 3];
        let mut by = [0.0f64; 3];
        let mut wsum = 0.0;
        for (p, r) in self.points.iter().zip(&self.residuals) {
            let dx = p.x - x.x;
            let dy = p.y - x.y;
            let w = (-(dx * dx + dy * dy) / s2).exp() + 1e-5;
            let phi = [1.0, dx, dy];
            for i in 0..3 {
                for j in 0..3 {
                    a[i][j] += w * phi[i] * phi[j];
                }
                bx[i] += w * phi[i] * r.x;
                by[i] += w * phi[i] * r.y;
            }
            wsum += w;
        }
        // Frein sur les pentes : loin des points, la correction reste plate.
        let brake = 0.05 * wsum * self.sigma * self.sigma;
        a[1][1] += brake;
        a[2][2] += brake;
        let cx = solve_small(a, bx)?;
        let cy = solve_small(a, by)?;
        Some(base + Point::new(cx[0], cy[0]))
    }

    /// Position et dérivée (px par mm), par différences finies de 0,5 mm.
    pub fn predict_with_jacobian(&self, x: Point) -> Option<(Point, Jacobian)> {
        if self.points.is_empty() {
            return self.base.apply_with_jacobian(x);
        }
        let h = 0.5;
        let p = self.predict(x)?;
        let px = self.predict(x + Point::new(h, 0.0))?;
        let py = self.predict(x + Point::new(0.0, h))?;
        Some((
            p,
            [
                [(px.x - p.x) / h, (py.x - p.x) / h],
                [(px.y - p.y) / h, (py.y - p.y) / h],
            ],
        ))
    }
}

/// Taille d'un mm de mire dans la photo, au point `at` : racine du
/// déterminant de la dérivée du trapèze.
pub(crate) fn local_scale(h: &Homography, at: Point) -> f64 {
    h.apply_with_jacobian(at).map_or(1.0, |(_, j)| {
        (j[0][0] * j[1][1] - j[0][1] * j[1][0]).abs().sqrt()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_solver_solves() {
        let x = solve_small(
            [[2.0, 1.0, 0.0], [1.0, 3.0, 1.0], [0.0, 1.0, 4.0]],
            [3.0, 5.0, 5.0],
        )
        .unwrap();
        for (v, e) in x.iter().zip([1.0, 1.0, 1.0]) {
            assert!((v - e).abs() < 1e-12);
        }
        assert!(solve_small([[1.0, 2.0], [2.0, 4.0]], [1.0, 2.0]).is_none());
    }

    #[test]
    fn robust_homography_ignores_a_wild_point() {
        let h = Homography::similarity(6.5, 0.3, 500.0, 200.0).unwrap();
        let mut mm = Vec::new();
        let mut px = Vec::new();
        for j in 0..5 {
            for i in 0..5 {
                let p = Point::new(i as f64 * 50.0, j as f64 * 50.0);
                mm.push(p);
                px.push(h.apply(p).unwrap());
            }
        }
        px[7] = px[7] + Point::new(300.0, -200.0);
        let fit = robust_homography(&mm, &px, 5.0).unwrap();
        for (k, (m, p)) in mm.iter().zip(&px).enumerate() {
            let e = fit.apply(*m).unwrap().distance(*p);
            if k == 7 {
                assert!(e > 100.0);
            } else {
                assert!(e < 1e-6, "écart {e}");
            }
        }
    }

    #[test]
    fn local_model_follows_a_smooth_bump() {
        let h = Homography::similarity(6.7, 0.1, 100.0, 50.0).unwrap();
        let bump = |p: Point| Point::new(3.0 * (p.x / 60.0).sin(), 2.0 * (p.y / 80.0).cos());
        let mut mm = Vec::new();
        let mut px = Vec::new();
        for j in 0..29 {
            for i in 0..29 {
                let p = Point::new(i as f64 * 10.0, j as f64 * 10.0);
                mm.push(p);
                px.push(h.apply(p).unwrap() + bump(p));
            }
        }
        let model = LocalModel::fit(&mm, &px, 15.0, Some(40.0)).unwrap();
        let q = Point::new(135.0, 145.0);
        let expected = h.apply(q).unwrap() + bump(q);
        assert!(model.predict(q).unwrap().distance(expected) < 0.3);
    }
}

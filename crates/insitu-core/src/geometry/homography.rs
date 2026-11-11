//! Le trapèze : transformation projective du plan (homographie).

use super::linalg::{factorize, symmetric_eigen, SquareMatrix};
use super::{
    validate_pairs, Differentiable, GeometryError, Jacobian, Normalization, Point, Transform,
};

/// Transformation en trapèze, définie par une matrice 3 × 3 à un facteur près.
///
/// Un point `(x, y)` devient `(X / W, Y / W)` avec
/// `(X, Y, W) = matrice · (x, y, 1)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Homography {
    m: [[f64; 3]; 3],
}

type Matrix3 = [[f64; 3]; 3];

fn multiply(a: &Matrix3, b: &Matrix3) -> Matrix3 {
    let mut out = [[0.0; 3]; 3];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    out
}

fn determinant(m: &Matrix3) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn max_abs(m: &Matrix3) -> f64 {
    m.iter().flatten().fold(0.0_f64, |a, v| a.max(v.abs()))
}

impl Homography {
    pub const IDENTITY: Homography = Homography {
        m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    /// Construit un trapèze à partir de sa matrice. Refuse une matrice non
    /// finie ou sans inverse.
    pub fn from_matrix(m: [[f64; 3]; 3]) -> Result<Self, GeometryError> {
        if m.iter().flatten().any(|v| !v.is_finite()) {
            return Err(GeometryError::InvalidParameter(
                "la matrice du trapèze contient une valeur non finie",
            ));
        }
        let h = Self { m }.rescaled();
        h.inverse()?;
        Ok(h)
    }

    pub fn translation(dx: f64, dy: f64) -> Self {
        Self {
            m: [[1.0, 0.0, dx], [0.0, 1.0, dy], [0.0, 0.0, 1.0]],
        }
    }

    /// Similitude : échelle `scale`, rotation de `angle` radians (de x vers
    /// y, donc dans le sens des aiguilles d'une montre à l'écran), puis
    /// translation `(dx, dy)`.
    pub fn similarity(scale: f64, angle: f64, dx: f64, dy: f64) -> Result<Self, GeometryError> {
        let (s, c) = angle.sin_cos();
        Self::from_matrix([
            [scale * c, -scale * s, dx],
            [scale * s, scale * c, dy],
            [0.0, 0.0, 1.0],
        ])
    }

    /// Trapèze exact passant par 4 paires de points.
    pub fn from_four_pairs(source: [Point; 4], target: [Point; 4]) -> Result<Self, GeometryError> {
        Self::fit(&source, &target)
    }

    /// Trapèze ajusté sur `N ≥ 4` paires : exact pour 4, aux moindres carrés
    /// au-delà (somme des carrés des écarts mesurés côté cible).
    ///
    /// Méthode : coordonnées normalisées des deux côtés, solution linéaire
    /// (DLT), puis affinage par Levenberg-Marquardt sur l'écart géométrique.
    pub fn fit(source: &[Point], target: &[Point]) -> Result<Self, GeometryError> {
        validate_pairs(source, target, 4)?;
        let ns = Normalization::of(source);
        let nt = Normalization::of(target);
        let s: Vec<Point> = source.iter().map(|p| ns.apply(*p)).collect();
        let t: Vec<Point> = target.iter().map(|p| nt.apply(*p)).collect();

        // Solution linéaire : plus petit vecteur propre de AᵀA.
        let mut ata = SquareMatrix::zeros(9);
        for (p, q) in s.iter().zip(&t) {
            let rows = [
                [-p.x, -p.y, -1.0, 0.0, 0.0, 0.0, q.x * p.x, q.x * p.y, q.x],
                [0.0, 0.0, 0.0, -p.x, -p.y, -1.0, q.y * p.x, q.y * p.y, q.y],
            ];
            for row in rows {
                for i in 0..9 {
                    for j in 0..9 {
                        ata.data[i * 9 + j] += row[i] * row[j];
                    }
                }
            }
        }
        let (values, vectors) = symmetric_eigen(ata);
        let mut order: Vec<usize> = (0..9).collect();
        order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
        // Si deux directions au moins annulent le système, le trapèze n'est
        // pas déterminé : points alignés.
        if values[order[1]] <= 1e-12 * values[order[8]] {
            return Err(GeometryError::DegenerateConfiguration);
        }
        let mut h = [0.0; 9];
        for (k, v) in h.iter_mut().enumerate() {
            *v = vectors.at(k, order[0]);
        }
        refine(&mut h, &s, &t);

        let hn = [[h[0], h[1], h[2]], [h[3], h[4], h[5]], [h[6], h[7], h[8]]];
        // `h` est de norme 1 et les deux nuages sont normalisés : un
        // déterminant minuscule signale un plan écrasé sur une ligne.
        if determinant(&hn).abs() < 1e-9 {
            return Err(GeometryError::DegenerateConfiguration);
        }
        let to_normalized = [
            [ns.scale, 0.0, -ns.scale * ns.center.x],
            [0.0, ns.scale, -ns.scale * ns.center.y],
            [0.0, 0.0, 1.0],
        ];
        let from_normalized = [
            [1.0 / nt.scale, 0.0, nt.center.x],
            [0.0, 1.0 / nt.scale, nt.center.y],
            [0.0, 0.0, 1.0],
        ];
        let result = Self {
            m: multiply(&from_normalized, &multiply(&hn, &to_normalized)),
        }
        .rescaled();

        // Tous les points doivent être du même côté de la ligne d'horizon.
        let mut signs = source.iter().map(|p| result.denominator(*p));
        let first = signs.next().unwrap_or(1.0);
        if first == 0.0 || signs.any(|w| w * first <= 0.0) {
            return Err(GeometryError::HorizonCrossing);
        }
        Ok(result)
    }

    /// La matrice, mise à l'échelle pour que son dernier coefficient vaille 1
    /// quand c'est possible.
    pub fn matrix(&self) -> [[f64; 3]; 3] {
        self.m
    }

    fn rescaled(mut self) -> Self {
        let biggest = max_abs(&self.m);
        let divisor = if self.m[2][2].abs() > 1e-9 * biggest {
            self.m[2][2]
        } else {
            biggest
        };
        if divisor != 0.0 {
            for v in self.m.iter_mut().flatten() {
                *v /= divisor;
            }
        }
        self
    }

    fn denominator(&self, p: Point) -> f64 {
        self.m[2][0] * p.x + self.m[2][1] * p.y + self.m[2][2]
    }

    /// Le trapèze inverse (de la cible vers la source).
    pub fn inverse(&self) -> Result<Self, GeometryError> {
        let m = &self.m;
        let det = determinant(m);
        let biggest = max_abs(m);
        if !det.is_finite() || det.abs() <= 1e-12 * biggest * biggest * biggest {
            return Err(GeometryError::SingularTransform);
        }
        let cofactor = |r1: usize, c1: usize, r2: usize, c2: usize| {
            m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1]
        };
        let adjugate = [
            [
                cofactor(1, 1, 2, 2),
                -cofactor(0, 1, 2, 2),
                cofactor(0, 1, 1, 2),
            ],
            [
                -cofactor(1, 0, 2, 2),
                cofactor(0, 0, 2, 2),
                -cofactor(0, 0, 1, 2),
            ],
            [
                cofactor(1, 0, 2, 1),
                -cofactor(0, 0, 2, 1),
                cofactor(0, 0, 1, 1),
            ],
        ];
        Ok(Self { m: adjugate }.rescaled())
    }

    /// Composition : applique `self`, puis `next`.
    pub fn then(&self, next: &Homography) -> Homography {
        Self {
            m: multiply(&next.m, &self.m),
        }
        .rescaled()
    }
}

impl Transform for Homography {
    fn apply(&self, p: Point) -> Option<Point> {
        self.apply_with_jacobian(p).map(|(q, _)| q)
    }

    /// Recalcule l'inverse à chaque appel : pour transformer beaucoup de
    /// points, appeler [`Homography::inverse`] une fois puis `apply`.
    fn apply_inverse(&self, p: Point) -> Option<Point> {
        self.inverse().ok()?.apply(p)
    }
}

impl Differentiable for Homography {
    fn apply_with_jacobian(&self, p: Point) -> Option<(Point, Jacobian)> {
        let m = &self.m;
        let w = self.denominator(p);
        let magnitude = (m[2][0] * p.x).abs() + (m[2][1] * p.y).abs() + m[2][2].abs();
        if !w.is_finite() || w.abs() <= 1e-14 * magnitude {
            return None;
        }
        let x = (m[0][0] * p.x + m[0][1] * p.y + m[0][2]) / w;
        let y = (m[1][0] * p.x + m[1][1] * p.y + m[1][2]) / w;
        let jacobian = [
            [(m[0][0] - x * m[2][0]) / w, (m[0][1] - x * m[2][1]) / w],
            [(m[1][0] - y * m[2][0]) / w, (m[1][1] - y * m[2][1]) / w],
        ];
        let q = Point::new(x, y);
        q.is_finite().then_some((q, jacobian))
    }
}

/// Somme des carrés des écarts côté cible, pour le vecteur `h` (9 coefficients).
fn cost(h: &[f64; 9], s: &[Point], t: &[Point]) -> f64 {
    let mut sum = 0.0;
    for (p, q) in s.iter().zip(t) {
        let w = h[6] * p.x + h[7] * p.y + h[8];
        if w.abs() < 1e-12 {
            return f64::INFINITY;
        }
        let dx = q.x - (h[0] * p.x + h[1] * p.y + h[2]) / w;
        let dy = q.y - (h[3] * p.x + h[4] * p.y + h[5]) / w;
        sum += dx * dx + dy * dy;
    }
    if sum.is_nan() {
        f64::INFINITY
    } else {
        sum
    }
}

/// Affinage de Levenberg-Marquardt : minimise l'écart géométrique côté cible.
/// `h` reste de norme 1 (une matrice de trapèze n'est définie qu'à un facteur
/// près ; l'amortissement lève cette indétermination).
#[allow(clippy::needless_range_loop)]
fn refine(h: &mut [f64; 9], s: &[Point], t: &[Point]) {
    let mut current = cost(h, s, t);
    let mut damping = 1e-4;
    for _ in 0..30 {
        if current.is_nan() || current <= 1e-28 * s.len() as f64 {
            return;
        }
        let mut jtj = [[0.0; 9]; 9];
        let mut jtr = [0.0; 9];
        for (p, q) in s.iter().zip(t) {
            let w = h[6] * p.x + h[7] * p.y + h[8];
            let x = (h[0] * p.x + h[1] * p.y + h[2]) / w;
            let y = (h[3] * p.x + h[4] * p.y + h[5]) / w;
            let rows = [
                (
                    [
                        p.x / w,
                        p.y / w,
                        1.0 / w,
                        0.0,
                        0.0,
                        0.0,
                        -x * p.x / w,
                        -x * p.y / w,
                        -x / w,
                    ],
                    q.x - x,
                ),
                (
                    [
                        0.0,
                        0.0,
                        0.0,
                        p.x / w,
                        p.y / w,
                        1.0 / w,
                        -y * p.x / w,
                        -y * p.y / w,
                        -y / w,
                    ],
                    q.y - y,
                ),
            ];
            for (row, residual) in rows {
                for i in 0..9 {
                    jtr[i] += row[i] * residual;
                    for j in 0..9 {
                        jtj[i][j] += row[i] * row[j];
                    }
                }
            }
        }
        let trace: f64 = (0..9).map(|i| jtj[i][i]).sum();
        let mut improved = None;
        for _ in 0..10 {
            let mut system = SquareMatrix::zeros(9);
            for i in 0..9 {
                for j in 0..9 {
                    system.set(i, j, jtj[i][j]);
                }
                system.data[i * 9 + i] += damping * trace / 9.0;
            }
            if let Some(lu) = factorize(system) {
                let delta = lu.solve(&jtr);
                let mut candidate = *h;
                for (c, d) in candidate.iter_mut().zip(&delta) {
                    *c += d;
                }
                let norm = candidate.iter().map(|v| v * v).sum::<f64>().sqrt();
                for c in candidate.iter_mut() {
                    *c /= norm;
                }
                let candidate_cost = cost(&candidate, s, t);
                if candidate_cost < current {
                    improved = Some((candidate, candidate_cost));
                    damping = (damping * 0.1).max(1e-12);
                    break;
                }
            }
            damping *= 10.0;
        }
        match improved {
            Some((candidate, candidate_cost)) => {
                let gain = current - candidate_cost;
                *h = candidate;
                current = candidate_cost;
                if gain <= 1e-14 * current {
                    return;
                }
            }
            None => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::{assert_close, slide_corners, trapezoid, Lcg};
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn assert_same_matrix(a: &Homography, b: &Homography, tolerance: f64) {
        for (x, y) in a.matrix().iter().flatten().zip(b.matrix().iter().flatten()) {
            assert!(
                (x - y).abs() <= tolerance * (1.0 + y.abs()),
                "{:?} ≠ {:?}",
                a.matrix(),
                b.matrix()
            );
        }
    }

    #[test]
    fn four_identical_pairs_give_the_identity() {
        let corners = slide_corners();
        let h = Homography::from_four_pairs(corners, corners).unwrap();
        assert_same_matrix(&h, &Homography::IDENTITY, 1e-12);
    }

    #[test]
    fn translation_is_recovered() {
        let corners = slide_corners();
        let moved = corners.map(|c| c + p(12.5, -7.25));
        let h = Homography::from_four_pairs(corners, moved).unwrap();
        assert_same_matrix(&h, &Homography::translation(12.5, -7.25), 1e-12);
    }

    #[test]
    fn similarity_is_recovered() {
        let truth = Homography::similarity(1.7, 0.4, 30.0, -12.0).unwrap();
        let corners = slide_corners();
        let moved = corners.map(|c| truth.apply(c).unwrap());
        let h = Homography::from_four_pairs(corners, moved).unwrap();
        assert_same_matrix(&h, &truth, 1e-12);
        // Une similitude d'échelle 1,7 multiplie toutes les longueurs par 1,7.
        let (a, b) = (p(10.0, 20.0), p(110.0, 95.0));
        let ratio = h.apply(a).unwrap().distance(h.apply(b).unwrap()) / a.distance(b);
        assert!((ratio - 1.7).abs() < 1e-12);
    }

    #[test]
    fn hand_computed_trapezoid() {
        // (x, y) ↦ (x, y) / (1 + x / 1000) : le bord droit du carré de 100
        // rétrécit d'un facteur 1,1. Images des coins calculées à la main.
        let source = [p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0), p(0.0, 100.0)];
        let target = [
            p(0.0, 0.0),
            p(100.0 / 1.1, 0.0),
            p(100.0 / 1.1, 100.0 / 1.1),
            p(0.0, 100.0),
        ];
        let h = Homography::from_four_pairs(source, target).unwrap();
        let truth =
            Homography::from_matrix([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.001, 0.0, 1.0]]).unwrap();
        assert_same_matrix(&h, &truth, 1e-11);
        // Le centre du carré : (50, 50) / 1,05.
        assert_close(
            h.apply(p(50.0, 50.0)).unwrap(),
            p(50.0 / 1.05, 50.0 / 1.05),
            1e-11,
        );
        // Dérivée au coin (100, 0), calculée à la main :
        // ∂x'/∂x = 1 / 1,1², ∂y'/∂y = 1 / 1,1, ∂y'/∂x = 0 car y = 0.
        let (_, j) = h.apply_with_jacobian(p(100.0, 0.0)).unwrap();
        assert!((j[0][0] - 1.0 / 1.21).abs() < 1e-12);
        assert!((j[1][1] - 1.0 / 1.1).abs() < 1e-12);
        assert!(j[0][1].abs() < 1e-12 && j[1][0].abs() < 1e-12);
    }

    #[test]
    fn four_pairs_are_matched_exactly() {
        let source = slide_corners();
        let target = [
            p(12.0, 8.0),
            p(262.0, -5.0),
            p(301.0, 290.0),
            p(-25.0, 271.0),
        ];
        let h = Homography::from_four_pairs(source, target).unwrap();
        for (s, t) in source.iter().zip(target) {
            assert_close(h.apply(*s).unwrap(), t, 1e-10);
        }
    }

    #[test]
    fn works_from_pixel_coordinates_far_from_the_origin() {
        // Une photo de 4 032 × 3 024 px vers le carré de 280 mm.
        let source = [
            p(912.3, 401.7),
            p(3211.9, 377.2),
            p(3304.4, 2702.8),
            p(871.0, 2655.1),
        ];
        let target = slide_corners();
        let h = Homography::from_four_pairs(source, target).unwrap();
        for (s, t) in source.iter().zip(target) {
            assert_close(h.apply(*s).unwrap(), t, 1e-9);
        }
    }

    #[test]
    fn inverse_and_composition() {
        let h = trapezoid();
        let back = h.inverse().unwrap();
        assert_same_matrix(&h.then(&back), &Homography::IDENTITY, 1e-12);
        let shift = Homography::translation(5.0, 3.0);
        let both = h.then(&shift);
        let q = p(70.0, 190.0);
        assert_close(
            both.apply(q).unwrap(),
            h.apply(q).unwrap() + p(5.0, 3.0),
            1e-11,
        );
        assert_close(h.apply_inverse(h.apply(q).unwrap()).unwrap(), q, 1e-11);
    }

    #[test]
    fn round_trip_on_a_dense_grid() {
        let h = trapezoid();
        let back = h.inverse().unwrap();
        let mut worst = 0.0_f64;
        for i in 0..=280 {
            for j in 0..=280 {
                let q = p(i as f64, j as f64);
                let r = back.apply(h.apply(q).unwrap()).unwrap();
                worst = worst.max(q.distance(r));
            }
        }
        assert!(worst < 1e-10, "aller-retour : {worst} mm");
    }

    #[test]
    fn least_squares_recovers_exact_data() {
        let truth = trapezoid();
        let mut rng = Lcg::new(7);
        let source: Vec<Point> = (0..40).map(|_| rng.point_in_slide()).collect();
        let target: Vec<Point> = source.iter().map(|s| truth.apply(*s).unwrap()).collect();
        let h = Homography::fit(&source, &target).unwrap();
        assert_same_matrix(&h, &truth, 1e-10);
    }

    #[test]
    fn least_squares_averages_noise() {
        // 400 points bruités de ± 0,3 mm : le trapèze ajusté doit être bien
        // plus près de la vérité que chaque point pris seul.
        let truth = trapezoid();
        let mut rng = Lcg::new(11);
        let source: Vec<Point> = (0..400).map(|_| rng.point_in_slide()).collect();
        let target: Vec<Point> = source
            .iter()
            .map(|s| truth.apply(*s).unwrap() + rng.offset(0.3))
            .collect();
        let h = Homography::fit(&source, &target).unwrap();
        let mut worst = 0.0_f64;
        for i in 0..=28 {
            for j in 0..=28 {
                let q = p(i as f64 * 10.0, j as f64 * 10.0);
                worst = worst.max(h.apply(q).unwrap().distance(truth.apply(q).unwrap()));
            }
        }
        assert!(worst < 0.12, "écart au vrai trapèze : {worst} mm");
    }

    #[test]
    fn refinement_does_not_increase_the_geometric_error() {
        // L'affinage part de la solution linéaire : il ne peut que réduire la
        // somme des carrés des écarts.
        let truth = trapezoid();
        let mut rng = Lcg::new(3);
        let source: Vec<Point> = (0..12).map(|_| rng.point_in_slide()).collect();
        let target: Vec<Point> = source
            .iter()
            .map(|s| truth.apply(*s).unwrap() + rng.offset(2.0))
            .collect();
        let h = Homography::fit(&source, &target).unwrap();
        let sum = |h: &Homography| -> f64 {
            source
                .iter()
                .zip(&target)
                .map(|(s, t)| h.apply(*s).unwrap().distance(*t).powi(2))
                .sum()
        };
        // Aucune petite perturbation de la matrice ne fait mieux.
        let best = sum(&h);
        let m = h.matrix();
        for i in 0..3 {
            for j in 0..3 {
                for step in [-1e-6, 1e-6] {
                    let mut n = m;
                    n[i][j] += step * (1.0 + m[i][j].abs());
                    let other = Homography::from_matrix(n).unwrap();
                    assert!(sum(&other) >= best * (1.0 - 1e-9));
                }
            }
        }
    }

    #[test]
    fn too_few_or_mismatched_points() {
        let three = [p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)];
        assert_eq!(
            Homography::fit(&three, &three),
            Err(GeometryError::NotEnoughPoints { needed: 4, got: 3 })
        );
        assert_eq!(
            Homography::fit(&slide_corners(), &three),
            Err(GeometryError::MismatchedLengths {
                source: 4,
                target: 3
            })
        );
    }

    #[test]
    fn aligned_points_are_refused() {
        let line = [p(0.0, 0.0), p(10.0, 10.0), p(20.0, 20.0), p(35.0, 35.0)];
        assert_eq!(
            Homography::fit(&line, &slide_corners()),
            Err(GeometryError::DegenerateConfiguration)
        );
        assert_eq!(
            Homography::fit(&slide_corners(), &line),
            Err(GeometryError::DegenerateConfiguration)
        );
        // Trois points alignés sur quatre suffisent à rendre le trapèze
        // impossible.
        let three_aligned = [p(0.0, 0.0), p(140.0, 0.0), p(280.0, 0.0), p(0.0, 280.0)];
        assert!(Homography::fit(&three_aligned, &slide_corners()).is_err());
        // Même avec beaucoup de points.
        let many: Vec<Point> = (0..50).map(|i| p(i as f64, 2.0 * i as f64 + 1.0)).collect();
        let mut rng = Lcg::new(5);
        let anywhere: Vec<Point> = (0..50).map(|_| rng.point_in_slide()).collect();
        assert_eq!(
            Homography::fit(&many, &anywhere),
            Err(GeometryError::DegenerateConfiguration)
        );
    }

    #[test]
    fn coincident_points_are_named() {
        let mut source = slide_corners();
        source[2] = source[0];
        assert_eq!(
            Homography::fit(&source, &slide_corners()),
            Err(GeometryError::CoincidentPoints {
                first: 0,
                second: 2,
                side: crate::geometry::Side::Source
            })
        );
        let err = Homography::fit(&slide_corners(), &source).unwrap_err();
        assert_eq!(
            err.to_string(),
            "les points cible n° 1 et n° 3 sont au même endroit"
        );
    }

    #[test]
    fn non_finite_coordinates_are_refused() {
        let mut source = slide_corners();
        source[1].x = f64::NAN;
        assert!(matches!(
            Homography::fit(&source, &slide_corners()),
            Err(GeometryError::NonFiniteCoordinate { index: 1, .. })
        ));
    }

    #[test]
    fn crossed_quadrilateral_is_refused() {
        // Deux coins échangés côté cible : le quadrilatère devient un nœud
        // papillon.
        let c = slide_corners();
        let crossed = [c[0], c[2], c[1], c[3]];
        assert_eq!(
            Homography::fit(&c, &crossed),
            Err(GeometryError::HorizonCrossing)
        );
    }

    #[test]
    fn singular_matrix_has_no_inverse() {
        assert_eq!(
            Homography::from_matrix([[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [0.0, 0.0, 1.0]]),
            Err(GeometryError::SingularTransform)
        );
    }

    #[test]
    fn point_on_the_horizon_has_no_image() {
        // W = 1 − x / 100 s'annule en x = 100.
        let h =
            Homography::from_matrix([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [-0.01, 0.0, 1.0]]).unwrap();
        assert_eq!(h.apply(p(100.0, 40.0)), None);
        assert!(h.apply(p(99.0, 40.0)).is_some());
    }
}

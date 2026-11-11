//! Spline en plaque mince : la surface la plus « souple » passant par des
//! valeurs données en des points quelconques.
//!
//! Elle sert ici à modéliser un champ de déplacements (deux valeurs par
//! point) : ce qui reste à corriger une fois le trapèze appliqué.

use super::linalg::{factorize, SquareMatrix};
use super::{GeometryError, Jacobian, Normalization, Point};

/// Champ de déplacements lisse défini par des points de contrôle.
#[derive(Debug, Clone, PartialEq)]
pub struct ThinPlateSpline {
    normalization: Normalization,
    /// Points de contrôle, en coordonnées normalisées.
    nodes: Vec<Point>,
    /// Poids de chaque point de contrôle (un par coordonnée du déplacement).
    weights: Vec<Point>,
    /// Partie affine : constante, coefficient de x, coefficient de y.
    affine: [Point; 3],
}

/// Noyau de la plaque mince, en fonction du carré de la distance :
/// `r² · ln r`.
fn kernel(r2: f64) -> f64 {
    if r2 > 0.0 {
        0.5 * r2 * r2.ln()
    } else {
        0.0
    }
}

/// Matrice du système : noyau entre points de contrôle (plus le lissage sur
/// la diagonale), bordé par la partie affine.
pub(crate) fn system_matrix(nodes: &[Point], smoothing: f64) -> SquareMatrix {
    let n = nodes.len();
    let mut m = SquareMatrix::zeros(n + 3);
    for (i, a) in nodes.iter().enumerate() {
        for (j, b) in nodes.iter().enumerate().skip(i + 1) {
            let d = *a - *b;
            let k = kernel(d.x * d.x + d.y * d.y);
            m.set(i, j, k);
            m.set(j, i, k);
        }
        m.set(i, i, smoothing);
        for (c, v) in [1.0, a.x, a.y].into_iter().enumerate() {
            m.set(i, n + c, v);
            m.set(n + c, i, v);
        }
    }
    m
}

pub(crate) fn check_smoothing(smoothing: f64) -> Result<(), GeometryError> {
    if smoothing.is_finite() && smoothing >= 0.0 {
        Ok(())
    } else {
        Err(GeometryError::InvalidParameter(
            "le lissage doit être un nombre positif ou nul",
        ))
    }
}

impl ThinPlateSpline {
    /// Ajuste le champ sur `values` (déplacements) donnés aux `centers`.
    ///
    /// `smoothing` vaut 0 pour passer exactement par les valeurs ; plus il
    /// est grand, plus la surface est raide et s'écarte des valeurs (à
    /// l'infini il ne reste qu'un plan). Il est sans unité : les points sont
    /// d'abord ramenés à une taille standard, donc la même valeur a le même
    /// effet en pixels et en millimètres.
    ///
    /// Les points doivent être distincts et non tous alignés (au moins 3).
    pub fn fit(centers: &[Point], values: &[Point], smoothing: f64) -> Result<Self, GeometryError> {
        check_smoothing(smoothing)?;
        super::validate_pairs(centers, centers, 3)?;
        if centers.len() != values.len() {
            return Err(GeometryError::MismatchedLengths {
                source: centers.len(),
                target: values.len(),
            });
        }
        let normalization = Normalization::of(centers);
        let nodes: Vec<Point> = centers.iter().map(|p| normalization.apply(*p)).collect();
        let n = nodes.len();
        let lu = factorize(system_matrix(&nodes, smoothing))
            .ok_or(GeometryError::DegenerateConfiguration)?;
        let mut rhs = vec![0.0; n + 3];
        for (r, v) in rhs.iter_mut().zip(values) {
            *r = v.x;
        }
        let cx = lu.solve(&rhs);
        for (r, v) in rhs.iter_mut().zip(values) {
            *r = v.y;
        }
        let cy = lu.solve(&rhs);
        let coefficient = |k: usize| Point::new(cx[k], cy[k]);
        Ok(Self {
            normalization,
            weights: (0..n).map(coefficient).collect(),
            affine: [coefficient(n), coefficient(n + 1), coefficient(n + 2)],
            nodes,
        })
    }

    /// Nombre de points de contrôle.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Déplacement au point `p`. Coût proportionnel au nombre de points de
    /// contrôle.
    pub fn value(&self, p: Point) -> Point {
        let q = self.normalization.apply(p);
        let mut sum = self.affine[0] + self.affine[1] * q.x + self.affine[2] * q.y;
        for (node, weight) in self.nodes.iter().zip(&self.weights) {
            let d = q - *node;
            sum = sum + *weight * kernel(d.x * d.x + d.y * d.y);
        }
        sum
    }

    /// Déplacement au point `p` et sa dérivée.
    pub fn value_with_jacobian(&self, p: Point) -> (Point, Jacobian) {
        let q = self.normalization.apply(p);
        let mut sum = self.affine[0] + self.affine[1] * q.x + self.affine[2] * q.y;
        let mut dx = self.affine[1];
        let mut dy = self.affine[2];
        for (node, weight) in self.nodes.iter().zip(&self.weights) {
            let d = q - *node;
            let r2 = d.x * d.x + d.y * d.y;
            if r2 > 0.0 {
                let log = r2.ln();
                sum = sum + *weight * (0.5 * r2 * log);
                // Gradient de r² ln r : (2 ln r + 1) · d.
                dx = dx + *weight * ((log + 1.0) * d.x);
                dy = dy + *weight * ((log + 1.0) * d.y);
            }
        }
        let s = self.normalization.scale;
        (sum, [[dx.x * s, dy.x * s], [dx.y * s, dy.y * s]])
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util::{assert_close, paper_wave, slide_grid, Lcg};
    use super::*;

    #[test]
    fn passes_through_its_control_points() {
        let centers = slide_grid(8);
        let values: Vec<Point> = centers.iter().map(|p| paper_wave(*p)).collect();
        let spline = ThinPlateSpline::fit(&centers, &values, 0.0).unwrap();
        assert_eq!(spline.len(), 64);
        for (c, v) in centers.iter().zip(&values) {
            assert_close(spline.value(*c), *v, 1e-9);
        }
    }

    #[test]
    fn reproduces_an_affine_field_everywhere() {
        // Un champ affine ne demande aucune flexion : la spline doit le
        // reproduire aussi loin qu'on veut des points de contrôle.
        let field = |p: Point| Point::new(1.0 + 0.01 * p.x - 0.02 * p.y, -3.0 + 0.005 * p.y);
        let mut rng = Lcg::new(1);
        let centers: Vec<Point> = (0..20).map(|_| rng.point_in_slide()).collect();
        let values: Vec<Point> = centers.iter().map(|p| field(*p)).collect();
        let spline = ThinPlateSpline::fit(&centers, &values, 0.0).unwrap();
        for q in [Point::new(-500.0, 900.0), Point::new(140.0, 140.0)] {
            assert_close(spline.value(q), field(q), 1e-8);
        }
    }

    #[test]
    fn jacobian_matches_finite_differences() {
        let centers = slide_grid(6);
        let values: Vec<Point> = centers.iter().map(|p| paper_wave(*p)).collect();
        let spline = ThinPlateSpline::fit(&centers, &values, 0.0).unwrap();
        let mut rng = Lcg::new(2);
        let h = 1e-5;
        for _ in 0..50 {
            let p = rng.point_in_slide();
            let (value, j) = spline.value_with_jacobian(p);
            assert_close(value, spline.value(p), 1e-12);
            let ddx = (spline.value(p + Point::new(h, 0.0)) - spline.value(p - Point::new(h, 0.0)))
                * (0.5 / h);
            let ddy = (spline.value(p + Point::new(0.0, h)) - spline.value(p - Point::new(0.0, h)))
                * (0.5 / h);
            assert_close(Point::new(j[0][0], j[1][0]), ddx, 1e-7);
            assert_close(Point::new(j[0][1], j[1][1]), ddy, 1e-7);
        }
    }

    #[test]
    fn smoothing_filters_noise() {
        // Champ très lisse + bruit de ± 0,5 mm sur une grille 12 × 12. Sans
        // lissage la spline suit le bruit ; avec, elle s'approche du champ.
        let centers = slide_grid(12);
        let mut rng = Lcg::new(4);
        let noisy: Vec<Point> = centers
            .iter()
            .map(|p| paper_wave(*p * 0.3) + rng.offset(0.5))
            .collect();
        let rms_to_truth = |smoothing: f64| -> f64 {
            let spline = ThinPlateSpline::fit(&centers, &noisy, smoothing).unwrap();
            let sum: f64 = centers
                .iter()
                .map(|p| spline.value(*p).distance(paper_wave(*p * 0.3)).powi(2))
                .sum();
            (sum / centers.len() as f64).sqrt()
        };
        let exact = rms_to_truth(0.0);
        let smoothed = rms_to_truth(0.3);
        assert!(exact > 0.35, "sans lissage : {exact}");
        assert!(smoothed < 0.6 * exact, "{smoothed} contre {exact}");
    }

    #[test]
    fn same_result_in_pixels_and_in_millimetres() {
        // Le lissage est sans unité : changer d'échelle les points de
        // contrôle ne change pas la surface.
        let centers = slide_grid(5);
        let mut rng = Lcg::new(9);
        let values: Vec<Point> = centers.iter().map(|_| rng.offset(1.0)).collect();
        let scaled: Vec<Point> = centers.iter().map(|p| *p * 37.0).collect();
        let a = ThinPlateSpline::fit(&centers, &values, 0.05).unwrap();
        let b = ThinPlateSpline::fit(&scaled, &values, 0.05).unwrap();
        let q = Point::new(77.0, 191.0);
        assert_close(a.value(q), b.value(q * 37.0), 1e-9);
    }

    #[test]
    fn degenerate_inputs_are_refused() {
        let line: Vec<Point> = (0..5).map(|i| Point::new(i as f64, i as f64)).collect();
        let zeros = vec![Point::new(0.0, 0.0); 5];
        assert_eq!(
            ThinPlateSpline::fit(&line, &zeros, 0.0),
            Err(GeometryError::DegenerateConfiguration)
        );
        assert!(matches!(
            ThinPlateSpline::fit(&line[..2], &zeros[..2], 0.0),
            Err(GeometryError::NotEnoughPoints { needed: 3, got: 2 })
        ));
        assert!(matches!(
            ThinPlateSpline::fit(&slide_grid(3), &zeros, -1.0),
            Err(GeometryError::InvalidParameter(_))
        ));
    }
}

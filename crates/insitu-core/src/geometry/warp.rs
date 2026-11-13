//! Déformation par points : un trapèze, plus une correction lisse qui passe
//! par les points de contrôle.

use super::grid::Rect;
use super::spline::{check_smoothing, ThinPlateSpline};
use super::{
    jacobian_det, jacobian_solve, validate_pairs, Differentiable, GeometryError, Homography,
    Jacobian, Point, Transform,
};

/// Déformation définie par des paires de points (source → cible).
///
/// Sens direct : `cible = trapèze(source) + correction(source)`, où la
/// correction est une spline en plaque mince ajustée sur ce que le trapèze
/// laisse comme écart en chaque point de contrôle.
///
/// - Avec 4 paires, c'est le trapèze exact, sans correction.
/// - Avec un lissage nul, la déformation passe exactement par tous les
///   points ; avec un lissage positif, elle s'en approche.
///
/// Chaque évaluation coûte un temps proportionnel au nombre de points de
/// contrôle : c'est fait pour quelques milliers de points (clics, contours,
/// diagnostics). Pour déformer une image, passer par
/// [`GridWarp`](super::GridWarp).
///
/// **Sens inverse.** Il n'a pas de formule : il est calculé par itérations
/// de Newton à partir du trapèze inverse, jusqu'à ce que le sens direct
/// retombe sur le point demandé à mieux qu'un millionième de millimètre. Si la déformation
/// se replie sur elle-même (deux points source envoyés au même endroit), le
/// sens inverse n'est plus défini : la fonction renvoie alors soit `None`,
/// soit l'un des antécédents — jamais un point faux. [`has_fold`] permet de
/// le savoir à l'avance.
#[derive(Debug, Clone, PartialEq)]
pub struct PointWarp {
    base: Homography,
    base_inverse: Homography,
    correction: Option<ThinPlateSpline>,
}

impl PointWarp {
    /// Ajuste la déformation sur `N ≥ 4` paires. `smoothing` : voir
    /// [`ThinPlateSpline::fit`].
    pub fn fit(source: &[Point], target: &[Point], smoothing: f64) -> Result<Self, GeometryError> {
        check_smoothing(smoothing)?;
        validate_pairs(source, target, 4)?;
        let base = Homography::fit(source, target)?;
        let base_inverse = base.inverse()?;
        let correction = if source.len() > 4 {
            let mut leftovers = Vec::with_capacity(source.len());
            for (s, t) in source.iter().zip(target) {
                let image = base.apply(*s).ok_or(GeometryError::HorizonCrossing)?;
                leftovers.push(*t - image);
            }
            Some(ThinPlateSpline::fit(source, &leftovers, smoothing)?)
        } else {
            None
        };
        Ok(Self {
            base,
            base_inverse,
            correction,
        })
    }

    /// Le trapèze seul, ajusté au mieux sur `N ≥ 4` paires (exact pour 4,
    /// aux moindres carrés au-delà), sans correction locale : une perspective
    /// pure, qui ne courbe jamais les droites.
    pub fn trapeze(source: &[Point], target: &[Point]) -> Result<Self, GeometryError> {
        validate_pairs(source, target, 4)?;
        let base = Homography::fit(source, target)?;
        let base_inverse = base.inverse()?;
        Ok(Self {
            base,
            base_inverse,
            correction: None,
        })
    }

    /// Le trapèze seul, sans la correction locale.
    pub fn base(&self) -> &Homography {
        &self.base
    }

    /// La correction locale au point `p` (zéro s'il n'y a que 4 points).
    pub fn correction(&self, p: Point) -> Point {
        match &self.correction {
            Some(spline) => spline.value(p),
            None => Point::new(0.0, 0.0),
        }
    }
}

impl Transform for PointWarp {
    fn apply(&self, p: Point) -> Option<Point> {
        let q = self.base.apply(p)? + self.correction(p);
        q.is_finite().then_some(q)
    }

    fn apply_inverse(&self, p: Point) -> Option<Point> {
        if self.correction.is_none() {
            return self.base_inverse.apply(p);
        }
        let start = self.base_inverse.apply(p)?;
        invert_by_newton(|q| self.apply_with_jacobian(q), p, start)
    }
}

impl Differentiable for PointWarp {
    fn apply_with_jacobian(&self, p: Point) -> Option<(Point, Jacobian)> {
        let (mut q, mut j) = self.base.apply_with_jacobian(p)?;
        if let Some(spline) = &self.correction {
            let (d, dj) = spline.value_with_jacobian(p);
            q = q + d;
            for (row, drow) in j.iter_mut().zip(dj) {
                row[0] += drow[0];
                row[1] += drow[1];
            }
        }
        q.is_finite().then_some((q, j))
    }
}

/// Cherche le point source dont l'image par `forward` est `target`, par la
/// méthode de Newton avec réduction du pas. `None` si elle ne converge pas.
pub(crate) fn invert_by_newton<F>(forward: F, target: Point, start: Point) -> Option<Point>
where
    F: Fn(Point) -> Option<(Point, Jacobian)>,
{
    if !target.is_finite() {
        return None;
    }
    let tolerance = 1e-12 * (1.0 + target.x.abs().max(target.y.abs()));
    let mut p = start;
    let (mut value, mut jacobian) = forward(p)?;
    let mut error = (target - value).norm();
    for _ in 0..50 {
        if error <= tolerance {
            return Some(p);
        }
        let step = jacobian_solve(&jacobian, target - value)?;
        let mut scale = 1.0;
        let mut advanced = false;
        for _ in 0..12 {
            let candidate = p + step * scale;
            if let Some((v, j)) = forward(candidate) {
                let e = (target - v).norm();
                if e < error {
                    p = candidate;
                    value = v;
                    jacobian = j;
                    error = e;
                    advanced = true;
                    break;
                }
            }
            scale *= 0.5;
        }
        if !advanced {
            break;
        }
    }
    (error <= tolerance).then_some(p)
}

/// Dit si la déformation se replie quelque part dans `region` (plan source).
///
/// Le contrôle porte sur `(samples_per_side + 1)²` points régulièrement
/// espacés : la déformation est repliée si elle retourne le plan en certains
/// points et pas en d'autres, ou si un point n'a pas d'image. Un repli plus
/// petit que l'espacement des points de contrôle peut échapper au test.
pub fn has_fold<T: Differentiable>(transform: &T, region: Rect, samples_per_side: usize) -> bool {
    let n = samples_per_side.max(1);
    let mut reference = 0.0;
    for j in 0..=n {
        for i in 0..=n {
            let p = Point::new(
                region.min.x + (region.max.x - region.min.x) * i as f64 / n as f64,
                region.min.y + (region.max.y - region.min.y) * j as f64 / n as f64,
            );
            let Some((_, jacobian)) = transform.apply_with_jacobian(p) else {
                return true;
            };
            let det = jacobian_det(&jacobian);
            if reference == 0.0 {
                reference = det;
            }
            if det.is_nan() || det * reference <= 0.0 {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::test_util::{
        assert_close, realistic, slide_corners, slide_grid, trapezoid, Lcg,
    };
    use super::*;
    use crate::units::SLIDE_SIZE_MM;

    fn slide() -> Rect {
        Rect::new(
            Point::new(0.0, 0.0),
            Point::new(SLIDE_SIZE_MM, SLIDE_SIZE_MM),
        )
        .unwrap()
    }

    #[test]
    fn four_pairs_give_the_plain_trapezoid() {
        let truth = trapezoid();
        let source = slide_corners();
        let target = source.map(|s| truth.apply(s).unwrap());
        let warp = PointWarp::fit(&source, &target, 0.0).unwrap();
        let q = Point::new(60.0, 200.0);
        assert_close(warp.apply(q).unwrap(), truth.apply(q).unwrap(), 1e-10);
        assert_eq!(warp.correction(q), Point::new(0.0, 0.0));
        assert_close(
            warp.apply_inverse(warp.apply(q).unwrap()).unwrap(),
            q,
            1e-10,
        );
    }

    #[test]
    fn passes_through_every_control_point() {
        let source = slide_grid(15);
        let target: Vec<Point> = source.iter().map(|s| realistic(*s)).collect();
        let warp = PointWarp::fit(&source, &target, 0.0).unwrap();
        for (s, t) in source.iter().zip(&target) {
            assert_close(warp.apply(*s).unwrap(), *t, 1e-8);
        }
    }

    #[test]
    fn extra_points_reduce_the_error() {
        // Le critère de la tâche 3.2 : 4 points donnent le trapèze, les
        // suivants réduisent l'écart. Mesuré contre la déformation vraie, sur
        // toute la diapositive.
        let worst_error = |count: usize| -> f64 {
            let source = slide_grid(count);
            let target: Vec<Point> = source.iter().map(|s| realistic(*s)).collect();
            let warp = PointWarp::fit(&source, &target, 0.0).unwrap();
            let mut worst = 0.0_f64;
            for q in slide_grid(57) {
                worst = worst.max(warp.apply(q).unwrap().distance(realistic(q)));
            }
            worst
        };
        let (four, nine, many, mire) = (
            worst_error(2),
            worst_error(3),
            worst_error(8),
            worst_error(15),
        );
        eprintln!(
            "écart max au modèle vrai : 4 points {four:.3} mm, 9 points {nine:.3} mm, \
             64 points {many:.3} mm, 225 points {mire:.4} mm"
        );
        assert!(four > 3.0, "le trapèze seul laisse {four} mm");
        assert!(many < 0.5 * four);
        assert!(mire < 0.5 * many, "avec 225 points : {mire} mm");
    }

    #[test]
    fn round_trip_with_the_exact_model() {
        let source = slide_grid(15);
        let target: Vec<Point> = source.iter().map(|s| realistic(*s)).collect();
        let warp = PointWarp::fit(&source, &target, 0.0).unwrap();
        assert!(!has_fold(&warp, slide(), 40));
        let mut worst = 0.0_f64;
        for q in slide_grid(36) {
            let back = warp.apply_inverse(warp.apply(q).unwrap()).unwrap();
            worst = worst.max(back.distance(q));
        }
        eprintln!("aller-retour, modèle exact (225 points) : {worst:.3e} mm");
        assert!(worst < 1e-6, "aller-retour : {worst} mm");
    }

    #[test]
    fn round_trip_from_pixels_to_millimetres() {
        // Cas du redressement : la source est en pixels de photo (mire
        // d'environ 2 300 px dans une photo de 4 032 × 3 024), la cible en mm.
        let to_photo = Homography::from_four_pairs(
            slide_corners(),
            [
                Point::new(912.3, 401.7),
                Point::new(3211.9, 377.2),
                Point::new(3304.4, 2702.8),
                Point::new(871.0, 2655.1),
            ],
        )
        .unwrap();
        let target = slide_grid(12);
        let source: Vec<Point> = target
            .iter()
            .map(|t| to_photo.apply(realistic(*t)).unwrap())
            .collect();
        let warp = PointWarp::fit(&source, &target, 0.0).unwrap();
        let mut rng = Lcg::new(21);
        let mut worst = 0.0_f64;
        for _ in 0..300 {
            let mm = rng.point_in_slide();
            let px = warp.apply_inverse(mm).unwrap();
            worst = worst.max(warp.apply(px).unwrap().distance(mm));
        }
        assert!(worst < 1e-6, "aller-retour : {worst} mm");
    }

    #[test]
    fn smoothing_lets_the_warp_miss_noisy_points() {
        let source = slide_grid(10);
        let mut rng = Lcg::new(8);
        let target: Vec<Point> = source
            .iter()
            .map(|s| realistic(*s) + rng.offset(0.4))
            .collect();
        let exact = PointWarp::fit(&source, &target, 0.0).unwrap();
        let smooth = PointWarp::fit(&source, &target, 0.02).unwrap();
        let rms = |warp: &PointWarp, reference: &dyn Fn(usize) -> Point| -> f64 {
            let sum: f64 = source
                .iter()
                .enumerate()
                .map(|(i, s)| warp.apply(*s).unwrap().distance(reference(i)).powi(2))
                .sum();
            (sum / source.len() as f64).sqrt()
        };
        // Sans lissage : passe par les points bruités.
        assert!(rms(&exact, &|i| target[i]) < 1e-8);
        // Avec lissage : s'en écarte, et se rapproche de la vérité.
        assert!(rms(&smooth, &|i| target[i]) > 0.05);
        assert!(rms(&smooth, &|i| realistic(source[i])) < rms(&exact, &|i| realistic(source[i])));
    }

    #[test]
    fn a_folded_warp_is_detected_and_never_inverted_wrongly() {
        // Un point central tiré de 50 mm vers la droite, alors que ses
        // voisins à 20 mm ne bougent pas : la déformation se replie.
        let mut source = slide_grid(15);
        let mut target = source.clone();
        let middle = 7 * 15 + 7;
        target[middle].x += 50.0;
        let warp = PointWarp::fit(&source, &target, 0.0).unwrap();
        assert!(has_fold(&warp, slide(), 140));
        let mut refused = 0;
        for q in slide_grid(29) {
            match warp.apply_inverse(q) {
                // Quand un antécédent est rendu, c'en est bien un.
                Some(back) => assert_close(warp.apply(back).unwrap(), q, 1e-6),
                None => refused += 1,
            }
        }
        eprintln!("déformation repliée : {refused} points sur 841 sans antécédent trouvé");
        // Le même point tiré de 3 mm seulement : pas de repli.
        target[middle].x -= 47.0;
        source.truncate(225);
        let gentle = PointWarp::fit(&source, &target, 0.0).unwrap();
        assert!(!has_fold(&gentle, slide(), 140));
    }

    #[test]
    fn errors_are_reported_not_panicked() {
        let line: Vec<Point> = (0..6).map(|i| Point::new(i as f64, 3.0)).collect();
        assert_eq!(
            PointWarp::fit(&line, &slide_grid(3)[..6], 0.0),
            Err(GeometryError::DegenerateConfiguration)
        );
        assert!(matches!(
            PointWarp::fit(&slide_corners(), &slide_corners(), f64::NAN),
            Err(GeometryError::InvalidParameter(_))
        ));
        let warp = PointWarp::fit(&slide_grid(3), &slide_grid(3), 0.0).unwrap();
        assert_eq!(warp.apply(Point::new(f64::NAN, 0.0)), None);
        assert_eq!(warp.apply_inverse(Point::new(f64::INFINITY, 0.0)), None);
    }
}

//! Écarts résiduels et repérage des points aberrants.
//!
//! Une déformation qui passe exactement par ses points de contrôle laisse un
//! écart nul en chacun d'eux : cet écart ne dit rien de la qualité d'un
//! point. Le diagnostic utile est l'écart **par validation croisée** : pour
//! chaque point, la distance entre sa position cible et celle que prédisent
//! *les autres points sans lui*. Un point mal cliqué s'y voit tout de suite ;
//! un relief réel aussi, mais partagé par les points voisins.

use serde::Serialize;

use super::linalg::{factorize, SquareMatrix};
use super::spline::{check_smoothing, system_matrix};
use super::{
    jacobian_solve, Differentiable, GeometryError, Normalization, Point, PointWarp, Transform,
};

/// En dessous de ce nombre de paires, pas de recherche de points aberrants :
/// il n'y a pas assez de points pour que les autres contredisent l'un d'eux.
pub const MIN_POINTS_FOR_OUTLIERS: usize = 6;

/// Écarts d'un ensemble de points : un par point, et leur résumé.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidualStats {
    /// Écart de chaque point, dans l'ordre des paires fournies.
    pub per_point: Vec<f64>,
    /// Moyenne quadratique des écarts.
    pub rms: f64,
    /// Plus grand écart.
    pub max: f64,
    /// Indice (à partir de 0) du point le plus écarté.
    pub worst_index: Option<usize>,
}

impl ResidualStats {
    pub fn from_values(per_point: Vec<f64>) -> Self {
        let mut max = 0.0;
        let mut worst_index = None;
        let mut sum = 0.0;
        for (i, v) in per_point.iter().enumerate() {
            sum += v * v;
            if worst_index.is_none() || *v > max {
                max = *v;
                worst_index = Some(i);
            }
        }
        let rms = if per_point.is_empty() {
            0.0
        } else {
            (sum / per_point.len() as f64).sqrt()
        };
        Self {
            per_point,
            rms,
            max,
            worst_index,
        }
    }
}

/// Écarts côté cible : distance entre l'image de chaque point source et son
/// point cible, dans l'unité de la cible. Un point sans image compte pour un
/// écart infini.
pub fn residuals<T: Transform>(transform: &T, source: &[Point], target: &[Point]) -> ResidualStats {
    ResidualStats::from_values(
        source
            .iter()
            .zip(target)
            .map(|(s, t)| match transform.apply(*s) {
                Some(image) => image.distance(*t),
                None => f64::INFINITY,
            })
            .collect(),
    )
}

/// Les mêmes écarts, ramenés côté source (dans l'unité de la source) : de
/// combien il faudrait déplacer chaque point source pour tomber juste.
/// Calculé au premier ordre, avec la dérivée de la transformation.
pub fn residuals_in_source<T: Differentiable>(
    transform: &T,
    source: &[Point],
    target: &[Point],
) -> ResidualStats {
    ResidualStats::from_values(
        source
            .iter()
            .zip(target)
            .map(|(s, t)| {
                transform
                    .apply_with_jacobian(*s)
                    .and_then(|(image, jacobian)| jacobian_solve(&jacobian, *t - image))
                    .map_or(f64::INFINITY, Point::norm)
            })
            .collect(),
    )
}

/// Réglages d'un ajustement avec diagnostic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitOptions {
    /// Lissage de la correction locale (0 = passe par les points). Voir
    /// [`ThinPlateSpline::fit`](super::ThinPlateSpline::fit).
    pub smoothing: f64,
    /// Un point est aberrant si son score dépasse `outlier_factor` fois le
    /// score médian des autres points (voir [`FitReport::outlier_scores`]).
    pub outlier_factor: f64,
    /// Score (unité de la cible) en dessous duquel un point n'est jamais
    /// déclaré aberrant : la précision qu'on attend d'un bon point. Zéro
    /// laisse le moteur prendre un millionième de l'étendue des points.
    pub outlier_floor: f64,
}

impl Default for FitOptions {
    fn default() -> Self {
        Self {
            smoothing: 0.0,
            outlier_factor: 6.0,
            outlier_floor: 0.0,
        }
    }
}

/// Compte rendu d'un ajustement.
///
/// Sauf mention contraire, les écarts sont dans l'unité des points cible.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitReport {
    pub point_count: usize,
    /// Matrice 3 × 3 du trapèze ajusté sur tous les points.
    pub homography: [[f64; 3]; 3],
    /// Ce que le trapèze seul laisse comme écart : la part de relief, de
    /// papier gondolé ou d'erreur de pointage qu'il n'explique pas.
    pub homography_residuals: ResidualStats,
    /// Les mêmes, ramenés côté source (unité de la source).
    pub homography_residuals_in_source: ResidualStats,
    /// Écarts de la déformation complète (trapèze + correction). Nuls, aux
    /// arrondis près, quand le lissage vaut 0.
    pub warp_residuals: ResidualStats,
    /// Écarts de validation croisée : chaque point comparé à la prédiction
    /// des autres. Pour un point aberrant, la prédiction ignore aussi les
    /// autres points aberrants. `None` avec moins de 6 points.
    pub leave_one_out: Option<ResidualStats>,
    /// Les mêmes, ramenés côté source (unité de la source).
    pub leave_one_out_in_source: Option<ResidualStats>,
    /// Score de chaque point : son écart de validation croisée, divisé par
    /// ce que sa position (bord, point isolé) amplifie naturellement. C'est
    /// ce score qui désigne les points aberrants. `None` avec moins de 6
    /// points.
    pub outlier_scores: Option<Vec<f64>>,
    /// Seuil de score appliqué au dernier point examiné. `None` avec moins
    /// de 6 points.
    pub outlier_threshold: Option<f64>,
    /// Indices (à partir de 0) des points aberrants, dans l'ordre où ils
    /// ont été écartés : le plus net d'abord.
    pub outliers: Vec<usize>,
}

/// Ajuste une déformation par points et rend son diagnostic.
///
/// La déformation rendue utilise **tous** les points, aberrants compris :
/// c'est à l'appelant (et à l'utilisateur) de corriger ou retirer un point
/// signalé, puis de relancer l'ajustement.
pub fn fit_with_report(
    source: &[Point],
    target: &[Point],
    options: &FitOptions,
) -> Result<(PointWarp, FitReport), GeometryError> {
    if !(options.outlier_factor.is_finite() && options.outlier_factor >= 1.0) {
        return Err(GeometryError::InvalidParameter(
            "le facteur de détection des points aberrants doit être au moins 1",
        ));
    }
    if !(options.outlier_floor.is_finite() && options.outlier_floor >= 0.0) {
        return Err(GeometryError::InvalidParameter(
            "le seuil minimal des points aberrants doit être positif ou nul",
        ));
    }
    let warp = PointWarp::fit(source, target, options.smoothing)?;
    let base = warp.base();
    let cross = cross_validate(&warp, source, target, options)?;
    let (leave_one_out, leave_one_out_in_source, outlier_scores, outlier_threshold, outliers) =
        match cross {
            Some(c) => {
                let in_source = c
                    .errors
                    .iter()
                    .zip(source)
                    .map(|(e, s)| {
                        warp.apply_with_jacobian(*s)
                            .and_then(|(_, jacobian)| jacobian_solve(&jacobian, *e))
                            .map_or(f64::INFINITY, Point::norm)
                    })
                    .collect();
                (
                    Some(ResidualStats::from_values(
                        c.errors.iter().map(|e| e.norm()).collect(),
                    )),
                    Some(ResidualStats::from_values(in_source)),
                    Some(c.scores),
                    Some(c.threshold),
                    c.outliers,
                )
            }
            None => (None, None, None, None, Vec::new()),
        };
    let report = FitReport {
        point_count: source.len(),
        homography: base.matrix(),
        homography_residuals: residuals(base, source, target),
        homography_residuals_in_source: residuals_in_source(base, source, target),
        warp_residuals: residuals(&warp, source, target),
        leave_one_out,
        leave_one_out_in_source,
        outlier_scores,
        outlier_threshold,
        outliers,
    };
    Ok((warp, report))
}

/// Nombre de points examinés à chaque tour de la recherche de points
/// aberrants.
const CANDIDATES_PER_ROUND: usize = 4;

/// Essai de retrait d'un point : somme des carrés des scores restants,
/// indice du point, inverse du système et écarts sans lui.
type Trial = (f64, usize, SquareMatrix, Vec<(Point, f64)>);

struct CrossValidation {
    /// Écart de validation croisée de chaque point (vecteur, côté cible).
    errors: Vec<Point>,
    /// Le même écart, ramené à l'échelle d'un point bien entouré.
    scores: Vec<f64>,
    threshold: f64,
    outliers: Vec<usize>,
}

/// Validation croisée « un point laissé de côté » et repérage des points
/// aberrants.
///
/// Refaire l'ajustement N fois serait trop long avec plusieurs centaines de
/// points. On utilise une identité exacte de la spline en plaque mince
/// (formule de Rippa) : l'écart du point `k` quand on l'ôte vaut
/// `coefficient_k / (M⁻¹)_kk`, où `M` est la matrice du système. Ôter
/// définitivement un point se fait par une mise à jour de `M⁻¹` en N²
/// opérations, sans refaire l'inversion.
///
/// **Score.** Un point au bord ou isolé est prédit par extrapolation : son
/// écart de validation croisée est naturellement plus grand, sans qu'il soit
/// faux. On divise donc l'écart par le facteur dont la géométrie amplifie
/// une erreur de pointage ordinaire (`‖ligne k de M⁻¹‖ / |(M⁻¹)_kk|`, qui
/// vaut au moins 1). Le score obtenu est comparable d'un point à l'autre.
///
/// **Repérage.** Glouton : parmi les quelques points de plus grand score, on
/// prend celui dont le retrait fait le plus baisser les scores des autres
/// (un point faux déforme la surface autour de lui et rend ses voisins
/// suspects : c'est en l'ôtant, lui, que tout rentre dans l'ordre). On le
/// déclare aberrant si son score dépasse `outlier_factor` fois le score
/// médian des points restants *recalculé sans lui* — un point faux ne peut
/// donc pas gonfler le seuil qui le juge. On recommence tant qu'on en trouve.
///
/// Le trapèze, lui, reste celui ajusté sur tous les points : la spline
/// contient une partie affine libre, qui rattrape l'essentiel de ce qu'un
/// point faux a fait bouger dans le trapèze.
fn cross_validate(
    warp: &PointWarp,
    source: &[Point],
    target: &[Point],
    options: &FitOptions,
) -> Result<Option<CrossValidation>, GeometryError> {
    let n = source.len();
    if n < MIN_POINTS_FOR_OUTLIERS {
        return Ok(None);
    }
    check_smoothing(options.smoothing)?;
    let base = warp.base();
    let mut leftovers = Vec::with_capacity(n);
    for (s, t) in source.iter().zip(target) {
        leftovers.push(*t - base.apply(*s).ok_or(GeometryError::HorizonCrossing)?);
    }
    let normalization = Normalization::of(source);
    let nodes: Vec<Point> = source.iter().map(|p| normalization.apply(*p)).collect();
    let mut inverse = factorize(system_matrix(&nodes, options.smoothing))
        .ok_or(GeometryError::DegenerateConfiguration)?
        .inverse();

    let floor = if options.outlier_floor > 0.0 {
        options.outlier_floor
    } else {
        1e-6 * extent(target)
    };
    let mut active = vec![true; n];
    let mut current = leave_one_out(&inverse, &leftovers, &active);
    let mut errors = vec![Point::new(0.0, 0.0); n];
    let mut scores = vec![0.0; n];
    let mut outliers = Vec::new();
    let mut threshold = floor;
    // On ne retire jamais plus d'un tiers des points, et on garde de quoi
    // ajuster une surface.
    let max_outliers = (n / 3).min(n - (MIN_POINTS_FOR_OUTLIERS - 1));
    while outliers.len() < max_outliers {
        // Candidats : les points de plus grand score. Un point faux gonfle
        // aussi le score de ses voisins ; parmi les candidats, le coupable
        // est celui dont le retrait assainit le mieux tous les autres.
        let mut candidates: Vec<usize> = (0..n).filter(|&i| active[i]).collect();
        candidates.sort_by(|&a, &b| current[b].1.total_cmp(&current[a].1));
        candidates.truncate(CANDIDATES_PER_ROUND);
        let mut best: Option<Trial> = None;
        for &candidate in &candidates {
            let mut trial = inverse.clone();
            if !remove_point(&mut trial, candidate) {
                continue;
            }
            active[candidate] = false;
            let trial_current = leave_one_out(&trial, &leftovers, &active);
            active[candidate] = true;
            let remaining: f64 = trial_current.iter().map(|(_, score)| score * score).sum();
            if best.as_ref().is_none_or(|(least, ..)| remaining < *least) {
                best = Some((remaining, candidate, trial, trial_current));
            }
        }
        let Some((_, suspect, trial, trial_current)) = best else {
            break;
        };
        // Le suspect est jugé à l'aune des autres points, sans lui.
        let mut others: Vec<f64> = (0..n)
            .filter(|&i| active[i] && i != suspect)
            .map(|i| trial_current[i].1)
            .collect();
        others.sort_by(f64::total_cmp);
        let median = others[others.len() / 2];
        threshold = floor.max(options.outlier_factor * median);
        if current[suspect].1 <= threshold {
            break;
        }
        (errors[suspect], scores[suspect]) = current[suspect];
        outliers.push(suspect);
        active[suspect] = false;
        inverse = trial;
        current = trial_current;
    }
    for i in (0..n).filter(|&i| active[i]) {
        (errors[i], scores[i]) = current[i];
    }
    Ok(Some(CrossValidation {
        errors,
        scores,
        threshold,
        outliers,
    }))
}

/// Diagonale du rectangle englobant.
fn extent(points: &[Point]) -> f64 {
    let (mut lo, mut hi) = (points[0], points[0]);
    for p in points {
        lo = Point::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    lo.distance(hi)
}

/// Écart de validation croisée (formule de Rippa) et score de chaque point
/// actif.
fn leave_one_out(inverse: &SquareMatrix, values: &[Point], active: &[bool]) -> Vec<(Point, f64)> {
    let n = values.len();
    let mut out = vec![(Point::new(0.0, 0.0), 0.0); n];
    for k in (0..n).filter(|&k| active[k]) {
        let row = inverse.row(k);
        let mut coefficient = Point::new(0.0, 0.0);
        let mut row_norm2 = 0.0;
        for (j, v) in values.iter().enumerate() {
            if active[j] {
                coefficient = coefficient + *v * row[j];
                row_norm2 += row[j] * row[j];
            }
        }
        let diagonal = row[k];
        if diagonal.abs() > 0.0 && row_norm2 > 0.0 {
            out[k] = (
                coefficient * (1.0 / diagonal),
                coefficient.norm() / row_norm2.sqrt(),
            );
        }
    }
    out
}

/// Met à jour l'inverse du système quand on ôte le point `k`. Renvoie
/// `false` si la mise à jour est impossible (pivot nul).
fn remove_point(inverse: &mut SquareMatrix, k: usize) -> bool {
    let size = inverse.n;
    let pivot = inverse.at(k, k);
    if !pivot.is_finite() || pivot == 0.0 {
        return false;
    }
    let column: Vec<f64> = (0..size).map(|i| inverse.at(i, k)).collect();
    let row: Vec<f64> = inverse.row(k).to_vec();
    for (i, c) in column.iter().enumerate() {
        let factor = c / pivot;
        if factor != 0.0 {
            let target = &mut inverse.data[i * size..(i + 1) * size];
            for (x, r) in target.iter_mut().zip(&row) {
                *x -= factor * r;
            }
        }
    }
    for i in 0..size {
        inverse.set(i, k, 0.0);
        inverse.set(k, i, 0.0);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::super::test_util::{realistic, slide_corners, slide_grid, trapezoid, Lcg};
    use super::super::Homography;
    use super::*;

    #[test]
    fn stats_of_known_values() {
        let stats = ResidualStats::from_values(vec![3.0, 4.0, 0.0, 5.0]);
        // √((9 + 16 + 0 + 25) / 4) = √12,5
        assert!((stats.rms - 12.5_f64.sqrt()).abs() < 1e-12);
        assert_eq!(stats.max, 5.0);
        assert_eq!(stats.worst_index, Some(3));
        let empty = ResidualStats::from_values(Vec::new());
        assert_eq!((empty.rms, empty.max, empty.worst_index), (0.0, 0.0, None));
    }

    #[test]
    fn residuals_of_a_known_shift() {
        // Cibles décalées de (3, 4) par rapport à l'identité : écart de 5
        // partout. Avec une échelle 2, l'écart côté source vaut 2,5.
        let source = slide_grid(4);
        let target: Vec<Point> = source.iter().map(|s| *s + Point::new(3.0, 4.0)).collect();
        let stats = residuals(&Homography::IDENTITY, &source, &target);
        assert!(stats.per_point.iter().all(|v| (v - 5.0).abs() < 1e-12));
        assert!((stats.rms - 5.0).abs() < 1e-12 && (stats.max - 5.0).abs() < 1e-12);
        let double = Homography::similarity(2.0, 0.0, 0.0, 0.0).unwrap();
        let target: Vec<Point> = source
            .iter()
            .map(|s| *s * 2.0 + Point::new(3.0, 4.0))
            .collect();
        let stats = residuals_in_source(&double, &source, &target);
        assert!(stats.per_point.iter().all(|v| (v - 2.5).abs() < 1e-12));
    }

    /// Douze points cliqués sur la diapositive (grille 4 × 3 un peu
    /// irrégulière) et leur position sur une façade en relief, à ± 0,3 mm.
    fn twelve_clicks(seed: u64) -> (Vec<Point>, Vec<Point>) {
        let mut rng = Lcg::new(seed);
        let mut source = Vec::new();
        for j in 0..3 {
            for i in 0..4 {
                source.push(
                    Point::new(25.0 + 77.0 * i as f64, 30.0 + 110.0 * j as f64) + rng.offset(15.0),
                );
            }
        }
        let target = source
            .iter()
            .map(|s| relief(*s) + rng.offset(0.3))
            .collect();
        (source, target)
    }

    /// Façade vue en biais avec un relief doux : trapèze marqué, plus un
    /// bombement d'environ 2 mm que le trapèze n'explique pas.
    fn relief(p: Point) -> Point {
        let d = p - Point::new(120.0, 160.0);
        let bulge = 2.0 * (-(d.x * d.x + d.y * d.y) / (2.0 * 180.0 * 180.0)).exp();
        trapezoid()
            .apply(p + Point::new(bulge, -0.6 * bulge))
            .unwrap()
    }

    #[test]
    fn one_bad_point_among_twelve_good_ones_is_singled_out() {
        // Un treizième point déplacé de 5 mm. Essayé à 40 endroits et avec 40
        // tirages différents : il est désigné à chaque fois, et lui seul.
        for seed in 0..40 {
            let (mut source, mut target) = twelve_clicks(seed);
            let mut rng = Lcg::new(1000 + seed);
            let bad = Point::new(30.0, 30.0) + rng.point_in_slide() * (220.0 / 280.0);
            let angle = rng.unit() * std::f64::consts::TAU;
            let position = (seed as usize) % 13;
            source.insert(position, bad);
            target.insert(
                position,
                relief(bad) + Point::new(angle.cos(), angle.sin()) * 5.0,
            );
            let (_, report) = fit_with_report(&source, &target, &FitOptions::default()).unwrap();
            assert_eq!(
                report.outliers,
                vec![position],
                "tirage {seed} : scores {:?}, seuil {:?}",
                report.outlier_scores,
                report.outlier_threshold
            );
            // L'écart annoncé pour ce point est proche des 5 mm réels.
            let announced = report.leave_one_out.unwrap().per_point[position];
            assert!((2.5..=9.0).contains(&announced), "{announced} mm");
        }
    }

    #[test]
    fn good_points_are_not_flagged() {
        let mut flagged = 0;
        for seed in 0..40 {
            let (source, target) = twelve_clicks(seed);
            let (_, report) = fit_with_report(&source, &target, &FitOptions::default()).unwrap();
            flagged += report.outliers.len();
        }
        assert_eq!(flagged, 0, "fausses alertes sur 40 tirages de 12 points");
    }

    #[test]
    fn exact_data_raise_no_alarm() {
        // Données parfaites : les écarts sont des arrondis de calcul, il ne
        // faut pas y chercher de point aberrant.
        let truth = trapezoid();
        let source = slide_grid(5);
        let target: Vec<Point> = source.iter().map(|s| truth.apply(*s).unwrap()).collect();
        let (_, report) = fit_with_report(&source, &target, &FitOptions::default()).unwrap();
        assert!(report.outliers.is_empty());
        assert!(report.homography_residuals.max < 1e-9);
        assert!(report.leave_one_out.unwrap().max < 1e-7);
    }

    #[test]
    fn leave_one_out_matches_an_actual_refit() {
        // La formule rapide donne le même écart que l'ajustement refait sans
        // le point (à trapèze constant).
        let (source, target) = twelve_clicks(3);
        let options = FitOptions {
            outlier_factor: 1e9,
            ..FitOptions::default()
        };
        let (warp, report) = fit_with_report(&source, &target, &options).unwrap();
        let fast = report.leave_one_out.unwrap().per_point;
        let base = warp.base();
        let leftovers: Vec<Point> = source
            .iter()
            .zip(&target)
            .map(|(s, t)| *t - base.apply(*s).unwrap())
            .collect();
        for k in 0..source.len() {
            let mut s = source.clone();
            let mut v = leftovers.clone();
            s.remove(k);
            v.remove(k);
            let spline = super::super::ThinPlateSpline::fit(&s, &v, 0.0).unwrap();
            let slow = (leftovers[k] - spline.value(source[k])).norm();
            assert!(
                (fast[k] - slow).abs() < 1e-8,
                "point {k} : {} ≠ {slow}",
                fast[k]
            );
        }
    }

    #[test]
    fn several_bad_detections_among_mire_intersections() {
        // 225 intersections de mire détectées à ± 0,05 mm, dont trois
        // fausses (1,5 à 4 mm d'erreur).
        let source = slide_grid(15);
        let mut rng = Lcg::new(77);
        let mut target: Vec<Point> = source
            .iter()
            .map(|s| realistic(*s) + rng.offset(0.05))
            .collect();
        target[17] = target[17] + Point::new(4.0, 0.0);
        target[112] = target[112] + Point::new(-1.5, 1.0);
        target[200] = target[200] + Point::new(0.0, -2.5);
        let (_, report) = fit_with_report(&source, &target, &FitOptions::default()).unwrap();
        // Les trois fausses détections sortent en premier, la plus nette en
        // tête.
        assert_eq!(report.outliers[0], 17);
        let mut first_three = report.outliers[..3].to_vec();
        first_three.sort_unstable();
        assert_eq!(first_three, vec![17, 112, 200]);
        // Limite connue : au bord de la mire, la surface est extrapolée et
        // un bon point peut être signalé à tort. Ici, 2 au plus sur 222, tous
        // sur le bord.
        let extras = &report.outliers[3..];
        eprintln!(
            "mire de 225 points, 3 faux : {} bon(s) point(s) du bord signalé(s) à tort {extras:?}",
            extras.len()
        );
        assert!(extras.len() <= 2, "{extras:?}");
        for &k in extras {
            let (i, j) = (k % 15, k / 15);
            assert!(i == 0 || i == 14 || j == 0 || j == 14, "point {k}");
        }
        // En donnant la précision attendue d'une détection (ici 1 mm), il ne
        // reste que les trois vrais.
        let strict = FitOptions {
            outlier_floor: 1.0,
            ..FitOptions::default()
        };
        let (_, report) = fit_with_report(&source, &target, &strict).unwrap();
        let mut found = report.outliers.clone();
        found.sort_unstable();
        assert_eq!(found, vec![17, 112, 200]);
    }

    #[test]
    fn floor_silences_small_deviations() {
        let (mut source, mut target) = twelve_clicks(5);
        source.push(Point::new(150.0, 150.0));
        target.push(relief(Point::new(150.0, 150.0)) + Point::new(5.0, 0.0));
        let tolerant = FitOptions {
            outlier_floor: 20.0,
            ..FitOptions::default()
        };
        let (_, report) = fit_with_report(&source, &target, &tolerant).unwrap();
        assert!(report.outliers.is_empty());
        assert_eq!(report.outlier_threshold, Some(20.0));
        let (_, report) = fit_with_report(&source, &target, &FitOptions::default()).unwrap();
        assert_eq!(report.outliers, vec![12]);
    }

    #[test]
    fn too_few_points_for_a_diagnosis() {
        // 4 points : trapèze exact, aucun diagnostic possible.
        let truth = trapezoid();
        let source = slide_corners();
        let target = source.map(|s| truth.apply(s).unwrap());
        let (_, report) = fit_with_report(&source, &target, &FitOptions::default()).unwrap();
        assert_eq!(report.point_count, 4);
        assert!(report.homography_residuals.max < 1e-9);
        assert!(report.leave_one_out.is_none() && report.outlier_threshold.is_none());
        assert!(report.outliers.is_empty());
    }

    #[test]
    fn invalid_options_are_refused() {
        let grid = slide_grid(3);
        for options in [
            FitOptions {
                outlier_factor: 0.5,
                ..FitOptions::default()
            },
            FitOptions {
                outlier_floor: -1.0,
                ..FitOptions::default()
            },
            FitOptions {
                smoothing: -0.1,
                ..FitOptions::default()
            },
        ] {
            assert!(matches!(
                fit_with_report(&grid, &grid, &options),
                Err(GeometryError::InvalidParameter(_))
            ));
        }
    }

    #[test]
    fn report_serializes_with_camel_case_keys() {
        let (source, target) = twelve_clicks(1);
        let (_, report) = fit_with_report(&source, &target, &FitOptions::default()).unwrap();
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["pointCount"], 12);
        assert!(json["homographyResiduals"]["perPoint"].is_array());
        assert!(json["leaveOneOut"]["rms"].is_number());
        assert!(json["outliers"].is_array());
    }
}

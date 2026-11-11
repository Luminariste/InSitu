//! Géométrie : correspondances entre deux plans (photo, diapositive, façade).
//!
//! Le module sert deux fois dans l'app :
//!
//! 1. **redressement** : de la photo brute (pixels) vers la diapositive (mm),
//!    à partir des intersections détectées de la grille de la mire ;
//! 2. **recalage** : de la diapositive (mm) vers la façade (mm), à partir de
//!    points cliqués.
//!
//! Dans les deux cas le modèle est le même, en deux étages :
//!
//! - un **trapèze** ([`Homography`]), exact avec 4 paires de points, ajusté
//!   aux moindres carrés au-delà ;
//! - une **correction locale lisse** par-dessus ([`PointWarp`]), une spline en
//!   plaque mince qui absorbe ce que le trapèze n'explique pas (papier qui
//!   gondole, objectif, relief, façade déroulée).
//!
//! Pour déformer des images entières, [`GridWarp`] échantillonne cette
//! correction sur une grille régulière : appliquer la transformation à un
//! point coûte alors un temps constant, dans les deux sens.
//!
//! Vocabulaire : une transformation va d'un plan **source** vers un plan
//! **cible** ([`Transform::apply`]) ; le sens inverse revient de la cible vers
//! la source ([`Transform::apply_inverse`]). Les unités sont celles des
//! points fournis (pixels à l'entrée d'une photo, millimètres ailleurs).

mod diagnostics;
mod grid;
mod homography;
mod linalg;
mod spline;
#[cfg(test)]
pub(crate) mod test_util;
mod warp;

use std::fmt;
use std::ops::{Add, Mul, Sub};

use serde::{Deserialize, Serialize};

pub use diagnostics::{
    fit_with_report, residuals, residuals_in_source, FitOptions, FitReport, ResidualStats,
    MIN_POINTS_FOR_OUTLIERS,
};
pub use grid::{GridWarp, Rect};
pub use homography::Homography;
pub use spline::ThinPlateSpline;
pub use warp::{has_fold, PointWarp};

/// Un point du plan. S'écrit `[x, y]` en JSON.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(from = "[f64; 2]", into = "[f64; 2]")]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Longueur du vecteur allant de l'origine à ce point.
    pub fn norm(self) -> f64 {
        self.x.hypot(self.y)
    }

    pub fn distance(self, other: Point) -> f64 {
        (self - other).norm()
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl From<[f64; 2]> for Point {
    fn from(v: [f64; 2]) -> Self {
        Self::new(v[0], v[1])
    }
}

impl From<Point> for [f64; 2] {
    fn from(p: Point) -> Self {
        [p.x, p.y]
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, o: Point) -> Point {
        Point::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Point {
    type Output = Point;
    fn sub(self, o: Point) -> Point {
        Point::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f64> for Point {
    type Output = Point;
    fn mul(self, k: f64) -> Point {
        Point::new(self.x * k, self.y * k)
    }
}

/// Dérivée d'une transformation en un point : `j[ligne][colonne]`, ligne =
/// coordonnée de sortie, colonne = coordonnée d'entrée.
pub type Jacobian = [[f64; 2]; 2];

pub(crate) fn jacobian_det(j: &Jacobian) -> f64 {
    j[0][0] * j[1][1] - j[0][1] * j[1][0]
}

/// Résout `j · d = v`. `None` si la dérivée n'est pas inversible.
pub(crate) fn jacobian_solve(j: &Jacobian, v: Point) -> Option<Point> {
    let det = jacobian_det(j);
    if !det.is_finite() || det.abs() < 1e-300 {
        return None;
    }
    Some(Point::new(
        (j[1][1] * v.x - j[0][1] * v.y) / det,
        (j[0][0] * v.y - j[1][0] * v.x) / det,
    ))
}

/// Interface commune des transformations du plan.
///
/// Les deux sens renvoient `None` quand le point n'a pas d'image : point
/// envoyé à l'infini par un trapèze, ou sens inverse introuvable (voir
/// [`PointWarp`] et [`GridWarp`] pour les limites).
pub trait Transform {
    /// Sens direct : de la source vers la cible.
    fn apply(&self, p: Point) -> Option<Point>;
    /// Sens inverse : de la cible vers la source.
    fn apply_inverse(&self, p: Point) -> Option<Point>;
}

/// Transformation dont on sait calculer la dérivée (sens direct).
pub trait Differentiable: Transform {
    fn apply_with_jacobian(&self, p: Point) -> Option<(Point, Jacobian)>;
}

/// Côté d'une paire de points, pour désigner où se trouve un problème.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Source,
    Target,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Side::Source => "source",
            Side::Target => "cible",
        })
    }
}

/// Ce qui peut empêcher un ajustement. Les indices sont ceux des paires
/// fournies, à partir de 0 ; les messages les affichent à partir de 1.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryError {
    /// Les deux listes de points n'ont pas la même longueur.
    MismatchedLengths {
        source: usize,
        target: usize,
    },
    NotEnoughPoints {
        needed: usize,
        got: usize,
    },
    /// Une coordonnée vaut l'infini ou n'est pas un nombre.
    NonFiniteCoordinate {
        index: usize,
        side: Side,
    },
    /// Deux points occupent la même position.
    CoincidentPoints {
        first: usize,
        second: usize,
        side: Side,
    },
    /// Points alignés (ou presque) : le trapèze n'est pas déterminé.
    DegenerateConfiguration,
    /// Le trapèze trouvé envoie une partie des points « derrière l'horizon » :
    /// les points ne sont pas donnés dans le même ordre des deux côtés, ou le
    /// quadrilatère est croisé.
    HorizonCrossing,
    /// La transformation n'a pas d'inverse.
    SingularTransform,
    /// Un réglage est hors de son domaine ; le texte dit lequel.
    InvalidParameter(&'static str),
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MismatchedLengths { source, target } => write!(
                f,
                "il y a {source} points source pour {target} points cible : il en faut autant de chaque côté"
            ),
            Self::NotEnoughPoints { needed, got } => write!(
                f,
                "il faut au moins {needed} paires de points, {got} fournies"
            ),
            Self::NonFiniteCoordinate { index, side } => write!(
                f,
                "le point {side} n° {} a une coordonnée qui n'est pas un nombre fini",
                index + 1
            ),
            Self::CoincidentPoints {
                first,
                second,
                side,
            } => write!(
                f,
                "les points {side} n° {} et n° {} sont au même endroit",
                first + 1,
                second + 1
            ),
            Self::DegenerateConfiguration => f.write_str(
                "les points sont alignés ou presque : ils ne suffisent pas à définir un trapèze",
            ),
            Self::HorizonCrossing => f.write_str(
                "les points ne se correspondent pas : le trapèze obtenu est croisé (vérifier l'ordre des points)",
            ),
            Self::SingularTransform => f.write_str(
                "la transformation écrase le plan sur une ligne : elle n'a pas d'inverse",
            ),
            Self::InvalidParameter(what) => write!(f, "réglage invalide : {what}"),
        }
    }
}

impl std::error::Error for GeometryError {}

/// Vérifie que les deux listes forment des paires utilisables.
pub(crate) fn validate_pairs(
    source: &[Point],
    target: &[Point],
    needed: usize,
) -> Result<(), GeometryError> {
    if source.len() != target.len() {
        return Err(GeometryError::MismatchedLengths {
            source: source.len(),
            target: target.len(),
        });
    }
    if source.len() < needed {
        return Err(GeometryError::NotEnoughPoints {
            needed,
            got: source.len(),
        });
    }
    for (side, points) in [(Side::Source, source), (Side::Target, target)] {
        if let Some(index) = points.iter().position(|p| !p.is_finite()) {
            return Err(GeometryError::NonFiniteCoordinate { index, side });
        }
    }
    for (side, points) in [(Side::Source, source), (Side::Target, target)] {
        if let Some((first, second)) = find_coincident(points) {
            return Err(GeometryError::CoincidentPoints {
                first,
                second,
                side,
            });
        }
    }
    Ok(())
}

/// Première paire de points confondus, à un milliardième de l'étendue près.
fn find_coincident(points: &[Point]) -> Option<(usize, usize)> {
    let first = *points.first()?;
    let (mut lo, mut hi) = (first, first);
    for p in points {
        lo = Point::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let tolerance = 1e-9 * lo.distance(hi);
    for (i, a) in points.iter().enumerate() {
        for (j, b) in points.iter().enumerate().skip(i + 1) {
            if a.distance(*b) <= tolerance {
                return Some((i, j));
            }
        }
    }
    None
}

/// Changement de repère qui centre un nuage de points et ramène sa distance
/// moyenne au centre à √2. Indispensable à la stabilité numérique : sans
/// lui, des coordonnées en milliers de pixels font perdre la moitié des
/// chiffres significatifs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Normalization {
    pub center: Point,
    pub scale: f64,
}

impl Normalization {
    /// Suppose au moins deux points distincts (voir [`validate_pairs`]).
    pub fn of(points: &[Point]) -> Self {
        let n = points.len() as f64;
        let sum = points.iter().fold(Point::new(0.0, 0.0), |a, p| a + *p);
        let center = sum * (1.0 / n);
        let mean_distance = points.iter().map(|p| p.distance(center)).sum::<f64>() / n;
        Self {
            center,
            scale: std::f64::consts::SQRT_2 / mean_distance,
        }
    }

    pub fn apply(&self, p: Point) -> Point {
        (p - self.center) * self.scale
    }
}

//! Redressement : de la photo brute de la mire (pixels) à l'image redressée
//! de la diapositive (280 mm), avec une carte de confiance.
//!
//! Méthode (étude 2.1, `docs/etudes/2.1-detection-mire.md`) :
//!
//! 1. **Amorce** : où est la mire, et dans quel sens. Pour l'ancienne mire,
//!    par le cercle rouge, la croix puis les six petits cercles (module
//!    `seed`) ; à défaut, par quatre coins donnés à la main
//!    ([`DetectOptions::corners`]).
//! 2. **Grille** : les croisements de la grille sont mesurés de proche en
//!    proche (module `nodes`), puis les mesures incohérentes sont écartées.
//! 3. **Modèle** : trapèze + correction en plaque mince, ajustés par le module
//!    [`geometry`](crate::geometry) sur tous les croisements. C'est la
//!    grille qui donne la précision, pas l'amorce.
//! 4. **Sorties** : l'image redressée ([`rectify`]), la carte de confiance
//!    ([`TargetDetection::confidence_map`]), les correspondances pour le
//!    fichier projet ([`TargetDetection::to_project_rectification`]).
//!
//! Les repères codés de la nouvelle mire (tâche 2.2) ne sont pas encore lus :
//! ce sera une autre amorce, la suite est commune.

mod legacy_target;
mod local_model;
mod nodes;
pub mod projector;
mod raster;
mod render;
mod seed;

use std::fmt;
use std::path::Path;

use image::{GrayImage, Luma, RgbImage};
use serde::Serialize;

use crate::geometry::{
    fit_with_report, FitOptions, GeometryError, GridWarp, Homography, Point, Rect, Transform,
};
use crate::project::{
    PointMm, Rectification, RectificationMethod, RectificationPoint, SourcePoint,
};
use crate::units::{self, SLIDE_SIZE_MM};
use legacy_target::{is_measurable, measurable_count, rotate_quarters, Node};
use local_model::local_scale;
use nodes::{grow_and_clean, Measured};
use raster::ColorPlanes;
use seed::{find_orientation, seed_from_circle_and_cross, MIN_ORIENTATION_MARGIN};

/// Lissage de la plaque mince du modèle final (voir D-2.4-d).
pub const DEFAULT_SMOOTHING: f64 = 0.0;
/// Pas de la grille qui échantillonne la déformation pour redresser l'image,
/// en mm (voir D-2.3-b).
const RENDER_GRID_STEP_MM: f64 = 2.5;
/// Une zone de la mire est « couverte » si un croisement mesuré est à moins
/// de cette distance, en mm.
pub const COVERAGE_RADIUS_MM: f64 = 8.0;
/// Côté d'une case de la carte de confiance, en mm.
pub const CONFIDENCE_CELL_MM: f64 = 2.0;

/// Réglages de la détection.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DetectOptions {
    /// Position approchée dans la photo (px) des quatre coins de la mire :
    /// dans n'importe quel ordre (voir D-2.5-i). Si elle est donnée, elle
    /// remplace la recherche du cercle rouge et de la croix ; le sens de la
    /// mire est ensuite lu sur les six petits cercles, et la grille affine.
    pub corners: Option<[Point; 4]>,
    /// Lissage de la plaque mince (0 : passe par les croisements).
    pub smoothing: f64,
}

/// Comment la mire a été trouvée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SeedMethod {
    /// Cercle rouge, croix et petits cercles de l'ancienne mire.
    RedCircleAndCross,
    /// Quatre coins donnés à la main.
    Corners,
}

/// Échec de la détection, avec sa cause en clair.
#[derive(Debug, Clone, PartialEq)]
pub enum DetectionError {
    /// La mire n'a pas été trouvée ou pas assez bien vue.
    NotFound(String),
    /// Le modèle n'a pas pu être ajusté (cas anormal).
    Geometry(GeometryError),
    /// Fichier illisible ou impossible à écrire.
    Image(String),
}

impl fmt::Display for DetectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(cause) => write!(f, "mire non détectée : {cause}"),
            Self::Geometry(e) => write!(f, "redressement impossible : {e}"),
            Self::Image(e) => f.write_str(e),
        }
    }
}

impl std::error::Error for DetectionError {}

impl From<GeometryError> for DetectionError {
    fn from(e: GeometryError) -> Self {
        Self::Geometry(e)
    }
}

/// Résumé d'une série d'écarts, en mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorSummary {
    pub mean: f64,
    pub p95: f64,
    pub max: f64,
}

impl ErrorSummary {
    pub fn of(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self {
                mean: 0.0,
                p95: 0.0,
                max: 0.0,
            };
        }
        let mut v = values.to_vec();
        v.sort_unstable_by(f64::total_cmp);
        let rank = ((v.len() - 1) as f64 * 0.95).round() as usize;
        Self {
            mean: v.iter().sum::<f64>() / v.len() as f64,
            p95: v[rank],
            max: v[v.len() - 1],
        }
    }
}

/// Un croisement de la grille, mesuré dans la photo.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedNode {
    /// Position sur la diapositive, en mm.
    pub slide: Point,
    /// Position dans la photo brute, en px (le centre du premier pixel est
    /// en 0,5 ; disposition des pixels du fichier, sans rotation EXIF).
    pub photo: Point,
    /// Netteté de la mesure (rapport signal sur bruit du moins net des deux
    /// traits).
    pub score: f64,
    /// Écart entre ce croisement et ce qu'en prédisent les autres, en mm de
    /// diapositive : un majorant de l'erreur du redressement à cet endroit.
    pub leave_one_out_mm: f64,
}

/// Résultat de la détection de la mire dans une photo brute.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetDetection {
    pub method: SeedMethod,
    pub photo_width: u32,
    pub photo_height: u32,
    /// Croisements retenus pour le modèle.
    pub nodes: Vec<DetectedNode>,
    /// Croisements que la mire permet de mesurer (804 pour l'ancienne).
    pub measurable_nodes: usize,
    /// Croisements mesurés avant le tri des mesures incohérentes.
    pub raw_nodes: usize,
    /// Croisements écartés par le diagnostic final (points aberrants).
    pub outliers_removed: usize,
    /// Quarts de tour entre l'amorce et le sens de la mire (information).
    pub quarter_turns: u8,
    /// Écart entre la note du sens retenu et la suivante (petits cercles) ;
    /// absent quand le sens est donné par les coins.
    pub orientation_margin: Option<f64>,
    /// Secteurs de 10° où le cercle rouge a été vu (sur 36).
    pub red_circle_sectors: Option<usize>,
    /// Taille d'un mm de mire dans la photo, au centre, en px.
    pub px_per_mm: f64,
    /// Ce qu'un simple trapèze laisse comme écart aux croisements, en mm :
    /// papier qui gondole, objectif.
    pub homography_residual_mm: ErrorSummary,
    /// Validation croisée (chaque croisement prédit par les autres), en mm.
    pub leave_one_out_mm: ErrorSummary,
    /// Part de la mire à moins de 8 mm d'un croisement mesuré (0 à 1).
    pub coverage: f64,
    /// Lissage de la plaque mince utilisé.
    pub smoothing: f64,
    /// Déformation mm de diapositive → px de photo.
    #[serde(skip)]
    pub warp: GridWarp,
}

/// Carte de confiance : pour chaque case de la diapositive, l'erreur
/// estimée du redressement, en mm, ou `None` si la case n'est pas couverte
/// (aucun croisement mesuré à moins de 8 mm).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidenceMap {
    pub cell_mm: f64,
    pub columns: usize,
    pub rows: usize,
    /// Ligne par ligne, depuis le coin haut-gauche.
    pub error_mm: Vec<Option<f64>>,
}

impl ConfidenceMap {
    /// Image de la carte, une case par pixel : blanc = sûr (0 mm), noir =
    /// non couvert ou erreur estimée d'1 mm et plus.
    pub fn to_image(&self) -> GrayImage {
        GrayImage::from_fn(self.columns as u32, self.rows as u32, |i, j| {
            let v = self.error_mm[j as usize * self.columns + i as usize];
            Luma([v.map_or(0, |e| (255.0 * (1.0 - e.clamp(0.0, 1.0))).round() as u8)])
        })
    }

    /// Erreur estimée au point `p` (mm de diapositive).
    pub fn at(&self, p: Point) -> Option<f64> {
        let i = (p.x / self.cell_mm).floor();
        let j = (p.y / self.cell_mm).floor();
        if i < 0.0 || j < 0.0 || i as usize >= self.columns || j as usize >= self.rows {
            return None;
        }
        self.error_mm[j as usize * self.columns + i as usize]
    }
}

/// Détecte la mire dans la photo brute et ajuste le redressement.
pub fn detect(
    photo: &RgbImage,
    options: &DetectOptions,
) -> Result<TargetDetection, DetectionError> {
    let color = ColorPlanes::from_rgb(photo);
    let gray = color.gray();
    let not_found = DetectionError::NotFound;
    let (nodes, raw_count, method, quarters, margin, sectors) = match options.corners {
        Some(corners) => {
            let corners = order_corners(corners);
            let slide = [
                Point::new(0.0, 0.0),
                Point::new(SLIDE_SIZE_MM, 0.0),
                Point::new(SLIDE_SIZE_MM, SLIDE_SIZE_MM),
                Point::new(0.0, SLIDE_SIZE_MM),
            ];
            let h0 = Homography::from_four_pairs(slide, corners).map_err(|_| {
                not_found("les quatre coins donnés ne forment pas un quadrilatère".into())
            })?;
            let every = |_: Node| true;
            let (nodes, raw) =
                grow_and_clean(&gray, &[], Some((&h0, &every))).map_err(not_found)?;
            // Les coins ne disent pas le sens de la mire : on le lit sur les
            // petits cercles. S'il reste incertain, on garde le sens de la
            // photo (le coin le plus en haut à gauche est le haut-gauche).
            match find_orientation(&gray, &nodes) {
                Some(o) if o.margin >= MIN_ORIENTATION_MARGIN => (
                    reorient(&nodes, o.quarters),
                    raw,
                    SeedMethod::Corners,
                    o.quarters,
                    Some(o.margin),
                    None,
                ),
                _ => (nodes, raw, SeedMethod::Corners, 0, None, None),
            }
        }
        None => {
            let seed = seed_from_circle_and_cross(&color, &gray).map_err(not_found)?;
            let orientation = find_orientation(&gray, &seed.nodes)
                .ok_or_else(|| not_found("sens de la mire introuvable".into()))?;
            if orientation.margin < MIN_ORIENTATION_MARGIN {
                return Err(not_found(format!(
                    "sens de la mire incertain : les six petits cercles ne se distinguent pas (écart {:.3}, il faut {MIN_ORIENTATION_MARGIN})",
                    orientation.margin
                )));
            }
            (
                reorient(&seed.nodes, orientation.quarters),
                seed.raw_count,
                SeedMethod::RedCircleAndCross,
                orientation.quarters,
                Some(orientation.margin),
                Some(seed.circle_sectors),
            )
        }
    };
    fit_model(
        photo,
        nodes,
        raw_count,
        method,
        quarters,
        margin,
        sectors,
        options.smoothing,
    )
}

/// Passe des coordonnées de l'amorce aux coordonnées vraies de la
/// diapositive, `quarters` étant le résultat de [`find_orientation`].
fn reorient(nodes: &[Measured], quarters: u8) -> Vec<Measured> {
    let back = (4 - quarters) % 4;
    nodes
        .iter()
        .filter_map(|m| {
            let p = rotate_quarters(m.node.position(), back);
            let node = Node {
                i: (p.x / legacy_target::GRID_STEP_MM).round() as usize,
                j: (p.y / legacy_target::GRID_STEP_MM).round() as usize,
            };
            is_measurable(node).then_some(Measured { node, ..*m })
        })
        .collect()
}

/// Range quatre coins posés dans n'importe quel ordre : sens des aiguilles
/// d'une montre à l'écran (y vers le bas), en commençant par le plus proche
/// du coin haut-gauche de la photo.
pub fn order_corners(corners: [Point; 4]) -> [Point; 4] {
    let cx = corners.iter().map(|p| p.x).sum::<f64>() / 4.0;
    let cy = corners.iter().map(|p| p.y).sum::<f64>() / 4.0;
    let mut sorted = corners;
    // atan2 croissant = sens horaire à l'écran, puisque y descend.
    sorted.sort_by(|a, b| {
        (a.y - cy)
            .atan2(a.x - cx)
            .total_cmp(&(b.y - cy).atan2(b.x - cx))
    });
    let first = (0..4)
        .min_by(|&a, &b| (sorted[a].x + sorted[a].y).total_cmp(&(sorted[b].x + sorted[b].y)))
        .unwrap_or(0);
    sorted.rotate_left(first);
    sorted
}

#[allow(clippy::too_many_arguments)]
fn fit_model(
    photo: &RgbImage,
    mut nodes: Vec<Measured>,
    raw_nodes: usize,
    method: SeedMethod,
    quarter_turns: u8,
    orientation_margin: Option<f64>,
    red_circle_sectors: Option<usize>,
    smoothing: f64,
) -> Result<TargetDetection, DetectionError> {
    nodes.sort_by_key(|m| m.node);
    let mut outliers_removed = 0;
    let (warp, report, nodes) = loop {
        let mm: Vec<Point> = nodes.iter().map(|m| m.node.position()).collect();
        let px: Vec<Point> = nodes.iter().map(|m| m.photo).collect();
        let scale =
            Homography::fit(&mm, &px).map_or(6.0, |h| local_scale(&h, Point::new(140.0, 140.0)));
        let options = FitOptions {
            smoothing,
            // Un bon croisement est mesuré à mieux que 0,15 mm (étude 2.1).
            outlier_floor: 0.15 * scale,
            ..FitOptions::default()
        };
        let (warp, report) = fit_with_report(&mm, &px, &options)?;
        if report.outliers.is_empty() || outliers_removed > 0 {
            break (warp, report, nodes);
        }
        outliers_removed = report.outliers.len();
        let bad: std::collections::HashSet<usize> = report.outliers.iter().copied().collect();
        nodes = nodes
            .into_iter()
            .enumerate()
            .filter(|(k, _)| !bad.contains(k))
            .map(|(_, m)| m)
            .collect();
    };
    let photo_size = (photo.width(), photo.height());
    let nodes: Vec<(Point, Point, f64)> = nodes
        .iter()
        .map(|m| (m.node.position(), m.photo, m.score))
        .collect();
    assemble(
        photo_size,
        &nodes,
        &warp,
        &report,
        Seed {
            method,
            raw_nodes,
            outliers_removed,
            quarter_turns,
            orientation_margin,
            red_circle_sectors,
        },
        smoothing,
    )
}

/// Ce que l'amorce apporte au compte rendu.
struct Seed {
    method: SeedMethod,
    raw_nodes: usize,
    outliers_removed: usize,
    quarter_turns: u8,
    orientation_margin: Option<f64>,
    red_circle_sectors: Option<usize>,
}

/// Compte rendu complet à partir du modèle ajusté. `nodes` : position sur
/// la diapositive (mm), dans la photo (px), netteté de la mesure.
fn assemble(
    (photo_width, photo_height): (u32, u32),
    nodes: &[(Point, Point, f64)],
    warp: &crate::geometry::PointWarp,
    report: &crate::geometry::FitReport,
    seed: Seed,
    smoothing: f64,
) -> Result<TargetDetection, DetectionError> {
    let px_per_mm = local_scale(warp.base(), Point::new(140.0, 140.0));
    let loo: Vec<f64> = report
        .leave_one_out_in_source
        .as_ref()
        .map_or_else(|| vec![0.0; nodes.len()], |s| s.per_point.clone());
    let homography_residual: Vec<f64> = report.homography_residuals_in_source.per_point.clone();
    let region = Rect::new(
        Point::new(-10.0, -10.0),
        Point::new(SLIDE_SIZE_MM + 10.0, SLIDE_SIZE_MM + 10.0),
    )?;
    let grid = GridWarp::from_point_warp(warp, region, RENDER_GRID_STEP_MM)?;
    let detected: Vec<DetectedNode> = nodes
        .iter()
        .zip(&loo)
        .map(|(&(slide, photo, score), e)| DetectedNode {
            slide,
            photo,
            score,
            leave_one_out_mm: *e,
        })
        .collect();
    let coverage = coverage(&detected);
    Ok(TargetDetection {
        method: seed.method,
        photo_width,
        photo_height,
        measurable_nodes: measurable_count(),
        raw_nodes: seed.raw_nodes,
        outliers_removed: seed.outliers_removed,
        quarter_turns: seed.quarter_turns,
        orientation_margin: seed.orientation_margin,
        red_circle_sectors: seed.red_circle_sectors,
        px_per_mm,
        homography_residual_mm: ErrorSummary::of(&homography_residual),
        leave_one_out_mm: ErrorSummary::of(&loo),
        coverage,
        smoothing,
        nodes: detected,
        warp: grid,
    })
}

/// Reconstruit le redressement à partir des correspondances enregistrées
/// dans le projet, sans relire la photo : même modèle (trapèze + plaque
/// mince, D-2.4-b) sur les mêmes points, donc la même image redressée.
/// Les mesures propres à l'amorce (sens, cercle rouge) ne sont pas connues.
pub fn from_rectification(
    photo_size: (u32, u32),
    rectification: &Rectification,
) -> Result<TargetDetection, DetectionError> {
    let points = &rectification.points;
    if points.len() < 4 {
        return Err(DetectionError::NotFound(format!(
            "le projet ne garde que {} point(s) de redressement, il en faut au moins 4",
            points.len()
        )));
    }
    let mm: Vec<Point> = points
        .iter()
        .map(|p| Point::new(p.slide.x, p.slide.y))
        .collect();
    let px: Vec<Point> = points
        .iter()
        .map(|p| Point::new(p.photo.x, p.photo.y))
        .collect();
    let options = FitOptions {
        smoothing: DEFAULT_SMOOTHING,
        ..FitOptions::default()
    };
    let (warp, report) = fit_with_report(&mm, &px, &options)?;
    let nodes: Vec<(Point, Point, f64)> = mm.iter().zip(&px).map(|(&a, &b)| (a, b, 0.0)).collect();
    let method = match rectification.method {
        RectificationMethod::Automatic => SeedMethod::RedCircleAndCross,
        RectificationMethod::Manual => SeedMethod::Corners,
    };
    assemble(
        photo_size,
        &nodes,
        &warp,
        &report,
        Seed {
            method,
            raw_nodes: points.len(),
            outliers_removed: 0,
            quarter_turns: 0,
            orientation_margin: None,
            red_circle_sectors: None,
        },
        DEFAULT_SMOOTHING,
    )
}

/// Part de la diapositive (échantillonnée au mm) à moins de
/// [`COVERAGE_RADIUS_MM`] d'un croisement.
fn coverage(nodes: &[DetectedNode]) -> f64 {
    let mut covered = 0usize;
    let n = SLIDE_SIZE_MM as usize;
    for j in 0..n {
        for i in 0..n {
            let p = Point::new(i as f64 + 0.5, j as f64 + 0.5);
            if nodes
                .iter()
                .any(|m| m.slide.distance(p) <= COVERAGE_RADIUS_MM)
            {
                covered += 1;
            }
        }
    }
    covered as f64 / (n * n) as f64
}

impl TargetDetection {
    /// Carte de confiance, par cases de [`CONFIDENCE_CELL_MM`].
    ///
    /// Une case est couverte si un croisement est à moins de 8 mm de son
    /// centre. Son erreur estimée est la moyenne quadratique, pondérée par
    /// la distance (gaussienne de 10 mm), des écarts de validation croisée
    /// des croisements voisins : un majorant, puisque chaque croisement y est
    /// prédit sans lui-même.
    pub fn confidence_map(&self) -> ConfidenceMap {
        let cell = CONFIDENCE_CELL_MM;
        let count = (SLIDE_SIZE_MM / cell).round() as usize;
        let mut error_mm = Vec::with_capacity(count * count);
        for j in 0..count {
            for i in 0..count {
                let p = Point::new((i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell);
                let nearest = self
                    .nodes
                    .iter()
                    .map(|m| m.slide.distance(p))
                    .fold(f64::INFINITY, f64::min);
                if nearest > COVERAGE_RADIUS_MM {
                    error_mm.push(None);
                    continue;
                }
                let (mut num, mut den) = (0.0, 0.0);
                for m in &self.nodes {
                    let d = m.slide.distance(p);
                    if d < 30.0 {
                        let w = (-d * d / 200.0).exp();
                        num += w * m.leave_one_out_mm * m.leave_one_out_mm;
                        den += w;
                    }
                }
                error_mm.push(Some((num / den).sqrt()));
            }
        }
        ConfidenceMap {
            cell_mm: cell,
            columns: count,
            rows: count,
            error_mm,
        }
    }

    /// Les correspondances, au format du fichier projet.
    pub fn to_project_rectification(&self) -> Rectification {
        let mut r = Rectification {
            method: RectificationMethod::Automatic,
            points: self
                .nodes
                .iter()
                .map(|m| {
                    RectificationPoint::new(
                        SourcePoint::new(m.photo.x, m.photo.y),
                        PointMm::new(m.slide.x, m.slide.y),
                    )
                })
                .collect(),
            residual_mm: Some(self.leave_one_out_mm.mean),
            mirror: Default::default(),
            extra: Default::default(),
        };
        if self.method == SeedMethod::Corners {
            r.method = RectificationMethod::Manual;
        }
        r
    }

    /// Position dans la photo (px) d'un point de la diapositive (mm).
    pub fn slide_to_photo(&self, p: Point) -> Option<Point> {
        self.warp.apply(p)
    }
}

/// Image redressée : la diapositive de 280 mm vue par le projecteur, à `dpi`
/// points par pouce (500 pour reproduire les photos redressées d'Épinal).
pub fn rectify(photo: &RgbImage, detection: &TargetDetection, dpi: f64) -> RgbImage {
    render::render(
        photo,
        &detection.warp,
        units::mm_to_px(SLIDE_SIZE_MM, dpi).max(1),
    )
}

/// Où tombent les croisements de la grille dans une image déjà redressée.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridCheck {
    /// Pour chaque croisement mesuré : sa place théorique et sa position
    /// mesurée dans l'image, en mm de diapositive.
    pub nodes: Vec<(Point, Point)>,
    pub measurable_nodes: usize,
    /// Distance entre place théorique et position mesurée, en mm.
    pub deviation_mm: ErrorSummary,
}

/// Mesure les croisements de la grille dans une image redressée (le carré
/// de 280 mm occupe toute l'image), pour juger un redressement : le nôtre
/// comme celui fait à la main dans Millumin.
pub fn check_rectified(image: &RgbImage) -> Result<GridCheck, DetectionError> {
    let gray = ColorPlanes::from_rgb(image).gray();
    let sx = f64::from(image.width()) / SLIDE_SIZE_MM;
    let sy = f64::from(image.height()) / SLIDE_SIZE_MM;
    let h0 = Homography::from_matrix([[sx, 0.0, 0.0], [0.0, sy, 0.0], [0.0, 0.0, 1.0]])?;
    let every = |_: Node| true;
    let (found, _) =
        grow_and_clean(&gray, &[], Some((&h0, &every))).map_err(DetectionError::NotFound)?;
    let nodes: Vec<(Point, Point)> = found
        .iter()
        .map(|m| {
            (
                m.node.position(),
                Point::new(m.photo.x / sx, m.photo.y / sy),
            )
        })
        .collect();
    let deviations: Vec<f64> = nodes.iter().map(|(a, b)| a.distance(*b)).collect();
    Ok(GridCheck {
        measurable_nodes: measurable_count(),
        deviation_mm: ErrorSummary::of(&deviations),
        nodes,
    })
}

/// Lit une photo (JPEG, PNG, TIFF, HEIC), sans appliquer de rotation EXIF
/// (voir [`crate::photo`]).
pub fn read_photo(path: &Path) -> Result<RgbImage, DetectionError> {
    crate::photo::read_rgb(path).map_err(|e| DetectionError::Image(e.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_summary_of_known_values() {
        let s = ErrorSummary::of(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(s.mean, 2.5);
        assert_eq!(s.max, 4.0);
        assert_eq!(s.p95, 4.0);
    }

    #[test]
    fn corners_in_any_order_are_put_back_clockwise_from_top_left() {
        let tl = Point::new(100.0, 120.0);
        let tr = Point::new(900.0, 90.0);
        let br = Point::new(950.0, 880.0);
        let bl = Point::new(80.0, 860.0);
        for shuffled in [
            [tl, tr, br, bl],
            [br, tl, bl, tr],
            [bl, br, tr, tl],
            [tr, bl, tl, br],
        ] {
            assert_eq!(order_corners(shuffled), [tl, tr, br, bl]);
        }
    }
}

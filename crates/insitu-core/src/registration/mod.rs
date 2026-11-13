//! Recalage d'un projecteur sur la façade (tâche 3.2) : des paires de points
//! communs entre la photo redressée (mm de diapositive) et la façade (mm
//! réels) donnent la correspondance diapositive → façade.
//!
//! La photo est posée par un **trapèze seul** (perspective pure) : exact
//! avec 4 paires, ajusté au mieux sur toutes au-delà. Il n'y a plus de
//! correction du relief, qui courbait l'image au-delà de 5 points (D-3.2-l,
//! demande de Benjamin du 13 novembre 2026). Chaque paire reçoit un écart,
//! mesuré en millimètres sur la diapositive et sur la façade :
//!
//! - avec 4 paires, le trapèze passe exactement par elles : pas d'écart ;
//! - au-delà, l'écart de chaque paire au trapèze ajusté sur toutes. À
//!   partir de 6 paires, le moteur signale aussi les points aberrants
//!   (D-2.3, réglages : [`RegistrationOptions`]).
//!
//! L'ajustement de diagnostic va de la façade vers la diapositive, pour que
//! les écarts et le seuil des points aberrants soient en mm de diapositive,
//! quelle que soit l'échelle de la façade. La correspondance utilisée
//! ensuite (emprise, export) va de la diapositive vers la façade ; elle
//! passe par les mêmes points.

pub mod auto;

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::geometry::{
    fit_with_report, FitOptions, GeometryError, Point, PointWarp, Transform,
    MIN_POINTS_FOR_OUTLIERS,
};
use crate::project::{
    PointMm, Project, ProjectDir, ProjectError, RegistrationLine, RegistrationPair,
};
use crate::rectification::projector::find_projector;
use crate::units::SLIDE_SIZE_MM;

/// Réglages du repérage des points aberrants.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RegistrationOptions {
    /// Un point est aberrant si son score dépasse ce nombre de fois le score
    /// médian des autres (voir [`FitOptions::outlier_factor`]).
    pub outlier_factor: f64,
    /// En dessous de cet écart sur la diapositive, en mm, un point n'est
    /// jamais aberrant : c'est la précision d'un clic soigneux sur la photo
    /// redressée (0,15 mm par pixel, D-2.4) et du redressement lui-même.
    pub outlier_floor_slide_mm: f64,
}

impl Default for RegistrationOptions {
    fn default() -> Self {
        Self {
            outlier_factor: 6.0,
            outlier_floor_slide_mm: 1.0,
        }
    }
}

/// Avancement du recalage d'un projecteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RegistrationStage {
    /// Moins de 4 paires : rien n'est encore calculé.
    Incomplete,
    /// 4 ou 5 paires : un trapèze seul.
    Trapeze,
    /// 6 paires ou plus : trapèze ajusté au mieux, points aberrants
    /// recherchés.
    Corrected,
}

/// Ce que le moteur dit d'une paire.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairReport {
    /// Écart sur la diapositive, en mm. `None` tant qu'il n'est pas mesurable
    /// (moins de 5 paires).
    pub slide_residual_mm: Option<f64>,
    /// Le même écart sur la façade, en mm réels.
    pub facade_residual_mm: Option<f64>,
    /// Le moteur juge ce point aberrant : à vérifier, déplacer ou retirer.
    pub outlier: bool,
}

/// Compte rendu du recalage d'un projecteur.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationReport {
    pub projector_id: String,
    pub stage: RegistrationStage,
    /// Une entrée par paire, dans l'ordre du projet.
    pub pairs: Vec<PairReport>,
    /// Moyenne quadratique des écarts sur la diapositive, points aberrants
    /// exclus, en mm.
    pub rms_slide_mm: Option<f64>,
    /// La même sur la façade, en mm.
    pub rms_facade_mm: Option<f64>,
    /// Ce que le trapèze seul laisse comme écart moyen sur la diapositive :
    /// la part du relief (ou des erreurs de pointage) qu'il n'explique pas.
    pub trapeze_rms_slide_mm: Option<f64>,
    /// Contour du carré de la diapositive posé sur la façade (mm), de
    /// haut-gauche dans le sens des aiguilles d'une montre, fermé : l'emprise
    /// du projecteur. Vide tant que le recalage est incomplet.
    pub footprint: Vec<PointMm>,
    pub options: RegistrationOptions,
}

/// Échec d'une opération de recalage.
#[derive(Debug)]
pub enum RegistrationError {
    Project(ProjectError),
    UnknownProjector(String),
    /// Le projet n'a pas encore de façade mise à l'échelle.
    NoScaledFacade,
    /// Une paire est hors de la diapositive ou n'est pas un nombre.
    BadPair(String),
    /// Les points ne permettent pas d'ajuster la correspondance.
    Geometry(GeometryError),
    BadOptions(String),
}

impl fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Project(e) => e.fmt(f),
            Self::UnknownProjector(key) => write!(f, "aucun projecteur « {key} » dans ce projet"),
            Self::NoScaledFacade => f.write_str(
                "la façade n'est pas encore mise à l'échelle : poser l'échelle dans l'écran Façade",
            ),
            Self::BadPair(message) | Self::BadOptions(message) => f.write_str(message),
            Self::Geometry(e) => write!(f, "points inutilisables : {e}"),
        }
    }
}

impl std::error::Error for RegistrationError {}

impl From<ProjectError> for RegistrationError {
    fn from(e: ProjectError) -> Self {
        Self::Project(e)
    }
}

impl From<GeometryError> for RegistrationError {
    fn from(e: GeometryError) -> Self {
        Self::Geometry(e)
    }
}

/// Points du contour de l'emprise sur chaque côté de la diapositive.
const FOOTPRINT_POINTS_PER_SIDE: usize = 24;

fn point(p: PointMm) -> Point {
    Point::new(p.x, p.y)
}

fn point_mm(p: Point) -> PointMm {
    PointMm::new(p.x, p.y)
}

/// Contrôle les paires : nombres finis, points de diapositive dans le carré.
pub fn validate_pairs(pairs: &[RegistrationPair]) -> Result<(), RegistrationError> {
    for (i, pair) in pairs.iter().enumerate() {
        let n = i + 1;
        let finite = [pair.slide.x, pair.slide.y, pair.facade.x, pair.facade.y]
            .iter()
            .all(|v| v.is_finite());
        if !finite {
            return Err(RegistrationError::BadPair(format!(
                "le point {n} a une coordonnée qui n'est pas un nombre"
            )));
        }
        let inside = |v: f64| (0.0..=SLIDE_SIZE_MM).contains(&v);
        if !inside(pair.slide.x) || !inside(pair.slide.y) {
            return Err(RegistrationError::BadPair(format!(
                "le point {n} est hors de la diapositive (0 à {SLIDE_SIZE_MM} mm)"
            )));
        }
    }
    Ok(())
}

/// Calcule le compte rendu de recalage pour ces paires.
pub fn report(
    projector_id: &str,
    pairs: &[RegistrationPair],
    options: &RegistrationOptions,
) -> Result<RegistrationReport, RegistrationError> {
    if !(options.outlier_factor.is_finite() && options.outlier_factor >= 1.0) {
        return Err(RegistrationError::BadOptions(
            "le facteur des points aberrants doit être au moins 1".into(),
        ));
    }
    if !(options.outlier_floor_slide_mm.is_finite() && options.outlier_floor_slide_mm > 0.0) {
        return Err(RegistrationError::BadOptions(
            "le seuil des points aberrants doit être positif".into(),
        ));
    }
    validate_pairs(pairs)?;
    let n = pairs.len();
    let mut result = RegistrationReport {
        projector_id: projector_id.to_string(),
        stage: RegistrationStage::Incomplete,
        pairs: vec![
            PairReport {
                slide_residual_mm: None,
                facade_residual_mm: None,
                outlier: false,
            };
            n
        ],
        rms_slide_mm: None,
        rms_facade_mm: None,
        trapeze_rms_slide_mm: None,
        footprint: Vec::new(),
        options: *options,
    };
    if n < 4 {
        return Ok(result);
    }
    let slide: Vec<Point> = pairs.iter().map(|p| point(p.slide)).collect();
    let facade: Vec<Point> = pairs.iter().map(|p| point(p.facade)).collect();

    // Diagnostic : façade → diapositive, écarts en mm de diapositive.
    let fit_options = FitOptions {
        smoothing: 0.0,
        outlier_factor: options.outlier_factor,
        outlier_floor: options.outlier_floor_slide_mm,
    };
    let (_, diagnostic) = fit_with_report(&facade, &slide, &fit_options)?;
    // Correspondance utile : diapositive → façade.
    let warp = PointWarp::trapeze(&slide, &facade)?;
    result.footprint = footprint(&warp);
    result.trapeze_rms_slide_mm = Some(diagnostic.homography_residuals.rms);

    result.stage = if n >= MIN_POINTS_FOR_OUTLIERS {
        RegistrationStage::Corrected
    } else {
        RegistrationStage::Trapeze
    };
    // L'écart de chaque point au trapèze, qui pose la photo (D-3.2-l).
    let (on_slide, on_facade) = if n == 4 {
        (None, None)
    } else {
        (
            Some(diagnostic.homography_residuals.per_point),
            Some(diagnostic.homography_residuals_in_source.per_point),
        )
    };
    for &i in &diagnostic.outliers {
        result.pairs[i].outlier = true;
    }
    if let (Some(on_slide), Some(on_facade)) = (on_slide, on_facade) {
        let (mut sum_slide, mut sum_facade, mut count) = (0.0, 0.0, 0usize);
        for (i, pair) in result.pairs.iter_mut().enumerate() {
            pair.slide_residual_mm = Some(on_slide[i]);
            pair.facade_residual_mm = Some(on_facade[i]);
            if !pair.outlier {
                sum_slide += on_slide[i] * on_slide[i];
                sum_facade += on_facade[i] * on_facade[i];
                count += 1;
            }
        }
        if count > 0 {
            result.rms_slide_mm = Some((sum_slide / count as f64).sqrt());
            result.rms_facade_mm = Some((sum_facade / count as f64).sqrt());
        }
    }
    Ok(result)
}

/// Correspondance diapositive → façade d'un projecteur : `None` tant qu'il
/// a moins de 4 paires, ou si elles ne permettent pas d'ajuster le trapèze.
pub fn slide_to_facade(pairs: &[RegistrationPair]) -> Option<PointWarp> {
    if pairs.len() < 4 || validate_pairs(pairs).is_err() {
        return None;
    }
    let slide: Vec<Point> = pairs.iter().map(|p| point(p.slide)).collect();
    let facade: Vec<Point> = pairs.iter().map(|p| point(p.facade)).collect();
    PointWarp::trapeze(&slide, &facade).ok()
}

/// Contour du carré de la diapositive envoyé sur la façade (emprise), fermé.
pub fn footprint(warp: &PointWarp) -> Vec<PointMm> {
    let s = SLIDE_SIZE_MM;
    let corners = [
        Point::new(0.0, 0.0),
        Point::new(s, 0.0),
        Point::new(s, s),
        Point::new(0.0, s),
    ];
    let mut outline = Vec::with_capacity(4 * FOOTPRINT_POINTS_PER_SIDE + 1);
    for k in 0..4 {
        let (a, b) = (corners[k], corners[(k + 1) % 4]);
        for step in 0..FOOTPRINT_POINTS_PER_SIDE {
            let t = step as f64 / FOOTPRINT_POINTS_PER_SIDE as f64;
            if let Some(p) = warp.apply(a + (b - a) * t) {
                outline.push(point_mm(p));
            }
        }
    }
    if let Some(&first) = outline.first() {
        outline.push(first);
    }
    outline
}

fn projector_id(project: &Project, key: &str) -> Result<String, RegistrationError> {
    find_projector(project, key)
        .map(|p| p.id.clone())
        .ok_or_else(|| RegistrationError::UnknownProjector(key.to_string()))
}

/// Compte rendu du recalage enregistré d'un projecteur.
pub fn projector_report(
    dir: &ProjectDir,
    projector: &str,
    options: &RegistrationOptions,
) -> Result<RegistrationReport, RegistrationError> {
    let project = dir.load()?.project;
    let id = projector_id(&project, projector)?;
    let pairs = &project
        .projector(&id)
        .expect("projecteur trouvé")
        .registration;
    report(&id, pairs, options)
}

/// Contrôle les lignes : nombres finis, points de diapositive dans le carré,
/// deux points distincts de chaque côté.
pub fn validate_lines(lines: &[RegistrationLine]) -> Result<(), RegistrationError> {
    for (i, line) in lines.iter().enumerate() {
        let n = i + 1;
        let all = [line.slide[0], line.slide[1], line.facade[0], line.facade[1]];
        if !all.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
            return Err(RegistrationError::BadPair(format!(
                "la ligne {n} a une coordonnée qui n'est pas un nombre"
            )));
        }
        let inside = |v: f64| (0.0..=SLIDE_SIZE_MM).contains(&v);
        if line.slide.iter().any(|p| !inside(p.x) || !inside(p.y)) {
            return Err(RegistrationError::BadPair(format!(
                "la ligne {n} sort de la diapositive (0 à {SLIDE_SIZE_MM} mm)"
            )));
        }
        let length = |a: PointMm, b: PointMm| (b.x - a.x).hypot(b.y - a.y);
        if length(line.slide[0], line.slide[1]) < 1e-3
            || length(line.facade[0], line.facade[1]) < 1e-3
        {
            return Err(RegistrationError::BadPair(format!(
                "la ligne {n} n'a pas de longueur : tracer un trait"
            )));
        }
    }
    Ok(())
}

/// Remplace les paires d'un projecteur, enregistre le projet et rend le
/// compte rendu. Les paires sont refusées (et le projet laissé tel quel) si
/// elles sont hors de la diapositive ou si le trapèze ne peut pas être
/// ajusté. La façade doit être mise à l'échelle : les points de façade sont
/// en mm réels.
pub fn set_registration(
    dir: &ProjectDir,
    projector: &str,
    pairs: Vec<RegistrationPair>,
    options: &RegistrationOptions,
) -> Result<(Project, RegistrationReport), RegistrationError> {
    set_registration_with_lines(dir, projector, pairs, None, options)
}

/// Comme [`set_registration`], et remplace aussi les lignes du projecteur
/// quand `lines` est donné (D-3.2-k) ; `None` les laisse telles quelles.
pub fn set_registration_with_lines(
    dir: &ProjectDir,
    projector: &str,
    pairs: Vec<RegistrationPair>,
    lines: Option<Vec<RegistrationLine>>,
    options: &RegistrationOptions,
) -> Result<(Project, RegistrationReport), RegistrationError> {
    if let Some(lines) = &lines {
        validate_lines(lines)?;
    }
    let mut project = dir.load()?.project;
    let id = projector_id(&project, projector)?;
    let scaled = project
        .facade
        .as_ref()
        .and_then(|f| f.scale.as_ref())
        .is_some();
    if !scaled && !pairs.is_empty() {
        return Err(RegistrationError::NoScaledFacade);
    }
    let result = report(&id, &pairs, options)?;
    let target = project.projector_mut(&id).expect("projecteur trouvé");
    target.registration = pairs;
    if let Some(lines) = lines {
        target.registration_lines = lines;
    }
    dir.save(&project)?;
    Ok((project, result))
}

#[cfg(test)]
mod tests;

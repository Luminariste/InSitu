//! Assemblage de photos de façade (tâche 6.4) : plusieurs photos, recalées
//! entre elles par points communs, deviennent une seule image qui sert de
//! fond de plan (la façade du projet).
//!
//! - La première photo ajoutée est la **référence** : l'image assemblée est
//!   dans son repère (ses pixels), agrandi pour contenir les autres.
//! - Chaque autre photo s'appuie sur une photo déjà placée (son **appui**) :
//!   au moins quatre paires de points communs donnent le trapèze
//!   (homographie) qui la pose sur son appui. Une rangée de photos se
//!   recale ainsi de proche en proche, sans que la dernière voie la
//!   référence.
//! - La façade est supposée plane : une photo prise d'un autre point de vue
//!   ne se superpose exactement que dans le plan des points cliqués (D-6.4-a).
//!
//! Chaque paire reçoit un écart en pixels de la photo d'appui ; à partir de
//! 5 paires, c'est l'écart de **validation croisée** (où les autres paires,
//! sans elle, placent ce point), et les points aberrants sont signalés.
//!
//! [`render`] fabrique l'image assemblée : chaque photo est déformée par le
//! moteur de la tâche 3.4, et les recouvrements sont fondus (poids qui
//! décroît vers le bord de chaque photo). L'image est écrite dans le projet
//! et devient la façade ; une mise à l'échelle déjà faite sur un assemblage
//! précédent est conservée, et les points de façade des recalages suivent.

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::time::Instant;

use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder};
use serde::Serialize;

use crate::facade::{inspect, FacadeError};
use crate::geometry::{GeometryError, Homography, Point, Transform};
use crate::photo;
use crate::project::model::new_id;
use crate::project::store::write_atomic;
use crate::project::{
    AssemblyOutput, AssemblyPair, AssemblyPhoto, AssetKind, ExtraFields, Facade, FacadeAssembly,
    FacadeScale, PointMm, Project, ProjectDir, ProjectError, RelativePath, SourcePoint,
};
use crate::resample::{warp_raster, WarpOptions};
use crate::tiles::Raster;

/// Sous-dossier du projet où sont copiées les photos à assembler.
pub const ASSEMBLY_DIR: &str = "facade/assemblage";

/// Nom de l'image assemblée, dans `facade/`.
pub const OUTPUT_FILE: &str = "facade/assemblage.jpg";

/// Plus grand côté de l'image assemblée, en pixels, par défaut : au-delà,
/// elle est réduite (D-6.4-c).
pub const DEFAULT_MAX_SIDE: u32 = 12_000;

/// Qualité JPEG de l'image assemblée.
const JPEG_QUALITY: u8 = 92;

/// Largeur du fondu au bord de chaque photo, en part de son petit côté.
const FEATHER_FRACTION: f64 = 0.1;

/// Un point n'est jamais aberrant sous cet écart (pixels de la photo
/// d'appui) : la précision d'un clic soigneux.
pub const OUTLIER_FLOOR_PX: f64 = 4.0;

/// Un point est aberrant si son écart dépasse ce nombre de fois l'écart
/// médian des autres.
pub const OUTLIER_FACTOR: f64 = 4.0;

/// Hauteur des bandes de l'image assemblée calculées d'un seul tenant.
const BAND_ROWS: u32 = 1024;

#[derive(Debug)]
pub enum AssemblyError {
    /// Aucune photo dans l'assemblage.
    Empty,
    UnknownPhoto(String),
    /// Une photo d'appui n'est pas acceptable (elle-même, inconnue, ou elle
    /// s'appuie sur la photo à recaler).
    BadAnchor(String),
    /// Une photo est encore citée comme appui par d'autres.
    StillAnchor {
        photo: String,
        others: Vec<String>,
    },
    /// Un point hors de sa photo, ou des points qui ne donnent pas de
    /// trapèze.
    BadPairs(String),
    /// Une photo n'est pas encore placée : moins de 4 paires, ou appui
    /// lui-même non placé.
    NotPlaced(Vec<String>),
    /// L'image assemblée serait vide ou démesurée (trapèze dégénéré).
    BadOutput(String),
    Unreadable(String),
    Project(ProjectError),
}

impl fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "L'assemblage n'a encore aucune photo."),
            Self::UnknownPhoto(id) => write!(f, "Aucune photo « {id} » dans l'assemblage."),
            Self::BadAnchor(why) => write!(f, "Photo d'appui refusée : {why}."),
            Self::StillAnchor { photo, others } => write!(
                f,
                "« {photo} » sert d'appui à {} : retirer ou recaler d'abord ces photos.",
                others.join(", ")
            ),
            Self::BadPairs(why) => write!(f, "Points refusés : {why}."),
            Self::NotPlaced(names) => write!(
                f,
                "Photos pas encore placées (4 points communs au moins avec leur appui) : {}.",
                names.join(", ")
            ),
            Self::BadOutput(why) => write!(f, "Assemblage impossible : {why}."),
            Self::Unreadable(why) => f.write_str(why),
            Self::Project(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for AssemblyError {}

impl From<ProjectError> for AssemblyError {
    fn from(error: ProjectError) -> Self {
        Self::Project(error)
    }
}

impl From<FacadeError> for AssemblyError {
    fn from(error: FacadeError) -> Self {
        Self::Unreadable(error.to_string())
    }
}

// ---------------------------------------------------------------------------
// Diagnostic
// ---------------------------------------------------------------------------

/// Ce que le moteur dit d'une paire.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyPairReport {
    /// Écart en pixels de la photo d'appui ; `None` avec moins de 5 paires.
    pub residual_px: Option<f64>,
    pub outlier: bool,
}

/// Ce que le moteur dit d'une photo.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyPhotoReport {
    pub id: String,
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub anchor: Option<String>,
    pub pairs: Vec<AssemblyPairReport>,
    /// Placée : la référence, ou recalée (4 paires au moins) sur une photo
    /// placée.
    pub placed: bool,
    /// Coins de la photo (haut-gauche, haut-droit, bas-droit, bas-gauche)
    /// dans le repère de la référence, en pixels ; absents si elle n'est pas
    /// placée.
    pub corners: Option<[SourcePoint; 4]>,
    /// Écart moyen des paires (hors points aberrants), en pixels ; `None`
    /// avec moins de 5 paires.
    pub mean_residual_px: Option<f64>,
    /// Ce qui empêche de la placer, s'il y a lieu.
    pub problem: Option<String>,
}

/// Diagnostic de tout l'assemblage.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyReport {
    pub photos: Vec<AssemblyPhotoReport>,
    /// Vrai si toutes les photos sont placées (et qu'il y en a au moins une).
    pub ready: bool,
}

fn to_point(p: SourcePoint) -> Point {
    Point::new(p.x, p.y)
}

fn to_source(p: Point) -> SourcePoint {
    SourcePoint::new(p.x, p.y)
}

/// Le trapèze qui pose une photo sur son appui, et l'écart de chaque paire.
fn fit_pairs(
    pairs: &[AssemblyPair],
) -> Result<(Homography, Vec<AssemblyPairReport>), GeometryError> {
    let here: Vec<Point> = pairs.iter().map(|p| to_point(p.here)).collect();
    let there: Vec<Point> = pairs.iter().map(|p| to_point(p.there)).collect();
    let h = Homography::fit(&here, &there)?;
    let n = pairs.len();
    if n < 5 {
        let reports = vec![
            AssemblyPairReport {
                residual_px: None,
                outlier: false,
            };
            n
        ];
        return Ok((h, reports));
    }
    // Validation croisée : chaque point prédit par les autres points
    // gardés. Recherche des points faux, un par un : on retire le point dont
    // l'absence rend les autres les plus cohérents, et on le déclare faux si
    // les autres le placent loin (plus de `OUTLIER_FACTOR` fois l'écart
    // médian des autres, et plus de `OUTLIER_FLOOR_PX`). Un point faux ne
    // fausse ainsi pas le jugement des autres.
    let fit_without = |skip: &[usize], kept: &[bool]| {
        let (src, dst): (Vec<Point>, Vec<Point>) = (0..n)
            .filter(|i| kept[*i] && !skip.contains(i))
            .map(|i| (here[i], there[i]))
            .unzip();
        Homography::fit(&src, &dst).ok().map(|g| {
            let sse: f64 = src
                .iter()
                .zip(&dst)
                .map(|(p, q)| {
                    g.apply(*p)
                        .map_or(f64::INFINITY, |r| r.distance(*q).powi(2))
                })
                .sum();
            (g, sse)
        })
    };
    let predict = |k: usize, kept: &[bool]| -> f64 {
        fit_without(&[k], kept)
            .and_then(|(g, _)| g.apply(here[k]))
            .map_or(f64::INFINITY, |q| q.distance(there[k]))
    };
    let mut kept = vec![true; n];
    // Il faut 5 points gardés (après retrait) pour juger un point.
    while n >= 6 && kept.iter().filter(|&&k| k).count() > 5 {
        let candidates: Vec<usize> = (0..n).filter(|&i| kept[i]).collect();
        let Some(suspect) = candidates
            .iter()
            .copied()
            .filter_map(|k| fit_without(&[k], &kept).map(|(_, sse)| (k, sse)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(k, _)| k)
        else {
            break;
        };
        let mut trial = kept.clone();
        trial[suspect] = false;
        let mut rest: Vec<f64> = candidates
            .iter()
            .filter(|&&i| i != suspect)
            .map(|&i| predict(i, &trial))
            .collect();
        rest.sort_by(f64::total_cmp);
        let median = rest[rest.len() / 2];
        if predict(suspect, &trial) <= OUTLIER_FLOOR_PX.max(OUTLIER_FACTOR * median) {
            break;
        }
        kept = trial;
    }
    let residuals: Vec<f64> = (0..n).map(|k| predict(k, &kept)).collect();
    let reports = (0..n)
        .map(|k| AssemblyPairReport {
            // Infini : les autres points ne suffisent pas à le prédire.
            residual_px: residuals[k].is_finite().then_some(residuals[k]),
            outlier: !kept[k],
        })
        .collect();
    Ok((h, reports))
}

/// Trapèze de chaque photo placée vers le repère de la référence, et
/// diagnostic.
fn solve(assembly: &FacadeAssembly) -> (HashMap<String, Homography>, AssemblyReport) {
    let mut to_reference: HashMap<String, Homography> = HashMap::new();
    let mut fits: HashMap<String, Result<(Homography, Vec<AssemblyPairReport>), String>> =
        HashMap::new();
    for photo in &assembly.photos {
        if photo.anchor.is_some() {
            let fit = if photo.pairs.len() < 4 {
                Err(format!(
                    "{} point{} commun{} sur 4",
                    photo.pairs.len(),
                    if photo.pairs.len() > 1 { "s" } else { "" },
                    if photo.pairs.len() > 1 { "s" } else { "" }
                ))
            } else {
                fit_pairs(&photo.pairs).map_err(|e| e.to_string())
            };
            fits.insert(photo.id.clone(), fit);
        }
    }
    if let Some(reference) = assembly.photos.first() {
        to_reference.insert(reference.id.clone(), Homography::translation(0.0, 0.0));
    }
    // De proche en proche : une photo est placée quand son appui l'est.
    loop {
        let mut progressed = false;
        for photo in &assembly.photos {
            if to_reference.contains_key(&photo.id) {
                continue;
            }
            let (Some(anchor), Some(Ok((h, _)))) = (&photo.anchor, fits.get(&photo.id)) else {
                continue;
            };
            if let Some(anchor_h) = to_reference.get(anchor) {
                let placed = h.then(anchor_h);
                to_reference.insert(photo.id.clone(), placed);
                progressed = true;
            }
        }
        if !progressed {
            break;
        }
    }
    let photos: Vec<AssemblyPhotoReport> = assembly
        .photos
        .iter()
        .map(|photo| {
            let fit = fits.get(&photo.id);
            let pairs = match fit {
                Some(Ok((_, reports))) => reports.clone(),
                _ => vec![
                    AssemblyPairReport {
                        residual_px: None,
                        outlier: false,
                    };
                    photo.pairs.len()
                ],
            };
            let h = to_reference.get(&photo.id);
            let corners = h.and_then(|h| {
                let (w, ht) = (photo.width, photo.height);
                let c = [
                    Point::new(0.0, 0.0),
                    Point::new(w, 0.0),
                    Point::new(w, ht),
                    Point::new(0.0, ht),
                ];
                Some([
                    to_source(h.apply(c[0])?),
                    to_source(h.apply(c[1])?),
                    to_source(h.apply(c[2])?),
                    to_source(h.apply(c[3])?),
                ])
            });
            let kept: Vec<f64> = pairs
                .iter()
                .filter(|p| !p.outlier)
                .filter_map(|p| p.residual_px)
                .collect();
            let mean_residual_px =
                (!kept.is_empty()).then(|| kept.iter().sum::<f64>() / kept.len() as f64);
            let problem = match (fit, h) {
                (_, Some(_)) => None,
                (Some(Err(why)), None) => Some(why.clone()),
                _ => Some("sa photo d'appui n'est pas encore placée".to_string()),
            };
            AssemblyPhotoReport {
                id: photo.id.clone(),
                name: photo.name.clone(),
                width: photo.width,
                height: photo.height,
                anchor: photo.anchor.clone(),
                pairs,
                placed: h.is_some(),
                corners,
                mean_residual_px,
                problem,
            }
        })
        .collect();
    let ready = !photos.is_empty() && photos.iter().all(|p| p.placed);
    (to_reference, AssemblyReport { photos, ready })
}

/// Diagnostic de l'assemblage du projet (vide s'il n'y en a pas).
pub fn report(project: &Project) -> AssemblyReport {
    match &project.facade_assembly {
        Some(assembly) => solve(assembly).1,
        None => AssemblyReport {
            photos: Vec::new(),
            ready: false,
        },
    }
}

/// Diagnostic de l'assemblage du projet enregistré dans `dir`.
pub fn project_report(dir: &ProjectDir) -> Result<AssemblyReport, AssemblyError> {
    Ok(report(&dir.load()?.project))
}

// ---------------------------------------------------------------------------
// Photos et points
// ---------------------------------------------------------------------------

/// Ajoute une photo à l'assemblage (copiée dans `facade/assemblage/`). La
/// première devient la référence ; les suivantes s'appuient sur `anchor`,
/// ou à défaut sur la dernière photo ajoutée. Le projet est enregistré.
pub fn add_photo(
    dir: &ProjectDir,
    path: &Path,
    anchor: Option<&str>,
) -> Result<(Project, String), AssemblyError> {
    let (width, height) =
        photo::dimensions(path).map_err(|e| AssemblyError::Unreadable(e.to_string()))?;
    let mut project = dir.load()?.project;
    let assembly = project
        .facade_assembly
        .get_or_insert_with(|| FacadeAssembly {
            photos: Vec::new(),
            output: None,
            extra: ExtraFields::new(),
        });
    let anchor = match (anchor, assembly.photos.last()) {
        (_, None) => None,
        (Some(id), Some(_)) => {
            if !assembly.photos.iter().any(|p| p.id == id) {
                return Err(AssemblyError::BadAnchor(format!("aucune photo « {id} »")));
            }
            Some(id.to_string())
        }
        (None, Some(last)) => Some(last.id.clone()),
    };
    let source = dir.import_file_into(path, ASSEMBLY_DIR)?;
    let id = new_id();
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    assembly.photos.push(AssemblyPhoto {
        id: id.clone(),
        name,
        source,
        width: f64::from(width),
        height: f64::from(height),
        anchor,
        pairs: Vec::new(),
        extra: ExtraFields::new(),
    });
    dir.save(&project)?;
    Ok((project, id))
}

/// Retire une photo de l'assemblage (son fichier reste dans le projet).
/// Refusé si d'autres photos s'appuient sur elle. Retirer la référence
/// fait de la photo suivante la nouvelle référence, si elle s'appuyait sur
/// elle ; sinon c'est refusé.
pub fn remove_photo(dir: &ProjectDir, id: &str) -> Result<Project, AssemblyError> {
    let mut project = dir.load()?.project;
    let assembly = project
        .facade_assembly
        .as_mut()
        .ok_or(AssemblyError::Empty)?;
    let index = assembly
        .photos
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| AssemblyError::UnknownPhoto(id.to_string()))?;
    let dependents: Vec<usize> = assembly
        .photos
        .iter()
        .enumerate()
        .filter(|(_, p)| p.anchor.as_deref() == Some(id))
        .map(|(i, _)| i)
        .collect();
    let next_reference = index == 0 && dependents == [1];
    if !dependents.is_empty() && !next_reference {
        return Err(AssemblyError::StillAnchor {
            photo: assembly.photos[index].name.clone(),
            others: dependents
                .iter()
                .map(|&i| assembly.photos[i].name.clone())
                .collect(),
        });
    }
    assembly.photos.remove(index);
    if next_reference {
        let new_reference = &mut assembly.photos[0];
        new_reference.anchor = None;
        new_reference.pairs.clear();
        // Le repère change : l'image précédente ne permet plus de reporter
        // l'échelle (elle sera à refaire après le prochain calcul).
        assembly.output = None;
    }
    if assembly.photos.is_empty() {
        project.facade_assembly = None;
    }
    dir.save(&project)?;
    Ok(project)
}

/// Remplace la photo d'appui et les points communs d'une photo ; le projet
/// est enregistré. Chaque point doit être dans sa photo ; avec 4 points ou
/// plus, ils doivent donner un trapèze.
pub fn set_pairs(
    dir: &ProjectDir,
    id: &str,
    anchor: &str,
    pairs: Vec<AssemblyPair>,
) -> Result<(Project, AssemblyReport), AssemblyError> {
    let mut project = dir.load()?.project;
    let assembly = project
        .facade_assembly
        .as_mut()
        .ok_or(AssemblyError::Empty)?;
    let photo = assembly
        .photos
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| AssemblyError::UnknownPhoto(id.to_string()))?;
    if assembly.photos.first().map(|p| p.id.as_str()) == Some(id) {
        return Err(AssemblyError::BadAnchor(
            "la photo de référence ne s'appuie sur aucune autre".into(),
        ));
    }
    let anchor_photo = assembly
        .photos
        .iter()
        .find(|p| p.id == anchor)
        .ok_or_else(|| AssemblyError::BadAnchor(format!("aucune photo « {anchor} »")))?;
    if anchor == id {
        return Err(AssemblyError::BadAnchor(
            "une photo ne peut pas s'appuyer sur elle-même".into(),
        ));
    }
    // Pas de boucle : l'appui ne doit pas dépendre de cette photo.
    let mut cursor = Some(anchor.to_string());
    let mut steps = 0;
    while let Some(current) = cursor {
        if current == id {
            return Err(AssemblyError::BadAnchor(format!(
                "« {} » s'appuie déjà, de proche en proche, sur « {} »",
                anchor_photo.name, photo.name
            )));
        }
        steps += 1;
        if steps > assembly.photos.len() {
            break;
        }
        cursor = assembly
            .photos
            .iter()
            .find(|p| p.id == current)
            .and_then(|p| p.anchor.clone());
    }
    let inside = |p: SourcePoint, w: f64, h: f64| {
        p.x.is_finite() && p.y.is_finite() && (0.0..=w).contains(&p.x) && (0.0..=h).contains(&p.y)
    };
    for (k, pair) in pairs.iter().enumerate() {
        if !inside(pair.here, photo.width, photo.height) {
            return Err(AssemblyError::BadPairs(format!(
                "le point {} est hors de « {} »",
                k + 1,
                photo.name
            )));
        }
        if !inside(pair.there, anchor_photo.width, anchor_photo.height) {
            return Err(AssemblyError::BadPairs(format!(
                "le point {} est hors de « {} »",
                k + 1,
                anchor_photo.name
            )));
        }
    }
    if pairs.len() >= 4 {
        fit_pairs(&pairs).map_err(|e| AssemblyError::BadPairs(e.to_string()))?;
    }
    let photo = assembly
        .photos
        .iter_mut()
        .find(|p| p.id == id)
        .expect("trouvée plus haut");
    photo.anchor = Some(anchor.to_string());
    photo.pairs = pairs;
    dir.save(&project)?;
    let report = report(&project);
    Ok((project, report))
}

// ---------------------------------------------------------------------------
// Image assemblée
// ---------------------------------------------------------------------------

/// Réglages de [`render`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderOptions {
    /// Plus grand côté de l'image assemblée, en pixels.
    pub max_side: u32,
    /// Fils de calcul ; 0 : tous les cœurs.
    pub threads: usize,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            max_side: DEFAULT_MAX_SIDE,
            threads: 0,
        }
    }
}

/// Compte rendu de [`render`].
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderReport {
    /// Taille de l'image assemblée, en pixels.
    pub width: u32,
    pub height: u32,
    /// Pixels de l'image assemblée par pixel de la photo de référence (1
    /// sauf réduction).
    pub scale: f64,
    /// Vrai si la mise à l'échelle de la façade a été reprise de
    /// l'assemblage précédent.
    pub kept_scale: bool,
    pub seconds: f64,
}

/// Rectangle (repère de la référence) qui contient toutes les photos.
fn bounds(
    assembly: &FacadeAssembly,
    to_reference: &HashMap<String, Homography>,
) -> Result<(Point, Point), AssemblyError> {
    let (mut min, mut max) = (
        Point::new(f64::INFINITY, f64::INFINITY),
        Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for photo in &assembly.photos {
        let h = &to_reference[&photo.id];
        // Le bord, pas seulement les coins : un trapèze reste un
        // quadrilatère, mais on vérifie au passage qu'il ne passe pas par
        // l'infini (photo vue « derrière » la référence).
        let steps = 16;
        for k in 0..=steps {
            let t = f64::from(k) / f64::from(steps);
            for p in [
                Point::new(t * photo.width, 0.0),
                Point::new(t * photo.width, photo.height),
                Point::new(0.0, t * photo.height),
                Point::new(photo.width, t * photo.height),
            ] {
                let q = h.apply(p).filter(|q| q.is_finite()).ok_or_else(|| {
                    AssemblyError::BadOutput(format!(
                        "« {} » est posée en trapèze dégénéré : revoir ses points",
                        photo.name
                    ))
                })?;
                min = Point::new(min.x.min(q.x), min.y.min(q.y));
                max = Point::new(max.x.max(q.x), max.y.max(q.y));
            }
        }
    }
    Ok((min, max))
}

/// Une photo lue, prête à être posée : couleur multipliée par le poids du
/// fondu, poids dans le quatrième canal.
fn weighted_photo(path: &Path) -> Result<Raster, AssemblyError> {
    let rgb = photo::read_rgb(path).map_err(|e| AssemblyError::Unreadable(e.to_string()))?;
    let (w, h) = rgb.dimensions();
    let feather = (f64::from(w.min(h)) * FEATHER_FRACTION).max(1.0);
    let mut data = Vec::with_capacity(w as usize * h as usize * 4);
    for (x, y, px) in rgb.enumerate_pixels() {
        let edge = (f64::from(x) + 0.5)
            .min(f64::from(y) + 0.5)
            .min(f64::from(w - x) - 0.5)
            .min(f64::from(h - y) - 0.5);
        // Poids de 1 à 255 : un pixel de photo n'a jamais un poids nul, même
        // au bord (sinon un liseré noir apparaîtrait hors recouvrement).
        let weight = (edge / feather).clamp(0.0, 1.0);
        let a = (1.0 + 254.0 * weight).round();
        for c in px.0 {
            data.push((f64::from(c) * a / 255.0).round() as u8);
        }
        data.push(a as u8);
    }
    Ok(Raster::new(w, h, 4, data))
}

/// Fabrique l'image assemblée, l'écrit dans le projet (`facade/assemblage.jpg`)
/// et en fait la façade. Toutes les photos doivent être placées.
///
/// Si la façade était déjà l'assemblage précédent et qu'elle était mise à
/// l'échelle, l'échelle est reprise, et les points de façade des recalages
/// et le cadre de la fresque sont décalés d'autant que l'origine de
/// l'image : ils restent sur les mêmes pierres.
pub fn render(
    dir: &ProjectDir,
    options: &RenderOptions,
) -> Result<(Project, RenderReport), AssemblyError> {
    let start = Instant::now();
    let mut project = dir.load()?.project;
    let assembly = project
        .facade_assembly
        .as_ref()
        .ok_or(AssemblyError::Empty)?;
    if assembly.photos.is_empty() {
        return Err(AssemblyError::Empty);
    }
    let (to_reference, diagnosis) = solve(assembly);
    let missing: Vec<String> = diagnosis
        .photos
        .iter()
        .filter(|p| !p.placed)
        .map(|p| p.name.clone())
        .collect();
    if !missing.is_empty() {
        return Err(AssemblyError::NotPlaced(missing));
    }
    let (min, max) = bounds(assembly, &to_reference)?;
    let (span_x, span_y) = (max.x - min.x, max.y - min.y);
    // Plus de 20 fois la référence : sûrement un trapèze presque dégénéré.
    let reference = &assembly.photos[0];
    let limit = 20.0 * reference.width.max(reference.height);
    if !(span_x > 0.0 && span_y > 0.0 && span_x.max(span_y) <= limit) {
        return Err(AssemblyError::BadOutput(
            "l'image assemblée serait démesurée : un trapèze est sans doute faux".into(),
        ));
    }
    let scale = (f64::from(options.max_side.max(1)) / span_x.max(span_y)).min(1.0);
    let width = ((span_x * scale).ceil() as u32).max(1);
    let height = ((span_y * scale).ceil() as u32).max(1);
    let origin = min;

    // Inverses : pixel de l'image assemblée → pixel de chaque photo.
    let mut layers = Vec::new();
    for photo in &assembly.photos {
        let inverse = to_reference[&photo.id]
            .inverse()
            .map_err(|e| AssemblyError::BadOutput(e.to_string()))?;
        layers.push((photo, inverse));
    }

    // Sommes pondérées, bande par bande pour borner la mémoire : chaque
    // photo est lue une fois et posée sur toutes les bandes qu'elle touche.
    let mut sums = vec![0u32; width as usize * height as usize * 4];
    for (photo, inverse) in &layers {
        let source = weighted_photo(&dir.resolve(&photo.source))?;
        let mut band_top = 0;
        while band_top < height {
            let band_height = BAND_ROWS.min(height - band_top);
            let top = f64::from(band_top);
            let map = |p: Point| {
                let reference = Point::new(p.x / scale + origin.x, (p.y + top) / scale + origin.y);
                let q = inverse.apply(reference)?;
                let inside =
                    (0.0..=photo.width).contains(&q.x) && (0.0..=photo.height).contains(&q.y);
                inside.then_some(q)
            };
            let band = warp_raster(
                &source,
                width,
                band_height,
                map,
                &WarpOptions {
                    threads: options.threads,
                    ..WarpOptions::default()
                },
            )
            .map_err(|e| AssemblyError::BadOutput(e.to_string()))?;
            let offset = band_top as usize * width as usize * 4;
            for (sum, value) in sums[offset..offset + band.data.len()]
                .iter_mut()
                .zip(&band.data)
            {
                *sum += u32::from(*value);
            }
            band_top += band_height;
        }
    }
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    for px in sums.as_chunks::<4>().0 {
        let weight = px[3];
        for &c in &px[..3] {
            // Aucune photo ici : noir.
            let v = (c * 255 + weight / 2)
                .checked_div(weight)
                .map_or(0, |v| v.min(255));
            rgb.push(v as u8);
        }
    }
    drop(sums);
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, JPEG_QUALITY)
        .write_image(&rgb, width, height, ExtendedColorType::Rgb8)
        .map_err(|e| AssemblyError::BadOutput(e.to_string()))?;
    drop(rgb);

    let relative = RelativePath::new(OUTPUT_FILE).expect("chemin fixe valide");
    let target = dir.resolve(&relative);
    std::fs::create_dir_all(target.parent().expect("facade/")).map_err(ProjectError::io(
        "de créer le dossier",
        dir.root().join(AssetKind::Facade.directory()),
    ))?;
    write_atomic(&target, &bytes)?;

    // Reprise de l'échelle et décalage des points de façade.
    let previous = assembly.output.clone();
    let output = AssemblyOutput {
        source: relative.clone(),
        origin: to_source(origin),
        scale,
    };
    let kept_scale = carry_over(&mut project, previous.as_ref(), &output, width, height);
    let info = inspect(&target, None)?;
    if !kept_scale {
        project.facade = Some(Facade {
            source: relative,
            source_width: info.width,
            source_height: info.height,
            scale: None,
            page: None,
            origin: None,
            extra: ExtraFields::new(),
        });
    }
    project
        .facade_assembly
        .as_mut()
        .expect("assemblage présent")
        .output = Some(output);
    dir.save(&project)?;
    Ok((
        project,
        RenderReport {
            width,
            height,
            scale,
            kept_scale,
            seconds: start.elapsed().as_secs_f64(),
        },
    ))
}

/// Si la façade est l'assemblage précédent, mis à l'échelle, reporte cette
/// échelle sur la nouvelle image et décale ce qui est en mm de façade.
/// Renvoie vrai si c'est fait (la façade est alors déjà à jour).
fn carry_over(
    project: &mut Project,
    previous: Option<&AssemblyOutput>,
    output: &AssemblyOutput,
    width: u32,
    height: u32,
) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    let Some(facade) = project.facade.as_mut() else {
        return false;
    };
    if facade.source != previous.source {
        return false;
    }
    let Some(old_factor) = facade
        .scale
        .as_ref()
        .and_then(FacadeScale::mm_per_source_unit)
    else {
        return false;
    };
    // Ancien pixel → pixel de la référence → nouveau pixel.
    let convert = |p: SourcePoint| {
        let reference = Point::new(
            p.x / previous.scale + previous.origin.x,
            p.y / previous.scale + previous.origin.y,
        );
        SourcePoint::new(
            (reference.x - output.origin.x) * output.scale,
            (reference.y - output.origin.y) * output.scale,
        )
    };
    let scale = facade.scale.as_mut().expect("vérifiée plus haut");
    scale.point_a = convert(scale.point_a);
    scale.point_b = convert(scale.point_b);
    // Mm de façade par pixel de référence (inchangé) : le décalage d'origine
    // se traduit en mm.
    let mm_per_reference_px = old_factor * previous.scale;
    let shift = PointMm::new(
        (previous.origin.x - output.origin.x) * mm_per_reference_px,
        (previous.origin.y - output.origin.y) * mm_per_reference_px,
    );
    facade.source_width = f64::from(width);
    facade.source_height = f64::from(height);
    for projector in &mut project.projectors {
        for pair in &mut projector.registration {
            pair.facade.x += shift.x;
            pair.facade.y += shift.y;
        }
    }
    if let Some(fresco) = project.fresco.as_mut() {
        fresco.frame.x += shift.x;
        fresco.frame.y += shift.y;
    }
    true
}

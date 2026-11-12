//! Le redressement dans un projet (tâche 2.5) : la photo brute d'un
//! projecteur entre dans le projet, la mire y est détectée (seule ou à
//! partir de quatre coins posés à la main), la photo redressée est
//! enregistrée dans `photos-redressees/`, et peut être exportée en TIFF ou
//! en PNG à la résolution voulue.
//!
//! Chaque fonction lit le projet, fait son travail et l'enregistre : c'est
//! ce qu'appellent les commandes de l'app et `insitu projector …`.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use super::{
    detect, from_rectification, rectify, DetectOptions, DetectionError, ErrorSummary,
    TargetDetection, DEFAULT_SMOOTHING,
};
use crate::geometry::Point;
use crate::photo::{self, OutputFormat, PhotoError};
use crate::project::store::write_atomic;
use crate::project::{
    AssetKind, Mirror, Project, ProjectDir, ProjectError, Projector, Rectification,
    RectificationMethod, RelativePath,
};
use crate::units::{self, RECTIFIED_PHOTO_DPI, SLIDE_SIZE_MM};

/// Résolutions d'export acceptées, en points par pouce.
pub const EXPORT_DPI_RANGE: (f64, f64) = (50.0, 1400.0);

/// Points calculés sur chaque côté du carré de la mire, pour en tracer le
/// contour dans la photo (il est courbe quand le papier gondole).
const OUTLINE_POINTS_PER_SIDE: usize = 28;

/// Échec d'une opération de redressement dans le projet.
#[derive(Debug)]
pub enum RectifyError {
    Project(ProjectError),
    Detection(DetectionError),
    Photo(PhotoError),
    /// Aucun projecteur ne porte cet identifiant ou ce nom.
    UnknownProjector(String),
    /// Le libellé est vide ou déjà pris.
    BadLabel(String),
    /// Le projecteur n'a pas encore de photo brute.
    NoRawPhoto(String),
    /// Le projecteur n'a pas encore été redressé.
    NotRectified(String),
    /// Demande d'export incorrecte.
    BadExport(String),
}

impl fmt::Display for RectifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Project(e) => e.fmt(f),
            Self::Detection(e) => e.fmt(f),
            Self::Photo(e) => e.fmt(f),
            Self::UnknownProjector(key) => write!(f, "aucun projecteur « {key} » dans ce projet"),
            Self::BadLabel(message) | Self::BadExport(message) => f.write_str(message),
            Self::NoRawPhoto(label) => {
                write!(f, "le projecteur {label} n'a pas encore de photo de mire")
            }
            Self::NotRectified(label) => {
                write!(
                    f,
                    "la photo du projecteur {label} n'est pas encore redressée"
                )
            }
        }
    }
}

impl std::error::Error for RectifyError {}

impl From<ProjectError> for RectifyError {
    fn from(e: ProjectError) -> Self {
        Self::Project(e)
    }
}

impl From<DetectionError> for RectifyError {
    fn from(e: DetectionError) -> Self {
        Self::Detection(e)
    }
}

impl From<PhotoError> for RectifyError {
    fn from(e: PhotoError) -> Self {
        Self::Photo(e)
    }
}

/// Un croisement retenu, pour l'affichage.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeView {
    /// Position dans la photo brute, en px (centre du premier pixel en 0,5).
    pub photo: Point,
    /// Position sur la diapositive, en mm.
    pub slide: Point,
    /// Écart de validation croisée, en mm (majorant de l'erreur locale).
    pub error_mm: f64,
}

/// Ce que l'écran de redressement affiche d'un projecteur redressé.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RectificationView {
    pub projector_id: String,
    pub method: RectificationMethod,
    pub photo_width: u32,
    pub photo_height: u32,
    pub nodes: Vec<NodeView>,
    /// Croisements que la mire permet de mesurer.
    pub measurable_nodes: usize,
    /// Croisements mesurés puis écartés comme aberrants.
    pub outliers_removed: usize,
    /// Coins de la diapositive dans la photo (px) : haut-gauche, haut-droit,
    /// bas-droit, bas-gauche.
    pub corners: [Point; 4],
    /// Contour du carré de 280 mm dans la photo (px), fermé.
    pub outline: Vec<Point>,
    /// Validation croisée des croisements, en mm.
    pub leave_one_out_mm: ErrorSummary,
    /// Erreur estimée sur les zones couvertes de la carte de confiance, en mm.
    pub estimated_error_mm: ErrorSummary,
    /// Part de la mire à moins de 8 mm d'un croisement mesuré (0 à 1).
    pub coverage: f64,
    /// Taille d'un mm de mire au centre de la photo, en px.
    pub px_per_mm: f64,
    /// Durée du calcul, en secondes (absente quand le redressement est
    /// relu dans le projet).
    pub seconds: Option<f64>,
    /// Miroir appliqué à la diapositive.
    pub mirror: Mirror,
}

impl RectificationView {
    fn new(projector_id: &str, detection: &TargetDetection, seconds: Option<f64>) -> Self {
        let corner = |x, y| {
            detection
                .slide_to_photo(Point::new(x, y))
                .unwrap_or(Point::new(f64::NAN, f64::NAN))
        };
        let s = SLIDE_SIZE_MM;
        let n = OUTLINE_POINTS_PER_SIDE;
        let mut outline = Vec::with_capacity(4 * n + 1);
        let sides = [
            (Point::new(0.0, 0.0), Point::new(s, 0.0)),
            (Point::new(s, 0.0), Point::new(s, s)),
            (Point::new(s, s), Point::new(0.0, s)),
            (Point::new(0.0, s), Point::new(0.0, 0.0)),
        ];
        for (a, b) in sides {
            for k in 0..n {
                let t = k as f64 / n as f64;
                let p = Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
                if let Some(q) = detection.slide_to_photo(p) {
                    outline.push(q);
                }
            }
        }
        if let Some(&first) = outline.first() {
            outline.push(first);
        }
        let map = detection.confidence_map();
        let covered: Vec<f64> = map.error_mm.iter().flatten().copied().collect();
        Self {
            projector_id: projector_id.to_string(),
            method: detection.to_project_rectification().method,
            photo_width: detection.photo_width,
            photo_height: detection.photo_height,
            nodes: detection
                .nodes
                .iter()
                .map(|m| NodeView {
                    photo: m.photo,
                    slide: m.slide,
                    error_mm: m.leave_one_out_mm,
                })
                .collect(),
            measurable_nodes: detection.measurable_nodes,
            outliers_removed: detection.outliers_removed,
            corners: [
                corner(0.0, 0.0),
                corner(s, 0.0),
                corner(s, s),
                corner(0.0, s),
            ],
            outline,
            leave_one_out_mm: detection.leave_one_out_mm,
            estimated_error_mm: ErrorSummary::of(&covered),
            coverage: detection.coverage,
            px_per_mm: detection.px_per_mm,
            seconds,
            mirror: Mirror::default(),
        }
    }
}

/// Le projecteur désigné par son identifiant, ou à défaut par son libellé
/// (sans tenir compte des majuscules).
pub fn find_projector<'a>(project: &'a Project, key: &str) -> Option<&'a Projector> {
    project.projector(key).or_else(|| {
        project
            .projectors
            .iter()
            .find(|p| p.label.trim().eq_ignore_ascii_case(key.trim()))
    })
}

fn projector_id(project: &Project, key: &str) -> Result<String, RectifyError> {
    find_projector(project, key)
        .map(|p| p.id.clone())
        .ok_or_else(|| RectifyError::UnknownProjector(key.to_string()))
}

/// Ajoute un projecteur au projet et renvoie le projet enregistré et
/// l'identifiant du nouveau projecteur.
pub fn add_projector(dir: &ProjectDir, label: &str) -> Result<(Project, String), RectifyError> {
    let label = label.trim();
    if label.is_empty() {
        return Err(RectifyError::BadLabel(
            "le nom du projecteur est vide".into(),
        ));
    }
    let mut project = dir.load()?.project;
    if project
        .projectors
        .iter()
        .any(|p| p.label.trim().eq_ignore_ascii_case(label))
    {
        return Err(RectifyError::BadLabel(format!(
            "un projecteur s'appelle déjà « {label} »"
        )));
    }
    let id = project.add_projector(label).id.clone();
    dir.save(&project)?;
    Ok((project, id))
}

/// Copie la photo `source` dans `photos-brutes/` et en fait la photo de
/// mire du projecteur. La photo est entièrement lue avant d'être copiée :
/// une photo illisible n'entre pas dans le projet. Le redressement
/// précédent du projecteur est oublié (il valait pour l'ancienne photo) ;
/// l'ancienne photo redressée reste dans le dossier mais n'est plus citée.
pub fn import_raw_photo(
    dir: &ProjectDir,
    projector: &str,
    source: &Path,
) -> Result<Project, RectifyError> {
    let mut project = dir.load()?.project;
    let id = projector_id(&project, projector)?;
    photo::read(source)?;
    let relative = dir.import_file(source, AssetKind::RawPhoto)?;
    let p = project.projector_mut(&id).expect("projecteur trouvé");
    p.raw_photo = Some(relative);
    p.rectified_photo = None;
    p.rectification = None;
    dir.save(&project)?;
    Ok(project)
}

/// Détecte la mire dans la photo brute du projecteur, enregistre la photo
/// redressée (PNG à 500 dpi, comme les photos redressées d'Épinal) et les
/// correspondances dans le projet.
///
/// `corners` : position approchée des quatre coins de la mire dans la
/// photo (haut-gauche, haut-droit, bas-droit, bas-gauche de la
/// diapositive), à la place de la recherche automatique ; la grille affine
/// ensuite (D-2.4-e). Le redressement est alors noté « manuel ».
///
/// En cas d'échec, le projet n'est pas modifié.
pub fn rectify_projector(
    dir: &ProjectDir,
    projector: &str,
    corners: Option<[Point; 4]>,
) -> Result<(Project, RectificationView), RectifyError> {
    let start = Instant::now();
    let mut project = dir.load()?.project;
    let id = projector_id(&project, projector)?;
    let (raw_path, label) = raw_photo_of(&project, &id)?;
    let image = photo::read_rgb(&dir.resolve(&raw_path))?;
    let detection = detect(
        &image,
        &DetectOptions {
            corners,
            smoothing: DEFAULT_SMOOTHING,
        },
    )?;
    // Le miroir choisi pour ce projecteur est gardé d'une détection à l'autre.
    let mirror = project
        .projector(&id)
        .and_then(|p| p.rectification.as_ref())
        .map(|r| r.mirror)
        .unwrap_or_default();
    let mut rectification = detection.to_project_rectification();
    let detection = if mirror.is_none() {
        detection
    } else {
        mirror_rectification(&mut rectification, mirror);
        from_rectification((image.width(), image.height()), &rectification)?
    };
    let rectified = rectify(&image, &detection, RECTIFIED_PHOTO_DPI);
    drop(image);
    let bytes = photo::encode_rgb(&rectified, OutputFormat::Png, RECTIFIED_PHOTO_DPI)?;
    let relative = rectified_path(dir, &project, &id, &label)?;
    write_atomic(&dir.resolve(&relative), &bytes)?;
    let p = project.projector_mut(&id).expect("projecteur trouvé");
    p.rectified_photo = Some(relative);
    p.rectification = Some(rectification);
    dir.save(&project)?;
    let mut view = RectificationView::new(&id, &detection, Some(start.elapsed().as_secs_f64()));
    view.mirror = mirror;
    Ok((project, view))
}

/// Applique `change` aux points de la diapositive d'un redressement, et le
/// note dans son réglage de miroir.
fn mirror_rectification(rectification: &mut Rectification, change: Mirror) {
    for p in &mut rectification.points {
        p.slide = change.apply(p.slide, SLIDE_SIZE_MM);
    }
    rectification.mirror.horizontal ^= change.horizontal;
    rectification.mirror.vertical ^= change.vertical;
}

/// Retourne la diapositive du projecteur (D-2.5-j) : `change` dit quels
/// axes basculer (appliquer deux fois le même miroir revient au départ).
/// Les points du redressement et ceux du recalage sont retournés ensemble,
/// pour rester cohérents, et la photo redressée est recalculée.
pub fn mirror_projector(
    dir: &ProjectDir,
    projector: &str,
    change: Mirror,
) -> Result<(Project, RectificationView), RectifyError> {
    let start = Instant::now();
    let mut project = dir.load()?.project;
    let id = projector_id(&project, projector)?;
    let (raw_path, label) = raw_photo_of(&project, &id)?;
    let mut rectification = project
        .projector(&id)
        .and_then(|p| p.rectification.clone())
        .ok_or_else(|| RectifyError::NotRectified(label.clone()))?;
    mirror_rectification(&mut rectification, change);
    let image = photo::read_rgb(&dir.resolve(&raw_path))?;
    let detection = from_rectification((image.width(), image.height()), &rectification)?;
    let rectified = rectify(&image, &detection, RECTIFIED_PHOTO_DPI);
    drop(image);
    let bytes = photo::encode_rgb(&rectified, OutputFormat::Png, RECTIFIED_PHOTO_DPI)?;
    let relative = rectified_path(dir, &project, &id, &label)?;
    write_atomic(&dir.resolve(&relative), &bytes)?;
    let mirror = rectification.mirror;
    let p = project.projector_mut(&id).expect("projecteur trouvé");
    for pair in &mut p.registration {
        pair.slide = change.apply(pair.slide, SLIDE_SIZE_MM);
    }
    p.rectified_photo = Some(relative);
    p.rectification = Some(rectification);
    dir.save(&project)?;
    let mut view = RectificationView::new(&id, &detection, Some(start.elapsed().as_secs_f64()));
    view.mirror = mirror;
    Ok((project, view))
}

/// Relit le redressement enregistré du projecteur, pour l'afficher sans
/// refaire la détection ; `None` s'il n'a pas encore été redressé.
pub fn projector_view(
    dir: &ProjectDir,
    projector: &str,
) -> Result<Option<RectificationView>, RectifyError> {
    let project = dir.load()?.project;
    let id = projector_id(&project, projector)?;
    let p = project.projector(&id).expect("projecteur trouvé");
    let (Some(rectification), Some(raw)) = (&p.rectification, &p.raw_photo) else {
        return Ok(None);
    };
    let size = photo::dimensions(&dir.resolve(raw))?;
    let detection = from_rectification(size, rectification)?;
    let mut view = RectificationView::new(&id, &detection, None);
    view.mirror = rectification.mirror;
    Ok(Some(view))
}

/// Ce que l'export a écrit.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedImage {
    pub path: PathBuf,
    pub format: OutputFormat,
    pub dpi: f64,
    /// Côté de l'image, en pixels.
    pub size_px: u32,
    pub bytes: u64,
    pub seconds: f64,
}

/// Exporte la photo redressée du projecteur en TIFF ou en PNG (d'après
/// l'extension de `destination`), à `dpi` points par pouce. L'image est
/// recalculée depuis la photo brute avec les correspondances enregistrées :
/// elle n'est pas un agrandissement de la photo redressée du projet.
pub fn export_rectified(
    dir: &ProjectDir,
    projector: &str,
    destination: &Path,
    dpi: f64,
) -> Result<ExportedImage, RectifyError> {
    let start = Instant::now();
    let format = OutputFormat::from_path(destination).ok_or_else(|| {
        RectifyError::BadExport("l'export se fait en TIFF (.tif, .tiff) ou en PNG (.png)".into())
    })?;
    let (low, high) = EXPORT_DPI_RANGE;
    if !(dpi.is_finite() && dpi >= low && dpi <= high) {
        return Err(RectifyError::BadExport(format!(
            "la résolution doit être comprise entre {low} et {high} dpi"
        )));
    }
    let project = dir.load()?.project;
    let id = projector_id(&project, projector)?;
    let (raw_path, label) = raw_photo_of(&project, &id)?;
    let rectification = project
        .projector(&id)
        .and_then(|p| p.rectification.clone())
        .ok_or(RectifyError::NotRectified(label))?;
    let image = photo::read_rgb(&dir.resolve(&raw_path))?;
    let detection = from_rectification((image.width(), image.height()), &rectification)?;
    let rectified = rectify(&image, &detection, dpi);
    drop(image);
    let bytes = photo::encode_rgb(&rectified, format, dpi)?;
    write_atomic(destination, &bytes)?;
    Ok(ExportedImage {
        path: destination.to_path_buf(),
        format,
        dpi,
        size_px: units::mm_to_px(SLIDE_SIZE_MM, dpi).max(1),
        bytes: bytes.len() as u64,
        seconds: start.elapsed().as_secs_f64(),
    })
}

fn raw_photo_of(project: &Project, id: &str) -> Result<(RelativePath, String), RectifyError> {
    let p = project.projector(id).expect("projecteur trouvé");
    p.raw_photo
        .clone()
        .map(|raw| (raw, p.label.clone()))
        .ok_or_else(|| RectifyError::NoRawPhoto(p.label.clone()))
}

/// Nom de la photo redressée d'un projecteur : `photos-redressees/<nom>.png`,
/// d'après son libellé ; un suffixe s'il est déjà pris par un autre
/// projecteur. Le même projecteur garde le même fichier d'une fois à
/// l'autre.
fn rectified_path(
    dir: &ProjectDir,
    project: &Project,
    id: &str,
    label: &str,
) -> Result<RelativePath, RectifyError> {
    if let Some(existing) = project
        .projector(id)
        .and_then(|p| p.rectified_photo.clone())
    {
        return Ok(existing);
    }
    let stem = file_stem_for(label);
    let folder = AssetKind::RectifiedPhoto.directory();
    std::fs::create_dir_all(dir.root().join(folder)).map_err(ProjectError::io(
        "de créer le dossier",
        &dir.root().join(folder),
    ))?;
    let mut attempt = 1u32;
    loop {
        let name = if attempt == 1 {
            format!("{stem}.png")
        } else {
            format!("{stem}-{attempt}.png")
        };
        let relative = RelativePath::new(format!("{folder}/{name}"))?;
        let used = project
            .projectors
            .iter()
            .any(|p| p.rectified_photo.as_ref() == Some(&relative));
        // Un fichier qu'aucun projecteur ne cite (photo redressée d'une
        // ancienne photo brute) est remplacé.
        if !used {
            return Ok(relative);
        }
        attempt += 1;
    }
}

/// Nom de fichier lisible à partir d'un libellé : minuscules, lettres et
/// chiffres, accents retirés ; « projecteur » s'il ne reste rien.
fn file_stem_for(label: &str) -> String {
    let mut stem = String::new();
    for c in label.trim().chars().flat_map(char::to_lowercase) {
        let plain = match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            other => other,
        };
        if plain.is_ascii_alphanumeric() {
            stem.push(plain);
        } else if !stem.ends_with('-') && !stem.is_empty() {
            stem.push('-');
        }
    }
    let stem = stem.trim_end_matches('-').to_string();
    if stem.is_empty() {
        "projecteur".into()
    } else {
        stem
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_stems_are_readable() {
        assert_eq!(file_stem_for("A"), "a");
        assert_eq!(file_stem_for("Façade est n°2"), "facade-est-n-2");
        assert_eq!(file_stem_for("  "), "projecteur");
        assert_eq!(file_stem_for("../x"), "x");
    }
}

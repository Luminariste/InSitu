//! Export des diapositives pour la gravure (tâche 3.5).
//!
//! La fresque est importée en une seule image, posée sur un rectangle de la
//! façade (en mm réels). Pour chaque projecteur recalé, le moteur calcule
//! l'image de sa diapositive : chaque pixel de la diapositive est envoyé
//! sur la façade par le recalage (3.2), puis lu dans la fresque, avec le
//! moteur de déformation (3.4). Le PNG est écrit avec sa résolution inscrite,
//! pour s'ouvrir à la bonne taille dans LightBurn.
//!
//! Chaîne d'un pixel de sortie : pixel de la diapositive → mm de
//! diapositive (miroir éventuel) → mm de façade → pixel de la fresque.
//!
//! Options (réglages d'export du projet) : résolution, miroir, négatif,
//! plaque entière avec nom et repères dans la marge, ou image de 280 mm
//! seule ; niveaux de gris ou noir et blanc pur (1 bit) seuillé.

pub mod marks;
pub mod preview;
#[cfg(test)]
mod tests;

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use crate::geometry::{Point, PointWarp, Transform};
use crate::library::flatten_on_black;
use crate::project::model::RectMm;
use crate::project::store::write_atomic;
use crate::project::{AssetKind, ExportSettings, FrescoImage, Project, ProjectDir, ProjectError};
use crate::registration::slide_to_facade;
use crate::resample::{warp_raster, WarpOptions};
use crate::tiles::{self, Raster};
use crate::units::{self, PLATE_MARGIN_MM, SLIDE_SIZE_MM};

/// Résolutions d'export acceptées, en points par pouce.
pub const EXPORT_DPI_RANGE: (f64, f64) = (50.0, 3000.0);

/// Ce qui peut empêcher l'import de la fresque ou l'export.
#[derive(Debug)]
pub enum ExportError {
    Project(ProjectError),
    /// La façade n'est pas importée ou pas mise à l'échelle.
    NoFacadeScale,
    /// Aucune fresque n'est importée.
    NoFresco,
    /// Le fichier de la fresque est illisible.
    Image(String),
    /// Aucun projecteur ne porte ce nom ou cet identifiant.
    UnknownProjector(String),
    /// Un réglage est hors de son domaine ; le texte dit lequel.
    BadSettings(String),
    /// Le dossier de sortie est inutilisable.
    Destination(String),
    /// Aucun projecteur n'est exportable ; le texte dit pourquoi.
    Nothing(String),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Project(e) => write!(f, "{e}"),
            Self::NoFacadeScale => f.write_str(
                "la façade doit être importée et mise à l'échelle avant d'importer la fresque",
            ),
            Self::NoFresco => f.write_str("aucune fresque n'est importée dans ce projet"),
            Self::Image(m) => write!(f, "image de la fresque illisible : {m}"),
            Self::UnknownProjector(p) => write!(f, "aucun projecteur « {p} » dans ce projet"),
            Self::BadSettings(m) => write!(f, "réglage d'export invalide : {m}"),
            Self::Destination(m) => write!(f, "dossier d'export inutilisable : {m}"),
            Self::Nothing(m) => write!(f, "rien à exporter : {m}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<ProjectError> for ExportError {
    fn from(e: ProjectError) -> Self {
        Self::Project(e)
    }
}

// ---------------------------------------------------------------------------
// Import de la fresque
// ---------------------------------------------------------------------------

/// Copie l'image de la fresque dans le projet (`fresque/`) et la pose sur la
/// façade : sur `frame` si donné (mm de façade), sinon sur toute la largeur
/// de la façade, à partir du coin haut-gauche, en gardant ses proportions.
/// Remplace la fresque précédente.
pub fn import_fresco(
    dir: &ProjectDir,
    source: &Path,
    frame: Option<RectMm>,
) -> Result<Project, ExportError> {
    let mut project = dir.load()?.project;
    let (width, height) = image_size(source)?;
    let frame = match frame {
        Some(frame) => frame,
        None => {
            let (facade_width, _) = project
                .facade
                .as_ref()
                .and_then(|f| f.size_mm())
                .ok_or(ExportError::NoFacadeScale)?;
            RectMm {
                x: 0.0,
                y: 0.0,
                width: facade_width,
                height: facade_width * f64::from(height) / f64::from(width),
            }
        }
    };
    if !(frame.width > 0.0 && frame.height > 0.0 && frame.x.is_finite() && frame.y.is_finite()) {
        return Err(ExportError::BadSettings(
            "le rectangle de la fresque doit avoir une largeur et une hauteur positives".into(),
        ));
    }
    let stored = dir.import_file(source, AssetKind::Fresco)?;
    project.fresco = Some(FrescoImage {
        source: stored,
        frame,
        extra: Default::default(),
    });
    dir.save(&project)?;
    Ok(project)
}

fn image_size(path: &Path) -> Result<(u32, u32), ExportError> {
    let (w, h) = image::image_dimensions(path)
        .map_err(|e| ExportError::Image(format!("{} : {e}", path.display())))?;
    if w == 0 || h == 0 {
        return Err(ExportError::Image(format!(
            "{} : image vide",
            path.display()
        )));
    }
    Ok((w, h))
}

/// La fresque en niveaux de gris, posée sur du noir si elle a de la
/// transparence. Une couleur devient sa luminance (Rec. 601).
pub fn read_fresco(path: &Path) -> Result<Raster, ExportError> {
    let raster = tiles::read_raster(path).map_err(|e| ExportError::Image(e.to_string()))?;
    Ok(to_gray(flatten_on_black(raster)))
}

fn to_gray(raster: Raster) -> Raster {
    if raster.channels == 1 {
        return raster;
    }
    let data = raster
        .data
        .as_chunks::<3>()
        .0
        .iter()
        .map(|&[r, g, b]| {
            ((299 * u32::from(r) + 587 * u32::from(g) + 114 * u32::from(b) + 500) / 1000) as u8
        })
        .collect();
    Raster::new(raster.width, raster.height, 1, data)
}

// ---------------------------------------------------------------------------
// Calcul d'une diapositive
// ---------------------------------------------------------------------------

/// Vérifie les réglages et donne la taille d'un pixel, en mm.
pub fn check_settings(settings: &ExportSettings) -> Result<f64, ExportError> {
    let (low, high) = EXPORT_DPI_RANGE;
    if !(settings.dpi.is_finite() && settings.dpi >= low && settings.dpi <= high) {
        return Err(ExportError::BadSettings(format!(
            "la résolution doit être comprise entre {low} et {high} dpi"
        )));
    }
    if settings.one_bit && settings.threshold == 0 {
        return Err(ExportError::BadSettings("le seuil va de 1 à 255".into()));
    }
    Ok(units::px_size_mm(settings.dpi))
}

/// Côté de l'image de la diapositive (280 mm) et largeur de la marge, en
/// pixels, à `dpi`. La plaque fait `slide + 2 × margin` pixels.
pub fn sizes_px(dpi: f64) -> (u32, u32) {
    (
        units::mm_to_px(SLIDE_SIZE_MM, dpi).max(1),
        units::mm_to_px(PLATE_MARGIN_MM, dpi),
    )
}

/// Calcule l'image d'une diapositive, en niveaux de gris : la fresque vue
/// par le projecteur dont `warp` est le recalage (mm de diapositive → mm de
/// façade), puis le miroir, la marge avec ses repères et `caption`, et le
/// négatif, selon `settings`. Le noir et blanc pur est appliqué à
/// l'écriture (voir [`encode_png`]).
pub fn render_slide(
    fresco: &Raster,
    frame: &RectMm,
    warp: &PointWarp,
    caption: &str,
    settings: &ExportSettings,
    threads: usize,
) -> Result<Raster, ExportError> {
    let px_mm = check_settings(settings)?;
    assert_eq!(fresco.channels, 1, "fresque en niveaux de gris");
    let (side, margin) = sizes_px(settings.dpi);
    let to_fresco_x = f64::from(fresco.width) / frame.width;
    let to_fresco_y = f64::from(fresco.height) / frame.height;
    let mirror = settings.mirror;
    let map = |p: Point| {
        let x = if mirror { f64::from(side) - p.x } else { p.x };
        let facade = warp.apply(Point::new(x * px_mm, p.y * px_mm))?;
        Some(Point::new(
            (facade.x - frame.x) * to_fresco_x,
            (facade.y - frame.y) * to_fresco_y,
        ))
    };
    let options = WarpOptions {
        threads,
        ..WarpOptions::default()
    };
    let slide = warp_raster(fresco, side, side, map, &options)
        .map_err(|e| ExportError::BadSettings(e.to_string()))?;
    let mut image = if settings.margin {
        let n = side + 2 * margin;
        let mut plate = Raster::new(n, n, 1, vec![0; n as usize * n as usize]);
        let (stride, row) = (n as usize, side as usize);
        for (j, line) in slide.data.chunks_exact(row).enumerate() {
            let start = (j + margin as usize) * stride + margin as usize;
            plate.data[start..start + row].copy_from_slice(line);
        }
        drop(slide);
        marks::draw(&mut plate, px_mm, caption);
        plate
    } else {
        slide
    };
    if settings.negative {
        for v in &mut image.data {
            *v = 255 - *v;
        }
    }
    Ok(image)
}

/// Encode l'image d'une diapositive en PNG, avec sa résolution inscrite :
/// en niveaux de gris, ou en noir et blanc pur (1 bit) si `settings.one_bit`
/// (un pixel au moins aussi clair que le seuil devient blanc).
pub fn encode_png(image: &Raster, settings: &ExportSettings) -> Result<Vec<u8>, ExportError> {
    assert_eq!(image.channels, 1);
    let failed = |e: png::EncodingError| ExportError::Destination(format!("écriture du PNG : {e}"));
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, image.width, image.height);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_compression(png::Compression::Fast);
    let per_metre = (settings.dpi / 0.0254).round() as u32;
    encoder.set_pixel_dims(Some(png::PixelDimensions {
        xppu: per_metre,
        yppu: per_metre,
        unit: png::Unit::Meter,
    }));
    if settings.one_bit {
        encoder.set_depth(png::BitDepth::One);
        let packed = pack_bits(image, settings.threshold);
        let mut writer = encoder.write_header().map_err(failed)?;
        writer.write_image_data(&packed).map_err(failed)?;
        writer.finish().map_err(failed)?;
    } else {
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(failed)?;
        writer.write_image_data(&image.data).map_err(failed)?;
        writer.finish().map_err(failed)?;
    }
    Ok(bytes)
}

/// Huit pixels par octet, le premier dans le bit de poids fort ; chaque
/// ligne commence sur un octet neuf.
fn pack_bits(image: &Raster, threshold: u8) -> Vec<u8> {
    let width = image.width as usize;
    let row_bytes = width.div_ceil(8);
    let mut out = vec![0u8; row_bytes * image.height as usize];
    for (line, packed) in image
        .data
        .chunks_exact(width)
        .zip(out.chunks_exact_mut(row_bytes))
    {
        for (i, &v) in line.iter().enumerate() {
            if v >= threshold {
                packed[i / 8] |= 0x80 >> (i % 8);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Export d'un projet
// ---------------------------------------------------------------------------

/// Avancement : diapositives terminées sur le total, et celle en cours.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub done: usize,
    pub total: usize,
    pub current: Option<String>,
}

/// Une diapositive écrite.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedSlide {
    pub projector_id: String,
    pub label: String,
    pub path: PathBuf,
    /// Côté du PNG, en pixels.
    pub size_px: u32,
    /// Côté du PNG, en mm, à la résolution inscrite.
    pub size_mm: f64,
    pub bytes: u64,
    pub seconds: f64,
}

/// Un projecteur laissé de côté, et pourquoi.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedSlide {
    pub projector_id: String,
    pub label: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub slides: Vec<ExportedSlide>,
    pub skipped: Vec<SkippedSlide>,
    pub settings: ExportSettings,
}

/// Nom du fichier d'une diapositive : `<projet>-<diapositive>.png`, sans
/// caractère interdit dans un nom de fichier.
pub fn slide_file_name(project: &str, label: &str) -> String {
    let clean = |s: &str| {
        s.trim()
            .chars()
            .map(|c| {
                if "/\\:*?\"<>|".contains(c) || c.is_control() {
                    '-'
                } else {
                    c
                }
            })
            .collect::<String>()
    };
    let project = clean(project);
    let label = clean(label);
    if project.is_empty() {
        format!("{label}.png")
    } else {
        format!("{project}-{label}.png")
    }
}

/// Légende gravée dans la marge.
pub fn caption(project: &str, label: &str, settings: &ExportSettings) -> String {
    let mut text = format!("{project} - {label} - {} DPI", settings.dpi.round());
    if settings.mirror {
        text.push_str(" - MIROIR");
    }
    if settings.negative {
        text.push_str(" - NEGATIF");
    }
    text
}

/// Exporte les diapositives des projecteurs recalés (tous, ou ceux de
/// `only`, par nom ou identifiant) dans `destination`, avec `settings`, qui
/// deviennent les réglages d'export du projet. Un projecteur non recalé est
/// signalé dans le compte rendu, pas exporté.
pub fn export_slides(
    dir: &ProjectDir,
    destination: &Path,
    settings: &ExportSettings,
    only: Option<&[String]>,
    threads: usize,
    progress: &dyn Fn(ExportProgress),
) -> Result<ExportReport, ExportError> {
    check_settings(settings)?;
    let mut project = dir.load()?.project;
    let fresco = project.fresco.clone().ok_or(ExportError::NoFresco)?;
    if let Some(keys) = only {
        for key in keys {
            if !project
                .projectors
                .iter()
                .any(|p| &p.id == key || &p.label == key)
            {
                return Err(ExportError::UnknownProjector(key.clone()));
            }
        }
    }
    let mut todo = Vec::new();
    let mut skipped = Vec::new();
    for p in &project.projectors {
        if let Some(keys) = only {
            if !keys.iter().any(|k| k == &p.id || k == &p.label) {
                continue;
            }
        }
        match slide_to_facade(&p.registration) {
            Some(warp) => todo.push((p.id.clone(), p.label.clone(), warp)),
            None => skipped.push(SkippedSlide {
                projector_id: p.id.clone(),
                label: p.label.clone(),
                reason: format!(
                    "recalage incomplet : {} paire(s) de points sur 4 au moins",
                    p.registration.len()
                ),
            }),
        }
    }
    if todo.is_empty() {
        return Err(ExportError::Nothing(if project.projectors.is_empty() {
            "le projet n'a aucun projecteur".into()
        } else {
            "aucun projecteur n'est recalé (4 paires de points au moins)".into()
        }));
    }
    std::fs::create_dir_all(destination)
        .map_err(|e| ExportError::Destination(format!("{} : {e}", destination.display())))?;

    let total = todo.len();
    progress(ExportProgress {
        done: 0,
        total,
        current: Some(todo[0].1.clone()),
    });
    let image = read_fresco(&dir.resolve(&fresco.source))?;
    let mut slides = Vec::new();
    for (k, (id, label, warp)) in todo.iter().enumerate() {
        let start = Instant::now();
        let text = caption(&project.name, label, settings);
        let rendered = render_slide(&image, &fresco.frame, warp, &text, settings, threads)?;
        let bytes = encode_png(&rendered, settings)?;
        let path = destination.join(slide_file_name(&project.name, label));
        write_atomic(&path, &bytes).map_err(|e| ExportError::Destination(e.to_string()))?;
        slides.push(ExportedSlide {
            projector_id: id.clone(),
            label: label.clone(),
            path,
            size_px: rendered.width,
            size_mm: f64::from(rendered.width) * units::px_size_mm(settings.dpi),
            bytes: bytes.len() as u64,
            seconds: start.elapsed().as_secs_f64(),
        });
        progress(ExportProgress {
            done: k + 1,
            total,
            current: todo.get(k + 1).map(|t| t.1.clone()),
        });
    }
    if project.export != *settings {
        project.export = settings.clone();
        dir.save(&project)?;
    }
    Ok(ExportReport {
        slides,
        skipped,
        settings: settings.clone(),
    })
}

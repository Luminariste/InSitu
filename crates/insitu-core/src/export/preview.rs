//! Vue diapositive (tâche 3.6) : la diapositive calculée, en aperçu, à
//! superposer à la photo redressée du projecteur.
//!
//! L'aperçu est calculé exactement comme l'export (même chaîne, mêmes
//! filtres, voir [`super::render_slide`]), à une résolution d'écran
//! (2 048 px par défaut, 0,14 mm par pixel), sans marge, sans miroir ni
//! négatif : il se pose donc sur la photo redressée, qui est elle aussi en
//! mm de diapositive. Le noir devient transparent (blanc à opacité variable)
//! pour que l'interface le colore à sa guise.
//!
//! Les aperçus sont rangés dans le cache de l'app (`diapositives/<clé>.png`) ;
//! la clé tient compte du fichier de la fresque (chemin, taille, date), de
//! son rectangle, du recalage et de la taille : changer l'un d'eux refait
//! l'aperçu, rien d'autre.

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use super::{read_fresco, render_slide, ExportError};
use crate::library::{black_to_alpha, encode_png};
use crate::project::model::new_id;
use crate::project::{ExportSettings, Project, ProjectDir};
use crate::registration::slide_to_facade;
use crate::tiles::{self, Raster};
use crate::units::{MM_PER_INCH, SLIDE_SIZE_MM};

/// Côté des aperçus de diapositive, en pixels.
pub const SLIDE_PREVIEW_SIDE: u32 = 2048;
/// Dossier des aperçus dans le cache de l'app.
pub const SLIDE_PREVIEW_DIR: &str = "diapositives";

/// Un projecteur dans la vue diapositive.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideView {
    pub projector_id: String,
    pub label: String,
    /// Photo redressée (chemin relatif au projet), en mm de diapositive.
    pub rectified_photo: Option<String>,
    /// Aperçu de la diapositive dans le cache (`diapositives/<clé>.png`).
    pub preview: Option<String>,
    /// Ce qui manque pour calculer l'aperçu.
    pub problem: Option<String>,
}

/// La vue diapositive de tous les projecteurs du projet : l'aperçu de
/// chaque projecteur recalé est calculé s'il n'est pas déjà dans le cache.
/// La fresque n'est lue que si un aperçu manque.
pub fn slide_views(
    dir: &ProjectDir,
    cache_root: &Path,
    side: u32,
) -> Result<Vec<SlideView>, ExportError> {
    let project = dir.load()?.project;
    let mut fresco_image: Option<Raster> = None;
    let mut views = Vec::new();
    for p in &project.projectors {
        let mut view = SlideView {
            projector_id: p.id.clone(),
            label: p.label.clone(),
            rectified_photo: p.rectified_photo.as_ref().map(|r| r.as_str().to_string()),
            preview: None,
            problem: None,
        };
        let warp = slide_to_facade(&p.registration);
        match (&project.fresco, warp) {
            (None, _) => view.problem = Some("aucune fresque importée".into()),
            (_, None) => {
                view.problem = Some(format!(
                    "recalage incomplet : {} paire(s) de points sur 4 au moins",
                    p.registration.len()
                ))
            }
            (Some(fresco), Some(warp)) => {
                let request = preview_request(dir, &project, &p.id, side)?;
                if !cache_root.join(&request).is_file() {
                    if fresco_image.is_none() {
                        fresco_image = Some(read_fresco(&dir.resolve(&fresco.source))?);
                    }
                    let image = fresco_image.as_ref().expect("fresque lue");
                    let settings = ExportSettings {
                        dpi: f64::from(side) / SLIDE_SIZE_MM * MM_PER_INCH,
                        margin: false,
                        mirror: false,
                        negative: false,
                        one_bit: false,
                        ..ExportSettings::default()
                    };
                    let gray = render_slide(image, &fresco.frame, &warp, "", &settings, 0)?;
                    let rgba = Raster::new(gray.width, gray.height, 4, black_to_alpha(&gray));
                    store(cache_root, &request, &rgba)?;
                }
                view.preview = Some(request);
            }
        }
        views.push(view);
    }
    Ok(views)
}

/// Adresse de l'aperçu d'un projecteur dans le cache.
fn preview_request(
    dir: &ProjectDir,
    project: &Project,
    id: &str,
    side: u32,
) -> Result<String, ExportError> {
    let fresco = project.fresco.as_ref().ok_or(ExportError::NoFresco)?;
    let path = dir.resolve(&fresco.source);
    let describe = |e: std::io::Error| ExportError::Image(format!("{} : {e}", path.display()));
    let canonical = fs::canonicalize(&path).map_err(describe)?;
    let metadata = fs::metadata(&canonical).map_err(describe)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    let pairs = project
        .projector(id)
        .map(|p| serde_json::to_string(&p.registration).expect("sérialisable"))
        .unwrap_or_default();
    let frame = &fresco.frame;
    let identity = format!(
        "{}|{}|{modified}|{}|{}|{}|{}|{pairs}|{side}|d1",
        canonical.display(),
        metadata.len(),
        frame.x,
        frame.y,
        frame.width,
        frame.height
    );
    Ok(format!(
        "{SLIDE_PREVIEW_DIR}/{:016x}.png",
        tiles::fnv1a_64(identity.as_bytes())
    ))
}

/// Écrit l'aperçu dans le cache, d'un seul coup (fichier temporaire puis
/// renommage).
fn store(cache_root: &Path, request: &str, raster: &Raster) -> Result<(), ExportError> {
    let target = cache_root.join(request);
    let failed = |reason: String| {
        ExportError::Destination(format!("cache « {} » : {reason}", target.display()))
    };
    let bytes = encode_png(raster).map_err(failed)?;
    let folder = target.parent().expect("diapositives/<clé>.png");
    fs::create_dir_all(folder).map_err(|e| failed(e.to_string()))?;
    let temp = folder.join(format!(".~{}.{}", new_id(), std::process::id()));
    fs::write(&temp, &bytes).map_err(|e| failed(e.to_string()))?;
    fs::rename(&temp, &target).map_err(|e| {
        let _ = fs::remove_file(&temp);
        failed(e.to_string())
    })
}

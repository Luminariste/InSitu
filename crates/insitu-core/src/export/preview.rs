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
//! son rectangle, du recalage, de la taille, et des dessins posés (leurs
//! placements, calques masqués exclus, et leurs fichiers) : changer l'un
//! d'eux refait l'aperçu, rien d'autre.

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use super::drawings::{slide_rect, DrawingCache, Placing};
use super::{read_fresco, render_artwork, Artwork, ExportError};
use crate::library::{black_to_alpha, encode_png};
use crate::placement::adjust::{slide_drawings_in, SlideDrawing};
use crate::placement::assign::{assignments, slide_placements_in};
use crate::project::model::new_id;
use crate::project::{ExportSettings, Placement, Project, ProjectDir};
use crate::registration::slide_to_facade;
use crate::tiles::{self, Raster};
use crate::units::{self, MM_PER_INCH, SLIDE_SIZE_MM};

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
    /// Les dessins posés que porte cette diapositive, situés dedans avant
    /// leur retouche (tâche 4.6).
    pub drawings: Vec<SlideDrawing>,
    /// Les dessins de cette diapositive qui en dépassent : leur nom, et la
    /// part qui ne sera pas projetée, en m² (D-4.5-h).
    pub cut: Vec<CutDrawing>,
}

/// Un dessin que sa diapositive ne projette qu'en partie (D-4.5-h).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CutDrawing {
    pub placement: String,
    pub name: String,
    pub lost_m2: f64,
}

/// La vue diapositive de tous les projecteurs du projet : l'aperçu de
/// chaque projecteur recalé est calculé s'il n'est pas déjà dans le cache.
/// La fresque et les dessins ne sont lus que si un aperçu manque.
pub fn slide_views(
    dir: &ProjectDir,
    cache_root: &Path,
    side: u32,
) -> Result<Vec<SlideView>, ExportError> {
    let project = dir.load()?.project;
    let mut fresco_image: Option<Raster> = None;
    let placements = project.visible_placements();
    let assigned = assignments(&project);
    let lost = crate::placement::lost::project_lost_parts(dir, &project, &assigned, cache_root)
        .unwrap_or_default();
    let mut cache = DrawingCache::default();
    let settings = ExportSettings {
        dpi: f64::from(side) / SLIDE_SIZE_MM * MM_PER_INCH,
        margin: false,
        mirror: false,
        negative: false,
        one_bit: false,
        ..ExportSettings::default()
    };
    let px_mm = units::px_size_mm(settings.dpi);
    let mut views = Vec::new();
    for p in &project.projectors {
        let mut view = SlideView {
            projector_id: p.id.clone(),
            label: p.label.clone(),
            rectified_photo: p.rectified_photo.as_ref().map(|r| r.as_str().to_string()),
            preview: None,
            problem: None,
            drawings: Vec::new(),
            cut: lost
                .iter()
                .filter(|l| l.projector.as_deref() == Some(p.id.as_str()))
                .map(|l| CutDrawing {
                    placement: l.placement.clone(),
                    name: project
                        .placements
                        .iter()
                        .find(|q| q.id == l.placement)
                        .and_then(|q| project.drawing(&q.drawing))
                        .map_or_else(|| l.placement.clone(), |d| d.name.clone()),
                    lost_m2: l.area_m2,
                })
                .collect(),
        };
        let warp = slide_to_facade(&p.registration);
        let nothing = project.fresco.is_none() && placements.is_empty();
        match warp {
            _ if nothing => {
                view.problem = Some("ni fresque importée, ni dessin posé sur la façade".into())
            }
            None => {
                view.problem = Some(format!(
                    "recalage incomplet : {} paire(s) de points sur 4 au moins",
                    p.registration.len()
                ))
            }
            Some(warp) => {
                let mine = slide_placements_in(&project, &assigned, &p.id);
                view.drawings = slide_drawings_in(&project, &assigned, &p.id);
                let request = preview_request(dir, &project, &p.id, side, &mine)?;
                if !cache_root.join(&request).is_file() {
                    if let (None, Some(fresco)) = (&fresco_image, &project.fresco) {
                        fresco_image = Some(read_fresco(&dir.resolve(&fresco.source))?);
                    }
                    let (n, _) = super::sizes_px(settings.dpi);
                    // Seuls les dessins affectés à cette diapositive (tâche 4.5).
                    let seen: Vec<_> = mine
                        .iter()
                        .copied()
                        .filter(|q| {
                            let place = Placing {
                                warp: &warp,
                                px_mm,
                                side: n,
                                mirror: false,
                                projector: Some(&p.id),
                            };
                            slide_rect(q, &place).is_some()
                        })
                        .collect();
                    cache.load(dir, &project, &seen, px_mm)?;
                    let art = Artwork {
                        fresco: fresco_image
                            .as_ref()
                            .zip(project.fresco.as_ref().map(|f| &f.frame)),
                        drawings: cache.pair(&seen),
                        projector: Some(&p.id),
                    };
                    let gray = render_artwork(&art, &warp, "", &settings, 0)?;
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
    placements: &[&Placement],
) -> Result<String, ExportError> {
    let pairs = project
        .projector(id)
        .map(|p| serde_json::to_string(&p.registration).expect("sérialisable"))
        .unwrap_or_default();
    let mut identity = String::new();
    if let Some(fresco) = &project.fresco {
        let frame = &fresco.frame;
        identity = format!(
            "{}|{}|{}|{}|{}",
            file_identity(&dir.resolve(&fresco.source))?,
            frame.x,
            frame.y,
            frame.width,
            frame.height
        );
    }
    // Les dessins posés visibles affectés à cette diapositive, et le
    // fichier de chacun.
    identity.push_str(&serde_json::to_string(&placements).expect("sérialisable"));
    let mut used: Vec<&str> = placements.iter().map(|p| p.drawing.as_str()).collect();
    used.sort_unstable();
    used.dedup();
    for drawing in used {
        if let Some(d) = project.library.iter().find(|d| d.id == drawing) {
            identity.push('|');
            // Le dessin retouché à la gomme ou au crayon (tâche 4.8) : son nom
            // dépend des retouches.
            let file = crate::library::retouch::readable_file(dir, d);
            identity.push_str(&file.to_string_lossy());
            identity.push_str(&file_identity(&file).unwrap_or_default());
            // Une silhouette retouchée change ce qui est caché (tâche 4.7).
            if let Some(s) = &d.silhouette {
                identity.push_str(&serde_json::to_string(s).expect("sérialisable"));
            }
        }
    }
    let identity = format!("{identity}|{pairs}|{side}|d4");
    Ok(format!(
        "{SLIDE_PREVIEW_DIR}/{:016x}.png",
        tiles::fnv1a_64(identity.as_bytes())
    ))
}

/// Chemin, taille et date d'un fichier, pour reconnaître qu'il a changé.
fn file_identity(path: &Path) -> Result<String, ExportError> {
    let describe = |e: std::io::Error| ExportError::Image(format!("{} : {e}", path.display()));
    let canonical = fs::canonicalize(path).map_err(describe)?;
    let metadata = fs::metadata(&canonical).map_err(describe)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    Ok(format!(
        "{}|{}|{modified}",
        canonical.display(),
        metadata.len()
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

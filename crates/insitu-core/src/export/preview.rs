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
use crate::placement::assign::{shape_assignments_for, slide_placements_in};
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
    /// Le plan de la façade vu par ce projecteur (`diapositives/plan-<clé>.png`,
    /// blanc à opacité variable), s'il est demandé, à l'échelle et sous
    /// l'emprise (D-3.6-e).
    pub plan: Option<String>,
    /// Les dessins posés que porte cette diapositive, situés dedans avant
    /// leur retouche (tâche 4.6).
    pub drawings: Vec<SlideDrawing>,
    /// Les dessins de cette diapositive qui en dépassent : leur nom, et la
    /// part qui ne sera pas projetée, en m² (D-4.5-h).
    pub cut: Vec<CutDrawing>,
    /// Vrai si cette diapositive n'a pas été demandée (D-3.6-i) : son aperçu
    /// n'est donné que s'il était déjà calculé, et ses dessins coupés ne
    /// sont pas cherchés. Ils le seront quand on la choisira.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub pending: bool,
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
    slide_views_with(dir, cache_root, side, SlideContent::default())
}

/// Ce que montre la vue diapositive en plus des dessins posés.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlideContent {
    /// La fresque (D-3.6-d) : cachée dans le menu Affichage, elle l'est aussi
    /// dans la vue diapositive ; les dessins gardent leur silhouette.
    /// L'export n'est pas concerné : il grave toujours la fresque.
    pub fresco: bool,
    /// Le plan de la façade, en repère (D-3.6-e) ; calculé seulement s'il
    /// est demandé.
    pub plan: bool,
}

impl Default for SlideContent {
    fn default() -> Self {
        Self {
            fresco: true,
            plan: false,
        }
    }
}

/// Comme [`slide_views`], avec ou sans fresque, avec ou sans plan.
pub fn slide_views_with(
    dir: &ProjectDir,
    cache_root: &Path,
    side: u32,
    content: SlideContent,
) -> Result<Vec<SlideView>, ExportError> {
    slide_views_for(dir, cache_root, side, content, None)
}

/// Comme [`slide_views_with`], mais seul l'aperçu du projecteur `only`
/// (identifiant) est calculé s'il manque (D-3.6-i) : les autres projecteurs
/// sont décrits (nom, photo, problème, dessins), avec leur aperçu s'il est
/// déjà dans le cache, et marqués `pending`. `None` : tous sont calculés.
pub fn slide_views_for(
    dir: &ProjectDir,
    cache_root: &Path,
    side: u32,
    content: SlideContent,
    only: Option<&str>,
) -> Result<Vec<SlideView>, ExportError> {
    let mut project = dir.load()?.project;
    let has_fresco = project.fresco.is_some();
    if !content.fresco {
        project.fresco = None;
    }
    let mut fresco_image: Option<Raster> = None;
    let placements = project.visible_placements();
    // Le tracé des dessins (« à cheval », parts perdues) n'est revu que pour
    // la diapositive demandée (D-3.6-i).
    let assigned = shape_assignments_for(dir, &project, cache_root, only);
    let lost =
        crate::placement::lost::project_lost_parts(dir, &project, &assigned, cache_root, only)
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
            plan: None,
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
            pending: false,
        };
        // Ce projecteur est-il à calculer maintenant ?
        let wanted = only.is_none_or(|id| id == p.id);
        let warp = slide_to_facade(&p.registration);
        if let (true, Some(warp), Some(facade)) = (content.plan, &warp, &project.facade) {
            view.plan = plan_preview(dir, cache_root, facade, warp, &p.registration, side, wanted);
        }
        let nothing = !has_fresco && placements.is_empty();
        // Des dessins posés dans sa diapositive (D-3.6-g) : l'aperçu se fait
        // même sans recalage, avec eux seuls.
        let own = placements
            .iter()
            .any(|q| q.slide.as_deref() == Some(p.id.as_str()));
        match warp {
            _ if nothing => {
                view.problem = Some("ni fresque importée, ni dessin posé sur la façade".into())
            }
            None if !own => {
                view.problem = Some(format!(
                    "recalage incomplet : {} paire(s) de points sur 4 au moins",
                    p.registration.len()
                ))
            }
            warp => {
                let mine = slide_placements_in(&project, &assigned, &p.id);
                view.drawings = slide_drawings_in(&project, &assigned, &p.id);
                let request = preview_request(dir, &project, &p.id, side, &mine)?;
                if !wanted {
                    // Déjà calculée : on la montre ; ses détails (dessins
                    // coupés) viendront quand on la choisira.
                    view.pending = true;
                    if cache_root.join(&request).is_file() {
                        view.preview = Some(request);
                    }
                    views.push(view);
                    continue;
                }
                if !cache_root.join(&request).is_file() {
                    if let (None, Some(fresco), true) =
                        (&fresco_image, &project.fresco, warp.is_some())
                    {
                        fresco_image = Some(read_fresco(&dir.resolve(&fresco.source))?);
                    }
                    let (n, _) = super::sizes_px(settings.dpi);
                    // Seuls les dessins affectés à cette diapositive (tâche 4.5).
                    let seen: Vec<_> = mine
                        .iter()
                        .copied()
                        .filter(|q| {
                            let place = Placing {
                                warp: warp.as_ref(),
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
                        mask: p.mask.as_ref(),
                    };
                    let gray = render_artwork(&art, warp.as_ref(), "", &settings, 0)?;
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

/// Le plan vu par un projecteur, dans le cache (calculé s'il n'y est pas) ;
/// `None` si le plan manque ou est illisible : la vue se fait sans.
fn plan_preview(
    dir: &ProjectDir,
    cache_root: &Path,
    facade: &crate::project::Facade,
    warp: &crate::geometry::PointWarp,
    registration: &[crate::project::RegistrationPair],
    side: u32,
    compute: bool,
) -> Option<String> {
    let identity = format!(
        "plan|{}|{}|{}|{:?}|{}|{side}|p1",
        file_identity(&dir.resolve(&facade.source)).ok()?,
        serde_json::to_string(&facade.scale).ok()?,
        facade.rotation,
        facade.page,
        serde_json::to_string(registration).ok()?,
    );
    let request = format!(
        "{SLIDE_PREVIEW_DIR}/plan-{:016x}.png",
        tiles::fnv1a_64(identity.as_bytes())
    );
    // Pas encore calculé, et pas demandé maintenant : il le sera plus tard.
    if !compute && !cache_root.join(&request).is_file() {
        return None;
    }
    if !cache_root.join(&request).is_file() {
        let (plan, frame) = super::plan_view::plan_under_footprint(
            dir,
            facade,
            warp,
            super::plan_view::PLAN_WINDOW_PX,
        )?;
        let settings = ExportSettings {
            dpi: f64::from(side) / SLIDE_SIZE_MM * MM_PER_INCH,
            margin: false,
            mirror: false,
            negative: false,
            one_bit: false,
            ..ExportSettings::default()
        };
        let art = Artwork {
            fresco: Some((&plan, &frame)),
            drawings: Vec::new(),
            projector: None,
            mask: None,
        };
        let gray = render_artwork(&art, Some(warp), "", &settings, 0).ok()?;
        let rgba = Raster::new(gray.width, gray.height, 4, black_to_alpha(&gray));
        store(cache_root, &request, &rgba).ok()?;
    }
    Some(request)
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
        .map(|p| {
            // Le masque général de la diapositive (D-3.6-h) change l'aperçu.
            let registration = serde_json::to_string(&p.registration).expect("sérialisable");
            let mask = serde_json::to_string(&p.mask).expect("sérialisable");
            format!("{registration}|{mask}")
        })
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

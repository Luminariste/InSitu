//! Coque de bureau : fenêtre, menus, accès aux fichiers, et pont vers le
//! moteur. Aucun calcul ici : une commande Tauri appelle `insitu_core` et
//! renvoie le résultat à l'interface.

mod menu;
mod updates;

use std::path::{Path, PathBuf};

use insitu_core::assembly::auto::AutoReport;
use insitu_core::assembly::{self, AssemblyReport, RenderOptions, RenderReport};
use insitu_core::export::preview::{self as slide_preview, SlideView};
use insitu_core::export::{self, ExportReport};
use insitu_core::facade::{self, Alignment, FacadeDisplay, PlanChoice, SourceInfo};
use insitu_core::geometry::Point;
use insitu_core::illustrator::{self, CompositionPreview, CompositionReport};
use insitu_core::library::organize;
use insitu_core::library::shared::{self, SharedCopyReport, SharedStatus};
use insitu_core::library::silhouette::{self, DrawingSilhouette};
use insitu_core::library::vectors::{self, VectorSummary, VectorizeReport};
use insitu_core::library::{self, ImportOptions, ImportReport, LibraryEntry};
use insitu_core::overview::{self, FacadeOverview, RegistrationPreview};
use insitu_core::placement;
use insitu_core::project::history::{self, HistoryState};
use insitu_core::project::{
    AssemblyPair, ExportSettings, Layer, Mirror, Placement, PointMm, Project, ProjectDir,
    RegistrationLine, RegistrationPair, RetouchStroke, SilhouettePolygon, SlideAdjustment,
    SourcePoint, VectorSettings,
};
use insitu_core::rectification::projector::{self as projectors, ExportedImage, RectificationView};
use insitu_core::registration::auto::AdjustReport;
use insitu_core::registration::{self, RegistrationOptions, RegistrationReport};
use insitu_core::settings::{self, AppSettings};
use insitu_core::sheet::{self, SheetReport};
use insitu_core::simulation::{self, Simulation};
use insitu_core::tiles;
use tauri::http::{header, Request, Response, StatusCode};
use tauri::{AppHandle, Emitter, Manager, Runtime, UriSchemeContext, UriSchemeResponder};

/// Schéma d'adresse des tuiles : l'interface les lit à
/// `tuiles://localhost/<clé>/<niveau>/<colonne>_<ligne>.png`.
const TILE_SCHEME: &str = "tuiles";

/// Dossier où l'app range les pyramides de tuiles (dans le cache de l'app :
/// macOS peut le vider, les tuiles se refabriquent).
fn tile_cache_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let cache = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    Ok(cache.join("tuiles"))
}

#[tauri::command]
fn engine_info() -> insitu_core::EngineInfo {
    insitu_core::engine_info()
}

/// Prépare une grande image pour la visionneuse. `async` : la fabrication
/// des tuiles ne bloque pas la fenêtre.
#[tauri::command(async)]
fn open_tiled_image(app: AppHandle, path: String) -> Result<tiles::CachedPyramid, String> {
    tiles::ensure_pyramid(
        Path::new(&path),
        &tile_cache_dir(&app)?,
        &tiles::TileOptions::default(),
    )
    .map_err(|e| e.to_string())
}

// --- Projet et façade (tâche 3.1) ------------------------------------------

/// Crée un projet vide dans le dossier `folder` (qui finit par `.insitu`).
#[tauri::command(async)]
fn project_create(folder: String, name: Option<String>) -> Result<Project, String> {
    ProjectDir::create(&folder, name.as_deref())
        .map(|(_, project)| project)
        .map_err(|e| e.to_string())
}

/// Lit le projet du dossier `folder`.
#[tauri::command(async)]
fn project_open(folder: String) -> Result<Project, String> {
    ProjectDir::open(&folder)
        .and_then(|dir| dir.load())
        .map(|loaded| loaded.project)
        .map_err(|e| e.to_string())
}

/// Nature, dimensions et pages d'un fichier de plan, avant import.
#[tauri::command(async)]
fn facade_inspect(path: String, page: Option<u32>) -> Result<SourceInfo, String> {
    facade::inspect(Path::new(&path), page).map_err(|e| e.to_string())
}

/// Copie le plan dans le projet et en fait sa façade.
#[tauri::command(async)]
fn facade_import(folder: String, path: String, page: Option<u32>) -> Result<Project, String> {
    facade::import_facade(&project_dir(&folder)?, Path::new(&path), page).map_err(|e| e.to_string())
}

/// Retire le plan de la façade ; le fichier reste dans le projet.
#[tauri::command(async)]
fn facade_remove(folder: String) -> Result<Project, String> {
    facade::remove_facade(&project_dir(&folder)?).map_err(|e| e.to_string())
}

/// Tourne le plan de la façade de quarts de tour (D15-k).
#[tauri::command(async)]
fn facade_rotate(folder: String, quarter_turns: i32) -> Result<Project, String> {
    facade::rotate_facade(&project_dir(&folder)?, quarter_turns).map_err(|e| e.to_string())
}

/// Met la façade du projet à l'échelle par deux points et une cote.
#[tauri::command(async)]
fn facade_set_scale(
    folder: String,
    a: SourcePoint,
    b: SourcePoint,
    distance_mm: f64,
) -> Result<Project, String> {
    facade::set_scale(&project_dir(&folder)?, a, b, distance_mm).map_err(|e| e.to_string())
}

/// Prépare l'affichage de la façade du projet (tuiles dans le cache).
#[tauri::command(async)]
fn facade_display(app: AppHandle, folder: String) -> Result<FacadeDisplay, String> {
    let dir = project_dir(&folder)?;
    facade::open_project_facade(&dir, &tile_cache_dir(&app)?).map_err(|e| e.to_string())
}

// --- Plans ajoutés à l'espace de travail (D-PL-a à g) ---------------------------

/// Ajoute un groupe de plans tiré d'un fichier (une vue par choix).
#[tauri::command(async)]
fn plans_add(folder: String, path: String, choices: Vec<PlanChoice>) -> Result<Project, String> {
    facade::add_plan_group(&project_dir(&folder)?, Path::new(&path), &choices)
        .map_err(|e| e.to_string())
}

/// Ajoute des vues (pages, calques) à un groupe de plans.
#[tauri::command(async)]
fn plans_add_views(
    folder: String,
    group: String,
    choices: Vec<PlanChoice>,
) -> Result<Project, String> {
    facade::add_plans(&project_dir(&folder)?, &group, &choices).map_err(|e| e.to_string())
}

/// Pose un groupe : coin haut-gauche (mm) et mm par unité du fichier.
#[tauri::command(async)]
fn plans_place(
    folder: String,
    group: String,
    origin: PointMm,
    mm_per_unit: f64,
) -> Result<Project, String> {
    facade::place_plan_group(&project_dir(&folder)?, &group, origin, mm_per_unit)
        .map_err(|e| e.to_string())
}

/// Cale un groupe par deux paires de points (mm de façade).
#[tauri::command(async)]
fn plans_align(
    folder: String,
    group: String,
    from: [PointMm; 2],
    to: [PointMm; 2],
) -> Result<Alignment, String> {
    facade::align_plan_group(&project_dir(&folder)?, &group, from, to).map_err(|e| e.to_string())
}

/// Tourne un groupe de quarts de tour autour de son centre.
#[tauri::command(async)]
fn plans_rotate(folder: String, group: String, quarter_turns: i32) -> Result<Project, String> {
    facade::rotate_plan_group(&project_dir(&folder)?, &group, quarter_turns)
        .map_err(|e| e.to_string())
}

/// Montre ou cache un groupe, ou une de ses vues.
#[tauri::command(async)]
fn plans_set_hidden(
    folder: String,
    group: String,
    plan: Option<String>,
    hidden: bool,
) -> Result<Project, String> {
    facade::set_plan_hidden(&project_dir(&folder)?, &group, plan.as_deref(), hidden)
        .map_err(|e| e.to_string())
}

/// Renomme un groupe, ou une de ses vues.
#[tauri::command(async)]
fn plans_rename(
    folder: String,
    group: String,
    plan: Option<String>,
    name: String,
) -> Result<Project, String> {
    facade::rename_plan(&project_dir(&folder)?, &group, plan.as_deref(), &name)
        .map_err(|e| e.to_string())
}

/// Retire un groupe, ou une de ses vues ; le fichier reste dans le projet.
#[tauri::command(async)]
fn plans_remove(folder: String, group: String, plan: Option<String>) -> Result<Project, String> {
    facade::remove_plan(&project_dir(&folder)?, &group, plan.as_deref()).map_err(|e| e.to_string())
}

/// Change la place d'un groupe dans la pile (0 : au fond).
#[tauri::command(async)]
fn plans_move(folder: String, group: String, to: usize) -> Result<Project, String> {
    facade::move_plan_group(&project_dir(&folder)?, &group, to).map_err(|e| e.to_string())
}

/// Prépare l'affichage d'une vue d'un groupe (tuiles dans le cache).
#[tauri::command(async)]
fn plans_display(
    app: AppHandle,
    folder: String,
    group: String,
    plan: String,
) -> Result<FacadeDisplay, String> {
    facade::open_plan_display(
        &project_dir(&folder)?,
        &group,
        &plan,
        &tile_cache_dir(&app)?,
    )
    .map_err(|e| e.to_string())
}

// --- Redressement des photos de mire (tâche 2.5) ------------------------------

/// Dossier de l'historique d'annulation (tâche 4.3), dans le cache de l'app ;
/// fixé au démarrage.
static HISTORY_ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Le projet, ouvert avec l'historique d'annulation : tout enregistrement
/// fait par une commande s'annule par ⌘Z.
fn project_dir(folder: &str) -> Result<ProjectDir, String> {
    let dir = ProjectDir::open(folder).map_err(|e| e.to_string())?;
    Ok(match HISTORY_ROOT.get() {
        Some(root) => dir.with_history(root),
        None => dir,
    })
}

// --- Annulation (tâche 4.3) ---------------------------------------------------------

/// Annule la dernière action (`redo` faux) ou rétablit la dernière annulée.
#[tauri::command(async)]
fn history_step(folder: String, redo: bool) -> Result<(Project, String), String> {
    history::step(&project_dir(&folder)?, redo).map_err(|e| e.to_string())
}

/// Ce que ⌘Z et ⇧⌘Z feraient.
#[tauri::command(async)]
fn history_state(folder: String) -> Result<HistoryState, String> {
    history::state(&project_dir(&folder)?).map_err(|e| e.to_string())
}

/// Ajoute un projecteur ; renvoie le projet et l'identifiant du nouveau.
#[tauri::command(async)]
fn projector_add(folder: String, label: String) -> Result<(Project, String), String> {
    projectors::add_projector(&project_dir(&folder)?, &label).map_err(|e| e.to_string())
}

/// Renomme un projecteur (son travail est gardé).
#[tauri::command(async)]
fn projector_rename(folder: String, projector: String, label: String) -> Result<Project, String> {
    projectors::rename_projector(&project_dir(&folder)?, &projector, &label)
        .map_err(|e| e.to_string())
}

/// Retire un projecteur du projet ; ses photos restent dans le dossier.
#[tauri::command(async)]
fn projector_remove(folder: String, projector: String) -> Result<Project, String> {
    projectors::remove_projector(&project_dir(&folder)?, &projector).map_err(|e| e.to_string())
}

/// Copie une photo de mire dans le projet et la donne au projecteur.
#[tauri::command(async)]
fn projector_import_photo(
    folder: String,
    projector: String,
    path: String,
) -> Result<Project, String> {
    projectors::import_raw_photo(&project_dir(&folder)?, &projector, Path::new(&path))
        .map_err(|e| e.to_string())
}

/// Détecte la mire (seule, ou à partir de quatre coins) et enregistre la
/// photo redressée.
#[tauri::command(async)]
fn projector_rectify(
    folder: String,
    projector: String,
    corners: Option<[Point; 4]>,
) -> Result<(Project, RectificationView), String> {
    projectors::rectify_projector(&project_dir(&folder)?, &projector, corners)
        .map_err(|e| e.to_string())
}

/// Retourne la diapositive d'un projecteur (miroir horizontal ou vertical).
#[tauri::command(async)]
fn projector_mirror(
    folder: String,
    projector: String,
    change: Mirror,
) -> Result<(Project, RectificationView), String> {
    projectors::mirror_projector(&project_dir(&folder)?, &projector, change)
        .map_err(|e| e.to_string())
}

/// Tourne la diapositive d'un projecteur d'un quart de tour.
#[tauri::command(async)]
fn projector_rotate(
    folder: String,
    projector: String,
) -> Result<(Project, RectificationView), String> {
    projectors::rotate_projector(&project_dir(&folder)?, &projector).map_err(|e| e.to_string())
}

/// Relit le redressement enregistré d'un projecteur.
#[tauri::command(async)]
fn projector_view(folder: String, projector: String) -> Result<Option<RectificationView>, String> {
    projectors::projector_view(&project_dir(&folder)?, &projector).map_err(|e| e.to_string())
}

/// Exporte la photo redressée en TIFF ou en PNG.
#[tauri::command(async)]
fn projector_export(
    folder: String,
    projector: String,
    destination: String,
    dpi: f64,
) -> Result<ExportedImage, String> {
    projectors::export_rectified(
        &project_dir(&folder)?,
        &projector,
        Path::new(&destination),
        dpi,
    )
    .map_err(|e| e.to_string())
}

// --- Recalage sur la façade (tâche 3.2) ---------------------------------------

/// Compte rendu du recalage enregistré d'un projecteur.
#[tauri::command(async)]
fn registration_show(folder: String, projector: String) -> Result<RegistrationReport, String> {
    registration::projector_report(
        &project_dir(&folder)?,
        &projector,
        &RegistrationOptions::default(),
    )
    .map_err(|e| e.to_string())
}

/// Remplace les paires de points d'un projecteur et rend le compte rendu.
#[tauri::command(async)]
fn registration_set(
    folder: String,
    projector: String,
    pairs: Vec<RegistrationPair>,
    lines: Option<Vec<RegistrationLine>>,
) -> Result<(Project, RegistrationReport), String> {
    registration::set_registration_with_lines(
        &project_dir(&folder)?,
        &projector,
        pairs,
        lines,
        &RegistrationOptions::default(),
    )
    .map_err(|e| e.to_string())
}

/// Pose provisoire de la diapositive sur la façade pendant un glisser (D-3.2-h) :
/// rien n'est enregistré.
#[tauri::command(async)]
fn registration_preview(pairs: Vec<RegistrationPair>) -> RegistrationPreview {
    overview::registration_preview(&pairs)
}

/// Cale la pose affichée de la photo sur les lignes du plan (D-3.2-i) :
/// rien n'est enregistré.
#[tauri::command(async)]
fn registration_adjust(
    folder: String,
    projector: String,
    pairs: Vec<RegistrationPair>,
) -> Result<AdjustReport, String> {
    registration::auto::projector_adjustment(&project_dir(&folder)?, &projector, Some(&pairs))
        .map_err(|e| e.to_string())
}

// --- Assemblage de photos de façade (tâche 6.4) -----------------------------

/// Compte rendu de l'assemblage : chaque photo, ses points, sa place.
#[tauri::command(async)]
fn assembly_show(folder: String) -> Result<AssemblyReport, String> {
    assembly::project_report(&project_dir(&folder)?).map_err(|e| e.to_string())
}

/// Ajoute une photo (copiée dans le projet) ; rend le projet et son id.
#[tauri::command(async)]
fn assembly_add(
    folder: String,
    path: String,
    anchor: Option<String>,
) -> Result<(Project, String), String> {
    assembly::add_photo(&project_dir(&folder)?, Path::new(&path), anchor.as_deref())
        .map_err(|e| e.to_string())
}

/// Retire une photo de l'assemblage.
#[tauri::command(async)]
fn assembly_remove(folder: String, photo: String) -> Result<Project, String> {
    assembly::remove_photo(&project_dir(&folder)?, &photo).map_err(|e| e.to_string())
}

/// Remplace la photo d'appui et les points communs d'une photo.
#[tauri::command(async)]
fn assembly_set_pairs(
    folder: String,
    photo: String,
    anchor: String,
    pairs: Vec<AssemblyPair>,
) -> Result<(Project, AssemblyReport), String> {
    assembly::set_pairs(&project_dir(&folder)?, &photo, &anchor, pairs).map_err(|e| e.to_string())
}

/// Raccord automatique : le moteur trouve les points communs (tâche 6.6).
#[tauri::command(async)]
fn assembly_auto(folder: String) -> Result<(Project, AutoReport), String> {
    assembly::auto::analyze(&project_dir(&folder)?).map_err(|e| e.to_string())
}

/// Calcule l'image assemblée et en fait la façade du projet.
#[tauri::command(async)]
fn assembly_render(folder: String) -> Result<(Project, RenderReport), String> {
    assembly::render(&project_dir(&folder)?, &RenderOptions::default()).map_err(|e| e.to_string())
}

// --- Bibliothèque de dessins (tâche 4.1) --------------------------------------

/// Événement d'avancement d'un import : `{ done, total }`.
const LIBRARY_PROGRESS_EVENT: &str = "bibliotheque-progression";

/// Importe un dossier de dessins (vignettes comprises) ; l'avancement est
/// envoyé à l'interface au fil de l'eau.
#[tauri::command(async)]
fn library_import(
    app: AppHandle,
    folder: String,
    source: String,
) -> Result<(Project, ImportReport), String> {
    let options = ImportOptions {
        cache_root: Some(tile_cache_dir(&app)?),
        threads: 0,
    };
    library::import_folder(
        &project_dir(&folder)?,
        Path::new(&source),
        &options,
        &|progress| {
            let _ = app.emit(LIBRARY_PROGRESS_EVENT, progress);
        },
    )
    .map_err(|e| e.to_string())
}

/// Importe des dessins choisis un à un, ou déposés depuis le Finder
/// (D-4.1-f) : `files` donne pour chacun le fichier et son chemin dans la
/// bibliothèque. `staging` : dossier de dépôt à effacer ensuite.
#[tauri::command(async)]
fn library_import_files(
    app: AppHandle,
    folder: String,
    files: Vec<(String, String)>,
    staging: Option<String>,
) -> Result<(Project, ImportReport, Vec<String>), String> {
    let options = ImportOptions {
        cache_root: Some(tile_cache_dir(&app)?),
        threads: 0,
    };
    let files: Vec<(PathBuf, String)> = files
        .into_iter()
        .map(|(path, inner)| (PathBuf::from(path), inner))
        .collect();
    let result = library::import_loose(&project_dir(&folder)?, &files, &options, &|progress| {
        let _ = app.emit(LIBRARY_PROGRESS_EVENT, progress);
    })
    .map_err(|e| e.to_string());
    if let Some(staging) = staging {
        let _ = std::fs::remove_dir_all(staging);
    }
    result
}

/// Reçoit en octets un fichier déposé depuis le Finder (la vue web ne donne
/// pas son chemin, D-4.1-f) et le range dans un dossier de dépôt du cache.
/// En-tête `x-depot` : JSON `{ "depot": "...", "chemin": "Oiseaux/merle.png" }`.
/// Rend `[fichier, chemin]`, ou rien si le fichier est ignoré (caché).
#[tauri::command(async)]
fn library_stage_dropped(
    app: AppHandle,
    request: tauri::ipc::Request<'_>,
) -> Result<Option<(String, String, String)>, String> {
    #[derive(serde::Deserialize)]
    struct Header {
        depot: String,
        chemin: String,
    }
    let header = request
        .headers()
        .get("x-depot")
        .and_then(|v| v.to_str().ok())
        .ok_or("fichier déposé sans en-tête")?;
    let header: Header = serde_json::from_str(header).map_err(|e| e.to_string())?;
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("fichier déposé sans contenu".into());
    };
    let staging = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("depot")
        .join(library::clean_staging_id(&header.depot));
    let staged =
        library::stage_dropped_file(&staging, &header.chemin, bytes).map_err(|e| e.to_string())?;
    Ok(staged.map(|(path, inner)| {
        (
            path.to_string_lossy().into_owned(),
            inner,
            staging.to_string_lossy().into_owned(),
        )
    }))
}

/// Recrée la composition d'un fichier Illustrator (tâche 6.3) : liens dans la
/// bibliothèque, images placées en dessins posés.
/// Ce que l'import d'un fichier Illustrator ferait, sans rien écrire.
#[tauri::command(async)]
fn illustrator_preview(folder: String, path: String) -> Result<CompositionPreview, String> {
    illustrator::preview_composition(&project_dir(&folder)?, Path::new(&path), None)
        .map_err(|e| e.to_string())
}

/// `images` : les rangs des images choisies dans l'aperçu ; absent : toutes.
#[tauri::command(async)]
fn illustrator_import(
    app: AppHandle,
    folder: String,
    path: String,
    images: Option<Vec<usize>>,
) -> Result<(Project, CompositionReport), String> {
    let options = ImportOptions {
        cache_root: Some(tile_cache_dir(&app)?),
        threads: 0,
    };
    illustrator::import_composition(
        &project_dir(&folder)?,
        Path::new(&path),
        None,
        images.as_deref(),
        &options,
        &|progress| {
            let _ = app.emit(LIBRARY_PROGRESS_EVENT, progress);
        },
    )
    .map_err(|e| e.to_string())
}

/// Fichier des réglages de l'app (fenêtre Réglages…, D-4.1-j).
fn settings_file<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join(settings::SETTINGS_FILE))
}

/// Les réglages de l'app.
#[tauri::command(async)]
fn settings_get(app: AppHandle) -> Result<AppSettings, String> {
    Ok(settings::load(&settings_file(&app)?))
}

/// Choisit le dossier de la bibliothèque commune (absent : aucune) ; la
/// bibliothèque y est créée si le dossier n'en est pas déjà une.
#[tauri::command(async)]
fn settings_set_shared_library(
    app: AppHandle,
    folder: Option<String>,
) -> Result<AppSettings, String> {
    let file = settings_file(&app)?;
    let mut current = settings::load(&file);
    current.shared_library = match folder {
        Some(folder) => {
            Some(shared::open_or_create(Path::new(&folder)).map_err(|e| e.to_string())?)
        }
        None => None,
    };
    settings::save(&file, &current)?;
    Ok(current)
}

/// L'état de chaque dessin du projet venu de la bibliothèque commune.
#[tauri::command(async)]
fn library_shared_status(folder: String, shared: String) -> Result<Vec<SharedStatus>, String> {
    shared::status(&project_dir(&folder)?, &project_dir(&shared)?).map_err(|e| e.to_string())
}

/// Copie des dessins de la bibliothèque commune dans le projet (ou reprend
/// leur nouvelle version).
#[tauri::command(async)]
fn library_shared_copy(
    app: AppHandle,
    folder: String,
    shared: String,
    drawings: Vec<String>,
) -> Result<(Project, SharedCopyReport), String> {
    let options = ImportOptions {
        cache_root: Some(tile_cache_dir(&app)?),
        threads: 0,
    };
    shared::copy_to_project(
        &project_dir(&shared)?,
        &project_dir(&folder)?,
        &drawings,
        &options,
        &|progress| {
            let _ = app.emit(LIBRARY_PROGRESS_EVENT, progress);
        },
    )
    .map_err(|e| e.to_string())
}

/// Envoie des dessins du projet dans la bibliothèque commune.
#[tauri::command(async)]
fn library_shared_send(
    app: AppHandle,
    folder: String,
    shared: String,
    drawings: Vec<String>,
) -> Result<(Project, SharedCopyReport), String> {
    let options = ImportOptions {
        cache_root: Some(tile_cache_dir(&app)?),
        threads: 0,
    };
    shared::send_to_shared(
        &project_dir(&folder)?,
        &project_dir(&shared)?,
        &drawings,
        &options,
        &|progress| {
            let _ = app.emit(LIBRARY_PROGRESS_EVENT, progress);
        },
    )
    .map_err(|e| e.to_string())
}

/// La bibliothèque du projet, avec la vignette de chaque dessin.
#[tauri::command(async)]
fn library_list(app: AppHandle, folder: String) -> Result<Vec<LibraryEntry>, String> {
    library::list(&project_dir(&folder)?, Some(&tile_cache_dir(&app)?), 0)
        .map_err(|e| e.to_string())
}

/// Retire un dessin de la bibliothèque, et ses placements ; le fichier reste
/// dans le projet. Rend aussi le nombre de placements retirés.
#[tauri::command(async)]
fn library_remove(folder: String, drawing: String) -> Result<(Project, usize), String> {
    library::remove_drawing(&project_dir(&folder)?, &drawing).map_err(|e| e.to_string())
}

/// Renomme un dessin de la bibliothèque ; le fichier garde son nom.
#[tauri::command(async)]
fn library_rename(folder: String, drawing: String, name: String) -> Result<Project, String> {
    organize::rename_drawing(&project_dir(&folder)?, &drawing, &name).map_err(|e| e.to_string())
}

/// Range des dessins dans un dossier de la bibliothèque (vide : la racine).
#[tauri::command(async)]
fn library_move(folder: String, drawings: Vec<String>, target: String) -> Result<Project, String> {
    organize::move_drawings(&project_dir(&folder)?, &drawings, &target).map_err(|e| e.to_string())
}

/// Crée un dossier vide dans la bibliothèque.
#[tauri::command(async)]
fn library_folder_create(folder: String, path: String) -> Result<Project, String> {
    organize::create_folder(&project_dir(&folder)?, &path).map_err(|e| e.to_string())
}

/// Renomme un dossier de la bibliothèque, avec ses sous-dossiers et ses dessins.
#[tauri::command(async)]
fn library_folder_rename(folder: String, path: String, name: String) -> Result<Project, String> {
    organize::rename_folder(&project_dir(&folder)?, &path, &name).map_err(|e| e.to_string())
}

/// Retire un dossier vide de la bibliothèque.
#[tauri::command(async)]
fn library_folder_remove(folder: String, path: String) -> Result<Project, String> {
    organize::remove_folder(&project_dir(&folder)?, &path).map_err(|e| e.to_string())
}

/// L'aperçu (1 024 px, fond transparent) d'un dessin ; `original` : sans
/// ses retouches à la gomme et au crayon (tâche 4.8).
#[tauri::command(async)]
fn library_preview(
    app: AppHandle,
    folder: String,
    drawing: String,
    original: Option<bool>,
) -> Result<String, String> {
    let (dir, cache) = (project_dir(&folder)?, tile_cache_dir(&app)?);
    if original == Some(true) {
        library::original_preview(&dir, &drawing, &cache)
    } else {
        library::preview(&dir, &drawing, &cache)
    }
    .map_err(|e| e.to_string())
}

/// Remplace les retouches à la gomme et au crayon d'un dessin (tâche 4.8) ;
/// liste vide : le dessin d'origine.
#[tauri::command(async)]
fn library_set_retouch(
    app: AppHandle,
    folder: String,
    drawing: String,
    strokes: Vec<RetouchStroke>,
) -> Result<(Project, LibraryEntry), String> {
    library::retouch::set_retouch(
        &project_dir(&folder)?,
        &drawing,
        strokes,
        Some(&tile_cache_dir(&app)?),
    )
    .map_err(|e| e.to_string())
}

// --- Dessins posés sur la façade (tâche 4.2) ----------------------------------

/// Remplace calques et dessins posés, à la fin d'un geste ; rien n'est
/// enregistré si l'ensemble est incorrect.
#[tauri::command(async)]
fn composition_set(
    folder: String,
    layers: Vec<Layer>,
    placements: Vec<Placement>,
) -> Result<Project, String> {
    placement::set_composition(&project_dir(&folder)?, layers, placements)
        .map_err(|e| e.to_string())
}

/// Remplace tous les dessins posés (les calques ne changent pas).
#[tauri::command(async)]
fn placements_set(folder: String, placements: Vec<Placement>) -> Result<Project, String> {
    placement::set_placements(&project_dir(&folder)?, placements).map_err(|e| e.to_string())
}

/// Pose un dessin lâché dans la vue Diapositive (D-3.6-f) : sur la façade,
/// là où ce projecteur l'éclaire. Rend le projet et l'identifiant du dessin posé.
#[tauri::command(async)]
fn placement_in_slide(
    folder: String,
    projector: String,
    drawing: String,
    center: PointMm,
    width: f64,
    layer: Option<String>,
) -> Result<(Project, String), String> {
    placement::place_in_slide(
        &project_dir(&folder)?,
        &projector,
        &drawing,
        center,
        width,
        layer,
    )
    .map_err(|e| e.to_string())
}

/// Enregistre la retouche d'un dessin dans une diapositive (tâche 4.6) ;
/// aucune : retirée.
#[tauri::command(async)]
fn placement_adjust(
    folder: String,
    placement: String,
    adjustment: Option<SlideAdjustment>,
) -> Result<Project, String> {
    placement::adjust::set_adjustment(&project_dir(&folder)?, &placement, adjustment)
        .map_err(|e| e.to_string())
}

/// La diapositive de chaque dessin posé : proposée par le moteur, ou
/// choisie à la main (tâche 4.5).
#[tauri::command(async)]
fn placements_assignment(folder: String) -> Result<Vec<placement::assign::Assignment>, String> {
    placement::assign::load_assignments(&project_dir(&folder)?).map_err(|e| e.to_string())
}

/// La part de chaque dessin posé que sa diapositive ne projette pas
/// (D-4.5-h), montrée en rouge sur la façade.
#[tauri::command(async)]
fn placements_lost(
    app: AppHandle,
    folder: String,
) -> Result<Vec<placement::lost::LostPart>, String> {
    placement::lost::load_lost_parts(&project_dir(&folder)?, &tile_cache_dir(&app)?)
        .map_err(|e| e.to_string())
}

const VECTOR_PROGRESS_EVENT: &str = "vecteurs-progression";

/// Vectorise les dessins `drawings` (tous si vide) qui ne le sont pas encore
/// (tâche 5.2) ; l'avancement est envoyé à l'interface.
#[tauri::command(async)]
fn library_vectorize(
    app: AppHandle,
    folder: String,
    drawings: Vec<String>,
) -> Result<VectorizeReport, String> {
    vectors::vectorize_drawings(
        &project_dir(&folder)?,
        &drawings,
        &tile_cache_dir(&app)?,
        0,
        &|progress| {
            let _ = app.emit(VECTOR_PROGRESS_EVENT, progress);
        },
    )
    .map_err(|e| e.to_string())
}

/// Les vecteurs d'un dessin (tâche 5.2), calculés s'ils ne sont pas en
/// cache, avec l'image de comparaison.
#[tauri::command(async)]
fn library_vectors(
    app: AppHandle,
    folder: String,
    drawing: String,
) -> Result<VectorSummary, String> {
    vectors::drawing_vectors(&project_dir(&folder)?, &drawing, &tile_cache_dir(&app)?)
        .map_err(|e| e.to_string())
}

/// La silhouette des dessins `drawings` (tous si vide) : retouchée, sinon
/// détectée (tâche 4.7).
#[tauri::command(async)]
fn library_silhouettes(
    app: AppHandle,
    folder: String,
    drawings: Vec<String>,
) -> Result<Vec<DrawingSilhouette>, String> {
    silhouette::load_silhouettes(&project_dir(&folder)?, &drawings, &tile_cache_dir(&app)?)
        .map_err(|e| e.to_string())
}

/// Retouche la silhouette d'un dessin (`None` : détection automatique).
#[tauri::command(async)]
fn library_set_silhouette(
    app: AppHandle,
    folder: String,
    drawing: String,
    polygons: Option<Vec<SilhouettePolygon>>,
) -> Result<(Project, DrawingSilhouette), String> {
    silhouette::set_silhouette(
        &project_dir(&folder)?,
        &drawing,
        polygons,
        &tile_cache_dir(&app)?,
    )
    .map_err(|e| e.to_string())
}

/// Change les réglages de vectorisation d'un dessin (`None` : par défaut).
#[tauri::command(async)]
fn library_set_vector_settings(
    app: AppHandle,
    folder: String,
    drawing: String,
    settings: Option<VectorSettings>,
) -> Result<(Project, VectorSummary), String> {
    vectors::set_vector_settings(
        &project_dir(&folder)?,
        &drawing,
        settings,
        &tile_cache_dir(&app)?,
    )
    .map_err(|e| e.to_string())
}

// --- Vue façade (tâche 3.3) -------------------------------------------------------

/// Emprises, maillages des photos, recouvrements de tous les projecteurs.
#[tauri::command(async)]
fn facade_overview(app: AppHandle, folder: String) -> Result<FacadeOverview, String> {
    overview::project_overview(&project_dir(&folder)?, Some(&tile_cache_dir(&app)?))
        .map_err(|e| e.to_string())
}

// --- Fresque et export des diapositives (tâche 3.5) -------------------------------

/// Événement d'avancement d'un export : `{ done, total, current }`.
const EXPORT_PROGRESS_EVENT: &str = "export-progression";

/// Importe la fresque (image, PDF, .ai ou SVG), posée sur toute la largeur de la façade.
#[tauri::command(async)]
fn fresco_import(folder: String, path: String) -> Result<Project, String> {
    export::import_fresco(&project_dir(&folder)?, Path::new(&path), None).map_err(|e| e.to_string())
}

/// Retire la fresque du projet ; ses fichiers restent dans `fresque/`.
#[tauri::command(async)]
fn fresco_remove(folder: String) -> Result<Project, String> {
    export::remove_fresco(&project_dir(&folder)?).map_err(|e| e.to_string())
}

/// Écrit un PNG par projecteur recalé dans `destination` : sa diapositive,
/// ou son plan de contrôle (D-3.5-m).
#[tauri::command(async)]
fn slides_export(
    app: AppHandle,
    folder: String,
    destination: String,
    settings: ExportSettings,
    only: Option<Vec<String>>,
    control_plan: Option<bool>,
) -> Result<ExportReport, String> {
    let run = if control_plan.unwrap_or(false) {
        export::export_control_plans
    } else {
        export::export_slides
    };
    run(
        &project_dir(&folder)?,
        Path::new(&destination),
        &settings,
        only.as_deref(),
        0,
        &|progress| {
            let _ = app.emit(EXPORT_PROGRESS_EVENT, progress);
        },
    )
    .map_err(|e| e.to_string())
}

// --- Vue diapositive (tâche 3.6) ----------------------------------------------------

/// Chaque projecteur : sa photo redressée et l'aperçu de sa diapositive,
/// avec ou sans la fresque, avec ou sans le plan (D-3.6-d, D-3.6-e).
#[tauri::command(async)]
fn slide_views(
    app: AppHandle,
    folder: String,
    fresco: Option<bool>,
    plan: Option<bool>,
) -> Result<Vec<SlideView>, String> {
    slide_preview::slide_views_with(
        &project_dir(&folder)?,
        &tile_cache_dir(&app)?,
        slide_preview::SLIDE_PREVIEW_SIDE,
        slide_preview::SlideContent {
            fresco: fresco.unwrap_or(true),
            plan: plan.unwrap_or(false),
        },
    )
    .map_err(|e| e.to_string())
}

// --- Simulation et alertes (tâche 6.1) -------------------------------------------

#[tauri::command(async)]
fn facade_simulation(app: AppHandle, folder: String) -> Result<Simulation, String> {
    simulation::simulate(&project_dir(&folder)?, &tile_cache_dir(&app)?).map_err(|e| e.to_string())
}

// --- Fiche de calepinage (tâche 6.2) ----------------------------------------------

#[tauri::command(async)]
fn layout_sheet(
    app: AppHandle,
    folder: String,
    path: String,
    settings: ExportSettings,
) -> Result<SheetReport, String> {
    sheet::write_sheet(
        &project_dir(&folder)?,
        &tile_cache_dir(&app)?,
        Path::new(&path),
        Some(&settings),
    )
    .map_err(|e| e.to_string())
}

/// Dossier du jeu d'essai, pour l'écran de démonstration. Dans l'ordre :
/// celui que désigne `INSITU_JEU_ESSAI`, celui du dépôt d'où l'app a été
/// compilée (lancement depuis le code), puis les images embarquées dans
/// l'app installée (voir `bundle.resources` dans `tauri.conf.json`).
#[tauri::command]
fn test_set_dir(app: AppHandle) -> Option<String> {
    let from_env = std::env::var_os("INSITU_JEU_ESSAI").map(PathBuf::from);
    let from_repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../jeu-essai");
    let bundled = app
        .path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join("jeu-essai"));
    [from_env, Some(from_repository), bundled]
        .into_iter()
        .flatten()
        .find_map(|dir| dir.canonicalize().ok())
        .map(|dir| dir.to_string_lossy().into_owned())
}

/// Répond à une demande de tuile : le moteur la lit dans le cache, ou la
/// rend s'il s'agit d'un plan vectoriel dont la tuile n'existe pas encore.
/// Le rendu peut prendre une fraction de seconde : il se fait hors du fil
/// de la fenêtre.
fn serve_tile<R: Runtime>(
    context: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let root = tile_cache_dir(context.app_handle()).ok();
    let path = request.uri().path().to_string();
    std::thread::spawn(move || {
        let file = root.and_then(|root| facade::read_or_render(&root, &path));
        // L'interface vient d'une autre origine (`tauri://localhost`, ou le
        // serveur de développement) : sans cet en-tête, WebKit refuse la lecture.
        let response = Response::builder().header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
        let response = match file {
            Some((content_type, bytes)) => response
                .header(header::CONTENT_TYPE, content_type)
                .body(bytes),
            None => response.status(StatusCode::NOT_FOUND).body(Vec::new()),
        };
        responder.respond(response.expect("en-têtes de réponse valides"));
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updates::PendingUpdate::default())
        .register_asynchronous_uri_scheme_protocol(TILE_SCHEME, serve_tile)
        .setup(|app| {
            if let Ok(cache) = app.path().app_cache_dir() {
                let _ = HISTORY_ROOT.set(cache.join("historique"));
            }
            Ok(())
        })
        .menu(menu::build)
        .on_menu_event(menu::on_event)
        .invoke_handler(tauri::generate_handler![
            engine_info,
            updates::update_check,
            updates::update_install,
            open_tiled_image,
            test_set_dir,
            project_create,
            project_open,
            facade_inspect,
            facade_import,
            facade_remove,
            facade_rotate,
            facade_set_scale,
            facade_display,
            plans_add,
            plans_add_views,
            plans_place,
            plans_align,
            plans_rotate,
            plans_set_hidden,
            plans_rename,
            plans_remove,
            plans_move,
            plans_display,
            projector_add,
            projector_rename,
            projector_remove,
            projector_import_photo,
            projector_rectify,
            projector_view,
            projector_export,
            projector_mirror,
            projector_rotate,
            registration_show,
            registration_set,
            registration_preview,
            registration_adjust,
            library_import,
            library_import_files,
            library_stage_dropped,
            illustrator_import,
            illustrator_preview,
            library_list,
            library_remove,
            library_rename,
            settings_get,
            settings_set_shared_library,
            library_shared_status,
            library_shared_copy,
            library_shared_send,
            library_move,
            library_folder_create,
            library_folder_rename,
            library_folder_remove,
            library_preview,
            library_set_retouch,
            placements_set,
            placements_assignment,
            placements_lost,
            placement_adjust,
            placement_in_slide,
            composition_set,
            history_step,
            history_state,
            library_vectorize,
            library_vectors,
            library_set_vector_settings,
            library_silhouettes,
            library_set_silhouette,
            facade_overview,
            fresco_import,
            fresco_remove,
            slides_export,
            slide_views,
            facade_simulation,
            layout_sheet,
            assembly_show,
            assembly_add,
            assembly_remove,
            assembly_set_pairs,
            assembly_auto,
            assembly_render
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement d'InSitu");
}

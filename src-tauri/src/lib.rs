//! Coque de bureau : fenêtre, menus, accès aux fichiers, et pont vers le
//! moteur. Aucun calcul ici : une commande Tauri appelle `insitu_core` et
//! renvoie le résultat à l'interface.

use std::path::{Path, PathBuf};

use insitu_core::facade::{self, FacadeDisplay, SourceInfo};
use insitu_core::geometry::Point;
use insitu_core::project::{Project, ProjectDir, RegistrationPair, SourcePoint};
use insitu_core::rectification::projector::{self as projectors, ExportedImage, RectificationView};
use insitu_core::registration::{self, RegistrationOptions, RegistrationReport};
use insitu_core::tiles;
use tauri::http::{header, Request, Response, StatusCode};
use tauri::{AppHandle, Manager, Runtime, UriSchemeContext, UriSchemeResponder};

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
    facade::import_facade(
        &ProjectDir::open(&folder).map_err(|e| e.to_string())?,
        Path::new(&path),
        page,
    )
    .map_err(|e| e.to_string())
}

/// Met la façade du projet à l'échelle par deux points et une cote.
#[tauri::command(async)]
fn facade_set_scale(
    folder: String,
    a: SourcePoint,
    b: SourcePoint,
    distance_mm: f64,
) -> Result<Project, String> {
    facade::set_scale(
        &ProjectDir::open(&folder).map_err(|e| e.to_string())?,
        a,
        b,
        distance_mm,
    )
    .map_err(|e| e.to_string())
}

/// Prépare l'affichage de la façade du projet (tuiles dans le cache).
#[tauri::command(async)]
fn facade_display(app: AppHandle, folder: String) -> Result<FacadeDisplay, String> {
    let dir = ProjectDir::open(&folder).map_err(|e| e.to_string())?;
    facade::open_project_facade(&dir, &tile_cache_dir(&app)?).map_err(|e| e.to_string())
}

// --- Redressement des photos de mire (tâche 2.5) ------------------------------

fn project_dir(folder: &str) -> Result<ProjectDir, String> {
    ProjectDir::open(folder).map_err(|e| e.to_string())
}

/// Ajoute un projecteur ; renvoie le projet et l'identifiant du nouveau.
#[tauri::command(async)]
fn projector_add(folder: String, label: String) -> Result<(Project, String), String> {
    projectors::add_projector(&project_dir(&folder)?, &label).map_err(|e| e.to_string())
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
) -> Result<(Project, RegistrationReport), String> {
    registration::set_registration(
        &project_dir(&folder)?,
        &projector,
        pairs,
        &RegistrationOptions::default(),
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
        .register_asynchronous_uri_scheme_protocol(TILE_SCHEME, serve_tile)
        .invoke_handler(tauri::generate_handler![
            engine_info,
            open_tiled_image,
            test_set_dir,
            project_create,
            project_open,
            facade_inspect,
            facade_import,
            facade_set_scale,
            facade_display,
            projector_add,
            projector_import_photo,
            projector_rectify,
            projector_view,
            projector_export,
            registration_show,
            registration_set
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement d'InSitu");
}

//! Coque de bureau : fenêtre, menus, accès aux fichiers, et pont vers le
//! moteur. Aucun calcul ici : une commande Tauri appelle `insitu_core` et
//! renvoie le résultat à l'interface.

use std::path::{Path, PathBuf};

use insitu_core::tiles;
use tauri::http::{header, Request, Response, StatusCode};
use tauri::{AppHandle, Manager, Runtime, UriSchemeContext};

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

/// Dossier du jeu d'essai, pour l'écran de démonstration : celui du dépôt
/// d'où l'app a été compilée, ou celui que désigne `INSITU_JEU_ESSAI`.
#[tauri::command]
fn test_set_dir() -> Option<String> {
    let dir = std::env::var_os("INSITU_JEU_ESSAI")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../jeu-essai"));
    let dir = dir.canonicalize().ok()?;
    Some(dir.to_string_lossy().into_owned())
}

/// Répond à une demande de tuile en lisant le cache par le moteur.
fn serve_tile<R: Runtime>(
    context: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let file = tile_cache_dir(context.app_handle())
        .ok()
        .and_then(|root| tiles::read_cached_file(&root, request.uri().path()));
    // L'interface vient d'une autre origine (`tauri://localhost`, ou le
    // serveur de développement) : sans cet en-tête, WebKit refuse la lecture.
    let response = Response::builder().header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
    let response = match file {
        Some((content_type, bytes)) => response
            .header(header::CONTENT_TYPE, content_type)
            .body(bytes),
        None => response.status(StatusCode::NOT_FOUND).body(Vec::new()),
    };
    response.expect("en-têtes de réponse valides")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .register_uri_scheme_protocol(TILE_SCHEME, serve_tile)
        .invoke_handler(tauri::generate_handler![
            engine_info,
            open_tiled_image,
            test_set_dir
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement d'InSitu");
}

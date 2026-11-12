//! Mise à jour automatique de l'app (demande de Benjamin, décision D16).
//!
//! Au lancement, l'interface demande `update_check` : la coque lit le
//! manifeste `latest.json` joint par la fabrication à la dernière version
//! publiée du dépôt (public, D16-c). S'il annonce une version plus récente,
//! l'interface le dit ; sur « Installer », `update_install` télécharge
//! l'archive de l'app, vérifie sa signature avec la clé publique de
//! `tauri.conf.json`, remplace l'app et la relance.
//!
//! Sans réseau, ou si la dernière version n'a pas de manifeste, la recherche
//! échoue sans bruit et l'app reste telle quelle. Aucun calcul ici : c'est
//! de la plomberie de la coque, pas du moteur.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime, State};
use tauri_plugin_updater::{Update, Updater, UpdaterExt};

/// Nom de l'évènement de progression envoyé à l'interface.
const PROGRESS_EVENT: &str = "mise-a-jour";

/// La mise à jour trouvée par la dernière recherche, gardée pour l'installer.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);

/// Ce que l'interface affiche d'une mise à jour disponible.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    current_version: String,
    version: String,
    notes: Option<String>,
}

/// Avancement du téléchargement, en octets (`total` inconnu si le serveur
/// ne l'annonce pas).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    done: u64,
    total: Option<u64>,
}

fn updater<R: Runtime>(app: &AppHandle<R>) -> Result<Updater, String> {
    app.updater_builder().build().map_err(|e| e.to_string())
}

/// Cherche une version plus récente. `None` : l'app est à jour.
#[tauri::command]
pub async fn update_check(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> Result<Option<UpdateInfo>, String> {
    let found = updater(&app)?.check().await.map_err(|e| e.to_string())?;
    let info = found.as_ref().map(|update| UpdateInfo {
        current_version: update.current_version.clone(),
        version: update.version.clone(),
        notes: update.body.clone().filter(|b| !b.trim().is_empty()),
    });
    *pending.0.lock().map_err(|e| e.to_string())? = found;
    Ok(info)
}

/// Télécharge, vérifie et installe la mise à jour trouvée, puis relance
/// l'app. Ne revient qu'en cas d'erreur.
#[tauri::command]
pub async fn update_install(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> Result<(), String> {
    let update = pending
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("aucune mise à jour en attente : relancez la recherche")?;
    let mut done: u64 = 0;
    update
        .download_and_install(
            |chunk, total| {
                done += chunk as u64;
                let _ = app.emit(PROGRESS_EVENT, Progress { done, total });
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;
    app.restart()
}

//! Coque de bureau : fenêtre, menus, accès aux fichiers, et pont vers le
//! moteur. Aucun calcul ici : une commande Tauri appelle `insitu_core` et
//! renvoie le résultat à l'interface.

#[tauri::command]
fn engine_info() -> insitu_core::EngineInfo {
    insitu_core::engine_info()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![engine_info])
        .run(tauri::generate_context!())
        .expect("erreur au lancement d'InSitu");
}

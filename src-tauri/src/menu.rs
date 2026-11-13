//! Barre de menus du Mac (système de design, docs/design.md § 7, D15-c).
//!
//! Les entrées propres à InSitu portent pour identifiant le raccourci de
//! l'interface (`raccourci:mod+n`…). Un choix dans le menu, ou son
//! raccourci clavier, est transmis à l'interface par l'évènement `menu`,
//! qui déclenche la même action que la touche (`src/ui/components.tsx`,
//! `runShortcut`). Aucune action n'est décidée ici.

use tauri::menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Runtime};

const PREFIX: &str = "raccourci:";

/// Entrée sans raccourci clavier : « Rechercher les mises à jour… »
/// (`src/update/UpdateDialog.tsx`).
const CHECK_UPDATES: &str = "mises-a-jour";

/// Une entrée : libellé, raccourci de l'interface, raccourci du Mac.
fn entry<R: Runtime>(
    app: &AppHandle<R>,
    label: &str,
    keys: &str,
    accelerator: &str,
) -> tauri::Result<tauri::menu::MenuItem<R>> {
    MenuItemBuilder::with_id(format!("{PREFIX}{keys}"), label)
        .accelerator(accelerator)
        .build(app)
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    // Recherche de mise à jour à la main (la recherche automatique se fait
    // au lancement, D16) : l'interface reconnaît cet identifiant.
    let check_updates = MenuItemBuilder::with_id(
        format!("{PREFIX}{CHECK_UPDATES}"),
        "Rechercher les mises à jour…",
    )
    .build(app)?;
    let insitu = SubmenuBuilder::new(app, "InSitu")
        .about(None)
        .item(&check_updates)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let file = SubmenuBuilder::new(app, "Fichier")
        .item(&entry(app, "Nouveau projet…", "mod+n", "CmdOrCtrl+N")?)
        .item(&entry(app, "Ouvrir un projet…", "mod+o", "CmdOrCtrl+O")?)
        .item(&entry(
            app,
            "Relire depuis le disque",
            "mod+r",
            "CmdOrCtrl+R",
        )?)
        .separator()
        .item(&entry(app, "Importer…", "mod+i", "CmdOrCtrl+I")?)
        .item(&entry(app, "Exporter…", "mod+e", "CmdOrCtrl+E")?)
        .separator()
        .close_window()
        .build()?;

    // Annuler et Rétablir passent par l'interface (tâche 4.3) : hors d'un
    // champ de texte, ils annulent la dernière modification du projet.
    let edit = SubmenuBuilder::new(app, "Édition")
        .item(&entry(app, "Annuler", "mod+z", "CmdOrCtrl+Z")?)
        .item(&entry(app, "Rétablir", "mod+shift+z", "CmdOrCtrl+Shift+Z")?)
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let view = SubmenuBuilder::new(app, "Présentation")
        .item(&entry(app, "Façade", "mod+1", "CmdOrCtrl+1")?)
        .item(&entry(app, "Redressement", "mod+2", "CmdOrCtrl+2")?)
        .item(&entry(app, "Recalage", "mod+3", "CmdOrCtrl+3")?)
        .item(&entry(app, "Diapositive", "mod+4", "CmdOrCtrl+4")?)
        .item(&entry(app, "Bibliothèque", "mod+5", "CmdOrCtrl+5")?)
        .separator()
        .item(&entry(app, "Tout voir", "mod+0", "CmdOrCtrl+0")?)
        .separator()
        .fullscreen()
        .build()?;

    let window = SubmenuBuilder::new(app, "Fenêtre")
        .minimize()
        .maximize()
        .build()?;

    let help = SubmenuBuilder::new(app, "Aide")
        .item(&entry(
            app,
            "Raccourcis clavier",
            "mod+/",
            "CmdOrCtrl+Slash",
        )?)
        .build()?;

    MenuBuilder::new(app)
        .item(&insitu)
        .item(&file)
        .item(&edit)
        .item(&view)
        .item(&window)
        .item(&help)
        .build()
}

/// Transmet à l'interface le raccourci de l'entrée choisie.
pub fn on_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    if let Some(keys) = event.id().as_ref().strip_prefix(PREFIX) {
        let _ = app.emit("menu", keys.to_string());
    }
}

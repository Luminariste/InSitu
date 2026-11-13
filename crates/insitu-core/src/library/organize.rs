//! Ranger la bibliothèque (demande de Benjamin, 13 novembre 2026, D-4.1-g) :
//! renommer un dessin ou un dossier, ranger des dessins dans un dossier,
//! créer et retirer un dossier.
//!
//! Tout se passe dans `projet.json` : le nom affiché (`library[].name`), le
//! dossier choisi (`library[].folder`) et les dossiers créés à la main
//! (`libraryFolders`). Aucun fichier ne bouge sur le disque, pour que ⌘Z
//! défasse tout (l'annulation ne remet que `projet.json`, D-4.3-d) et que
//! les chemins cités ailleurs (fresque Illustrator, caches) restent bons.

use super::LibraryError;
use crate::project::{Project, ProjectDir};

/// Un nom de dessin ou de dossier tapé à la main, nettoyé : espaces du bord
/// retirés ; refusé s'il est vide ou contient « / ».
pub fn clean_name(name: &str) -> Result<String, LibraryError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(LibraryError::Organize("le nom est vide".into()));
    }
    if name.contains('/') {
        return Err(LibraryError::Organize(
            "un nom ne peut pas contenir « / »".into(),
        ));
    }
    if name.chars().any(char::is_control) {
        return Err(LibraryError::Organize(
            "le nom contient un caractère invisible".into(),
        ));
    }
    Ok(name.to_string())
}

/// Un chemin de dossier (`Oiseaux/Mer`, vide : la racine), nettoyé morceau
/// par morceau.
pub fn clean_folder(path: &str) -> Result<String, LibraryError> {
    let parts: Vec<&str> = path
        .split('/')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let cleaned = parts
        .into_iter()
        .map(clean_name)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(cleaned.join("/"))
}

/// Vrai si `folder` est `parent` ou l'un de ses sous-dossiers.
fn inside(folder: &str, parent: &str) -> bool {
    folder == parent || folder.starts_with(&format!("{parent}/"))
}

/// Tous les dossiers de la bibliothèque : ceux des dessins (et leurs
/// parents) et ceux créés à la main.
pub fn folders(project: &Project) -> Vec<String> {
    let mut all: Vec<String> = Vec::new();
    let mut add = |path: &str| {
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        for i in 1..=parts.len() {
            let sub = parts[..i].join("/");
            if !all.contains(&sub) {
                all.push(sub);
            }
        }
    };
    for drawing in &project.library {
        add(drawing.folder());
    }
    for folder in &project.library_folders {
        add(folder);
    }
    all.sort();
    all
}

fn unknown(id: &str) -> LibraryError {
    LibraryError::UnknownDrawing(id.to_string())
}

/// Donne au dessin `id` le nom `name`, et enregistre le projet. Le fichier
/// garde son nom.
pub fn rename_drawing(dir: &ProjectDir, id: &str, name: &str) -> Result<Project, LibraryError> {
    let name = clean_name(name)?;
    let mut project = dir.load()?.project;
    let drawing = project
        .library
        .iter_mut()
        .find(|d| d.id == id)
        .ok_or_else(|| unknown(id))?;
    if drawing.name == name {
        return Ok(project);
    }
    drawing.name = name;
    dir.save(&project)?;
    Ok(project)
}

/// Range les dessins `ids` dans le dossier `folder` (vide : à la racine), et
/// enregistre le projet. Le dossier est créé au besoin.
pub fn move_drawings(
    dir: &ProjectDir,
    ids: &[String],
    folder: &str,
) -> Result<Project, LibraryError> {
    let folder = clean_folder(folder)?;
    let mut project = dir.load()?.project;
    if let Some(id) = ids.iter().find(|id| project.drawing(id).is_none()) {
        return Err(unknown(id));
    }
    let mut changed = false;
    for drawing in project.library.iter_mut().filter(|d| ids.contains(&d.id)) {
        if drawing.folder() == folder {
            continue;
        }
        // Revenu dans le dossier de son fichier : plus rien à retenir.
        drawing.folder = (drawing.file_folder() != folder).then(|| folder.clone());
        changed = true;
    }
    if changed {
        dir.save(&project)?;
    }
    Ok(project)
}

/// Crée le dossier `path` (`Oiseaux/Mer`), vide, et enregistre le projet.
pub fn create_folder(dir: &ProjectDir, path: &str) -> Result<Project, LibraryError> {
    let path = clean_folder(path)?;
    if path.is_empty() {
        return Err(LibraryError::Organize("le nom est vide".into()));
    }
    let mut project = dir.load()?.project;
    if folders(&project).contains(&path) {
        return Err(LibraryError::Organize(format!(
            "le dossier « {path} » existe déjà"
        )));
    }
    project.library_folders.push(path);
    dir.save(&project)?;
    Ok(project)
}

/// Renomme le dossier `path` en `name` (même dossier parent), ses
/// sous-dossiers et ses dessins avec lui, et enregistre le projet.
pub fn rename_folder(dir: &ProjectDir, path: &str, name: &str) -> Result<Project, LibraryError> {
    let path = clean_folder(path)?;
    let name = clean_name(name)?;
    let mut project = dir.load()?.project;
    if path.is_empty() || !folders(&project).contains(&path) {
        return Err(LibraryError::Organize(format!(
            "le dossier « {path} » n'existe pas"
        )));
    }
    let target = match path.rsplit_once('/') {
        Some((parent, _)) => format!("{parent}/{name}"),
        None => name,
    };
    if target == path {
        return Ok(project);
    }
    if folders(&project).contains(&target) {
        return Err(LibraryError::Organize(format!(
            "le dossier « {target} » existe déjà"
        )));
    }
    let renamed = |folder: &str| format!("{target}{}", &folder[path.len()..]);
    for drawing in project.library.iter_mut() {
        let folder = drawing.folder().to_string();
        if inside(&folder, &path) {
            let next = renamed(&folder);
            drawing.folder = (drawing.file_folder() != next).then_some(next);
        }
    }
    for folder in project.library_folders.iter_mut() {
        if inside(folder, &path) {
            *folder = renamed(folder);
        }
    }
    dir.save(&project)?;
    Ok(project)
}

/// Retire le dossier `path` s'il est vide (ni dessin ni sous-dossier
/// rempli), et enregistre le projet. Un dossier qui contient des dessins
/// est refusé : on retire ou on range d'abord ses dessins.
pub fn remove_folder(dir: &ProjectDir, path: &str) -> Result<Project, LibraryError> {
    let path = clean_folder(path)?;
    let mut project = dir.load()?.project;
    let count = project
        .library
        .iter()
        .filter(|d| inside(d.folder(), &path))
        .count();
    if count > 0 {
        return Err(LibraryError::Organize(format!(
            "le dossier « {path} » contient {count} dessin{} : les ranger ailleurs d'abord",
            if count > 1 { "s" } else { "" }
        )));
    }
    let before = project.library_folders.len();
    project.library_folders.retain(|f| !inside(f, &path));
    if project.library_folders.len() == before {
        return Err(LibraryError::Organize(format!(
            "le dossier « {path} » n'existe pas"
        )));
    }
    dir.save(&project)?;
    Ok(project)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::library::tests::line_drawing;
    use crate::library::{entries, import_folder, ImportOptions};
    use crate::project::history;
    use crate::project::test_support::TempDir;

    fn save_png(side: u32, seed: u32, path: &Path) {
        let r = line_drawing(side, seed);
        image::save_buffer(path, &r.data, r.width, r.height, image::ColorType::L8)
            .expect("écriture");
    }

    /// Un projet avec annulation, et « Oiseaux » importé : merle et pie à la
    /// racine du dossier, héron dans « Grands ».
    fn fixture() -> (TempDir, ProjectDir) {
        let temp = TempDir::new("ranger");
        let (dir, _) =
            ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).expect("projet");
        let dir = dir.with_history(temp.path().join("historique"));
        let source = temp.path().join("Oiseaux");
        fs::create_dir_all(source.join("Grands")).expect("dossiers");
        save_png(40, 1, &source.join("merle.png"));
        save_png(40, 2, &source.join("pie.png"));
        save_png(40, 3, &source.join("Grands").join("héron.png"));
        let options = ImportOptions {
            cache_root: None,
            threads: 1,
        };
        import_folder(&dir, &source, &options, &|_| {}).expect("import");
        (temp, dir)
    }

    fn id(dir: &ProjectDir, name: &str) -> String {
        let project = dir.load().expect("projet").project;
        project
            .library
            .iter()
            .find(|d| d.name == name)
            .unwrap_or_else(|| panic!("{name}"))
            .id
            .clone()
    }

    /// (nom, dossier) de chaque dessin, comme les liste l'écran.
    fn shown(dir: &ProjectDir) -> Vec<(String, String)> {
        let project = dir.load().expect("projet").project;
        let mut list: Vec<_> = entries(dir, &project, None, 1)
            .into_iter()
            .map(|e| (e.name, e.folder))
            .collect();
        list.sort();
        list
    }

    fn pair(name: &str, folder: &str) -> (String, String) {
        (name.to_string(), folder.to_string())
    }

    #[test]
    fn renommer_un_dessin_garde_le_fichier_et_s_annule() {
        let (_temp, dir) = fixture();
        let merle = id(&dir, "merle");
        let before = dir.load().unwrap().project;
        let project = rename_drawing(&dir, &merle, "  Merle noir ").expect("renommé");
        let drawing = project.drawing(&merle).unwrap();
        assert_eq!(drawing.name, "Merle noir");
        assert_eq!(drawing.source.as_str(), "bibliotheque/Oiseaux/merle.png");
        assert!(dir.resolve(&drawing.source).is_file());
        // Relu depuis le disque : le nom est enregistré.
        assert_eq!(
            dir.load().unwrap().project.drawing(&merle).unwrap().name,
            "Merle noir"
        );
        // ⌘Z remet l'ancien nom.
        let (undone, _) = history::step(&dir, false).expect("annulé");
        assert_eq!(undone.library, before.library);

        assert!(rename_drawing(&dir, &merle, "  ").is_err());
        assert!(rename_drawing(&dir, &merle, "a/b").is_err());
        assert!(rename_drawing(&dir, "inconnu", "x").is_err());
    }

    #[test]
    fn ranger_des_dessins_dans_un_autre_dossier() {
        let (_temp, dir) = fixture();
        let (merle, pie) = (id(&dir, "merle"), id(&dir, "pie"));
        move_drawings(&dir, &[merle.clone(), pie.clone()], "Oiseaux/Grands").expect("rangés");
        assert_eq!(
            shown(&dir),
            vec![
                pair("héron", "Oiseaux/Grands"),
                pair("merle", "Oiseaux/Grands"),
                pair("pie", "Oiseaux/Grands"),
            ]
        );
        // Le fichier n'a pas bougé.
        let project = dir.load().unwrap().project;
        let drawing = project.drawing(&merle).unwrap();
        assert_eq!(drawing.source.as_str(), "bibliotheque/Oiseaux/merle.png");
        assert!(dir.resolve(&drawing.source).is_file());
        // « Oiseaux » existe encore, comme parent de « Grands ».
        assert_eq!(folders(&project), vec!["Oiseaux", "Oiseaux/Grands"]);

        // Vers un nouveau dossier, puis à la racine.
        move_drawings(&dir, std::slice::from_ref(&pie), "Mer / Côte").expect("rangé");
        move_drawings(&dir, std::slice::from_ref(&merle), "").expect("racine");
        assert_eq!(
            shown(&dir),
            vec![
                pair("héron", "Oiseaux/Grands"),
                pair("merle", ""),
                pair("pie", "Mer/Côte"),
            ]
        );
        // Revenu dans son dossier d'origine : plus de dossier retenu.
        move_drawings(&dir, std::slice::from_ref(&merle), "Oiseaux").expect("retour");
        let project = dir.load().unwrap().project;
        assert_eq!(project.drawing(&merle).unwrap().folder, None);

        assert!(move_drawings(&dir, &["inconnu".to_string()], "X").is_err());
    }

    #[test]
    fn renommer_un_dossier_emporte_ses_dessins_et_sous_dossiers() {
        let (_temp, dir) = fixture();
        create_folder(&dir, "Oiseaux/Grands/Vides").expect("créé");
        rename_folder(&dir, "Oiseaux", "Volatiles").expect("renommé");
        assert_eq!(
            shown(&dir),
            vec![
                pair("héron", "Volatiles/Grands"),
                pair("merle", "Volatiles"),
                pair("pie", "Volatiles"),
            ]
        );
        let project = dir.load().unwrap().project;
        assert_eq!(
            folders(&project),
            vec!["Volatiles", "Volatiles/Grands", "Volatiles/Grands/Vides"]
        );
        // Sous-dossier renommé dans son parent ; nom déjà pris refusé.
        rename_folder(&dir, "Volatiles/Grands", "Échassiers").expect("renommé");
        assert_eq!(shown(&dir)[0], pair("héron", "Volatiles/Échassiers"));
        create_folder(&dir, "Volatiles/Mer").expect("créé");
        assert!(rename_folder(&dir, "Volatiles/Mer", "Échassiers").is_err());
        assert!(rename_folder(&dir, "Absent", "X").is_err());
        // Revenir au nom d'origine efface les dossiers retenus.
        rename_folder(&dir, "Volatiles/Échassiers", "Grands").expect("renommé");
        rename_folder(&dir, "Volatiles", "Oiseaux").expect("renommé");
        let project = dir.load().unwrap().project;
        assert!(project.library.iter().all(|d| d.folder.is_none()));
    }

    #[test]
    fn creer_et_retirer_un_dossier() {
        let (_temp, dir) = fixture();
        create_folder(&dir, "Poissons").expect("créé");
        assert!(create_folder(&dir, "Poissons").is_err());
        assert!(create_folder(&dir, "Oiseaux/Grands").is_err());
        let project = dir.load().unwrap().project;
        assert!(folders(&project).contains(&"Poissons".to_string()));
        // Rempli, il ne se retire pas ; vidé, si.
        let merle = id(&dir, "merle");
        move_drawings(&dir, std::slice::from_ref(&merle), "Poissons").expect("rangé");
        assert!(remove_folder(&dir, "Poissons").is_err());
        move_drawings(&dir, std::slice::from_ref(&merle), "Oiseaux").expect("rangé");
        remove_folder(&dir, "Poissons").expect("retiré");
        let project = dir.load().unwrap().project;
        assert!(!folders(&project).contains(&"Poissons".to_string()));
        assert!(remove_folder(&dir, "Poissons").is_err());
    }

    #[test]
    fn un_projet_range_se_relit_et_se_valide() {
        let (_temp, dir) = fixture();
        let merle = id(&dir, "merle");
        move_drawings(&dir, std::slice::from_ref(&merle), "Mer").expect("rangé");
        create_folder(&dir, "Vide").expect("créé");
        let text = fs::read_to_string(dir.project_file()).expect("projet.json");
        assert!(text.contains("\"folder\": \"Mer\""));
        assert!(text.contains("\"libraryFolders\""));
        let loaded = dir.load().expect("relu");
        assert!(loaded.unknown_fields.is_empty());
        assert!(crate::project::validate::validate(&dir)
            .errors()
            .next()
            .is_none());
        assert_eq!(loaded.project.library_folders, vec!["Vide"]);
    }
}

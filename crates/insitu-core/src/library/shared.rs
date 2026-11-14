//! Bibliothèque commune à tous les projets (demande de Benjamin, 13 novembre
//! 2026, option A, D-4.1-i).
//!
//! La bibliothèque commune est un projet InSitu comme un autre, rangé dans
//! Dropbox (`Bibliothèque commune.insitu`), dont on n'utilise que la
//! bibliothèque : import, rangement, gomme et crayon, silhouette, vecteurs
//! s'y font avec les mêmes outils. Un dessin commun n'est jamais cité par un
//! projet : il y est **copié**, avec ses retouches, sa silhouette et ses
//! réglages de vecteurs (le projet reste complet sur l'autre Mac, D-1.4-b).
//! Le projet note d'où vient la copie et l'empreinte du dessin commun à ce
//! moment-là (`sharedLinks`) : quand le dessin commun change, l'écran le
//! signale et propose de reprendre la nouvelle version, sans jamais la
//! reprendre seul.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::organize::move_drawings;
use super::{import_files, ImportOptions, ImportProgress, LibraryError};
use crate::project::{Drawing, Project, ProjectDir, SharedLink};
use crate::tiles::fnv1a_64;

/// Nom du projet créé quand le dossier choisi n'est pas déjà une
/// bibliothèque commune.
pub const SHARED_PROJECT_NAME: &str = "Bibliothèque commune.insitu";

/// Dossier, sous `bibliotheque/` du projet, où arrivent les copies.
pub const COPY_FOLDER: &str = "Commune";

/// Le projet de la bibliothèque commune pour le dossier `folder` choisi dans
/// les Réglages : `folder` lui-même s'il est un projet InSitu, sinon
/// `folder/Bibliothèque commune.insitu`, créé s'il n'existe pas.
pub fn open_or_create(folder: &Path) -> Result<PathBuf, LibraryError> {
    if ProjectDir::open(folder).is_ok() {
        return Ok(folder.to_path_buf());
    }
    if !folder.is_dir() {
        return Err(LibraryError::NotAFolder(folder.to_path_buf()));
    }
    let root = folder.join(SHARED_PROJECT_NAME);
    if ProjectDir::open(&root).is_err() {
        ProjectDir::create(&root, Some("Bibliothèque commune"))?;
    }
    Ok(root)
}

/// Empreinte d'un dessin : son fichier, ses retouches, sa silhouette et ses
/// réglages de vecteurs. Le nom et le dossier n'y entrent pas : les changer
/// dans la bibliothèque commune ne demande pas de reprendre le dessin.
pub fn fingerprint(dir: &ProjectDir, drawing: &Drawing) -> Result<String, LibraryError> {
    let path = dir.resolve(&drawing.source);
    let bytes = fs::read(&path).map_err(|e| LibraryError::Unreadable {
        path: path.clone(),
        reason: e.to_string(),
    })?;
    let settings = serde_json::to_vec(&(&drawing.retouch, &drawing.silhouette, &drawing.vector))
        .expect("sérialisable");
    Ok(format!(
        "{:016x}-{:016x}",
        fnv1a_64(&bytes),
        fnv1a_64(&settings)
    ))
}

/// Où en est un dessin du projet par rapport à la bibliothèque commune.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SharedState {
    /// Copié, et le dessin commun n'a pas changé depuis.
    UpToDate,
    /// Le dessin commun a changé depuis la copie : une version plus récente
    /// peut être reprise.
    Newer,
    /// Le dessin commun n'est plus dans la bibliothèque commune (retiré, ou
    /// une autre bibliothèque est choisie).
    Missing,
}

/// L'état d'un dessin copié, pour l'écran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedStatus {
    /// Le dessin du projet.
    pub drawing: String,
    /// Le dessin de la bibliothèque commune.
    pub origin: String,
    pub state: SharedState,
}

/// L'état de chaque dessin du projet venu de la bibliothèque commune.
pub fn status(
    project_dir: &ProjectDir,
    shared_dir: &ProjectDir,
) -> Result<Vec<SharedStatus>, LibraryError> {
    let project = project_dir.load()?.project;
    let shared = shared_dir.load()?.project;
    let mut out = Vec::new();
    for link in &project.shared_links {
        if project.drawing(&link.drawing).is_none() {
            continue;
        }
        let state = match shared.drawing(&link.origin) {
            Some(origin) if link.library == shared.id => match fingerprint(shared_dir, origin) {
                Ok(now) if now == link.fingerprint => SharedState::UpToDate,
                Ok(_) => SharedState::Newer,
                Err(_) => SharedState::Missing,
            },
            _ => SharedState::Missing,
        };
        out.push(SharedStatus {
            drawing: link.drawing.clone(),
            origin: link.origin.clone(),
            state,
        });
    }
    Ok(out)
}

/// Compte rendu d'une copie dans un sens ou dans l'autre.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedCopyReport {
    /// Dessins ajoutés à la bibliothèque d'arrivée.
    pub added: Vec<String>,
    /// Dessins déjà copiés et à jour : rien n'a été fait.
    pub already_present: usize,
    /// Dessins repris dans leur nouvelle version.
    pub updated: Vec<String>,
    /// Fichiers qui n'ont pas pu être copiés, avec la raison.
    pub failures: Vec<String>,
}

/// Le chemin d'un dessin sous `bibliotheque/`, sans ce préfixe
/// (`Oiseaux/merle.png`).
fn inner_path(drawing: &Drawing) -> String {
    let path = drawing.source.as_str();
    path.split_once('/')
        .map_or(path, |(_, rest)| rest)
        .to_string()
}

/// Recopie dans `target` ce qui fait le dessin `from` (nom, retouches,
/// silhouette, vecteurs), sans toucher à son fichier ni à son identifiant.
fn copy_settings(target: &mut Drawing, from: &Drawing) {
    target.name = from.name.clone();
    target.retouch = from.retouch.clone();
    target.silhouette = from.silhouette.clone();
    target.vector = from.vector;
}

/// Copie les dessins `ids` de la bibliothèque commune dans le projet, sous
/// `bibliotheque/Commune/`, rangés dans le même dossier que dans la
/// bibliothèque commune. Un dessin déjà copié et à jour n'est pas recopié ;
/// un dessin déjà copié dont le dessin commun a changé est repris dans sa
/// nouvelle version (même identifiant : ses placements sur la façade
/// restent). Rend le projet et le compte rendu.
pub fn copy_to_project(
    shared_dir: &ProjectDir,
    project_dir: &ProjectDir,
    ids: &[String],
    options: &ImportOptions,
    progress: &(dyn Fn(ImportProgress) + Sync),
) -> Result<(Project, SharedCopyReport), LibraryError> {
    let shared = shared_dir.load()?.project;
    let project = project_dir.load()?.project;
    let mut report = SharedCopyReport::default();
    let mut wanted: Vec<&Drawing> = Vec::new();
    for id in ids {
        let origin = shared
            .drawing(id)
            .ok_or_else(|| LibraryError::UnknownDrawing(id.clone()))?;
        wanted.push(origin);
    }
    // Déjà copiés : à jour (rien à faire) ou à reprendre.
    let mut to_update: Vec<(&Drawing, String)> = Vec::new();
    let mut to_add: Vec<&Drawing> = Vec::new();
    for origin in wanted {
        let link = project.shared_links.iter().find(|l| {
            l.library == shared.id && l.origin == origin.id && project.drawing(&l.drawing).is_some()
        });
        match link {
            Some(link) if fingerprint(shared_dir, origin)? == link.fingerprint => {
                report.already_present += 1;
            }
            Some(link) => to_update.push((origin, link.drawing.clone())),
            None => to_add.push(origin),
        }
    }

    // Les fichiers, en un seul passage (vignettes comprises).
    let files: Vec<(PathBuf, String)> = to_add
        .iter()
        .chain(to_update.iter().map(|(origin, _)| origin))
        .map(|d| (shared_dir.resolve(&d.source), inner_path(d)))
        .collect();
    let base = format!("bibliotheque/{COPY_FOLDER}");
    let copied = if files.is_empty() {
        Vec::new()
    } else {
        import_files(project_dir, &base, &files, options, progress)?.1
    };

    let mut project = project_dir.load()?.project;
    let mut placed: Vec<(String, String)> = Vec::new();
    for (i, result) in copied.into_iter().enumerate() {
        let (origin, update) = if i < to_add.len() {
            (to_add[i], None)
        } else {
            let (origin, existing) = &to_update[i - to_add.len()];
            (*origin, Some(existing.clone()))
        };
        let new_id = match result {
            Ok(id) => id,
            Err(reason) => {
                report.failures.push(format!("{} : {reason}", origin.name));
                continue;
            }
        };
        let print = fingerprint(shared_dir, origin)?;
        match update {
            None => {
                let drawing = project
                    .library
                    .iter_mut()
                    .find(|d| d.id == new_id)
                    .expect("dessin copié");
                copy_settings(drawing, origin);
                project.shared_links.retain(|l| l.drawing != new_id);
                project.shared_links.push(SharedLink {
                    drawing: new_id.clone(),
                    library: shared.id.clone(),
                    origin: origin.id.clone(),
                    fingerprint: print,
                });
                placed.push((new_id.clone(), origin.folder().to_string()));
                report.added.push(new_id);
            }
            Some(existing) => {
                // Le nouveau fichier prend la place de l'ancien dans le
                // dessin déjà posé ; l'entrée faite par la copie disparaît.
                let copy = project
                    .library
                    .iter()
                    .find(|d| d.id == new_id)
                    .map(|d| (d.source.clone(), d.width, d.height))
                    .expect("dessin copié");
                if new_id != existing {
                    project.library.retain(|d| d.id != new_id);
                }
                let drawing = project
                    .library
                    .iter_mut()
                    .find(|d| d.id == existing)
                    .expect("dessin lié");
                drawing.source = copy.0;
                drawing.width = copy.1;
                drawing.height = copy.2;
                copy_settings(drawing, origin);
                if let Some(link) = project
                    .shared_links
                    .iter_mut()
                    .find(|l| l.drawing == existing)
                {
                    link.fingerprint = print;
                }
                report.updated.push(existing);
            }
        }
    }
    if !report.added.is_empty() || !report.updated.is_empty() {
        project_dir.save(&project)?;
    }
    // Rangés comme dans la bibliothèque commune.
    for (id, folder) in placed {
        project = move_drawings(project_dir, &[id], &folder)?;
    }
    Ok((project, report))
}

/// Envoie les dessins `ids` du projet dans la bibliothèque commune, sous
/// `bibliotheque/<nom du projet>/`, avec leurs retouches, leur silhouette et
/// leurs réglages de vecteurs, et les lie dans le projet (ils sont dès lors
/// « à jour »). Un dessin déjà lié n'est pas renvoyé. Rend le projet et le
/// compte rendu (identifiants des dessins communs ajoutés).
pub fn send_to_shared(
    project_dir: &ProjectDir,
    shared_dir: &ProjectDir,
    ids: &[String],
    options: &ImportOptions,
    progress: &(dyn Fn(ImportProgress) + Sync),
) -> Result<(Project, SharedCopyReport), LibraryError> {
    let project = project_dir.load()?.project;
    let shared_id = shared_dir.load()?.project.id;
    let mut report = SharedCopyReport::default();
    let mut to_send: Vec<&Drawing> = Vec::new();
    for id in ids {
        let drawing = project
            .drawing(id)
            .ok_or_else(|| LibraryError::UnknownDrawing(id.clone()))?;
        if project
            .shared_links
            .iter()
            .any(|l| l.drawing == drawing.id && l.library == shared_id)
        {
            report.already_present += 1;
        } else {
            to_send.push(drawing);
        }
    }
    if to_send.is_empty() {
        return Ok((project, report));
    }
    let project_name =
        super::organize::clean_name(&project.name).unwrap_or_else(|_| "Projet".into());
    let files: Vec<(PathBuf, String)> = to_send
        .iter()
        .map(|d| {
            let path = project_dir.resolve(&d.source);
            let file = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let folder = d.folder();
            let inner = if folder.is_empty() {
                file
            } else {
                format!("{folder}/{file}")
            };
            (path, inner)
        })
        .collect();
    let base = format!("bibliotheque/{project_name}");
    let (_, sent) = import_files(shared_dir, &base, &files, options, progress)?;

    let mut shared = shared_dir.load()?.project;
    let mut links = Vec::new();
    for (drawing, result) in to_send.iter().zip(sent) {
        let shared_drawing_id = match result {
            Ok(id) => id,
            Err(reason) => {
                report.failures.push(format!("{} : {reason}", drawing.name));
                continue;
            }
        };
        let target = shared
            .library
            .iter_mut()
            .find(|d| d.id == shared_drawing_id)
            .expect("dessin envoyé");
        copy_settings(target, drawing);
        links.push((drawing.id.clone(), shared_drawing_id.clone()));
        report.added.push(shared_drawing_id);
    }
    shared_dir.save(&shared)?;

    let mut project = project_dir.load()?.project;
    for (drawing, origin) in links {
        let print = fingerprint(shared_dir, shared.drawing(&origin).expect("dessin envoyé"))?;
        project.shared_links.retain(|l| l.drawing != drawing);
        project.shared_links.push(SharedLink {
            drawing,
            library: shared.id.clone(),
            origin,
            fingerprint: print,
        });
    }
    project_dir.save(&project)?;
    Ok((project, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::organize::{move_drawings, rename_drawing};
    use crate::library::tests::line_drawing;
    use crate::library::{entries, import_folder};
    use crate::project::test_support::TempDir;
    use crate::project::{RetouchStroke, RetouchTool};

    fn save_png(side: u32, seed: u32, path: &Path) {
        let r = line_drawing(side, seed);
        image::save_buffer(path, &r.data, r.width, r.height, image::ColorType::L8)
            .expect("écriture");
    }

    const OPTIONS: ImportOptions = ImportOptions {
        cache_root: None,
        threads: 1,
    };

    fn quiet(_: ImportProgress) {}

    /// Un dossier Dropbox avec une bibliothèque commune (merle et héron dans
    /// « Oiseaux », héron dans « Oiseaux/Grands »), et un projet vide.
    fn fixture() -> (TempDir, ProjectDir, ProjectDir) {
        let temp = TempDir::new("commune");
        let dropbox = temp.path().join("Dropbox");
        fs::create_dir_all(&dropbox).expect("dossier");
        let root = open_or_create(&dropbox).expect("bibliothèque commune");
        assert_eq!(root, dropbox.join(SHARED_PROJECT_NAME));
        // Déjà là : la même est rendue.
        assert_eq!(open_or_create(&dropbox).unwrap(), root);
        assert_eq!(open_or_create(&root).unwrap(), root);
        let shared = ProjectDir::open(&root).expect("ouverte");
        let source = temp.path().join("Oiseaux");
        fs::create_dir_all(source.join("Grands")).expect("dossiers");
        save_png(40, 1, &source.join("merle.png"));
        save_png(40, 3, &source.join("Grands").join("héron.png"));
        import_folder(&shared, &source, &OPTIONS, &quiet).expect("import");
        let (project, _) =
            ProjectDir::create(temp.path().join("Trévarez.insitu"), Some("Trévarez"))
                .expect("projet");
        (temp, shared, project)
    }

    fn id_of(dir: &ProjectDir, name: &str) -> String {
        dir.load()
            .unwrap()
            .project
            .library
            .iter()
            .find(|d| d.name == name)
            .unwrap()
            .id
            .clone()
    }

    #[test]
    fn un_dessin_commun_est_copie_avec_ses_reglages_et_son_dossier() {
        let (_temp, shared, project) = fixture();
        let heron = id_of(&shared, "héron");
        // Retouché dans la bibliothèque commune.
        let mut lib = shared.load().unwrap().project;
        lib.library
            .iter_mut()
            .find(|d| d.id == heron)
            .unwrap()
            .retouch = vec![RetouchStroke {
            tool: RetouchTool::Erase,
            fill: false,
            size: 0.05,
            points: vec![[0.1, 0.1], [0.2, 0.2]],
        }];
        shared.save(&lib).unwrap();

        let (updated, report) = copy_to_project(
            &shared,
            &project,
            std::slice::from_ref(&heron),
            &OPTIONS,
            &quiet,
        )
        .expect("copie");
        assert_eq!(report.added.len(), 1);
        let copy = updated.drawing(&report.added[0]).unwrap();
        assert_eq!(copy.name, "héron");
        assert_eq!(copy.retouch.len(), 1);
        assert_eq!(copy.folder(), "Oiseaux/Grands");
        assert!(copy
            .source
            .as_str()
            .starts_with("bibliotheque/Commune/Oiseaux/Grands/"));
        // Le fichier est dans le projet, pas cité dans Dropbox.
        assert!(project.resolve(&copy.source).is_file());
        assert_eq!(
            status(&project, &shared).unwrap()[0].state,
            SharedState::UpToDate
        );
        // Recopier : rien de nouveau.
        let (_, again) =
            copy_to_project(&shared, &project, &[heron], &OPTIONS, &quiet).expect("copie");
        assert_eq!(again.already_present, 1);
        assert!(again.added.is_empty());
        assert_eq!(project.load().unwrap().project.library.len(), 1);
    }

    #[test]
    fn une_version_plus_recente_est_signalee_et_reprise_sans_perdre_les_placements() {
        let (_temp, shared, project) = fixture();
        let merle = id_of(&shared, "merle");
        let (_, report) = copy_to_project(
            &shared,
            &project,
            std::slice::from_ref(&merle),
            &OPTIONS,
            &quiet,
        )
        .expect("copie");
        let copy = report.added[0].clone();
        let old_source = project
            .load()
            .unwrap()
            .project
            .drawing(&copy)
            .unwrap()
            .source
            .clone();

        // Renommer ou ranger le dessin commun ne compte pas comme un changement.
        rename_drawing(&shared, &merle, "Merle noir").unwrap();
        move_drawings(&shared, std::slice::from_ref(&merle), "Passereaux").unwrap();
        assert_eq!(
            status(&project, &shared).unwrap()[0].state,
            SharedState::UpToDate
        );

        // Le fichier commun est redessiné : version plus récente.
        let lib = shared.load().unwrap().project;
        let file = shared.resolve(&lib.drawing(&merle).unwrap().source);
        save_png(40, 7, &file);
        assert_eq!(
            status(&project, &shared).unwrap()[0].state,
            SharedState::Newer
        );
        // Le projet, lui, n'a pas bougé.
        assert_eq!(
            project
                .load()
                .unwrap()
                .project
                .drawing(&copy)
                .unwrap()
                .source,
            old_source
        );

        let (updated, report) =
            copy_to_project(&shared, &project, &[merle], &OPTIONS, &quiet).expect("reprise");
        assert_eq!(report.updated, vec![copy.clone()]);
        assert_eq!(updated.library.len(), 1);
        let drawing = updated.drawing(&copy).unwrap();
        assert_ne!(drawing.source, old_source);
        assert_eq!(drawing.name, "Merle noir");
        // L'ancien fichier reste (⌘Z le retrouve).
        assert!(project.resolve(&old_source).is_file());
        assert_eq!(
            status(&project, &shared).unwrap()[0].state,
            SharedState::UpToDate
        );
    }

    #[test]
    fn un_dessin_commun_retire_est_signale() {
        let (_temp, shared, project) = fixture();
        let merle = id_of(&shared, "merle");
        copy_to_project(
            &shared,
            &project,
            std::slice::from_ref(&merle),
            &OPTIONS,
            &quiet,
        )
        .expect("copie");
        crate::library::remove_drawing(&shared, &merle).unwrap();
        assert_eq!(
            status(&project, &shared).unwrap()[0].state,
            SharedState::Missing
        );
    }

    #[test]
    fn un_dessin_du_projet_est_envoye_dans_la_bibliotheque_commune() {
        let (temp, shared, project) = fixture();
        let source = temp.path().join("Poissons");
        fs::create_dir_all(&source).unwrap();
        save_png(40, 9, &source.join("diodon.png"));
        import_folder(&project, &source, &OPTIONS, &quiet).expect("import");
        let diodon = id_of(&project, "diodon");
        let (updated, report) = send_to_shared(
            &project,
            &shared,
            std::slice::from_ref(&diodon),
            &OPTIONS,
            &quiet,
        )
        .expect("envoi");
        assert_eq!(report.added.len(), 1);
        let lib = shared.load().unwrap().project;
        let sent = lib.drawing(&report.added[0]).unwrap();
        assert_eq!(sent.name, "diodon");
        assert_eq!(sent.folder(), "Trévarez/Poissons");
        assert_eq!(updated.shared_links.len(), 1);
        assert_eq!(
            status(&project, &shared).unwrap()[0].state,
            SharedState::UpToDate
        );
        // Déjà envoyé : pas de doublon.
        let (_, again) = send_to_shared(&project, &shared, &[diodon], &OPTIONS, &quiet).unwrap();
        assert_eq!(again.already_present, 1);
        assert_eq!(shared.load().unwrap().project.library.len(), 3);
        // L'écran de la bibliothèque commune le montre.
        let shown = entries(&shared, &shared.load().unwrap().project, None, 1);
        assert!(shown.iter().any(|e| e.name == "diodon"));
    }
}

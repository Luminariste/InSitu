//! Le dossier projet sur le disque : création, lecture, écriture.
//!
//! ```text
//! Epinal.insitu/
//!   projet.json          description du projet
//!   verrou.json          présent tant que quelqu'un a le projet ouvert
//!   facade/              plan ou photo de la façade
//!   photos-brutes/       photos de mire en chambre noire
//!   photos-redressees/   photos redressées
//!   fresque/             fresque importée en une image
//! ```
//!
//! L'écriture est atomique : le contenu est écrit en entier dans un fichier
//! temporaire voisin, puis ce fichier prend la place de l'ancien d'un seul
//! renommage. Une coupure, ou Dropbox qui passe au mauvais moment, ne voit
//! donc jamais un `projet.json` à moitié écrit : soit l'ancien, soit le
//! nouveau.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::error::ProjectError;
use super::migrate::{self, Migration, CURRENT_FORMAT_VERSION};
use super::model::{new_id, Project};
use super::path::RelativePath;

/// Extension du dossier projet.
pub const PROJECT_EXTENSION: &str = "insitu";

/// Nom du fichier de description, dans le dossier projet.
pub const PROJECT_FILE: &str = "projet.json";

/// Début du nom des fichiers temporaires. Dropbox ne synchronise pas les
/// fichiers dont le nom commence par `.~`.
const TEMP_PREFIX: &str = ".~";

/// Les familles de fichiers d'un projet, chacune dans son sous-dossier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Facade,
    RawPhoto,
    RectifiedPhoto,
    Fresco,
    /// Les dessins de la bibliothèque (tâche 4.1).
    Drawing,
}

impl AssetKind {
    pub const ALL: [AssetKind; 5] = [
        AssetKind::Facade,
        AssetKind::RawPhoto,
        AssetKind::RectifiedPhoto,
        AssetKind::Fresco,
        AssetKind::Drawing,
    ];

    /// Nom du sous-dossier, tel qu'on le voit dans le Finder.
    pub fn directory(self) -> &'static str {
        match self {
            AssetKind::Facade => "facade",
            AssetKind::RawPhoto => "photos-brutes",
            AssetKind::RectifiedPhoto => "photos-redressees",
            AssetKind::Fresco => "fresque",
            AssetKind::Drawing => "bibliotheque",
        }
    }
}

/// Ce que rend la lecture d'un projet.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedProject {
    pub project: Project,
    /// Version d'origine si le projet a été mis à niveau à la lecture. Le
    /// fichier sur le disque n'a pas encore changé.
    pub migrated_from: Option<u32>,
    /// Champs du fichier inconnus de cette version de l'app. Ils sont
    /// conservés à l'enregistrement.
    pub unknown_fields: Vec<String>,
}

/// Un dossier projet `<nom>.insitu`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDir {
    root: PathBuf,
}

impl ProjectDir {
    /// Crée un nouveau projet : le dossier, ses sous-dossiers et un
    /// `projet.json` vide. Le nom du dossier doit se terminer par `.insitu`.
    pub fn create(
        root: impl Into<PathBuf>,
        name: Option<&str>,
    ) -> Result<(Self, Project), ProjectError> {
        let root = root.into();
        if !has_project_extension(&root) {
            return Err(ProjectError::BadExtension { path: root });
        }
        if root.join(PROJECT_FILE).exists() {
            return Err(ProjectError::AlreadyExists { path: root });
        }
        if root.exists() {
            let mut entries =
                fs::read_dir(&root).map_err(ProjectError::io("de lire le dossier", &root))?;
            if entries.next().is_some() {
                return Err(ProjectError::DirectoryNotEmpty { path: root });
            }
        }
        fs::create_dir_all(&root).map_err(ProjectError::io("de créer le dossier", &root))?;
        for kind in AssetKind::ALL {
            let sub = root.join(kind.directory());
            fs::create_dir_all(&sub).map_err(ProjectError::io("de créer le dossier", &sub))?;
        }
        let default_name = root
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let project = Project::new(name.map(str::to_string).unwrap_or(default_name));
        let dir = Self { root };
        dir.save(&project)?;
        Ok((dir, project))
    }

    /// Désigne un projet existant. Ne lit pas encore son contenu.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, ProjectError> {
        let root = root.into();
        if !root.join(PROJECT_FILE).is_file() {
            return Err(ProjectError::NotAProject { path: root });
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn project_file(&self) -> PathBuf {
        self.root.join(PROJECT_FILE)
    }

    /// Emplacement, sur cette machine, d'un fichier du projet.
    pub fn resolve(&self, path: &RelativePath) -> PathBuf {
        path.resolve(&self.root)
    }

    /// Lit le projet, en le mettant à niveau si son format est plus ancien.
    /// Ne modifie rien sur le disque.
    pub fn load(&self) -> Result<LoadedProject, ProjectError> {
        self.load_with(migrate::FIRST_FORMAT_VERSION, migrate::MIGRATIONS)
    }

    /// Comme [`ProjectDir::load`], avec les étapes de mise à niveau
    /// fournies ; voir [`migrate::upgrade_with`].
    pub(crate) fn load_with(
        &self,
        first: u32,
        migrations: &[Migration],
    ) -> Result<LoadedProject, ProjectError> {
        let file = self.project_file();
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(ProjectError::NotAProject {
                    path: self.root.clone(),
                })
            }
            Err(e) => return Err(ProjectError::io("de lire", &file)(e)),
        };
        let mut document: Value =
            serde_json::from_str(&text).map_err(|e| ProjectError::InvalidJson {
                path: file.clone(),
                message: format!(
                    "ce n'est pas du JSON valide (ligne {}, colonne {} : {e})",
                    e.line(),
                    e.column()
                ),
            })?;
        let migrated_from = migrate::upgrade_with(&mut document, &file, first, migrations)?;
        let project: Project =
            serde_json::from_value(document).map_err(|e| ProjectError::InvalidJson {
                path: file.clone(),
                message: format!("la description du projet est incorrecte ({e})"),
            })?;
        let unknown_fields = project.unknown_fields();
        Ok(LoadedProject {
            project,
            migrated_from,
            unknown_fields,
        })
    }

    /// Enregistre le projet, de façon atomique. Si le fichier en place est
    /// d'un format plus ancien, il est d'abord copié en `projet.v<N>.json`.
    pub fn save(&self, project: &Project) -> Result<(), ProjectError> {
        if let Some((field, _)) = project
            .numbers()
            .into_iter()
            .find(|(_, value)| !value.is_finite())
        {
            return Err(ProjectError::NonFiniteNumber { field });
        }
        let file = self.project_file();
        let mut text =
            serde_json::to_string_pretty(project).map_err(|e| ProjectError::InvalidJson {
                path: file.clone(),
                message: format!("le projet ne peut pas être écrit en JSON ({e})"),
            })?;
        text.push('\n');
        self.back_up_older_format(&file)?;
        remove_stale_temp_files(&file);
        write_atomic(&file, text.as_bytes())
    }

    /// Garde une copie du fichier en place s'il est d'un format plus ancien
    /// que celui qu'on s'apprête à écrire.
    fn back_up_older_format(&self, file: &Path) -> Result<(), ProjectError> {
        let Ok(text) = fs::read_to_string(file) else {
            return Ok(());
        };
        let Some(version) = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|document| document.get("formatVersion").and_then(Value::as_u64))
        else {
            return Ok(());
        };
        if version >= u64::from(CURRENT_FORMAT_VERSION) {
            return Ok(());
        }
        let backup = self.root.join(format!("projet.v{version}.json"));
        if backup.exists() {
            return Ok(());
        }
        write_atomic(&backup, text.as_bytes())
    }

    /// Copie un fichier dans le sous-dossier de sa famille et renvoie son
    /// chemin relatif. Si un fichier différent porte déjà ce nom, la copie
    /// reçoit un suffixe (`a-brut-2.jpg`) ; si c'est le même contenu, il est
    /// réutilisé.
    pub fn import_file(
        &self,
        source: &Path,
        kind: AssetKind,
    ) -> Result<RelativePath, ProjectError> {
        self.import_file_into(source, kind.directory())
    }

    /// Comme [`Self::import_file`], dans le sous-dossier `directory` du
    /// projet (chemin relatif, par exemple `bibliotheque/Oiseaux`).
    pub fn import_file_into(
        &self,
        source: &Path,
        directory: &str,
    ) -> Result<RelativePath, ProjectError> {
        let folder = RelativePath::new(directory)?;
        if !source.is_file() {
            return Err(ProjectError::ImportSourceMissing {
                path: source.to_path_buf(),
            });
        }
        let file_name = source
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (stem, extension) = match file_name.rsplit_once('.') {
            Some((stem, extension)) if !stem.is_empty() => (stem, format!(".{extension}")),
            _ => (file_name.as_str(), String::new()),
        };
        let directory = folder.resolve(&self.root);
        fs::create_dir_all(&directory)
            .map_err(ProjectError::io("de créer le dossier", &directory))?;
        let mut attempt = 1u32;
        loop {
            let candidate = if attempt == 1 {
                file_name.clone()
            } else {
                format!("{stem}-{attempt}{extension}")
            };
            let relative = RelativePath::new(format!("{folder}/{candidate}"))?;
            let destination = directory.join(&candidate);
            if !destination.exists() {
                copy_atomic(source, &destination)?;
                return Ok(relative);
            }
            if same_content(source, &destination)? {
                return Ok(relative);
            }
            attempt += 1;
        }
    }
}

/// Vrai si le nom du dossier se termine par `.insitu`.
pub(crate) fn has_project_extension(root: &Path) -> bool {
    root.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case(PROJECT_EXTENSION))
        && root.file_stem().is_some_and(|stem| !stem.is_empty())
}

/// Nom d'un fichier temporaire voisin de `target`.
fn temp_path_for(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    target.with_file_name(format!("{TEMP_PREFIX}{name}.{}", new_id()))
}

/// Écrit `bytes` dans `target` sans jamais laisser voir un fichier partiel :
/// écriture complète dans un fichier temporaire du même dossier, vidage sur
/// le disque, puis renommage, qui remplace l'ancien fichier d'un coup.
pub(crate) fn write_atomic(target: &Path, bytes: &[u8]) -> Result<(), ProjectError> {
    let temp = temp_path_for(target);
    let written = (|| -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, target)
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&temp);
        return Err(ProjectError::io("d'enregistrer", target)(e));
    }
    sync_parent(target);
    Ok(())
}

/// Copie un fichier sans jamais laisser voir une copie partielle.
fn copy_atomic(source: &Path, destination: &Path) -> Result<(), ProjectError> {
    let temp = temp_path_for(destination);
    let copied = fs::copy(source, &temp).and_then(|_| fs::rename(&temp, destination));
    if let Err(e) = copied {
        let _ = fs::remove_file(&temp);
        return Err(ProjectError::io("de copier dans le projet", source)(e));
    }
    sync_parent(destination);
    Ok(())
}

/// Demande au système d'inscrire le renommage sur le disque. Sans effet
/// visible ; un échec n'empêche rien.
fn sync_parent(target: &Path) {
    if let Some(parent) = target.parent() {
        if let Ok(directory) = File::open(parent) {
            let _ = directory.sync_all();
        }
    }
}

/// Efface les fichiers temporaires qu'une écriture interrompue aurait
/// laissés à côté de `target`.
fn remove_stale_temp_files(target: &Path) {
    let (Some(parent), Some(name)) = (target.parent(), target.file_name()) else {
        return;
    };
    let prefix = format!("{TEMP_PREFIX}{}.", name.to_string_lossy());
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with(&prefix) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Compare le contenu de deux fichiers.
fn same_content(a: &Path, b: &Path) -> Result<bool, ProjectError> {
    let size = |path: &Path| {
        fs::metadata(path)
            .map(|m| m.len())
            .map_err(ProjectError::io("de lire", path))
    };
    if size(a)? != size(b)? {
        return Ok(false);
    }
    let mut file_a = File::open(a).map_err(ProjectError::io("de lire", a))?;
    let mut file_b = File::open(b).map_err(ProjectError::io("de lire", b))?;
    let mut buffer_a = vec![0u8; 64 * 1024];
    let mut buffer_b = vec![0u8; 64 * 1024];
    loop {
        let read_a =
            read_full(&mut file_a, &mut buffer_a).map_err(ProjectError::io("de lire", a))?;
        let read_b =
            read_full(&mut file_b, &mut buffer_b).map_err(ProjectError::io("de lire", b))?;
        if read_a != read_b || buffer_a[..read_a] != buffer_b[..read_b] {
            return Ok(false);
        }
        if read_a == 0 {
            return Ok(true);
        }
    }
}

/// Remplit `buffer` autant que possible ; renvoie le nombre d'octets lus.
fn read_full(file: &mut File, buffer: &mut [u8]) -> io::Result<usize> {
    let mut total = 0;
    while total < buffer.len() {
        match file.read(&mut buffer[total..])? {
            0 => break,
            n => total += n,
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::{ExtraFields, PointMm, RegistrationPair};
    use crate::project::test_support::TempDir;
    use serde_json::json;

    fn temp_files_in(directory: &Path) -> Vec<String> {
        fs::read_dir(directory)
            .expect("dossier lisible")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(TEMP_PREFIX))
            .collect()
    }

    #[test]
    fn a_new_project_has_its_folders_and_reads_back_identical() {
        let temp = TempDir::new("creation");
        let root = temp.path().join("Épinal.insitu");
        let (dir, project) = ProjectDir::create(&root, None).expect("création");
        assert_eq!(project.name, "Épinal");
        for sub in ["facade", "photos-brutes", "photos-redressees", "fresque"] {
            assert!(root.join(sub).is_dir(), "{sub} manque");
        }
        let loaded = ProjectDir::open(&root)
            .expect("ouverture")
            .load()
            .expect("lecture");
        assert_eq!(loaded.project, project);
        assert_eq!(loaded.migrated_from, None);
        assert!(loaded.unknown_fields.is_empty());
        assert_eq!(dir.root(), root);
    }

    #[test]
    fn creation_refuses_wrong_names_and_existing_content() {
        let temp = TempDir::new("refus");
        let error = ProjectDir::create(temp.path().join("Epinal"), None).expect_err("extension");
        assert!(matches!(error, ProjectError::BadExtension { .. }));

        let root = temp.path().join("Epinal.insitu");
        ProjectDir::create(&root, Some("Épinal")).expect("création");
        let error = ProjectDir::create(&root, None).expect_err("déjà là");
        assert!(matches!(error, ProjectError::AlreadyExists { .. }));

        let busy = temp.path().join("Autre.insitu");
        fs::create_dir_all(&busy).expect("dossier");
        fs::write(busy.join("notes.txt"), "à garder").expect("fichier");
        let error = ProjectDir::create(&busy, None).expect_err("non vide");
        assert!(matches!(error, ProjectError::DirectoryNotEmpty { .. }));
    }

    #[test]
    fn opening_something_else_than_a_project_is_refused() {
        let temp = TempDir::new("pas-un-projet");
        let error = ProjectDir::open(temp.path().join("Absent.insitu")).expect_err("absent");
        assert!(matches!(error, ProjectError::NotAProject { .. }));
        assert!(error.to_string().contains("n'est pas un projet InSitu"));
    }

    #[test]
    fn the_file_is_readable_json_ending_with_a_newline() {
        let temp = TempDir::new("lisible");
        let (dir, _) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
        let text = fs::read_to_string(dir.project_file()).expect("lecture");
        assert!(text.starts_with("{\n  \"formatVersion\": 1,\n"), "{text}");
        assert!(text.ends_with("}\n"));
    }

    #[test]
    fn saving_leaves_no_temporary_file_and_cleans_old_ones() {
        let temp = TempDir::new("temporaires");
        let (dir, mut project) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
        // Reste d'une écriture interrompue : un fichier temporaire tronqué.
        let leftover = dir.root().join(".~projet.json.0123456789abcdef");
        fs::write(&leftover, "{ \"formatVersion\": 1, \"na").expect("reste");
        // Le projet en place reste lisible malgré ce reste.
        assert_eq!(dir.load().expect("lecture").project, project);

        project.name = "Renommé".to_string();
        dir.save(&project).expect("enregistrement");
        assert!(temp_files_in(dir.root()).is_empty());
        assert_eq!(dir.load().expect("lecture").project.name, "Renommé");
    }

    #[test]
    fn a_failed_write_keeps_the_previous_file_intact() {
        let temp = TempDir::new("echec");
        let target = temp.path().join("projet.json");
        write_atomic(&target, b"ancien contenu").expect("première écriture");
        // On rend le remplacement impossible : la cible devient un dossier
        // non vide, que le renommage ne peut pas écraser.
        let blocked = temp.path().join("bloque.json");
        fs::create_dir_all(blocked.join("occupant")).expect("dossier");
        let error = write_atomic(&blocked, b"nouveau contenu").expect_err("échec attendu");
        assert!(matches!(error, ProjectError::Io { .. }));
        assert!(error.to_string().starts_with("Impossible d"), "{error}");
        assert!(
            temp_files_in(temp.path()).is_empty(),
            "pas de reste après échec"
        );
        assert_eq!(
            fs::read(&target).expect("lecture"),
            b"ancien contenu".to_vec()
        );
    }

    #[test]
    fn a_replaced_file_is_never_seen_half_written() {
        let temp = TempDir::new("atomique");
        let target = temp.path().join("projet.json");
        let old = vec![b'a'; 200_000];
        let new = vec![b'b'; 300_000];
        write_atomic(&target, &old).expect("écriture");
        // Un lecteur qui tient l'ancien fichier ouvert le lit en entier,
        // même si le remplacement a lieu entre-temps.
        let mut reader = File::open(&target).expect("ouverture");
        write_atomic(&target, &new).expect("remplacement");
        let mut seen = Vec::new();
        reader.read_to_end(&mut seen).expect("lecture");
        assert_eq!(seen, old);
        assert_eq!(fs::read(&target).expect("lecture"), new);
    }

    #[test]
    fn numbers_survive_the_round_trip_to_the_last_bit() {
        let temp = TempDir::new("nombres");
        let (dir, mut project) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
        let awkward = [
            0.1 + 0.2,
            1.0 / 3.0,
            280.0,
            -0.0,
            5e-324,
            1.797_693_134_862_315_7e308,
            123_456.789_012_345_68,
            280.0 / 15_433.0 * 938.0,
            2.0f64.sqrt() * 1e-7,
        ];
        let projector = project.add_projector("A");
        for (i, value) in awkward.iter().enumerate() {
            projector.registration.push(RegistrationPair::new(
                PointMm::new(*value, i as f64),
                PointMm::new(-*value, *value / 3.0),
            ));
        }
        dir.save(&project).expect("enregistrement");
        let loaded = dir.load().expect("lecture").project;
        let bits =
            |p: &Project| -> Vec<u64> { p.numbers().iter().map(|(_, v)| v.to_bits()).collect() };
        assert_eq!(bits(&loaded), bits(&project));
    }

    #[test]
    fn a_non_finite_number_is_refused_and_nothing_is_written() {
        let temp = TempDir::new("infini");
        let (dir, mut project) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
        let before = fs::read(dir.project_file()).expect("lecture");
        project.export.dpi = f64::NAN;
        let error = dir.save(&project).expect_err("refus");
        assert!(
            matches!(error, ProjectError::NonFiniteNumber { ref field } if field == "export.dpi")
        );
        assert_eq!(fs::read(dir.project_file()).expect("lecture"), before);
    }

    #[test]
    fn unknown_fields_are_reported_and_written_back() {
        let temp = TempDir::new("inconnus");
        let (dir, _) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
        // Un fichier comme l'écrirait une version qui connaît déjà les
        // placements de dessins et un réglage de plus par projecteur.
        let document = json!({
            "formatVersion": 1,
            "id": "0123456789abcdef",
            "name": "Essai",
            "createdAt": "2026-11-11T10:00:00Z",
            "placements": [{ "id": "p1", "drawing": "d1" }],
            "projectors": [{
                "id": "aaaaaaaaaaaaaaaa",
                "label": "A",
                "lens": { "focalMm": 320.0 },
                "registration": [{
                    "slide": { "x": 10.0, "y": 20.0 },
                    "facade": { "x": 1000.0, "y": 2000.0 },
                    "note": "angle du tympan"
                }]
            }],
            "export": { "dpi": 1000.0, "dither": "floyd" }
        });
        fs::write(dir.project_file(), document.to_string()).expect("écriture");

        let loaded = dir.load().expect("lecture");
        assert_eq!(
            loaded.unknown_fields,
            vec![
                "placements",
                "projectors[0].lens",
                "projectors[0].registration[0].note",
                "export.dither",
            ]
        );
        let mut project = loaded.project;
        project.name = "Essai modifié".to_string();
        dir.save(&project).expect("enregistrement");

        let text = fs::read_to_string(dir.project_file()).expect("lecture");
        let written: Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(written["placements"], document["placements"]);
        assert_eq!(
            written["projectors"][0]["lens"],
            json!({ "focalMm": 320.0 })
        );
        assert_eq!(
            written["projectors"][0]["registration"][0]["note"],
            "angle du tympan"
        );
        assert_eq!(written["export"]["dither"], "floyd");
        assert_eq!(written["name"], "Essai modifié");
    }

    #[test]
    fn a_future_version_is_refused_and_left_untouched() {
        let temp = TempDir::new("futur");
        let (dir, _) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
        let future = json!({
            "formatVersion": CURRENT_FORMAT_VERSION + 1,
            "id": "0123456789abcdef",
            "name": "Demain",
            "createdAt": "2027-01-01T00:00:00Z"
        })
        .to_string();
        fs::write(dir.project_file(), &future).expect("écriture");
        let error = dir.load().expect_err("refus");
        assert!(matches!(error, ProjectError::FormatTooRecent { .. }));
        assert!(error.to_string().contains("version plus récente d'InSitu"));
        assert_eq!(
            fs::read_to_string(dir.project_file()).expect("lecture"),
            future
        );
    }

    /// Étape fictive v1 → v2 : les projecteurs s'appelaient `devices` et
    /// leur libellé `name`.
    fn rename_devices(document: &mut Value) -> Result<(), String> {
        let object = document.as_object_mut().ok_or("objet attendu")?;
        let mut devices = object.remove("devices").unwrap_or(json!([]));
        for device in devices.as_array_mut().ok_or("liste attendue")? {
            let device = device.as_object_mut().ok_or("objet attendu")?;
            let name = device.remove("name").ok_or("libellé absent")?;
            device.insert("label".to_string(), name);
        }
        object.insert("projectors".to_string(), devices);
        Ok(())
    }

    #[test]
    fn an_older_project_is_upgraded_when_opened() {
        let temp = TempDir::new("migration");
        let (dir, _) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
        let old = json!({
            "formatVersion": 0,
            "id": "0123456789abcdef",
            "name": "Ancien",
            "createdAt": "2026-01-01T00:00:00Z",
            "devices": [
                { "id": "aaaaaaaaaaaaaaaa", "name": "A" },
                { "id": "bbbbbbbbbbbbbbbb", "name": "B" }
            ]
        })
        .to_string();
        fs::write(dir.project_file(), &old).expect("écriture");

        // Format fictif 0, mis à niveau vers le format 1 par une étape.
        let loaded = dir.load_with(0, &[rename_devices]).expect("lecture");
        assert_eq!(loaded.migrated_from, Some(0));
        assert_eq!(loaded.project.format_version, CURRENT_FORMAT_VERSION);
        let labels: Vec<&str> = loaded
            .project
            .projectors
            .iter()
            .map(|p| p.label.as_str())
            .collect();
        assert_eq!(labels, ["A", "B"]);
        // La lecture seule ne touche pas au disque.
        assert_eq!(
            fs::read_to_string(dir.project_file()).expect("lecture"),
            old
        );

        // L'enregistrement écrit le nouveau format et garde l'ancien fichier.
        dir.save(&loaded.project).expect("enregistrement");
        assert_eq!(
            fs::read_to_string(dir.root().join("projet.v0.json")).expect("copie"),
            old
        );
        let reloaded = dir.load().expect("relecture");
        assert_eq!(reloaded.migrated_from, None);
        assert_eq!(reloaded.project, loaded.project);
    }

    #[test]
    fn broken_json_is_reported_with_its_position() {
        let temp = TempDir::new("casse");
        let (dir, _) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
        fs::write(
            dir.project_file(),
            "{\n  \"formatVersion\": 1,\n  \"name\": ",
        )
        .expect("écriture");
        let error = dir.load().expect_err("refus");
        let message = error.to_string();
        assert!(matches!(error, ProjectError::InvalidJson { .. }));
        assert!(message.contains("ligne 3"), "{message}");
    }

    #[test]
    fn an_absolute_path_in_the_file_is_refused() {
        let temp = TempDir::new("absolu");
        let (dir, _) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
        let document = json!({
            "formatVersion": 1,
            "id": "0123456789abcdef",
            "name": "Essai",
            "createdAt": "2026-11-11T10:00:00Z",
            "projectors": [{
                "id": "aaaaaaaaaaaaaaaa",
                "label": "A",
                "rawPhoto": "/Users/benjamin/Dropbox/a-brut.jpg"
            }]
        });
        fs::write(dir.project_file(), document.to_string()).expect("écriture");
        let message = dir.load().expect_err("refus").to_string();
        assert!(message.contains("chemins relatifs"), "{message}");
    }

    #[test]
    fn imported_files_get_relative_paths_inside_the_project() {
        let temp = TempDir::new("import");
        let outside = temp.path().join("ailleurs");
        fs::create_dir_all(&outside).expect("dossier");
        let photo = outside.join("a-brut.jpg");
        fs::write(&photo, b"photo A").expect("fichier");
        let (dir, _) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");

        let first = dir
            .import_file(&photo, AssetKind::RawPhoto)
            .expect("import");
        assert_eq!(first.as_str(), "photos-brutes/a-brut.jpg");
        assert_eq!(fs::read(dir.resolve(&first)).expect("lecture"), b"photo A");

        // Même contenu : réutilisé. Contenu différent : suffixe.
        let again = dir
            .import_file(&photo, AssetKind::RawPhoto)
            .expect("import");
        assert_eq!(again, first);
        fs::write(&photo, b"autre photo").expect("fichier");
        let other = dir
            .import_file(&photo, AssetKind::RawPhoto)
            .expect("import");
        assert_eq!(other.as_str(), "photos-brutes/a-brut-2.jpg");
        assert!(temp_files_in(&dir.root().join("photos-brutes")).is_empty());

        let error = dir
            .import_file(&outside.join("absent.jpg"), AssetKind::RawPhoto)
            .expect_err("absent");
        assert!(matches!(error, ProjectError::ImportSourceMissing { .. }));
    }

    #[test]
    fn a_project_folder_can_be_moved_and_still_opens() {
        let temp = TempDir::new("deplacement");
        let outside = temp.path().join("a-brut.jpg");
        fs::write(&outside, b"photo A").expect("fichier");
        let first_home = temp.path().join("Mac de Benjamin").join("Essai.insitu");
        fs::create_dir_all(first_home.parent().expect("parent")).expect("dossier");
        let (dir, mut project) = ProjectDir::create(&first_home, None).expect("création");
        let photo = dir
            .import_file(&outside, AssetKind::RawPhoto)
            .expect("import");
        project.add_projector("A").raw_photo = Some(photo.clone());
        project.extra = ExtraFields::new();
        dir.save(&project).expect("enregistrement");

        // Le fichier ne contient aucune trace de l'emplacement d'origine.
        let text = fs::read_to_string(dir.project_file()).expect("lecture");
        assert!(!text.contains(&temp.path().to_string_lossy().into_owned()));
        assert!(!text.contains("Mac de Benjamin"));

        // Déplacé ailleurs (l'autre Mac), le projet s'ouvre et retrouve sa photo.
        let second_home = temp.path().join("Mac de Marc").join("Essai.insitu");
        fs::create_dir_all(second_home.parent().expect("parent")).expect("dossier");
        fs::rename(&first_home, &second_home).expect("déplacement");
        let moved = ProjectDir::open(&second_home).expect("ouverture");
        let loaded = moved.load().expect("lecture").project;
        assert_eq!(loaded, project);
        assert_eq!(fs::read(moved.resolve(&photo)).expect("photo"), b"photo A");
    }
}

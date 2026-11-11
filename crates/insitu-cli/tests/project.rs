//! Les commandes `insitu project …`, lancées comme le ferait un utilisateur.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// Dossier temporaire, effacé à la fin du test.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("insitu-cli-test-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("création du dossier temporaire");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn insitu(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_insitu"))
        .args(args)
        .output()
        .expect("lancement de insitu")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("chemin en UTF-8")
}

#[test]
fn new_then_info_then_validate() {
    let temp = TempDir::new("cycle");
    let root = temp.0.join("Épinal.insitu");
    let folder = path_str(&root);

    let created = insitu(&["project", "new", folder]);
    assert!(created.status.success(), "{}", stderr(&created));
    assert!(stdout(&created).contains("Projet « Épinal » créé"));
    assert!(root.join("projet.json").is_file());
    assert!(root.join("photos-brutes").is_dir());

    let info = insitu(&["project", "info", folder]);
    assert!(info.status.success(), "{}", stderr(&info));
    let summary: Value = serde_json::from_str(&stdout(&info)).expect("JSON");
    assert_eq!(summary["name"], "Épinal");
    assert_eq!(summary["formatVersion"], 1);
    assert_eq!(summary["projectors"], serde_json::json!([]));
    assert_eq!(summary["export"]["dpi"], 1400.0);
    assert_eq!(summary["lock"], Value::Null);

    let validated = insitu(&["project", "validate", folder]);
    assert!(validated.status.success(), "{}", stderr(&validated));
    assert!(stdout(&validated).contains("Projet valide"));
}

#[test]
fn new_accepts_a_name_and_refuses_bad_folders() {
    let temp = TempDir::new("nom");
    let root = temp.0.join("essai.insitu");
    let created = insitu(&[
        "project",
        "new",
        path_str(&root),
        "--name",
        "Portail d'Épinal",
    ]);
    assert!(created.status.success(), "{}", stderr(&created));
    let info = insitu(&["project", "info", path_str(&root)]);
    let summary: Value = serde_json::from_str(&stdout(&info)).expect("JSON");
    assert_eq!(summary["name"], "Portail d'Épinal");

    let again = insitu(&["project", "new", path_str(&root)]);
    assert_eq!(again.status.code(), Some(1));
    assert!(stderr(&again).contains("Un projet existe déjà"));

    let wrong = insitu(&["project", "new", path_str(&temp.0.join("essai"))]);
    assert_eq!(wrong.status.code(), Some(1));
    assert!(stderr(&wrong).contains("doit se terminer par .insitu"));
}

#[test]
fn validate_reports_missing_files_and_fails() {
    let temp = TempDir::new("manquant");
    let root = temp.0.join("Essai.insitu");
    let folder = path_str(&root);
    assert!(insitu(&["project", "new", folder]).status.success());

    // On ajoute à la main un projecteur dont la photo n'existe pas.
    let file = root.join("projet.json");
    let mut document: Value =
        serde_json::from_str(&fs::read_to_string(&file).expect("lecture")).expect("JSON");
    document["projectors"] = serde_json::json!([{
        "id": "aaaaaaaaaaaaaaaa",
        "label": "A",
        "rawPhoto": "photos-brutes/a-brut.jpg"
    }]);
    fs::write(&file, document.to_string()).expect("écriture");

    let validated = insitu(&["project", "validate", folder]);
    assert_eq!(validated.status.code(), Some(1));
    let text = stdout(&validated);
    assert!(
        text.contains(
            "Erreur (projectors[0].rawPhoto) : fichier introuvable : photos-brutes/a-brut.jpg"
        ),
        "{text}"
    );
    assert!(text.contains("Projet invalide : 1 erreur(s)"), "{text}");

    // Une fois le fichier présent, le projet est valide.
    fs::write(root.join("photos-brutes").join("a-brut.jpg"), b"photo").expect("écriture");
    let validated = insitu(&["project", "validate", folder]);
    assert!(validated.status.success(), "{}", stdout(&validated));
}

#[test]
fn a_project_from_a_newer_app_is_refused_in_french() {
    let temp = TempDir::new("futur");
    let root = temp.0.join("Essai.insitu");
    let folder = path_str(&root);
    assert!(insitu(&["project", "new", folder]).status.success());
    let file = root.join("projet.json");
    let text = fs::read_to_string(&file)
        .expect("lecture")
        .replace("\"formatVersion\": 1", "\"formatVersion\": 99");
    fs::write(&file, text).expect("écriture");

    let info = insitu(&["project", "info", folder]);
    assert_eq!(info.status.code(), Some(1));
    assert!(
        stderr(&info).contains("version plus récente d'InSitu (format 99)"),
        "{}",
        stderr(&info)
    );

    let validated = insitu(&["project", "validate", folder]);
    assert_eq!(validated.status.code(), Some(1));
    assert!(stdout(&validated).contains("version plus récente d'InSitu"));
}

#[test]
fn mistakes_in_the_command_show_the_usage() {
    let missing = insitu(&["project", "info", "/nulle/part/Absent.insitu"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(stderr(&missing).contains("n'est pas un projet InSitu"));

    let unknown = insitu(&["project", "ouvrir"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(stderr(&unknown).contains("Usage : insitu"));

    let incomplete = insitu(&["project", "new"]);
    assert_eq!(incomplete.status.code(), Some(2));

    let engine = insitu(&["info"]);
    assert!(engine.status.success());
    assert!(stdout(&engine).contains("\"slideSizeMm\": 280.0"));
}

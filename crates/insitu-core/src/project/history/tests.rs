use super::*;
use crate::project::test_support::TempDir;

fn setup(label: &str) -> (TempDir, ProjectDir) {
    let temp = TempDir::new(label);
    let (dir, _) = ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
    let dir = ProjectDir::open(dir.root())
        .unwrap()
        .with_history(temp.path().join("historique"));
    (temp, dir)
}

fn rename(dir: &ProjectDir, name: &str) {
    let mut project = dir.load().unwrap().project;
    project.name = name.to_string();
    dir.save(&project).unwrap();
}

fn name(dir: &ProjectDir) -> String {
    dir.load().unwrap().project.name
}

#[test]
fn every_save_can_be_undone_and_redone_without_limit() {
    let (_temp, dir) = setup("historique");
    assert!(matches!(
        step(&dir, false),
        Err(HistoryError::Nothing { redo: false })
    ));
    for i in 1..=30 {
        rename(&dir, &format!("v{i}"));
    }
    assert_eq!(state(&dir).unwrap().depth, 30);
    for i in (1..30).rev() {
        let (project, label) = step(&dir, false).unwrap();
        assert_eq!(project.name, format!("v{i}"));
        assert_eq!(label, "le nom du projet");
    }
    let (project, _) = step(&dir, false).unwrap();
    assert_eq!(project.name, "Essai");
    assert!(step(&dir, false).is_err());

    // Rétablir remonte toute la pile…
    for i in 1..=3 {
        assert_eq!(step(&dir, true).unwrap().0.name, format!("v{i}"));
    }
    assert_eq!(
        state(&dir).unwrap().redo.as_deref(),
        Some("le nom du projet")
    );
    // … jusqu'à une nouvelle action, qui efface ce qu'on pouvait rétablir.
    rename(&dir, "autre");
    assert!(matches!(
        step(&dir, true),
        Err(HistoryError::Nothing { redo: true })
    ));
    assert_eq!(step(&dir, false).unwrap().0.name, "v3");
}

#[test]
fn history_survives_reopening_the_project() {
    let (temp, dir) = setup("historique-reouvert");
    rename(&dir, "premier");
    rename(&dir, "second");
    let reopened = ProjectDir::open(dir.root())
        .unwrap()
        .with_history(temp.path().join("historique"));
    assert_eq!(step(&reopened, false).unwrap().0.name, "premier");
    assert_eq!(name(&dir), "premier");
}

#[test]
fn a_change_made_elsewhere_is_never_overwritten() {
    let (_temp, dir) = setup("historique-ailleurs");
    rename(&dir, "ici");
    // L'autre Mac enregistre par Dropbox, sans passer par cet historique.
    let elsewhere = ProjectDir::open(dir.root()).unwrap();
    rename(&elsewhere, "là-bas");
    assert!(matches!(
        step(&dir, false),
        Err(HistoryError::ChangedElsewhere)
    ));
    assert_eq!(name(&dir), "là-bas");
    assert_eq!(state(&dir).unwrap().depth, 0);
}

#[test]
fn a_project_without_history_records_nothing() {
    let (temp, dir) = setup("historique-sans");
    let plain = ProjectDir::open(dir.root()).unwrap();
    rename(&plain, "x");
    assert!(matches!(step(&plain, false), Err(HistoryError::NoHistory)));
    assert!(!temp.path().join("historique").exists());
}

#[test]
fn labels_name_what_changed() {
    let a = r#"{"id":"1","name":"a","placements":[]}"#;
    assert_eq!(
        describe(a, r#"{"id":"1","name":"a","placements":[1]}"#),
        "les dessins posés"
    );
    assert_eq!(
        describe(a, r#"{"id":"1","name":"b","layers":[1],"placements":[]}"#),
        "les calques et le nom du projet"
    );
}

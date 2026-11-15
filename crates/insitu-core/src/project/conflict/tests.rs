use std::collections::BTreeMap;
use std::fs;

use serde_json::{json, Value};

use super::*;
use crate::project::history;
use crate::project::test_support::TempDir;

const COPY: &str = "projet (Copie en conflit de Benjamin Nesme 2026-11-15).json";

fn drawing(id: &str, name: &str) -> Value {
    json!({ "id": id, "name": name, "source": format!("bibliotheque/{name}.png"), "width": 100.0, "height": 50.0 })
}

fn placed(id: &str, drawing: &str, x: f64) -> Value {
    json!({ "id": id, "drawing": drawing, "center": { "x": x, "y": 500.0 }, "width": 100.0, "height": 50.0 })
}

/// Le projet avant le conflit : deux dessins posés, un projecteur.
fn base(id: &str) -> Value {
    json!({
        "formatVersion": 1,
        "id": id,
        "name": "Trévarez",
        "createdAt": "2026-11-11T14:03:22Z",
        "projectors": [{ "id": "pb", "label": "B" }],
        "library": [drawing("algue", "Algue"), drawing("diodon", "Diodon")],
        "placements": [placed("p1", "algue", 100.0), placed("p2", "diodon", 200.0)],
    })
}

fn write(dir: &ProjectDir, value: &Value) {
    let project: Project = serde_json::from_value(value.clone()).expect("projet");
    dir.save(&project).expect("enregistrement");
}

fn push(value: &mut Value, section: &str, element: Value) {
    value[section].as_array_mut().unwrap().push(element);
}

/// Le cas de Trévarez, en petit. Ce Mac est celui de Benjamin : son
/// historique a connu la version d'avant le conflit, puis la sienne, que
/// Dropbox a renommée en copie en conflit. `projet.json` est arrivé de chez
/// Marc.
fn setup(label: &str) -> (TempDir, ProjectDir) {
    let temp = TempDir::new(label);
    let (created, project) =
        ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
    let dir = ProjectDir::open(created.root())
        .unwrap()
        .with_history(temp.path().join("historique"));
    let id = project.id.clone();
    write(&dir, &base(&id));

    // Benjamin : importe Disamare et Daurade, les pose, déplace p1, renomme B.
    let mut benjamin = base(&id);
    push(&mut benjamin, "library", drawing("b-disamare", "Disamare"));
    push(&mut benjamin, "library", drawing("b-daurade", "Daurade"));
    push(
        &mut benjamin,
        "placements",
        placed("pb1", "b-disamare", 300.0),
    );
    push(
        &mut benjamin,
        "placements",
        placed("pb2", "b-daurade", 400.0),
    );
    benjamin["placements"][0]["center"]["x"] = json!(150.0);
    benjamin["projectors"][0]["label"] = json!("B avant-toit");
    write(&dir, &benjamin);
    fs::rename(dir.project_file(), dir.root().join(COPY)).unwrap();

    // Marc : importe aussi Disamare (même fichier), le pose, déplace p2,
    // renomme B autrement. Arrivé par Dropbox : pas dans l'historique.
    let mut marc = base(&id);
    push(&mut marc, "library", drawing("m-disamare", "Disamare"));
    push(&mut marc, "placements", placed("pm1", "m-disamare", 600.0));
    marc["placements"][1]["center"]["x"] = json!(250.0);
    marc["projectors"][0]["label"] = json!("B mur");
    let text = serde_json::to_string_pretty(&marc).unwrap();
    fs::write(dir.project_file(), text).unwrap();
    (temp, dir)
}

fn count(list: &[SectionCount], section: &str) -> usize {
    list.iter()
        .find(|c| c.section == section)
        .map_or(0, |c| c.count)
}

#[test]
fn conflict_copies_are_found_and_sorted_by_kind() {
    let (_temp, dir) = setup("conflit-trouve");
    fs::write(
        dir.root()
            .join("verrou (Copie en conflit de Marc 2026-11-15).json"),
        "{}",
    )
    .unwrap();
    let photos = dir.root().join("photos-redressees");
    fs::write(
        photos.join("h (Copie en conflit de Marc 2026-11-15).png"),
        b"x",
    )
    .unwrap();
    fs::write(photos.join("h.png"), b"x").unwrap();

    let scan = find_conflicts(&dir).expect("recherche");
    assert_eq!(scan.projects.len(), 1);
    assert_eq!(scan.projects[0].file, COPY);
    assert_eq!(
        scan.others,
        vec!["photos-redressees/h (Copie en conflit de Marc 2026-11-15).png".to_string()]
    );
    assert!(is_conflict_copy_name(
        "projet (Marc's conflicted copy 2026-11-15).json"
    ));
    assert!(!is_conflict_copy_name("projet.json"));
}

#[test]
fn the_comparison_keeps_additions_and_asks_only_for_what_changed_on_both_sides() {
    let (_temp, dir) = setup("conflit-compare");
    let report = compare(&dir, COPY).expect("comparaison");

    // Disamare, importé des deux côtés, ne compte qu'une fois.
    assert_eq!(count(&report.only_copy, "Bibliothèque"), 1);
    assert_eq!(count(&report.only_copy, "Dessins posés"), 2);
    assert_eq!(count(&report.only_current, "Dessins posés"), 1);
    assert_eq!(count(&report.only_current, "Bibliothèque"), 0);

    let find = |key: &str| report.differences.iter().find(|d| d.key == key).cloned();
    assert_eq!(report.differences.len(), 3, "{:?}", report.differences);
    let p1 = find("placements/p1").expect("p1");
    assert_eq!(
        (p1.label.as_str(), p1.detail.as_str()),
        ("Algue", "position")
    );
    // Benjamin a déplacé p1 : l'historique a connu la valeur de Marc avant.
    assert_eq!(p1.suggested, Some(Side::Copy));
    // Marc a déplacé p2 : l'historique a connu la valeur de la copie.
    assert_eq!(
        find("placements/p2").unwrap().suggested,
        Some(Side::Current)
    );
    // Renommé des deux côtés : à choisir.
    let b = find("projectors/pb").expect("B");
    assert_eq!(
        (b.label.as_str(), b.detail.as_str()),
        ("Projecteur B mur", "nom")
    );
    assert_eq!(b.suggested, None);
}

#[test]
fn resolving_merges_both_sides_files_the_copy_away_and_can_be_undone() {
    let (_temp, dir) = setup("conflit-reunion");
    let mut choices = BTreeMap::new();
    choices.insert("projectors/pb".to_string(), Side::Copy);
    let project = resolve(&dir, COPY, &choices).expect("réunion");

    let names: Vec<&str> = project.library.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, vec!["Algue", "Diodon", "Disamare", "Daurade"]);
    let x = |id: &str| {
        project
            .placements
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .center
            .x
    };
    assert_eq!(x("p1"), 150.0, "déplacé par Benjamin");
    assert_eq!(x("p2"), 250.0, "déplacé par Marc");
    let ids: Vec<&str> = project.placements.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec!["p1", "p2", "pb1", "pb2", "pm1"]);
    // Le Disamare posé par Benjamin pointe vers l'unique Disamare.
    let pb1 = project.placements.iter().find(|p| p.id == "pb1").unwrap();
    assert_eq!(pb1.drawing, "m-disamare");
    assert_eq!(project.projectors[0].label, "B avant-toit");

    // Enregistré, la copie rangée et plus signalée.
    assert_eq!(dir.load().unwrap().project, project);
    assert!(!dir.root().join(COPY).exists());
    assert!(dir.root().join(RESOLVED_DIR).join(COPY).is_file());
    assert!(find_conflicts(&dir).unwrap().projects.is_empty());

    // ⌘Z remet la version de Marc.
    let (back, _) = history::step(&dir, false).expect("annulation");
    assert_eq!(back.placements.len(), 3);
    assert_eq!(back.projectors[0].label, "B mur");
}

#[test]
fn without_choices_the_current_version_wins_where_nothing_is_known() {
    let (_temp, dir) = setup("conflit-defaut");
    let project = resolve(&dir, COPY, &BTreeMap::new()).expect("réunion");
    assert_eq!(project.projectors[0].label, "B mur");
    assert_eq!(project.placements.len(), 5);
}

#[test]
fn a_copy_of_another_project_is_refused() {
    let (_temp, dir) = setup("conflit-autre");
    let other = base("autreprojet");
    fs::write(
        dir.root().join(COPY),
        serde_json::to_string(&other).unwrap(),
    )
    .unwrap();
    assert!(compare(&dir, COPY).is_err());
    assert!(resolve(&dir, COPY, &BTreeMap::new()).is_err());
    assert!(
        dir.root().join(COPY).is_file(),
        "la copie n'est pas touchée"
    );
    assert!(compare(&dir, "../projet.json").is_err());
}

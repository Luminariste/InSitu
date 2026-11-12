use super::*;
use crate::project::test_support::TempDir;
use crate::project::{Drawing, RelativePath};

fn project_with_drawing(label: &str) -> (TempDir, ProjectDir) {
    let temp = TempDir::new(label);
    let (dir, mut project) =
        ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
    project.library.push(Drawing {
        id: "merle".to_string(),
        name: "Merle".to_string(),
        source: RelativePath::new("bibliotheque/merle.png").unwrap(),
        width: 2_000.0,
        height: 1_000.0,
        extra: Default::default(),
    });
    dir.save(&project).expect("enregistrement");
    (temp, dir)
}

fn close(a: PointMm, x: f64, y: f64) -> bool {
    (a.x - x).abs() < 1e-9 && (a.y - y).abs() < 1e-9
}

#[test]
fn corners_follow_size_rotation_and_mirror() {
    let mut p = Placement {
        id: "p".into(),
        drawing: "merle".into(),
        center: PointMm::new(1_000.0, 500.0),
        width: 200.0,
        height: 100.0,
        rotation: 0.0,
        mirrored: false,
        group: None,
        extra: Default::default(),
    };
    let c = p.corners();
    assert!(close(c[0], 900.0, 450.0) && close(c[2], 1_100.0, 550.0));
    // Un quart de tour dans le sens des aiguilles d'une montre (y vers le
    // bas) : le coin haut-gauche part en haut à droite.
    p.rotation = 90.0;
    let c = p.corners();
    assert!(close(c[0], 1_050.0, 400.0), "{:?}", c[0]);
    let b = p.bounds();
    assert!((b.width - 100.0).abs() < 1e-9 && (b.height - 200.0).abs() < 1e-9);
    // Retourné : le coin haut-gauche du fichier passe à droite.
    p.rotation = 0.0;
    p.mirrored = true;
    assert!(close(p.corners()[0], 1_100.0, 450.0));
}

#[test]
fn placements_are_saved_and_read_back() {
    let (_temp, dir) = project_with_drawing("pose-aller-retour");
    let project = dir.load().unwrap().project;
    let placement =
        new_placement(&project, "Merle", PointMm::new(3_000.0, 2_000.0), 800.0).unwrap();
    assert_eq!(placement.drawing, "merle");
    assert!(
        (placement.height - 400.0).abs() < 1e-9,
        "proportions du fichier"
    );
    let (_, id) = add(&dir, placement).unwrap();

    let mut placements = dir.load().unwrap().project.placements;
    assert_eq!(placements.len(), 1);
    let mut copy = placements[0].clone();
    copy.id = String::new();
    copy.rotation = 30.0;
    copy.mirrored = true;
    copy.group = Some("g".into());
    placements[0].group = Some("g".into());
    placements.push(copy);
    let saved = set_placements(&dir, placements).unwrap();
    assert_eq!(saved.placements.len(), 2);
    assert!(!saved.placements[1].id.is_empty(), "identifiant attribué");
    assert_eq!(dir.load().unwrap().project, saved);

    let left = remove(&dir, &[id]).unwrap();
    assert_eq!(left.placements.len(), 1);
    assert_eq!(left.placements[0].rotation, 30.0);
}

#[test]
fn wrong_placements_are_refused_and_nothing_is_saved() {
    let (_temp, dir) = project_with_drawing("pose-refus");
    let project = dir.load().unwrap().project;
    let good = new_placement(&project, "merle", PointMm::new(0.0, 0.0), 100.0).unwrap();
    add(&dir, good.clone()).unwrap();

    let mut unknown = good.clone();
    unknown.drawing = "pie".into();
    unknown.id = "autre".into();
    let error = set_placements(&dir, vec![good.clone(), unknown]).unwrap_err();
    assert!(
        error.to_string().contains("aucun dessin « pie »"),
        "{error}"
    );

    let twice = set_placements(&dir, vec![good.clone(), good.clone()]).unwrap_err();
    assert!(twice.to_string().contains("déjà utilisé"), "{twice}");

    let mut flat = good.clone();
    flat.height = 0.0;
    assert!(set_placements(&dir, vec![flat]).is_err());

    let mut infinite = good.clone();
    infinite.rotation = f64::NAN;
    assert!(matches!(
        set_placements(&dir, vec![infinite]),
        Err(PlacementError::NotFinite(_))
    ));

    assert!(matches!(
        new_placement(&project, "pie", PointMm::new(0.0, 0.0), 1.0),
        Err(PlacementError::UnknownDrawing(_))
    ));
    assert!(remove(&dir, &["rien".into()]).is_err());
    assert_eq!(dir.load().unwrap().project.placements, vec![good]);
}

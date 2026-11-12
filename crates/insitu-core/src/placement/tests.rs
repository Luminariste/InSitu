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
        vector: None,
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
        layer: None,
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

#[test]
fn layers_order_the_stack_and_are_checked() {
    let (_temp, dir) = project_with_drawing("pose-calques");
    let project = dir.load().unwrap().project;
    let mut front = new_placement(&project, "merle", PointMm::new(0.0, 0.0), 10.0).unwrap();
    front.layer = Some("devant".into());
    let back = new_placement(&project, "merle", PointMm::new(0.0, 0.0), 20.0).unwrap();
    let layers = vec![
        Layer {
            id: String::new(),
            name: "Fond".into(),
            visible: true,
            locked: true,
            extra: Default::default(),
        },
        Layer {
            id: "devant".into(),
            name: "Devant".into(),
            visible: false,
            locked: false,
            extra: Default::default(),
        },
    ];
    let saved = set_composition(&dir, layers.clone(), vec![front.clone(), back.clone()]).unwrap();
    assert!(!saved.layers[0].id.is_empty());
    let reread = dir.load().unwrap().project;
    assert_eq!(reread, saved);
    assert!(!reread.layers[1].visible && reread.layers[0].locked);
    // Le dessin sans calque est sur le premier (le fond) : il passe derrière.
    let stack: Vec<f64> = reread
        .stacked_placements()
        .iter()
        .map(|p| p.width)
        .collect();
    assert_eq!(stack, vec![20.0, 10.0]);

    let mut lost = front.clone();
    lost.layer = Some("nulle-part".into());
    let error = set_composition(&dir, layers, vec![lost]).unwrap_err();
    assert!(error.to_string().contains("aucun calque"), "{error}");
}

#[test]
fn drawing_point_undoes_facade_point() {
    let mut p = Placement {
        id: "p".into(),
        drawing: "merle".into(),
        center: PointMm::new(1_000.0, 500.0),
        width: 200.0,
        height: 100.0,
        rotation: 37.0,
        mirrored: true,
        group: None,
        layer: None,
        extra: Default::default(),
    };
    for mirrored in [false, true] {
        p.mirrored = mirrored;
        for (u, v) in [(0.0, 0.0), (1.0, 0.0), (0.25, 0.75), (1.3, -0.2)] {
            let (a, b) = p.drawing_point(p.facade_point(u, v));
            assert!(
                (a - u).abs() < 1e-12 && (b - v).abs() < 1e-12,
                "{u},{v} → {a},{b}"
            );
        }
    }
}

#[test]
fn hidden_layers_are_not_exported() {
    let placed = |id: &str, layer: Option<&str>| Placement {
        id: id.into(),
        drawing: "merle".into(),
        center: PointMm::new(0.0, 0.0),
        width: 10.0,
        height: 10.0,
        rotation: 0.0,
        mirrored: false,
        group: None,
        layer: layer.map(str::to_string),
        extra: Default::default(),
    };
    let layer = |id: &str, visible: bool| Layer {
        id: id.into(),
        name: id.into(),
        visible,
        locked: false,
        extra: Default::default(),
    };
    let mut project = Project::new("Essai");
    project.placements = vec![
        placed("a", Some("devant")),
        placed("b", None),
        placed("c", Some("fond")),
    ];
    let ids = |p: &Project| {
        p.visible_placements()
            .iter()
            .map(|p| p.id.clone())
            .collect::<Vec<_>>()
    };
    // Sans calque : tout, dans l'ordre de la liste.
    assert_eq!(ids(&project), ["a", "b", "c"]);
    project.layers = vec![layer("fond", true), layer("devant", true)];
    assert_eq!(ids(&project), ["b", "c", "a"]);
    // Le premier calque masqué emporte les dessins sans calque.
    project.layers[0].visible = false;
    assert_eq!(ids(&project), ["a"]);
    project.layers[0].visible = true;
    project.layers[1].visible = false;
    assert_eq!(ids(&project), ["b", "c"]);
}

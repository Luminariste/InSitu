use super::*;
use crate::project::test_support::TempDir;
use crate::project::{Deformation, Drawing, RelativePath};

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
        deformation: None,
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
        deformation: None,
        extra: Default::default(),
    };
    for mirrored in [false, true] {
        p.mirrored = mirrored;
        for (u, v) in [(0.0, 0.0), (1.0, 0.0), (0.25, 0.75), (1.3, -0.2)] {
            let (a, b) = p.drawing_point(p.facade_point(u, v)).unwrap();
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
        deformation: None,
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

// --- Déformation libre (tâche 4.4) ------------------------------------------

fn straight(rotation: f64, mirrored: bool) -> Placement {
    Placement {
        id: "p".into(),
        drawing: "merle".into(),
        center: PointMm::new(1_000.0, 500.0),
        width: 400.0,
        height: 200.0,
        rotation,
        mirrored,
        group: None,
        layer: None,
        deformation: None,
        extra: Default::default(),
    }
}

/// Une voussure : la grille de 6 × 1 cases du dessin, ses deux rangs de
/// nœuds posés sur deux arcs de cercle concentriques (centre sous le
/// dessin), comme un cintre de porte.
fn arch(p: &Placement) -> Deformation {
    let frame = deformation_frame(&[p]).unwrap();
    let mut d = mesh::identity(frame, 6, 1);
    // En fractions du cadre : centre de l'arc en (0,5 ; 1,5), rayons 1,5 et
    // 0,5 hauteur de cadre plus bas ; on reste en fractions, le cadre fait
    // 400 × 200 mm, donc 1 en s vaut 2 en t.
    for j in 0..=1 {
        let radius_mm = if j == 0 { 300.0 } else { 100.0 };
        for i in 0..=6 {
            let angle = std::f64::consts::PI * (1.0 - i as f64 / 6.0);
            let x_mm = radius_mm * angle.cos();
            let y_mm = -radius_mm * angle.sin();
            d.nodes[j * 7 + i] = [0.5 + x_mm / 400.0, 1.0 + y_mm / 200.0 + 0.5];
        }
    }
    d
}

#[test]
fn a_mesh_at_rest_changes_nothing() {
    for (rotation, mirrored) in [(0.0, false), (30.0, true)] {
        let mut p = straight(rotation, mirrored);
        let rigid = p.clone();
        for (c, r) in [(1, 1), (3, 2), (5, 5)] {
            p.deformation = Some(mesh::identity(deformation_frame(&[&rigid]).unwrap(), c, r));
            assert!(mesh::check(p.deformation.as_ref().unwrap()).is_none());
            for (u, v) in [(0.0, 0.0), (0.3, 0.9), (1.0, 1.0)] {
                let (a, b) = (p.facade_point(u, v), rigid.facade_point(u, v));
                assert!(close(a, b.x, b.y), "{c}×{r} {u},{v} : {a:?} ≠ {b:?}");
            }
        }
    }
}

#[test]
fn four_corners_keep_straight_lines_and_invert_exactly() {
    let mut p = straight(15.0, false);
    let mut d = mesh::identity(deformation_frame(&[&p]).unwrap(), 1, 1);
    d.nodes[1] = [0.9, 0.2]; // coin haut-droit rentré
    d.nodes[3] = [1.1, 1.05];
    p.deformation = Some(d);
    assert!(mesh::check(p.deformation.as_ref().unwrap()).is_none());
    // Le coin haut-droit du dessin va où on l'a mis.
    let corner = p.deformation.as_ref().unwrap().frame.facade_point(0.9, 0.2);
    assert!(close(p.facade_point(1.0, 0.0), corner.x, corner.y));
    // Perspective : trois points d'une même ligne du dessin restent alignés.
    let (a, b, c) = (
        p.facade_point(0.0, 0.5),
        p.facade_point(0.5, 0.5),
        p.facade_point(1.0, 0.5),
    );
    let cross = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    assert!(cross.abs() < 1e-6, "{cross}");
    for (u, v) in [(0.1, 0.1), (0.5, 0.5), (0.95, 0.7)] {
        let (a, b) = p.drawing_point(p.facade_point(u, v)).unwrap();
        assert!((a - u).abs() < 1e-9 && (b - v).abs() < 1e-9);
    }
}

#[test]
fn a_drawing_follows_an_arch() {
    let mut p = straight(0.0, false);
    let d = arch(&p);
    p.deformation = Some(d.clone());
    assert!(mesh::check(&d).is_none(), "{:?}", mesh::check(&d));
    let centre = PointMm::new(1_000.0, 700.0);
    // Le haut du dessin longe l'arc extérieur (rayon 300 mm), le bas l'arc
    // intérieur (100 mm), à 1 % près entre les nœuds.
    for k in 0..=40 {
        let u = k as f64 / 40.0;
        for (v, radius) in [(0.0, 300.0), (1.0, 100.0)] {
            let q = p.facade_point(u, v);
            let r = ((q.x - centre.x).powi(2) + (q.y - centre.y).powi(2)).sqrt();
            assert!((r - radius).abs() < 0.01 * radius, "u = {u}, v = {v} : {r}");
        }
    }
    // Aux nœuds, exactement.
    let top_left = p.facade_point(0.0, 0.0);
    assert!(close(top_left, 700.0, 700.0), "{top_left:?}");
    // L'aller-retour façade → dessin retrouve le point.
    for (u, v) in [(0.05, 0.1), (0.5, 0.5), (0.8, 0.95)] {
        let (a, b) = p.drawing_point(p.facade_point(u, v)).unwrap();
        assert!(
            (a - u).abs() < 1e-9 && (b - v).abs() < 1e-9,
            "{u},{v} → {a},{b}"
        );
    }
}

#[test]
fn folded_or_malformed_meshes_are_refused() {
    let p = straight(0.0, false);
    let frame = deformation_frame(&[&p]).unwrap();
    let mut d = mesh::identity(frame.clone(), 2, 2);
    d.nodes[4] = [1.4, 0.5]; // le nœud central passe au-delà du bord droit
    assert!(mesh::check(&d).unwrap().contains("replie"));
    let mut d = mesh::identity(frame.clone(), 1, 1);
    d.nodes.swap(0, 1); // coins croisés
    assert!(mesh::check(&d).is_some());
    let mut d = mesh::identity(frame.clone(), 2, 2);
    d.nodes.pop();
    assert!(mesh::check(&d).unwrap().contains("nœuds"));
    assert!(mesh::check(&mesh::identity(frame, 17, 1)).is_some());
}

#[test]
fn a_group_is_deformed_as_one_and_saved() {
    let (_temp, dir) = project_with_drawing("deformation-groupe");
    let (_, a) = add(
        &dir,
        new_placement(
            &dir.load().unwrap().project,
            "merle",
            PointMm::new(100.0, 100.0),
            200.0,
        )
        .unwrap(),
    )
    .unwrap();
    let (_, b) = add(
        &dir,
        new_placement(
            &dir.load().unwrap().project,
            "merle",
            PointMm::new(500.0, 300.0),
            200.0,
        )
        .unwrap(),
    )
    .unwrap();
    let project = deform(&dir, &[a.clone(), b.clone()], 3, 2).unwrap();
    let (pa, pb) = (&project.placements[0], &project.placements[1]);
    let d = pa.deformation.as_ref().unwrap();
    assert_eq!(Some(d), pb.deformation.as_ref());
    // Le cadre commun contient les deux dessins : de (0 ; 50) à (600 ; 350).
    assert!(close(d.frame.center, 300.0, 200.0), "{:?}", d.frame.center);
    assert!((d.frame.width - 600.0).abs() < 1e-9 && (d.frame.height - 300.0).abs() < 1e-9);
    // Une grille repliée est refusée, rien n'est enregistré.
    let mut bad = project.placements.clone();
    bad[0].deformation.as_mut().unwrap().nodes[5] = [2.0, 0.5];
    assert!(matches!(
        set_placements(&dir, bad),
        Err(PlacementError::Invalid(_))
    ));
    // Retirer la déformation.
    let project = deform(&dir, &[a], 0, 0).unwrap();
    assert!(project.placements[0].deformation.is_none());
    assert!(project.placements[1].deformation.is_some());
}

/// Cas de référence partagés avec l'interface (`src/compose/deformation-cases.json`) :
/// l'aperçu de la façade (TypeScript) doit placer chaque point du dessin
/// exactement où l'export (ce moteur) le place. Pour refaire le fichier
/// après un changement voulu : `INSITU_REFAIRE_CAS=1 cargo test`.
#[test]
fn preview_reference_cases_match_the_engine() {
    let mut corners = straight(25.0, true);
    let mut d = mesh::identity(deformation_frame(&[&corners]).unwrap(), 1, 1);
    d.nodes[0] = [0.1, -0.05];
    d.nodes[3] = [1.05, 0.9];
    corners.deformation = Some(d);
    let mut vault = straight(0.0, false);
    vault.deformation = Some(arch(&vault));
    let mut grid = straight(-40.0, false);
    let mut d = mesh::identity(
        DeformationFrame {
            center: PointMm::new(950.0, 520.0),
            width: 700.0,
            height: 450.0,
            rotation: 10.0,
        },
        3,
        2,
    );
    d.nodes[5] = [0.4, 0.62];
    d.nodes[0] = [-0.05, 0.02];
    grid.deformation = Some(d);

    let cases: Vec<serde_json::Value> = [corners, vault, grid]
        .iter()
        .map(|p| {
            let samples: Vec<serde_json::Value> = (0..=4)
                .flat_map(|j| (0..=4).map(move |i| (i as f64 / 4.0, j as f64 / 4.0)))
                .map(|(u, v)| {
                    let q = p.facade_point(u, v);
                    serde_json::json!({ "u": u, "v": v, "x": q.x, "y": q.y })
                })
                .collect();
            serde_json::json!({ "placement": p, "samples": samples })
        })
        .collect();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../src/compose/deformation-cases.json");
    let text = serde_json::to_string_pretty(&serde_json::json!({ "cases": cases })).unwrap() + "\n";
    if std::env::var_os("INSITU_REFAIRE_CAS").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("fichier des cas")).unwrap();
    let fresh: serde_json::Value = serde_json::from_str(&text).unwrap();
    for (a, b) in stored["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(fresh["cases"].as_array().unwrap())
    {
        assert_eq!(
            a["placement"], b["placement"],
            "cas refaits : INSITU_REFAIRE_CAS=1"
        );
        for (s, t) in a["samples"]
            .as_array()
            .unwrap()
            .iter()
            .zip(b["samples"].as_array().unwrap())
        {
            for k in ["x", "y"] {
                let (x, y) = (s[k].as_f64().unwrap(), t[k].as_f64().unwrap());
                assert!((x - y).abs() < 1e-9, "{s} ≠ {t}");
            }
        }
    }
}

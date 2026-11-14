use super::*;
use crate::project::test_support::TempDir;
use crate::project::{Drawing, Layer, RegistrationPair, RelativePath};

/// Une diapositive posée sur la façade, agrandie 10 fois, coin haut-gauche
/// en (`x`, 0) : elle couvre `x` à `x` + 2 800 mm.
fn pairs(x: f64) -> Vec<RegistrationPair> {
    [(0.0, 0.0), (280.0, 0.0), (280.0, 280.0), (0.0, 280.0)]
        .iter()
        .map(|&(sx, sy)| {
            RegistrationPair::new(PointMm::new(sx, sy), PointMm::new(x + sx * 10.0, sy * 10.0))
        })
        .collect()
}

/// Deux projecteurs, A (0 à 2 800 mm) et B (2 000 à 4 800 mm) : ils se
/// recouvrent de 2 000 à 2 800 mm.
fn two_projectors() -> Project {
    let mut project = Project::new("Essai");
    project.add_projector("A").registration = pairs(0.0);
    project.add_projector("B").registration = pairs(2_000.0);
    project.library.push(Drawing {
        id: "d".into(),
        name: "étoile".into(),
        source: RelativePath::new("bibliotheque/etoile.png").unwrap(),
        width: 100.0,
        height: 100.0,
        vector: None,
        silhouette: None,
        retouch: Vec::new(),
        folder: None,
        extra: Default::default(),
    });
    project
}

fn ids(project: &Project) -> (String, String) {
    (
        project.projectors[0].id.clone(),
        project.projectors[1].id.clone(),
    )
}

fn place(project: &mut Project, id: &str, x: f64, y: f64, width: f64) {
    project.placements.push(Placement {
        id: id.into(),
        drawing: "d".into(),
        center: PointMm::new(x, y),
        width,
        height: width,
        rotation: 0.0,
        mirrored: false,
        group: None,
        layer: None,
        deformation: None,
        projector: None,
        adjustment: None,
        slide: None,
        hidden: false,
        locked: false,
        extra: Default::default(),
    });
}

fn contains(r: &RectMm, p: PointMm) -> bool {
    p.x >= r.x && p.y >= r.y && p.x <= r.x + r.width && p.y <= r.y + r.height
}

/// Critère de la tâche 6.1 : chaque alerte mène au dessin concerné. Un
/// dessin coupé, un dessin hors emprise et un choix à la main qui coupe
/// sont signalés ; chaque alerte nomme le placement et son rectangle
/// contient le dessin. Un dessin bien rangé ou masqué ne l'est pas.
#[test]
fn each_drawing_alert_leads_to_its_drawing() {
    let mut project = two_projectors();
    let (_, b) = ids(&project);
    place(&mut project, "bien", 1_000.0, 1_400.0, 600.0);
    // À cheval sur les deux, plus large que le recouvrement : coupé.
    place(&mut project, "coupe", 2_400.0, 1_400.0, 1_200.0);
    place(&mut project, "dehors", 6_000.0, 1_400.0, 500.0);
    // Tient dans A seulement, mis à la main dans B : coupé.
    place(&mut project, "main", 1_000.0, 600.0, 400.0);
    project.placements.last_mut().unwrap().projector = Some(b.clone());
    // Hors emprise, mais sur un calque masqué : il ne part pas, pas d'alerte.
    project.layers.push(Layer {
        id: "fond".into(),
        name: "Fond".into(),
        visible: true,
        locked: false,
        extra: Default::default(),
    });
    project.layers.push(Layer {
        id: "cache".into(),
        name: "Caché".into(),
        visible: false,
        locked: false,
        extra: Default::default(),
    });
    place(&mut project, "masque", 7_000.0, 1_400.0, 500.0);
    project.placements.last_mut().unwrap().layer = Some("cache".into());

    let found = alerts(&project, &HashMap::new());
    let kinds: Vec<(&str, AlertKind)> = found
        .iter()
        .map(|x| (x.placement.as_deref().unwrap(), x.kind))
        .collect();
    assert_eq!(
        kinds,
        [
            ("coupe", AlertKind::Cut),
            ("dehors", AlertKind::Outside),
            ("main", AlertKind::Cut),
        ]
    );
    for alert in &found {
        let p = project
            .placements
            .iter()
            .find(|p| Some(p.id.as_str()) == alert.placement.as_deref())
            .unwrap();
        for corner in p.corners() {
            assert!(contains(&alert.bounds, corner), "{alert:?}");
        }
        assert!(alert.bounds.width <= p.width + 1e-6, "{alert:?}");
        assert!(alert.message.contains("« étoile »"), "{}", alert.message);
    }
    assert_eq!(found[0].projectors.len(), 1);
    assert!(found[0].message.contains("aucune diapositive"));
    assert!(found[1].projectors.is_empty());
    assert_eq!(found[2].projectors, [b]);
    assert!(
        found[2].message.contains("A le contient en entier"),
        "{}",
        found[2].message
    );
}

/// Un aperçu de diapositive noir, avec des traits verticaux allumés aux
/// abscisses données (mm de diapositive), de `y0` à `y1` mm.
fn slide_with_lines(xs: &[f64], y0: f64, y1: f64) -> LitSlide {
    let side = 1_024u32;
    let px = SLIDE_SIZE_MM / f64::from(side);
    let mut data = vec![0u8; (side * side) as usize];
    for &x in xs {
        let i = (x / px) as usize;
        for j in (y0 / px) as usize..(y1 / px) as usize {
            for k in i..i + 2 {
                data[j * side as usize + k] = 255;
            }
        }
    }
    LitSlide { side, data }
}

/// Un trait de la façade allumé dans les deux diapositives du recouvrement
/// donne une alerte, là où il est ; un trait d'une seule diapositive, même
/// dans le recouvrement, n'en donne pas.
#[test]
fn light_projected_twice_in_an_overlap_is_found_where_it_is() {
    let mut project = two_projectors();
    let (a, b) = ids(&project);
    project.fresco = Some(crate::project::FrescoImage {
        source: RelativePath::new("fresque/f.png").unwrap(),
        frame: RectMm {
            x: 0.0,
            y: 0.0,
            width: 4_800.0,
            height: 2_800.0,
        },
        vector: None,
        extra: Default::default(),
    });
    // Façade x = 2 400 mm : 240 mm dans A, 40 mm dans B, de y = 500 à
    // 1 500 mm. Façade x = 2 200 : dans A seulement. Façade x = 1 000 :
    // hors du recouvrement.
    let lit = HashMap::from([
        (
            a.clone(),
            slide_with_lines(&[100.0, 220.0, 240.0], 50.0, 150.0),
        ),
        (b.clone(), slide_with_lines(&[40.0], 50.0, 150.0)),
    ]);
    let found = alerts(&project, &lit);
    assert_eq!(found.len(), 1, "{found:?}");
    let alert = &found[0];
    assert_eq!(alert.kind, AlertKind::Doubled);
    assert_eq!(alert.projectors, [a.clone(), b.clone()]);
    assert_eq!(alert.placement, None);
    assert!(alert.message.contains("la fresque"), "{}", alert.message);
    let r = &alert.bounds;
    assert!((r.x - 2_400.0).abs() < 10.0 && r.width < 10.0, "{r:?}");
    assert!(
        (r.y - 500.0).abs() < 10.0 && (r.y + r.height - 1_500.0).abs() < 10.0,
        "{r:?}"
    );

    // Un dessin posé sur ce trait, affecté à A : l'alerte mène à lui.
    place(&mut project, "trait", 2_400.0, 1_000.0, 300.0);
    let found = alerts(&project, &lit);
    let doubled: Vec<_> = found
        .iter()
        .filter(|x| x.kind == AlertKind::Doubled)
        .collect();
    assert_eq!(doubled.len(), 1);
    assert_eq!(doubled[0].placement.as_deref(), Some("trait"));
    assert!(doubled[0].message.contains("« étoile »"));
}

/// De bout en bout : une fresque importée sur toute la façade est découpée
/// entre les deux diapositives, donc projetée deux fois dans le
/// recouvrement, et seulement là.
#[test]
fn an_imported_fresco_is_projected_twice_in_the_overlap() {
    let temp = TempDir::new("simulation");
    let (dir, mut project) =
        ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).unwrap();
    let fresh = two_projectors();
    project.projectors = fresh.projectors;
    dir.save(&project).unwrap();
    // Fresque de 480 × 280 px sur 4 800 × 2 800 mm : un trait vertical
    // tous les 400 mm.
    let (w, h) = (480u32, 280u32);
    let mut data = vec![0u8; (w * h) as usize];
    for y in 0..h {
        for x in (20..w).step_by(40) {
            for k in x..x + 3 {
                data[(y * w + k) as usize] = 255;
            }
        }
    }
    let source = temp.path().join("fresque.png");
    image::save_buffer(&source, &data, w, h, image::ExtendedColorType::L8).unwrap();
    crate::export::import_fresco(
        &dir,
        &source,
        Some(RectMm {
            x: 0.0,
            y: 0.0,
            width: 4_800.0,
            height: 2_800.0,
        }),
    )
    .unwrap();
    let cache = temp.path().join("cache");
    let simulation = simulate(&dir, &cache).unwrap();
    assert_eq!(simulation.slides.len(), 2);
    assert!(simulation.slides.iter().all(|s| s.preview.is_some()));
    let doubled: Vec<_> = simulation
        .alerts
        .iter()
        .filter(|x| x.kind == AlertKind::Doubled)
        .collect();
    assert!(!doubled.is_empty());
    for alert in &doubled {
        let r = &alert.bounds;
        assert!(r.x >= 1_990.0 && r.x + r.width <= 2_810.0, "{r:?}");
    }
    // Les deux traits du recouvrement (x = 2 200 à 2 230 et 2 600 à 2 630 mm) sont trouvés.
    for x in [2_215.0, 2_615.0] {
        assert!(
            doubled
                .iter()
                .any(|a| a.bounds.x - 20.0 <= x && x <= a.bounds.x + a.bounds.width + 20.0),
            "trait en {x} : {doubled:?}"
        );
    }
}

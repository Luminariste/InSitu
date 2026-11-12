use super::*;
use crate::export::{export_slides, to_gray};
use crate::project::test_support::TempDir;
use crate::project::{Drawing, ExportSettings, Placement, PointMm, RegistrationPair, RelativePath};

fn stroke(tool: RetouchTool, size: f64, points: &[[f64; 2]]) -> RetouchStroke {
    RetouchStroke {
        tool,
        fill: false,
        size,
        points: points.to_vec(),
    }
}

fn zone(tool: RetouchTool, points: &[[f64; 2]]) -> RetouchStroke {
    RetouchStroke {
        tool,
        fill: true,
        size: 0.0,
        points: points.to_vec(),
    }
}

/// Un trait blanc horizontal de 5 px d'épaisseur au milieu d'une image
/// noire de `side` px.
fn line_image(side: u32) -> Raster {
    let mut data = vec![0u8; (side * side) as usize];
    let thickness = side / 20;
    for y in side / 2 - thickness / 2..side / 2 + thickness / 2 + 1 {
        for x in 0..side {
            data[(y * side + x) as usize] = 255;
        }
    }
    Raster::new(side, side, 1, data)
}

fn at(raster: &Raster, x: u32, y: u32) -> u8 {
    raster.data[(y * raster.width + x) as usize]
}

#[test]
fn the_eraser_removes_ink_and_the_pencil_adds_some() {
    let mut image = line_image(100);
    assert_eq!(at(&image, 50, 50), 255);
    paint(
        &mut image,
        &[
            // Gomme verticale de 10 px au milieu.
            stroke(RetouchTool::Erase, 0.1, &[[0.5, 0.1], [0.5, 0.9]]),
            // Crayon horizontal de 4 px en haut.
            stroke(RetouchTool::Draw, 0.04, &[[0.1, 0.2], [0.9, 0.2]]),
            // Zone remplie au crayon, en bas à droite.
            zone(
                RetouchTool::Draw,
                &[[0.7, 0.7], [0.9, 0.7], [0.9, 0.9], [0.7, 0.9]],
            ),
        ],
    );
    // Le trait d'origine est effacé sous la gomme, intact ailleurs.
    assert_eq!(at(&image, 50, 50), 0);
    assert_eq!(at(&image, 47, 50), 0);
    assert_eq!(at(&image, 20, 50), 255);
    assert_eq!(at(&image, 80, 50), 255);
    // Le trait au crayon et la zone sont blancs ; le fond autour reste noir.
    assert_eq!(at(&image, 30, 20), 255);
    assert_eq!(at(&image, 30, 25), 0);
    assert_eq!(at(&image, 80, 80), 255);
    assert_eq!(at(&image, 75, 65), 0);
    // La zone commence pile à x = 70 : le pixel 70 est entier, le 69 libre.
    assert_eq!(at(&image, 70, 80), 255);
    assert_eq!(at(&image, 69, 80), 0);
    // Une gomme en zone efface tout ce qu'elle couvre.
    paint(
        &mut image,
        &[zone(
            RetouchTool::Erase,
            &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        )],
    );
    assert!(image.data.iter().all(|&v| v == 0));
}

#[test]
fn a_zone_edge_is_antialiased() {
    let mut image = Raster::new(10, 10, 1, vec![0; 100]);
    // Bord gauche à x = 2,5 : le pixel 2 est couvert à moitié.
    paint(
        &mut image,
        &[zone(
            RetouchTool::Draw,
            &[[0.25, 0.0], [1.0, 0.0], [1.0, 1.0], [0.25, 1.0]],
        )],
    );
    assert_eq!(at(&image, 1, 5), 0);
    assert!(
        (at(&image, 2, 5) as i32 - 128).abs() <= 1,
        "{}",
        at(&image, 2, 5)
    );
    assert_eq!(at(&image, 3, 5), 255);
}

#[test]
fn retouches_follow_the_drawing_at_any_resolution() {
    let strokes = [
        stroke(
            RetouchTool::Erase,
            0.08,
            &[[0.2, 0.3], [0.6, 0.7], [0.8, 0.4]],
        ),
        stroke(RetouchTool::Draw, 0.02, &[[0.1, 0.9], [0.9, 0.1]]),
        zone(RetouchTool::Draw, &[[0.3, 0.1], [0.5, 0.15], [0.4, 0.35]]),
    ];
    let mut small = line_image(200);
    let mut large = line_image(400);
    paint(&mut small, &strokes);
    paint(&mut large, &strokes);
    let reduced = tiles::downsample(&large);
    assert_eq!((reduced.width, reduced.height), (200, 200));
    let differing = small
        .data
        .iter()
        .zip(&reduced.data)
        .filter(|(&a, &b)| a.abs_diff(b) > 96)
        .count();
    // Seuls des pixels du bord diffèrent franchement (moins de 1 %).
    assert!(differing < 400, "{differing} pixels diffèrent");
}

#[test]
fn bad_retouches_are_refused() {
    let problem = |s: RetouchStroke| crate::project::retouch_problem(&[s]);
    assert!(problem(stroke(RetouchTool::Draw, 0.0, &[[0.5, 0.5]])).is_some());
    assert!(problem(stroke(RetouchTool::Draw, 0.1, &[])).is_some());
    assert!(problem(stroke(RetouchTool::Draw, 0.1, &[[f64::NAN, 0.5]])).is_some());
    assert!(problem(zone(RetouchTool::Erase, &[[0.1, 0.1], [0.2, 0.2]])).is_some());
    assert!(problem(stroke(RetouchTool::Draw, 0.1, &[[0.5, 0.5]])).is_none());
    // Lu depuis projet.json : les champs inconnus sont refusés, `fill` et
    // `size` ont une valeur par défaut.
    let read: RetouchStroke =
        serde_json::from_str(r#"{"tool":"erase","fill":true,"points":[[0,0],[1,0],[1,1]]}"#)
            .unwrap();
    assert!(read.fill && read.tool == RetouchTool::Erase);
    assert!(serde_json::from_str::<RetouchStroke>(r#"{"tool":"draw","points":[],"x":1}"#).is_err());
}

/// Projet avec un dessin (un trait horizontal de 400 px) dans la
/// bibliothèque.
fn project_with_drawing(label: &str) -> (TempDir, ProjectDir) {
    let temp = TempDir::new(label);
    let (dir, mut project) =
        ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).unwrap();
    let image = line_image(400);
    let file = dir.resolve(&RelativePath::new("bibliotheque/trait.png").unwrap());
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    image::save_buffer(&file, &image.data, 400, 400, image::ExtendedColorType::L8).unwrap();
    project.library.push(Drawing {
        id: "trait".into(),
        name: "trait".into(),
        source: RelativePath::new("bibliotheque/trait.png").unwrap(),
        width: 400.0,
        height: 400.0,
        vector: None,
        silhouette: None,
        retouch: Vec::new(),
        extra: Default::default(),
    });
    dir.save(&project).unwrap();
    (temp, dir)
}

#[test]
fn a_retouch_keeps_the_original_file_and_feeds_the_whole_library() {
    let (temp, dir) = project_with_drawing("retouche-bibliotheque");
    let cache = temp.path().join("cache");
    let original = dir.resolve(&RelativePath::new("bibliotheque/trait.png").unwrap());
    let original_bytes = fs::read(&original).unwrap();
    let before = super::super::list(&dir, Some(&cache), 1).unwrap().remove(0);

    // La gomme efface toute la moitié gauche.
    let erase_left = zone(
        RetouchTool::Erase,
        &[[-0.1, -0.1], [0.5, -0.1], [0.5, 1.1], [-0.1, 1.1]],
    );
    let (project, entry) =
        set_retouch(&dir, "trait", vec![erase_left.clone()], Some(&cache)).unwrap();
    assert_eq!(project.library[0].retouch, vec![erase_left.clone()]);
    assert_eq!(
        dir.load().unwrap().project.library[0].retouch,
        vec![erase_left]
    );
    // Le fichier importé n'a pas bougé ; le dessin retouché est à côté.
    assert_eq!(fs::read(&original).unwrap(), original_bytes);
    let retouched = retouched_file(&dir, &project.library[0]).unwrap();
    assert!(retouched.starts_with(dir.root().join(RETOUCH_DIR)));
    let pixels = to_gray(tiles::read_raster(&retouched).unwrap());
    assert_eq!(at(&pixels, 100, 200), 0, "effacé");
    assert_eq!(at(&pixels, 300, 200), 255, "gardé");
    // Vignette, aperçu, silhouette et vecteurs suivent le dessin retouché.
    assert_ne!(entry.thumbnail, before.thumbnail);
    assert!(entry.problem.is_none());
    let preview = super::super::preview(&dir, "trait", &cache).unwrap();
    let shown = tiles::read_raster(&cache.join(&preview)).unwrap();
    let alpha = |x: u32| shown.data[((200 * shown.width + x) * 4 + 3) as usize];
    assert_eq!((alpha(100), alpha(300)), (0, 255));
    let original_preview = super::super::original_preview(&dir, "trait", &cache).unwrap();
    assert_ne!(original_preview, preview);
    let silhouette = super::super::silhouette::load_silhouettes(&dir, &[], &cache)
        .unwrap()
        .remove(0);
    let left = silhouette.polygons[0]
        .iter()
        .map(|p| p[0])
        .fold(f64::INFINITY, f64::min);
    assert!(
        left > 0.45,
        "la silhouette commence au trait restant : {left}"
    );
    let vectors = super::super::vectors::drawing_vectors(&dir, "trait", &cache).unwrap();
    // Le trait de 400 × 21 px n'a plus que sa moitié droite.
    let ink = vectors.comparison.source_pixels;
    assert!((3_000..6_000).contains(&ink), "{ink} pixels de trait");

    // Sans retouche : le dessin d'origine, et le fichier retouché est effacé.
    let (project, entry) = set_retouch(&dir, "trait", Vec::new(), Some(&cache)).unwrap();
    assert!(project.library[0].retouch.is_empty());
    assert_eq!(entry.thumbnail, before.thumbnail);
    assert!(!retouched.exists());
    assert_eq!(retouched_file(&dir, &project.library[0]).unwrap(), original);

    // Une retouche refusée ne change rien.
    let bad = stroke(RetouchTool::Draw, 0.0, &[[0.5, 0.5]]);
    assert!(matches!(
        set_retouch(&dir, "trait", vec![bad], None),
        Err(LibraryError::Retouch(_))
    ));
    assert!(matches!(
        set_retouch(&dir, "inconnu", Vec::new(), None),
        Err(LibraryError::UnknownDrawing(_))
    ));
}

#[test]
fn a_missing_retouched_file_is_made_again() {
    let (_temp, dir) = project_with_drawing("retouche-refaite");
    let pencil = stroke(RetouchTool::Draw, 0.05, &[[0.5, 0.1], [0.5, 0.3]]);
    let (project, _) = set_retouch(&dir, "trait", vec![pencil], None).unwrap();
    let file = retouched_file(&dir, &project.library[0]).unwrap();
    let first = fs::read(&file).unwrap();
    // Le dossier `retouches/` n'est pas arrivé sur l'autre Mac.
    fs::remove_dir_all(dir.root().join(RETOUCH_DIR)).unwrap();
    let again = retouched_file(&dir, &project.library[0]).unwrap();
    assert_eq!(again, file);
    assert_eq!(fs::read(&again).unwrap(), first);
}

/// Recalage d'essai (celui des tests d'export) : la diapositive tombe en
/// trapèze sur la façade.
fn pairs() -> Vec<RegistrationPair> {
    [
        ((0.0, 0.0), (1_000.0, 800.0)),
        ((280.0, 0.0), (5_100.0, 600.0)),
        ((280.0, 280.0), (5_300.0, 4_700.0)),
        ((0.0, 280.0), (900.0, 4_500.0)),
        ((140.0, 140.0), (3_080.0, 2_690.0)),
        ((70.0, 210.0), (1_990.0, 3_640.0)),
    ]
    .iter()
    .map(|&((sx, sy), (fx, fy))| RegistrationPair::new(PointMm::new(sx, sy), PointMm::new(fx, fy)))
    .collect()
}

fn lit(path: &Path) -> Raster {
    to_gray(tiles::read_raster(path).unwrap())
}

#[test]
fn the_export_uses_the_retouched_drawing() {
    let (temp, dir) = project_with_drawing("retouche-export");
    let mut project = dir.load().unwrap().project;
    project.add_projector("A").registration = pairs();
    project.placements.push(Placement {
        id: "p1".into(),
        drawing: "trait".into(),
        center: PointMm::new(3_000.0, 2_600.0),
        width: 1_600.0,
        height: 1_600.0,
        rotation: 0.0,
        mirrored: false,
        group: None,
        layer: None,
        deformation: None,
        projector: None,
        adjustment: None,
        hidden: false,
        locked: false,
        extra: Default::default(),
    });
    dir.save(&project).unwrap();
    let settings = ExportSettings {
        dpi: 100.0,
        margin: false,
        ..ExportSettings::default()
    };
    let export = |name: &str| {
        let report =
            export_slides(&dir, &temp.path().join(name), &settings, None, 0, &|_| {}).unwrap();
        lit(&report.slides[0].path)
    };
    let count = |r: &Raster| r.data.iter().filter(|&&v| v >= 128).count();
    let untouched = export("avant");
    assert!(count(&untouched) > 500);

    // Gomme sur la moitié gauche, crayon : un trait vertical à droite.
    let strokes = vec![
        zone(
            RetouchTool::Erase,
            &[[-0.1, -0.1], [0.5, -0.1], [0.5, 1.1], [-0.1, 1.1]],
        ),
        stroke(RetouchTool::Draw, 0.05, &[[0.8, 0.1], [0.8, 0.9]]),
    ];
    let (project, _) = set_retouch(&dir, "trait", strokes, None).unwrap();
    let retouched = export("apres");
    assert!(count(&retouched) > 0);
    assert_ne!(retouched.data, untouched.data);

    // Le même export qu'avec le dessin retouché importé tel quel : la
    // retouche passe par toute la chaîne de l'export, comme l'image.
    let painted = retouched_file(&dir, &project.library[0]).unwrap();
    let copy = dir.resolve(&RelativePath::new("bibliotheque/trait-retouche.png").unwrap());
    fs::copy(&painted, &copy).unwrap();
    let mut project = dir.load().unwrap().project;
    project.library[0].source = RelativePath::new("bibliotheque/trait-retouche.png").unwrap();
    project.library[0].retouch.clear();
    dir.save(&project).unwrap();
    let reference = export("reference");
    assert_eq!(retouched.data, reference.data);
}

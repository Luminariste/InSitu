//! Tâche 2.5 : redressement d'une photo dans un projet, comme le fait
//! l'écran de redressement (import, détection, coins à la main, export).
//!
//! Critère d'acceptation (plan) : « une photo réelle est redressée en moins
//! d'une minute ». Ce test prouve la part du moteur : de l'import de la
//! photo HEIC à l'export TIFF, sur la vraie photo A d'Épinal. Le temps passé
//! par l'utilisateur à l'écran ne se mesure que sur le Mac.

use std::path::{Path, PathBuf};
use std::time::Instant;

use insitu_core::geometry::Point;
use insitu_core::photo;
use insitu_core::project::{ProjectDir, RectificationMethod};
use insitu_core::rectification::projector::{
    add_projector, export_rectified, import_raw_photo, projector_view, rectify_projector,
    RectifyError,
};
use insitu_core::rectification::{self, DetectOptions};

fn test_set() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jeu-essai")
}

/// Dossier provisoire propre à un test, supprimé en fin de test.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("insitu-redressement-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn distance(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

#[test]
fn heic_photo_reads_like_its_jpeg() {
    // Même photo, encodée en HEIC par libheif (jeu-essai/heic/fabriquer.py).
    let heic = test_set().join("heic/a-brut.heic");
    let jpeg = test_set().join("epinal/references/photos-brutes/a-brut.jpg");
    assert!(photo::is_heif_file(&heic));
    assert!(!photo::is_heif_file(&jpeg));
    assert_eq!(photo::dimensions(&heic).unwrap(), (4032, 3024));
    let a = rectification::read_photo(&heic).unwrap();
    let b = rectification::read_photo(&jpeg).unwrap();
    assert_eq!(
        a.dimensions(),
        b.dimensions(),
        "même disposition des pixels"
    );
    let da = rectification::detect(&a, &DetectOptions::default()).unwrap();
    let db = rectification::detect(&b, &DetectOptions::default()).unwrap();
    for (x, y) in [
        (0.0, 0.0),
        (280.0, 0.0),
        (280.0, 280.0),
        (0.0, 280.0),
        (140.0, 140.0),
    ] {
        let p = Point::new(x, y);
        let gap = distance(da.slide_to_photo(p).unwrap(), db.slide_to_photo(p).unwrap());
        let mm = gap / db.px_per_mm;
        eprintln!("({x}, {y}) mm : HEIC et JPEG à {gap:.3} px, {mm:.3} mm");
        // Aux coins, le modèle extrapole au-delà des derniers croisements.
        assert!(mm < 0.1, "({x}, {y}) : {mm:.3} mm");
    }
    // Les tuiles de la visionneuse se fabriquent aussi depuis un HEIC.
    let raster = insitu_core::tiles::read_raster(&heic).unwrap();
    assert_eq!(
        (raster.width, raster.height, raster.channels),
        (4032, 3024, 3)
    );
}

#[test]
fn a_real_photo_is_rectified_in_the_project_in_under_a_minute() {
    let scratch = Scratch::new("complet");
    let start = Instant::now();
    let (dir, _) = ProjectDir::create(scratch.0.join("Essai.insitu"), None).unwrap();
    let (_, id) = add_projector(&dir, "A").unwrap();
    let project = import_raw_photo(&dir, "a", &test_set().join("heic/a-brut.heic")).unwrap();
    assert_eq!(
        project
            .projector(&id)
            .unwrap()
            .raw_photo
            .as_ref()
            .unwrap()
            .as_str(),
        "photos-brutes/a-brut.heic"
    );

    // Détection automatique : la photo redressée entre dans le projet.
    let (project, view) = rectify_projector(&dir, &id, None).unwrap();
    let detected = start.elapsed().as_secs_f64();
    let p = project.projector(&id).unwrap();
    assert_eq!(
        p.rectified_photo.as_ref().unwrap().as_str(),
        "photos-redressees/a.png"
    );
    let rectification = p.rectification.as_ref().unwrap();
    assert_eq!(rectification.method, RectificationMethod::Automatic);
    assert_eq!(rectification.points.len(), view.nodes.len());
    assert!(view.nodes.len() >= 600, "{} croisements", view.nodes.len());
    assert!(
        view.leave_one_out_mm.p95 < 0.3,
        "{:?}",
        view.leave_one_out_mm
    );
    assert!(view.coverage > 0.9);
    assert_eq!(view.outline.len(), 4 * 28 + 1);
    let saved = photo::read_rgb(&dir.resolve(p.rectified_photo.as_ref().unwrap())).unwrap();
    assert_eq!(saved.dimensions(), (5512, 5512));
    let check = rectification::check_rectified(&saved).unwrap();
    eprintln!(
        "photo redressée du projet : {} croisements relus, p95 {:.3} mm",
        check.nodes.len(),
        check.deviation_mm.p95
    );
    assert!(check.deviation_mm.p95 < 0.3);

    // Export TIFF à 500 dpi, depuis les correspondances enregistrées.
    let tiff = scratch.0.join("A redressée.tif");
    let exported = export_rectified(&dir, "A", &tiff, 500.0).unwrap();
    let total = start.elapsed().as_secs_f64();
    eprintln!(
        "import + détection + photo du projet : {detected:.1} s ; avec l'export TIFF ({} Mo) : {total:.1} s",
        exported.bytes / 1_000_000
    );
    assert!(total < 60.0, "{total:.1} s");
    let back = photo::read_rgb(&tiff).unwrap();
    assert_eq!(back.dimensions(), (5512, 5512));
    // L'export recalcule la même image que celle du projet.
    let differing = back
        .as_raw()
        .iter()
        .zip(saved.as_raw())
        .filter(|(a, b)| a.abs_diff(**b) > 1)
        .count();
    assert!(
        differing * 1000 < back.as_raw().len(),
        "{differing} valeurs diffèrent"
    );

    // Relu sans détection : mêmes croisements, mêmes coins.
    let reread = projector_view(&dir, &id).unwrap().unwrap();
    assert_eq!(reread.nodes.len(), view.nodes.len());
    assert!(reread.seconds.is_none());
    for k in 0..4 {
        assert!(distance(reread.corners[k], view.corners[k]) < 1e-6);
    }

    // Export PNG à une autre résolution ; extension et résolution refusées.
    let png = scratch.0.join("a-300.png");
    let exported = export_rectified(&dir, &id, &png, 300.0).unwrap();
    assert_eq!(exported.size_px, 3307);
    assert_eq!(photo::dimensions(&png).unwrap(), (3307, 3307));
    assert!(matches!(
        export_rectified(&dir, &id, &scratch.0.join("a.jpg"), 500.0),
        Err(RectifyError::BadExport(_))
    ));
    assert!(matches!(
        export_rectified(&dir, &id, &scratch.0.join("a.png"), 5000.0),
        Err(RectifyError::BadExport(_))
    ));
}

#[test]
fn four_corners_placed_by_hand_restart_the_detection() {
    let scratch = Scratch::new("coins");
    let (dir, _) = ProjectDir::create(scratch.0.join("Coins.insitu"), None).unwrap();
    let (_, id) = add_projector(&dir, "D").unwrap();
    import_raw_photo(
        &dir,
        &id,
        &test_set().join("epinal/references/photos-brutes/d-brut.jpg"),
    )
    .unwrap();
    // Sans détection, rien à relire.
    assert!(projector_view(&dir, &id).unwrap().is_none());
    let (_, automatic) = rectify_projector(&dir, &id, None).unwrap();

    // Des coins à 10 px près (1,5 mm), comme des poignées posées à la main.
    let offsets = [(9.0, -7.0), (-12.0, 4.0), (6.0, 11.0), (-8.0, -9.0)];
    let corners: [Point; 4] = std::array::from_fn(|k| {
        Point::new(
            automatic.corners[k].x + offsets[k].0,
            automatic.corners[k].y + offsets[k].1,
        )
    });
    let (project, manual) = rectify_projector(&dir, &id, Some(corners)).unwrap();
    let rectification = project
        .projector(&id)
        .unwrap()
        .rectification
        .as_ref()
        .unwrap();
    assert_eq!(rectification.method, RectificationMethod::Manual);
    assert!(manual.nodes.len() >= 600);
    // La grille ramène les coins sur ceux de la détection automatique. Un
    // coin est extrapolé au-delà des derniers croisements mesurés : il
    // bouge davantage que l'intérieur selon les croisements retenus.
    for k in 0..4 {
        let mm = distance(manual.corners[k], automatic.corners[k]) / automatic.px_per_mm;
        eprintln!("coin {k} : à {mm:.3} mm de la détection automatique (posé à 1,5 mm)");
        assert!(mm < 0.5, "coin {k} : {mm:.3} mm");
    }
    let inside = |v: &insitu_core::rectification::projector::RectificationView| {
        v.nodes
            .iter()
            .find(|n| n.slide == Point::new(60.0, 60.0))
            .map(|n| n.photo)
    };
    if let (Some(a), Some(b)) = (inside(&automatic), inside(&manual)) {
        let mm = distance(a, b) / automatic.px_per_mm;
        eprintln!("croisement (60, 60) : {mm:.4} mm d'écart");
        assert!(mm < 0.05, "{mm:.4} mm");
    }
    // Même fichier pour la photo redressée du même projecteur.
    assert_eq!(
        project
            .projector(&id)
            .unwrap()
            .rectified_photo
            .as_ref()
            .unwrap()
            .as_str(),
        "photos-redressees/d.png"
    );

    // Des coins confondus : refus, et le projet reste tel quel.
    let before = std::fs::read_to_string(dir.project_file()).unwrap();
    let flat = [Point::new(10.0, 10.0); 4];
    assert!(rectify_projector(&dir, &id, Some(flat)).is_err());
    assert_eq!(std::fs::read_to_string(dir.project_file()).unwrap(), before);
}

#[test]
fn photo_import_refuses_what_it_cannot_read() {
    let scratch = Scratch::new("import");
    let (dir, _) = ProjectDir::create(scratch.0.join("Import.insitu"), None).unwrap();
    assert!(matches!(
        import_raw_photo(&dir, "A", Path::new("/nulle/part.jpg")),
        Err(RectifyError::UnknownProjector(_))
    ));
    add_projector(&dir, "A").unwrap();
    assert!(matches!(
        add_projector(&dir, " a "),
        Err(RectifyError::BadLabel(_))
    ));
    assert!(matches!(
        add_projector(&dir, "  "),
        Err(RectifyError::BadLabel(_))
    ));
    let fake = scratch.0.join("faux.heic");
    std::fs::write(&fake, b"\0\0\0\x18ftypheic\0\0\0\0 pas une photo").unwrap();
    let error = import_raw_photo(&dir, "A", &fake).unwrap_err();
    assert!(error.to_string().contains("HEIC"), "{error}");
    assert!(
        std::fs::read_dir(dir.root().join("photos-brutes"))
            .unwrap()
            .next()
            .is_none(),
        "rien n'est copié"
    );
    assert!(matches!(
        rectify_projector(&dir, "A", None),
        Err(RectifyError::NoRawPhoto(_))
    ));
}

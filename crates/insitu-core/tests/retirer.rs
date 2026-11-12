//! Retirer un dessin de la bibliothèque, retirer le plan de la façade
//! (système de design, D15-h) : le projet oublie l'élément, le disque garde
//! le fichier.

use std::path::{Path, PathBuf};

use insitu_core::facade::{self, FacadeError};
use insitu_core::library::{import_folder, remove_drawing, ImportOptions, LibraryError};
use insitu_core::placement::{self, new_placement};
use insitu_core::project::{PointMm, ProjectDir};

fn test_set() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jeu-essai")
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("insitu-retirer-{name}-{}", std::process::id()));
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

/// Un petit dessin : un carré blanc sur fond noir.
fn write_drawing(path: &Path, side: u32) {
    let image = image::GrayImage::from_fn(side, side, |x, y| {
        let inside = (side / 4..3 * side / 4).contains(&x) && (side / 4..3 * side / 4).contains(&y);
        image::Luma([if inside { 255 } else { 0 }])
    });
    image.save(path).unwrap();
}

#[test]
fn removing_a_drawing_removes_its_placements_and_keeps_the_file() {
    let scratch = Scratch::new("dessin");
    let source = scratch.0.join("Dessins");
    std::fs::create_dir_all(&source).unwrap();
    write_drawing(&source.join("merle.png"), 32);
    write_drawing(&source.join("pie.png"), 48);
    let (dir, _) = ProjectDir::create(scratch.0.join("Essai.insitu"), None).unwrap();
    let (project, _) = import_folder(&dir, &source, &ImportOptions::default(), &|_| {}).unwrap();
    let merle = project
        .library
        .iter()
        .find(|d| d.name == "merle")
        .unwrap()
        .clone();

    // Le merle posé deux fois, la pie une fois.
    for x in [100.0, 300.0] {
        let p = new_placement(&project, "merle", PointMm::new(x, 100.0), 50.0).unwrap();
        placement::add(&dir, p).unwrap();
    }
    let project = dir.load().unwrap().project;
    placement::add(
        &dir,
        new_placement(&project, "pie", PointMm::new(200.0, 200.0), 50.0).unwrap(),
    )
    .unwrap();

    let (project, removed) = remove_drawing(&dir, &merle.id).unwrap();
    assert_eq!(removed, 2, "les deux placements du merle partent avec lui");
    assert_eq!(project.library.len(), 1);
    assert_eq!(project.placements.len(), 1);
    assert!(
        dir.resolve(&merle.source).is_file(),
        "le fichier reste sur le disque"
    );

    // Relu depuis le disque, et deux fois de suite : inconnu.
    let reread = dir.load().unwrap().project;
    assert!(reread.drawing(&merle.id).is_none());
    assert!(matches!(
        remove_drawing(&dir, &merle.id),
        Err(LibraryError::UnknownDrawing(_))
    ));

    // Un nouvel import reprend le fichier tel quel, sans doublon « merle-2 ».
    let (project, report) =
        import_folder(&dir, &source, &ImportOptions::default(), &|_| {}).unwrap();
    assert_eq!(report.added.len(), 1);
    let again = project.library.iter().find(|d| d.name == "merle").unwrap();
    assert_eq!(again.source, merle.source);
}

#[test]
fn removing_the_plan_keeps_what_is_in_millimetres() {
    let scratch = Scratch::new("plan");
    let (dir, _) = ProjectDir::create(scratch.0.join("Essai.insitu"), None).unwrap();
    assert!(matches!(
        facade::remove_facade(&dir),
        Err(FacadeError::NoFacade)
    ));

    let plan = test_set().join("epinal/reduits/facade/portail-deroule.png");
    let project = facade::import_facade(&dir, &plan, None).unwrap();
    let source = project.facade.as_ref().unwrap().source.clone();
    let source_path = dir.resolve(&source);

    let project = facade::remove_facade(&dir).unwrap();
    assert!(project.facade.is_none());
    assert!(source_path.is_file(), "le plan reste dans le dossier");
    assert!(dir.load().unwrap().project.facade.is_none());

    // Le même plan se réimporte sans copie en double.
    let project = facade::import_facade(&dir, &plan, None).unwrap();
    assert_eq!(project.facade.unwrap().source, source);
}

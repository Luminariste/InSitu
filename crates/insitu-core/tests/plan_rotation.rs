//! Tourner le plan de la façade d'un quart de tour (demande de Benjamin,
//! D15-k) : le plan s'affiche tourné, l'échelle est gardée, et ce qui est
//! déjà posé en mm de façade tourne avec lui.

use std::path::{Path, PathBuf};

use insitu_core::facade::{self, display::render_whole, FacadeError};
use insitu_core::library::{import_folder, ImportOptions};
use insitu_core::placement::{self, new_placement};
use insitu_core::project::{
    ExtraFields, FrescoImage, PointMm, ProjectDir, RectMm, RegistrationPair, SourcePoint,
};
use insitu_core::tiles::Raster;

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("insitu-rotation-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Un plan SVG de 400 × 200 qui n'a aucune symétrie : un L épais et un
/// carré dans le coin haut-droit.
fn write_svg_plan(path: &Path) {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="200" viewBox="0 0 400 200">
  <rect x="0" y="0" width="400" height="200" fill="#fff"/>
  <path d="M 20 20 L 60 20 L 60 140 L 180 140 L 180 180 L 20 180 Z" fill="#000"/>
  <rect x="330" y="10" width="50" height="30" fill="#000"/>
</svg>"##;
    std::fs::write(path, svg).unwrap();
}

/// Une image de plan de 6 × 4 px, noire, avec deux pixels blancs : en
/// haut à droite (5, 0) et en bas à gauche (0, 3).
fn write_image_plan(path: &Path) {
    let image = image::GrayImage::from_fn(6, 4, |x, y| {
        image::Luma([if (x, y) == (5, 0) || (x, y) == (0, 3) {
            255
        } else {
            0
        }])
    });
    image.save(path).unwrap();
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn a_raster_turns_exactly_by_quarter_turns() {
    let data: Vec<u8> = (0..6u8).collect();
    let raster = Raster::new(3, 2, 1, data);
    // 0 1 2      3 0
    // 3 4 5  →   4 1   (un quart de tour, sens des aiguilles d'une montre)
    //            5 2
    let turned = raster.clone().rotated(1);
    assert_eq!((turned.width, turned.height), (2, 3));
    assert_eq!(turned.data, [3, 0, 4, 1, 5, 2]);
    assert_eq!(raster.clone().rotated(2).data, [5, 4, 3, 2, 1, 0]);
    assert_eq!(raster.clone().rotated(3).data, [2, 5, 1, 4, 0, 3]);
    assert_eq!(raster.clone().rotated(1).rotated(3), raster);
    assert_eq!(raster.clone().rotated(4), raster);
}

#[test]
fn a_turned_vector_plan_renders_as_the_turned_image() {
    let scratch = Scratch::new("vecteur");
    let plan = scratch.path("plan.svg");
    write_svg_plan(&plan);
    // 400 px de côté : une unité du SVG = un pixel, le rendu tombe juste.
    let (w, h, flat) = render_whole(&plan, None, 0, 400).unwrap();
    assert_eq!((w, h), (400, 200));
    let flat = Raster::new(w, h, 4, flat);
    for turns in 1..=3u8 {
        let (tw, th, turned) = render_whole(&plan, None, turns, 400).unwrap();
        let expected = flat.clone().rotated(turns);
        assert_eq!(
            (tw, th),
            (expected.width, expected.height),
            "{turns} quart(s)"
        );
        let worst = turned
            .iter()
            .zip(&expected.data)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(worst <= 1, "{turns} quart(s) : écart de {worst} niveaux");
    }
}

#[test]
fn turning_keeps_the_scale_and_carries_what_is_placed() {
    let scratch = Scratch::new("projet");
    let plan = scratch.path("plan.svg");
    write_svg_plan(&plan);
    let drawings = scratch.path("Dessins");
    std::fs::create_dir_all(&drawings).unwrap();
    image::GrayImage::from_fn(16, 16, |x, _| image::Luma([if x < 8 { 255 } else { 0 }]))
        .save(drawings.join("merle.png"))
        .unwrap();
    let (dir, _) = ProjectDir::create(scratch.path("Essai.insitu"), None).unwrap();
    facade::import_facade(&dir, &plan, None).unwrap();
    // 1 unité = 10 mm : la façade fait 4 m × 2 m.
    facade::set_scale(
        &dir,
        SourcePoint::new(0.0, 0.0),
        SourcePoint::new(100.0, 0.0),
        1000.0,
    )
    .unwrap();
    let (project, _) = import_folder(&dir, &drawings, &ImportOptions::default(), &|_| {}).unwrap();
    let mut placed = new_placement(&project, "merle", PointMm::new(500.0, 300.0), 200.0).unwrap();
    placed.rotation = 30.0;
    placement::add(&dir, placed).unwrap();
    let mut project = dir.load().unwrap().project;
    project.projectors.push(
        serde_json::from_value(serde_json::json!({
            "id": "p1", "label": "P1",
        }))
        .unwrap(),
    );
    project.projectors[0].registration.push(RegistrationPair {
        slide: PointMm::new(10.0, 20.0),
        facade: PointMm::new(3_900.0, 100.0),
        extra: ExtraFields::new(),
    });
    dir.save(&project).unwrap();
    let before = dir.load().unwrap().project;

    let turned = facade::rotate_facade(&dir, 1).unwrap();
    let facade_after = turned.facade.as_ref().unwrap();
    assert_eq!(facade_after.rotation, 1);
    assert_eq!(
        (facade_after.source_width, facade_after.source_height),
        (200.0, 400.0)
    );
    let (width, height) = facade_after.size_mm().unwrap();
    assert!(
        close(width, 2_000.0) && close(height, 4_000.0),
        "échelle gardée"
    );
    // (x, y) → (h − y, x), h = 2 000 mm avant le quart de tour.
    let p = &turned.placements[0];
    assert!(close(p.center.x, 1_700.0) && close(p.center.y, 500.0));
    assert!(close(p.rotation, 120.0));
    let pair = &turned.projectors[0].registration[0];
    assert!(close(pair.facade.x, 1_900.0) && close(pair.facade.y, 3_900.0));
    assert_eq!(
        pair.slide,
        PointMm::new(10.0, 20.0),
        "côté diapositive inchangé"
    );
    // Le projet relu du disque est le même.
    assert_eq!(dir.load().unwrap().project, turned);

    // Trois quarts de tour de plus : retour au départ.
    let back = facade::rotate_facade(&dir, -1).unwrap();
    let facade_back = back.facade.as_ref().unwrap();
    assert_eq!(facade_back.rotation, 0);
    let scale = facade_back.scale.as_ref().unwrap();
    let start = before.facade.as_ref().unwrap().scale.as_ref().unwrap();
    assert!(close(scale.point_b.x, start.point_b.x) && close(scale.point_b.y, start.point_b.y));
    let p = &back.placements[0];
    assert!(close(p.center.x, 500.0) && close(p.center.y, 300.0));
    assert!(close(p.rotation, 30.0));
}

#[test]
fn a_turned_image_plan_is_shown_turned() {
    let scratch = Scratch::new("image");
    let plan = scratch.path("plan.png");
    write_image_plan(&plan);
    let (dir, _) = ProjectDir::create(scratch.path("Essai.insitu"), None).unwrap();
    facade::import_facade(&dir, &plan, None).unwrap();
    let cache = scratch.path("cache");
    let flat = facade::open_project_facade(&dir, &cache).unwrap();
    assert_eq!((flat.pyramid.width, flat.pyramid.height), (6, 4));

    let project = facade::rotate_facade(&dir, 3).unwrap();
    let facade_now = project.facade.as_ref().unwrap();
    assert_eq!(facade_now.rotation, 3);
    assert_eq!(
        (facade_now.source_width, facade_now.source_height),
        (4.0, 6.0)
    );
    let turned = facade::open_project_facade(&dir, &cache).unwrap();
    assert_eq!((turned.pyramid.width, turned.pyramid.height), (4, 6));
    assert_eq!((turned.source_width, turned.source_height), (4.0, 6.0));
    assert_ne!(turned.key, flat.key, "une autre pyramide");
    // Un quart de tour dans le sens inverse : le pixel en haut à droite
    // (5, 0) de l'image arrive en haut à gauche.
    let tile = image::open(cache.join(&turned.key).join("0/0_0.png"))
        .unwrap()
        .into_luma8();
    let white: Vec<(u32, u32)> = tile
        .enumerate_pixels()
        .filter(|(_, _, p)| p.0[0] > 127)
        .map(|(x, y, _)| (x, y))
        .collect();
    // (0, 3) arrive en bas à droite.
    assert_eq!(white, [(0, 0), (3, 5)]);
}

#[test]
fn a_fresco_on_a_scaled_plan_blocks_the_turn() {
    let scratch = Scratch::new("fresque");
    let plan = scratch.path("plan.png");
    write_image_plan(&plan);
    let (dir, _) = ProjectDir::create(scratch.path("Essai.insitu"), None).unwrap();
    facade::import_facade(&dir, &plan, None).unwrap();
    facade::set_scale(
        &dir,
        SourcePoint::new(0.0, 0.0),
        SourcePoint::new(6.0, 0.0),
        600.0,
    )
    .unwrap();
    let mut project = dir.load().unwrap().project;
    project.fresco = Some(FrescoImage {
        source: project.facade.as_ref().unwrap().source.clone(),
        frame: RectMm {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 400.0,
        },
        extra: ExtraFields::new(),
    });
    dir.save(&project).unwrap();
    let error = facade::rotate_facade(&dir, 1).unwrap_err();
    assert!(matches!(error, FacadeError::CannotRotate(_)), "{error}");
    assert_eq!(dir.load().unwrap().project, project, "rien n'a changé");
}

#[test]
fn turning_without_a_plan_is_refused() {
    let scratch = Scratch::new("sans-plan");
    let (dir, _) = ProjectDir::create(scratch.path("Essai.insitu"), None).unwrap();
    assert!(matches!(
        facade::rotate_facade(&dir, 1),
        Err(FacadeError::NoFacade)
    ));
}

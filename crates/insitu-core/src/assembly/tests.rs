use std::path::{Path, PathBuf};

use super::*;
use crate::project::test_support::TempDir;
use crate::project::{validate, FacadeScale, Projector, RegistrationPair};
use crate::tiles::read_raster;

/// La façade d'Épinal (réduit), qui sert de « vérité ».
fn truth() -> Raster {
    read_raster(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../jeu-essai/epinal/reduits/facade/portail-deroule.png"),
    )
    .expect("façade d'essai")
}

/// Une « photo » de la vérité : `to_truth` envoie un pixel de la photo sur
/// la vérité (trapèze, comme un autre point de vue).
fn shoot(truth: &Raster, to_truth: &Homography, width: u32, height: u32, path: &Path) {
    let map = |p: Point| to_truth.apply(p);
    let photo = warp_raster(truth, width, height, map, &WarpOptions::default()).unwrap();
    image::save_buffer(path, &photo.data, width, height, image::ColorType::Rgb8).unwrap();
}

/// Trapèze qui pose le rectangle `w × h` de la photo sur le quadrilatère
/// `corners` de la vérité.
fn view(w: f64, h: f64, corners: [(f64, f64); 4]) -> Homography {
    let source = [
        Point::new(0.0, 0.0),
        Point::new(w, 0.0),
        Point::new(w, h),
        Point::new(0.0, h),
    ];
    let target = corners.map(|(x, y)| Point::new(x, y));
    Homography::from_four_pairs(source, target).unwrap()
}

struct Scene {
    _temp: TempDir,
    dir: ProjectDir,
    truth: Raster,
    /// Photo → vérité, pour chaque photo.
    views: Vec<Homography>,
    files: Vec<PathBuf>,
}

const W: u32 = 520;
const H: u32 = 420;

/// Trois photos qui se recouvrent, de gauche à droite, chacune en biais.
fn scene() -> Scene {
    let temp = TempDir::new("assemblage");
    let (dir, _) = ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).unwrap();
    let truth = truth();
    let (w, h) = (f64::from(W), f64::from(H));
    let views = vec![
        view(
            w,
            h,
            [(40.0, 80.0), (700.0, 60.0), (720.0, 560.0), (30.0, 600.0)],
        ),
        view(
            w,
            h,
            [
                (330.0, 90.0),
                (960.0, 110.0),
                (940.0, 620.0),
                (350.0, 580.0),
            ],
        ),
        view(
            w,
            h,
            [
                (640.0, 70.0),
                (1250.0, 50.0),
                (1240.0, 600.0),
                (660.0, 590.0),
            ],
        ),
    ];
    let files: Vec<PathBuf> = ["gauche", "milieu", "droite"]
        .iter()
        .zip(&views)
        .map(|(name, v)| {
            let path = temp.path().join(format!("{name}.png"));
            shoot(&truth, v, W, H, &path);
            path
        })
        .collect();
    Scene {
        _temp: temp,
        dir,
        truth,
        views,
        files,
    }
}

/// Points communs entre la photo `a` (ici) et la photo `b` (appui) : des
/// points de la vérité vus par les deux, avec une erreur de clic `noise`.
fn pairs_between(
    s: &Scene,
    a: usize,
    b: usize,
    truth_points: &[(f64, f64)],
    noise: f64,
) -> Vec<AssemblyPair> {
    let to_a = s.views[a].inverse().unwrap();
    let to_b = s.views[b].inverse().unwrap();
    truth_points
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| {
            let p = Point::new(x, y);
            let here = to_a.apply(p).unwrap();
            let there = to_b.apply(p).unwrap();
            // Erreur de clic déterministe, d'au plus `noise` pixels.
            let jitter = noise * ((k as f64 * 2.3).sin());
            AssemblyPair {
                here: SourcePoint::new(here.x + jitter, here.y - jitter),
                there: SourcePoint::new(there.x, there.y),
            }
        })
        .collect()
}

const LEFT_OVERLAP: [(f64, f64); 8] = [
    (390.0, 140.0),
    (660.0, 130.0),
    (520.0, 230.0),
    (400.0, 340.0),
    (650.0, 360.0),
    (530.0, 450.0),
    (390.0, 540.0),
    (670.0, 530.0),
];
const RIGHT_OVERLAP: [(f64, f64); 8] = [
    (700.0, 140.0),
    (910.0, 150.0),
    (800.0, 250.0),
    (690.0, 350.0),
    (920.0, 360.0),
    (810.0, 450.0),
    (700.0, 540.0),
    (900.0, 550.0),
];

fn build(s: &Scene, noise: f64) -> Vec<String> {
    let mut ids = Vec::new();
    for file in &s.files {
        let (_, id) = add_photo(&s.dir, file, None).unwrap();
        ids.push(id);
    }
    set_pairs(
        &s.dir,
        &ids[1],
        &ids[0],
        pairs_between(s, 1, 0, &LEFT_OVERLAP, noise),
    )
    .unwrap();
    set_pairs(
        &s.dir,
        &ids[2],
        &ids[1],
        pairs_between(s, 2, 1, &RIGHT_OVERLAP, noise),
    )
    .unwrap();
    ids
}

#[test]
fn three_photos_are_assembled_into_the_facade() {
    let s = scene();
    // Sans erreur de clic, la photo de droite, recalée sur celle du milieu
    // seulement, tombe exactement à sa place dans le repère de la première.
    build(&s, 0.0);
    let project = s.dir.load().unwrap().project;
    let diagnosis = report(&project);
    assert!(diagnosis.ready, "{diagnosis:?}");
    // La troisième photo, recalée sur la deuxième seulement, tombe à sa
    // place dans le repère de la première (la vérité vue par la photo 0).
    let truth_to_reference = s.views[0].inverse().unwrap();
    let corners = diagnosis.photos[2].corners.unwrap();
    let expected = [
        Point::new(0.0, 0.0),
        Point::new(f64::from(W), 0.0),
        Point::new(f64::from(W), f64::from(H)),
        Point::new(0.0, f64::from(H)),
    ]
    .map(|p| {
        truth_to_reference
            .apply(s.views[2].apply(p).unwrap())
            .unwrap()
    });
    for (got, want) in corners.iter().zip(expected) {
        let error = Point::new(got.x, got.y).distance(want);
        assert!(error < 0.5, "coin à {error:.2} px de sa place");
    }
    // Avec une erreur de clic de 0,3 px au plus, l'écart reste faible.
    let ids: Vec<String> = project
        .facade_assembly
        .as_ref()
        .unwrap()
        .photos
        .iter()
        .map(|p| p.id.clone())
        .collect();
    set_pairs(
        &s.dir,
        &ids[1],
        &ids[0],
        pairs_between(&s, 1, 0, &LEFT_OVERLAP, 0.3),
    )
    .unwrap();
    set_pairs(
        &s.dir,
        &ids[2],
        &ids[1],
        pairs_between(&s, 2, 1, &RIGHT_OVERLAP, 0.3),
    )
    .unwrap();
    let diagnosis = report(&s.dir.load().unwrap().project);
    for photo in &diagnosis.photos[1..] {
        let mean = photo.mean_residual_px.unwrap();
        assert!(mean < 1.0, "{} : écart moyen {mean:.2} px", photo.name);
        assert!(photo.pairs.iter().all(|p| !p.outlier));
    }

    let (project, done) = render(&s.dir, &RenderOptions::default()).unwrap();
    assert_eq!(done.scale, 1.0);
    assert!(!done.kept_scale);
    let facade = project.facade.as_ref().expect("la façade est l'assemblage");
    assert_eq!(facade.source.as_str(), OUTPUT_FILE);
    assert_eq!(
        (facade.source_width, facade.source_height),
        (f64::from(done.width), f64::from(done.height))
    );
    assert!(validate(&s.dir).is_valid());

    // Comparaison avec la vérité, pixel à pixel, là où une photo couvre.
    let output = read_raster(&s.dir.resolve(&facade.source)).unwrap();
    let origin = project
        .facade_assembly
        .as_ref()
        .unwrap()
        .output
        .as_ref()
        .unwrap()
        .origin;
    let reference_to_truth = &s.views[0];
    let (mut total, mut count) = (0.0, 0usize);
    let width = output.width as usize;
    for y in (0..output.height).step_by(3) {
        for x in (0..output.width).step_by(3) {
            let reference =
                Point::new(f64::from(x) + 0.5 + origin.x, f64::from(y) + 0.5 + origin.y);
            let t = reference_to_truth.apply(reference).unwrap();
            // Loin des bords de la vérité et des bords des photos.
            if !(20.0..1260.0).contains(&t.x) || !(70.0..580.0).contains(&t.y) {
                continue;
            }
            let covered = s.views.iter().any(|v| {
                v.inverse().unwrap().apply(t).is_some_and(|q| {
                    q.x > 4.0 && q.y > 4.0 && q.x < f64::from(W) - 4.0 && q.y < f64::from(H) - 4.0
                })
            });
            if !covered {
                continue;
            }
            let (tx, ty) = (t.x as usize, t.y as usize);
            let want = &s.truth.data[(ty * s.truth.width as usize + tx) * 3..][..3];
            let got = &output.data[(y as usize * width + x as usize) * 3..][..3];
            let diff: f64 = want
                .iter()
                .zip(got)
                .map(|(a, b)| (f64::from(*a) - f64::from(*b)).abs())
                .sum::<f64>()
                / 3.0;
            total += diff;
            count += 1;
        }
    }
    let mean = total / count as f64;
    assert!(count > 20_000, "trop peu de points comparés : {count}");
    // Les photos sont des rééchantillonnages de la vérité (flou, JPEG) : on
    // demande un accord de quelques niveaux, pas l'identité.
    assert!(mean < 12.0, "écart moyen à la vérité : {mean:.1} niveaux");
}

#[test]
fn a_misclicked_point_is_flagged() {
    let s = scene();
    let ids = build(&s, 0.3);
    let mut pairs = pairs_between(&s, 1, 0, &LEFT_OVERLAP, 0.3);
    pairs[3].there.x += 25.0;
    let (_, diagnosis) = set_pairs(&s.dir, &ids[1], &ids[0], pairs).unwrap();
    let flags: Vec<bool> = diagnosis.photos[1]
        .pairs
        .iter()
        .map(|p| p.outlier)
        .collect();
    assert_eq!(
        flags,
        [false, false, false, true, false, false, false, false]
    );
}

#[test]
fn wrong_points_and_anchors_are_refused_and_change_nothing() {
    let s = scene();
    let ids = build(&s, 0.0);
    let before = std::fs::read(s.dir.project_file()).unwrap();
    let mut outside = pairs_between(&s, 1, 0, &LEFT_OVERLAP, 0.0);
    outside[0].here.x = -5.0;
    assert!(matches!(
        set_pairs(&s.dir, &ids[1], &ids[0], outside),
        Err(AssemblyError::BadPairs(_))
    ));
    // Une boucle : la photo 0 ne peut pas s'appuyer sur la 2, ni la 1 sur la 2.
    let pairs = pairs_between(&s, 1, 2, &RIGHT_OVERLAP, 0.0);
    assert!(matches!(
        set_pairs(&s.dir, &ids[1], &ids[2], pairs),
        Err(AssemblyError::BadAnchor(_))
    ));
    assert!(matches!(
        set_pairs(&s.dir, &ids[0], &ids[1], Vec::new()),
        Err(AssemblyError::BadAnchor(_))
    ));
    // La photo du milieu sert d'appui à la droite.
    assert!(matches!(
        remove_photo(&s.dir, &ids[1]),
        Err(AssemblyError::StillAnchor { .. })
    ));
    assert_eq!(std::fs::read(s.dir.project_file()).unwrap(), before);
}

#[test]
fn an_unplaced_photo_blocks_the_assembly() {
    let s = scene();
    let ids = build(&s, 0.0);
    let three = pairs_between(&s, 2, 1, &RIGHT_OVERLAP[..3], 0.0);
    let (_, diagnosis) = set_pairs(&s.dir, &ids[2], &ids[1], three).unwrap();
    assert!(!diagnosis.ready);
    assert!(diagnosis.photos[2]
        .problem
        .as_deref()
        .unwrap()
        .contains("3 points"));
    assert!(matches!(
        render(&s.dir, &RenderOptions::default()),
        Err(AssemblyError::NotPlaced(names)) if names == ["droite"]
    ));
    // Retirer la dernière photo débloque.
    remove_photo(&s.dir, &ids[2]).unwrap();
    assert!(render(&s.dir, &RenderOptions::default()).is_ok());
}

#[test]
fn a_new_photo_keeps_the_scale_and_the_registrations_in_place() {
    let s = scene();
    let ids = build(&s, 0.0);
    remove_photo(&s.dir, &ids[2]).unwrap();
    // Assemblage de deux photos, mis à l'échelle, avec un recalage.
    render(&s.dir, &RenderOptions::default()).unwrap();
    let mut project = s.dir.load().unwrap().project;
    let first_origin = project
        .facade_assembly
        .as_ref()
        .unwrap()
        .output
        .as_ref()
        .unwrap()
        .origin;
    let facade = project.facade.as_mut().unwrap();
    facade.scale = Some(FacadeScale {
        point_a: SourcePoint::new(100.0, 100.0),
        point_b: SourcePoint::new(300.0, 100.0),
        distance_mm: 2000.0,
        extra: ExtraFields::new(),
    });
    let mut projector = Projector::new("A");
    projector.registration.push(RegistrationPair {
        slide: PointMm::new(10.0, 10.0),
        facade: PointMm::new(1500.0, 800.0),
        extra: ExtraFields::new(),
    });
    project.projectors.push(projector);
    s.dir.save(&project).unwrap();
    // Le point de façade (1 500 ; 800) mm, en pixels de référence :
    let reference_px =
        |o: SourcePoint, mm: PointMm| Point::new(mm.x / 10.0 + o.x, mm.y / 10.0 + o.y);
    let before = reference_px(first_origin, PointMm::new(1500.0, 800.0));

    // On rajoute la photo de droite et on refait l'assemblage.
    let (_, id) = add_photo(&s.dir, &s.files[2], Some(&ids[1])).unwrap();
    set_pairs(
        &s.dir,
        &id,
        &ids[1],
        pairs_between(&s, 2, 1, &RIGHT_OVERLAP, 0.0),
    )
    .unwrap();
    let (project, done) = render(&s.dir, &RenderOptions::default()).unwrap();
    assert!(done.kept_scale);
    let output = project
        .facade_assembly
        .as_ref()
        .unwrap()
        .output
        .clone()
        .unwrap();
    let facade = project.facade.as_ref().unwrap();
    let factor = facade.scale.as_ref().unwrap().mm_per_source_unit().unwrap();
    assert!((factor - 10.0).abs() < 1e-9, "échelle changée : {factor}");
    assert_eq!(facade.source_width, f64::from(done.width));
    let moved = project.projectors[0].registration[0].facade;
    let after = reference_px(output.origin, moved);
    assert!(before.distance(after) < 1e-9, "le point de façade a bougé");
}

#[test]
fn a_large_assembly_is_reduced_to_the_maximum_side() {
    let s = scene();
    build(&s, 0.0);
    let (project, done) = render(
        &s.dir,
        &RenderOptions {
            max_side: 400,
            threads: 2,
        },
    )
    .unwrap();
    assert!(done.scale < 1.0);
    assert_eq!(done.width.max(done.height), 400);
    let output = read_raster(&s.dir.resolve(&project.facade.unwrap().source)).unwrap();
    assert_eq!((output.width, output.height), (done.width, done.height));
}

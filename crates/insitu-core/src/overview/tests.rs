use super::*;
use crate::project::test_support::TempDir;
use crate::project::{
    ExtraFields, Facade, FacadeScale, RegistrationPair, RelativePath, SourcePoint,
};

/// Paires d'un projecteur dont la diapositive tombe, agrandie 10 fois, sur
/// le carré de façade de coin (x, y) : 2 800 mm de côté.
fn pairs_at(x: f64, y: f64, count: usize) -> Vec<RegistrationPair> {
    let slide = [
        (0.0, 0.0),
        (280.0, 0.0),
        (280.0, 280.0),
        (0.0, 280.0),
        (140.0, 100.0),
        (60.0, 200.0),
    ];
    slide[..count]
        .iter()
        .map(|&(s, t)| {
            RegistrationPair::new(PointMm::new(s, t), PointMm::new(x + 10.0 * s, y + 10.0 * t))
        })
        .collect()
}

fn project(temp: &TempDir) -> (ProjectDir, Project) {
    let (dir, mut project) =
        ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).expect("projet");
    project.facade = Some(Facade {
        source: RelativePath::new("facade/plan.png").unwrap(),
        source_width: 10_000.0,
        source_height: 5_000.0,
        scale: Some(FacadeScale {
            point_a: SourcePoint::new(0.0, 0.0),
            point_b: SourcePoint::new(1000.0, 0.0),
            distance_mm: 1000.0,
            extra: ExtraFields::new(),
        }),
        page: None,
        origin: None,
        rotation: 0,
        extra: ExtraFields::new(),
    });
    project.add_projector("A").registration = pairs_at(1000.0, 500.0, 6);
    project.add_projector("B").registration = pairs_at(3000.0, 500.0, 4);
    project.add_projector("C").registration = pairs_at(9000.0, 500.0, 3);
    (dir, project)
}

#[test]
fn footprints_meshes_and_overlaps_of_registered_projectors() {
    let temp = TempDir::new("vue-facade");
    let (dir, project) = project(&temp);
    let view = overview(&dir, &project, None);
    assert_eq!(view.projectors.len(), 3);
    let a = &view.projectors[0];
    assert!((a.area_m2.unwrap() - 7.84).abs() < 1e-6, "{:?}", a.area_m2);
    let mesh = a.mesh.as_ref().unwrap();
    assert_eq!(mesh.facade.len(), (MESH_DIVISIONS + 1).pow(2));
    // Dernier point : coin bas-droit de la diapositive.
    let last = mesh.facade.last().unwrap();
    assert!((last.x - 3800.0).abs() < 1e-6 && (last.y - 3300.0).abs() < 1e-6);
    let c = &view.projectors[2];
    assert!(c.footprint.is_empty() && c.mesh.is_none());
    assert!(c.problem.as_deref().unwrap().contains("4 points"));

    // A et B se chevauchent sur 800 × 2 800 mm ; C ne touche personne.
    assert_eq!(view.overlaps.len(), 1);
    let overlap = &view.overlaps[0];
    assert_eq!(
        overlap.projectors,
        [
            a.projector_id.clone(),
            view.projectors[1].projector_id.clone()
        ]
    );
    assert!(
        (overlap.area_m2 - 2.24).abs() / 2.24 < 0.01,
        "{}",
        overlap.area_m2
    );
    assert_eq!(overlap.outlines.len(), 1);
    let (x0, _, x1, _) = bounds(&overlap.outlines[0]);
    assert!(
        (x0 - 3000.0).abs() < 10.0 && (x1 - 3800.0).abs() < 10.0,
        "{x0} {x1}"
    );
}

#[test]
fn a_concave_overlap_is_measured_exactly_enough() {
    // Un L (3 × 3 m moins la bande x de 2 à 3, y de 0 à 2) et un carré qui
    // couvre cette bande et la bande voisine de l'L : 1 × 2 m en commun.
    let l = [
        (0.0, 0.0),
        (2.0, 0.0),
        (2.0, 2.0),
        (3.0, 2.0),
        (3.0, 3.0),
        (0.0, 3.0),
    ]
    .map(|(x, y)| PointMm::new(x * 1000.0, y * 1000.0));
    let square = [(1.0, 0.0), (3.0, 0.0), (3.0, 2.0), (1.0, 2.0)]
        .map(|(x, y)| PointMm::new(x * 1000.0, y * 1000.0));
    let (area, outlines) = intersection(&l, &square).unwrap();
    assert!((area - 2.0e6).abs() / 2.0e6 < 0.01, "{area}");
    assert_eq!(outlines.len(), 1);
    assert_eq!(polygon_area(&l), 7.0e6);
    let far = square.map(|p| PointMm::new(p.x + 10_000.0, p.y));
    assert!(intersection(&l, &far).is_none());
}

#[test]
fn the_rectified_photo_gets_a_small_preview_in_the_cache() {
    let temp = TempDir::new("vue-facade-photo");
    let (dir, mut project) = project(&temp);
    fs::create_dir_all(dir.root().join("photos-redressees")).unwrap();
    image::save_buffer(
        dir.root().join("photos-redressees/a.png"),
        &vec![90u8; 3000 * 3000],
        3000,
        3000,
        image::ColorType::L8,
    )
    .unwrap();
    project.projectors[0].rectified_photo =
        Some(RelativePath::new("photos-redressees/a.png").unwrap());
    let cache = temp.path().join("cache");
    let view = overview(&dir, &project, Some(&cache));
    let request = view.projectors[0].photo_preview.as_deref().expect("aperçu");
    let preview = image::open(cache.join(request)).unwrap();
    assert_eq!((preview.width(), preview.height()), (750, 750));
    assert!(
        view.projectors[1].photo_preview.is_none(),
        "B n'a pas de photo"
    );
    // Une seconde fois : le même fichier, sans le refaire.
    assert_eq!(
        overview(&dir, &project, Some(&cache)).projectors[0]
            .photo_preview
            .as_deref(),
        Some(request)
    );
}

#[test]
fn ten_projectors_with_many_points_are_computed_quickly() {
    let temp = TempDir::new("vue-facade-dix");
    let (dir, mut project) = project(&temp);
    project.projectors.clear();
    for k in 0..10 {
        // Dix diapositives côte à côte, chacune chevauchant sa voisine, avec
        // 12 points dont un léger relief.
        let mut pairs = pairs_at(2000.0 * k as f64, 500.0, 6);
        for extra in 0..6 {
            let (s, t) = (30.0 + 40.0 * extra as f64, 50.0 + 30.0 * extra as f64);
            let bump = 15.0 * (s / 280.0 * std::f64::consts::PI).sin();
            pairs.push(RegistrationPair::new(
                PointMm::new(s, t),
                PointMm::new(2000.0 * k as f64 + 10.0 * s, 500.0 + 10.0 * t + bump),
            ));
        }
        project.add_projector(format!("P{k}")).registration = pairs;
    }
    let start = std::time::Instant::now();
    let view = overview(&dir, &project, None);
    let seconds = start.elapsed().as_secs_f64();
    assert!(view.projectors.iter().all(|p| p.mesh.is_some()));
    assert_eq!(
        view.overlaps.len(),
        9,
        "chaque projecteur chevauche son voisin"
    );
    assert!(seconds < 2.0, "vue façade trop lente : {seconds:.2} s");
}

#[test]
fn an_unregistered_photo_gets_a_preview_too() {
    // Pose par les coins (D-3.2-h) : la photo se montre avant le 4e point.
    let temp = TempDir::new("vue-facade-apercu-sans-recalage");
    let (dir, mut project) = project(&temp);
    fs::create_dir_all(dir.root().join("photos-redressees")).unwrap();
    image::save_buffer(
        dir.root().join("photos-redressees/c.png"),
        &vec![128u8; 200 * 200],
        200,
        200,
        image::ColorType::L8,
    )
    .unwrap();
    project.projectors[2].rectified_photo =
        Some(RelativePath::new("photos-redressees/c.png").unwrap());
    let cache = temp.path().join("cache");
    let view = overview(&dir, &project, Some(&cache));
    let c = &view.projectors[2];
    assert!(c.mesh.is_none(), "C n'a que 3 points");
    assert!(c.photo_preview.is_some(), "mais sa photo a un aperçu");
}

#[test]
fn the_preview_of_unsaved_pairs_matches_the_overview() {
    let temp = TempDir::new("vue-facade-provisoire");
    let (dir, project) = project(&temp);
    let view = overview(&dir, &project, None);
    let a = registration_preview(&project.projectors[0].registration);
    assert_eq!(a.footprint, view.projectors[0].footprint);
    assert_eq!(a.mesh, view.projectors[0].mesh);
    let c = registration_preview(&project.projectors[2].registration);
    assert!(
        c.footprint.is_empty() && c.mesh.is_none(),
        "3 points : rien"
    );
}

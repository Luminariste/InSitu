use super::*;
use crate::facade::{display::render_whole, import_facade, inspect, set_scale};
use crate::project::test_support::TempDir;
use crate::project::{Drawing, RegistrationPair, RelativePath, SourcePoint};

/// Une diapositive posée sur la façade, agrandie 10 fois, coin haut-gauche
/// en (`x`, 200) mm.
fn pairs(x: f64) -> Vec<RegistrationPair> {
    [(0.0, 0.0), (280.0, 0.0), (280.0, 280.0), (0.0, 280.0)]
        .iter()
        .map(|&(sx, sy)| {
            RegistrationPair::new(
                PointMm::new(sx, sy),
                PointMm::new(x + sx * 10.0, 200.0 + sy * 10.0),
            )
        })
        .collect()
}

/// Un projet complet : un plan de 5 000 × 3 200 mm (image grise avec un
/// cadre), sept projecteurs (dont un pas recalé), un dessin posé dans le
/// premier, un dessin à cheval, un dessin hors emprise.
fn project(temp: &TempDir) -> ProjectDir {
    let (dir, _) = ProjectDir::create(temp.path().join("Essai.insitu"), Some("Épinal")).unwrap();
    // Le plan : 500 × 320 px, 1 px = 10 mm après mise à l'échelle.
    let (w, h) = (500u32, 320u32);
    let plan = GrayImage::from_fn(w, h, |x, y| {
        let edge = x < 4 || y < 4 || x >= w - 4 || y >= h - 4;
        image::Luma([if edge { 0 } else { 230 }])
    });
    let plan_path = temp.path().join("plan.png");
    plan.save(&plan_path).unwrap();
    import_facade(&dir, &plan_path, None).unwrap();
    set_scale(
        &dir,
        SourcePoint { x: 0.0, y: 0.0 },
        SourcePoint { x: 500.0, y: 0.0 },
        5_000.0,
    )
    .unwrap();
    let mut project = dir.load().unwrap().project;
    for (k, x) in [0.0, 2_000.0, 4_000.0, 6_000.0, 8_000.0, 10_000.0]
        .iter()
        .enumerate()
    {
        project.add_projector(format!("P{}", k + 1)).registration = pairs(*x);
    }
    project.add_projector("P7");
    // Le dessin : un anneau blanc sur noir.
    let side = 200u32;
    let ring = GrayImage::from_fn(side, side, |x, y| {
        let r = (f64::from(x) - 100.0).hypot(f64::from(y) - 100.0);
        image::Luma([if (r - 70.0).abs() < 8.0 { 255 } else { 0 }])
    });
    let file = dir.resolve(&RelativePath::new("bibliotheque/anneau.png").unwrap());
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    ring.save(&file).unwrap();
    project.library.push(Drawing {
        id: "anneau".into(),
        name: "anneau".into(),
        source: RelativePath::new("bibliotheque/anneau.png").unwrap(),
        width: f64::from(side),
        height: f64::from(side),
        vector: None,
        silhouette: None,
        extra: Default::default(),
    });
    for (id, x, size) in [
        ("bien", 900.0, 1_200.0),
        ("coupe", 2_400.0, 1_400.0),
        ("dehors", 20_000.0, 500.0),
    ] {
        project.placements.push(Placement {
            id: id.into(),
            drawing: "anneau".into(),
            center: PointMm::new(x, 1_600.0),
            width: size,
            height: size,
            rotation: 0.0,
            mirrored: false,
            group: None,
            layer: None,
            deformation: None,
            projector: None,
            adjustment: None,
            extra: Default::default(),
        });
    }
    dir.save(&project).unwrap();
    dir
}

fn pixels_below(level: u8, rgba: &[u8], width: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> usize {
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * width + x) * 4) as usize;
            if rgba[i] < level && rgba[i + 1] < level && rgba[i + 2] < level {
                n += 1;
            }
        }
    }
    n
}

fn dark_pixels(rgba: &[u8], width: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> usize {
    pixels_below(100, rgba, width, x0, y0, x1, y1)
}

/// Critère de la tâche 6.2, pour ce qui se vérifie sans imprimante : la
/// fiche est un PDF A3 lisible par un lecteur PDF (relu ici par le même
/// moteur de rendu que les plans), plan d'abord puis les diapositives six
/// par page ; aucun texte sous 3 mm de corps ; la vignette de chaque
/// diapositive montre son dessin.
#[test]
fn the_layout_sheet_is_an_a3_pdf_with_the_plan_and_every_slide() {
    let temp = TempDir::new("fiche");
    let dir = project(&temp);
    let out = temp.path().join("fiche.pdf");
    let report = write_sheet(&dir, &temp.path().join("cache"), &out, None).unwrap();
    // Plan, puis 7 diapositives : 6 sur la page 2, 1 sur la page 3.
    assert_eq!(report.pages, 3);
    assert_eq!(report.slides, 7);
    assert!(report.smallest_text_mm >= MIN_TEXT_MM, "{report:?}");
    // Le dessin à cheval et le dessin hors emprise sont dans les alertes.
    assert!(report.alerts >= 2, "{report:?}");

    let info = inspect(&out, Some(3)).unwrap();
    assert_eq!(info.page_count, Some(3));
    assert!((info.width - 420.0 * PT_PER_MM).abs() < 0.5, "{info:?}");
    assert!((info.height - 297.0 * PT_PER_MM).abs() < 0.5, "{info:?}");

    // Page 1 rendue : le plan (éclairci) et les emprises sont dans le cadre
    // de gauche.
    let (w, h, page1) = render_whole(&out, Some(1), 1_680).unwrap();
    assert_eq!(w, 1_680);
    assert!((1_187..=1_189).contains(&h), "{h}");
    let px = f64::from(w) / 420.0;
    let at = |mm: f64| (mm * px) as u32;
    assert!(pixels_below(200, &page1, w, at(14.0), at(40.0), at(290.0), at(283.0)) > 2_000);

    // Page 2 : la vignette de P1 (en haut à gauche) montre l'anneau, en
    // traits sombres sur fond clair.
    let (w, _, page2) = render_whole(&out, Some(2), 1_680).unwrap();
    let thumb = dark_pixels(&page2, w, at(14.0), at(34.0), at(92.0), at(112.0));
    assert!(thumb > 300, "vignette de P1 : {thumb}");
    // P7 (pas recalé) est seul sur la page 3, avec son problème en clair.
    let (_, _, page3) = render_whole(&out, Some(3), 1_680).unwrap();
    assert!(dark_pixels(&page3, w, at(14.0), at(34.0), at(140.0), at(140.0)) > 100);
}

#[test]
fn texts_are_encoded_for_the_pdf_fonts_and_wrapped() {
    assert_eq!(winansi("Façade é"), b"Fa\xe7ade \xe9");
    assert_eq!(winansi("≥"), b"?");
    let lines = wrap(
        "un deux trois quatre cinq six sept huit neuf dix",
        3.0,
        30.0,
    );
    assert!(lines.len() > 1);
    assert!(lines.iter().all(|l| text_width(l, 3.0, false) <= 30.0));
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(20_769), (2026, 11, 12));
    let lines = settings_lines(&ExportSettings::default());
    assert_eq!(lines[1], "Image de 280 mm : 15433 px");
}

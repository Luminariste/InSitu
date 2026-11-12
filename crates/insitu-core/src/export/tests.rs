use super::*;
use crate::project::test_support::TempDir;
use crate::project::{PointMm, RegistrationPair};

/// Recalage d'essai : la diapositive tombe, agrandie environ 14 fois et en
/// trapèze, sur la façade ; deux points de plus ajoutent du relief.
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

const FRAME: RectMm = RectMm {
    x: 500.0,
    y: 300.0,
    width: 6_000.0,
    height: 5_000.0,
};

fn read_gray(path: &Path) -> Raster {
    to_gray(flatten_on_black(tiles::read_raster(path).unwrap()))
}

fn verre_a_reduit() -> Raster {
    read_gray(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../jeu-essai/epinal/reduits/verres/verre-a.png"),
    )
}

/// Fabrique une fresque de `width` px de large sur `FRAME` : la diapositive
/// `slide` (280 mm) posée sur la façade par `warp`. C'est le chemin inverse
/// de l'export.
fn fresco_from(slide: &Raster, warp: &PointWarp, width: u32) -> Raster {
    let height = (f64::from(width) * FRAME.height / FRAME.width).round() as u32;
    let mm_per_px = FRAME.width / f64::from(width);
    let slide_px = f64::from(slide.width) / SLIDE_SIZE_MM;
    warp_raster(
        slide,
        width,
        height,
        |p| {
            let facade = Point::new(FRAME.x + p.x * mm_per_px, FRAME.y + p.y * mm_per_px);
            let s = warp.apply_inverse(facade)?;
            Some(Point::new(s.x * slide_px, s.y * slide_px))
        },
        &WarpOptions::default(),
    )
    .unwrap()
}

/// Part des pixels « trait » (≥ 128) de `a` qui ont un pixel « trait » de
/// `b` à un pixel près (eux-mêmes ou un voisin).
fn lines_found(a: &Raster, b: &Raster) -> f64 {
    let w = a.width as usize;
    let h = a.height as usize;
    let lit = |r: &Raster, i: usize, j: usize| r.data[j * w + i] >= 128;
    let (mut total, mut found) = (0u64, 0u64);
    for j in 1..h - 1 {
        for i in 1..w - 1 {
            if !lit(a, i, j) {
                continue;
            }
            total += 1;
            let near = (j - 1..=j + 1).any(|y| (i - 1..=i + 1).any(|x| lit(b, x, y)));
            found += u64::from(near);
        }
    }
    found as f64 / total as f64
}

/// Décalage de `a` par rapport à `b` sur le carré de côté `size` au coin
/// `(x0, y0)`, en pixels : le déplacement entier (±3 px) qui accorde le
/// mieux les deux images, affiné au dixième par une parabole. `None` si le
/// carré est presque vide.
fn local_shift(a: &Raster, b: &Raster, x0: usize, y0: usize, size: usize) -> Option<(f64, f64)> {
    let w = a.width as usize;
    let score = |dx: i64, dy: i64| {
        let mut sum = 0.0;
        for j in y0..y0 + size {
            for i in x0..x0 + size {
                let (ai, aj) = ((i as i64 + dx) as usize, (j as i64 + dy) as usize);
                sum += f64::from(a.data[aj * w + ai]) * f64::from(b.data[j * w + i]);
            }
        }
        sum
    };
    if score(0, 0) < 1e6 {
        return None;
    }
    let mut best = (0, 0, f64::MIN);
    for dy in -3..=3 {
        for dx in -3..=3 {
            let v = score(dx, dy);
            if v > best.2 {
                best = (dx, dy, v);
            }
        }
    }
    let refine = |m: f64, c: f64, p: f64| {
        let d = m - 2.0 * c + p;
        if d.abs() < 1e-12 {
            0.0
        } else {
            0.5 * (m - p) / d
        }
    };
    let (dx, dy, c) = best;
    let fx = refine(score(dx - 1, dy), c, score(dx + 1, dy));
    let fy = refine(score(dx, dy - 1), c, score(dx, dy + 1));
    Some((dx as f64 + fx, dy as f64 + fy))
}

fn settings(dpi: f64) -> ExportSettings {
    ExportSettings {
        dpi,
        margin: false,
        ..ExportSettings::default()
    }
}

#[test]
fn plate_and_slide_sizes_at_1400_dpi() {
    assert_eq!(sizes_px(1400.0), (15_433, 551));
    assert_eq!(
        15_433 + 2 * 551,
        units::mm_to_px(units::PLATE_SIZE_MM, 1400.0)
    );
}

/// Critère de la tâche 3.5, en réduit : Verre-A posé sur une façade puis
/// exporté de nouveau redonne Verre-A.
#[test]
fn verre_a_survives_a_trip_through_the_facade() {
    let verre = verre_a_reduit();
    let warp = slide_to_facade(&pairs()).unwrap();
    let fresco = fresco_from(&verre, &warp, 8_000);
    let dpi = f64::from(verre.width) / SLIDE_SIZE_MM * units::MM_PER_INCH;
    let out = render_slide(&fresco, &FRAME, &warp, "", &settings(dpi), 0).unwrap();
    assert_eq!((out.width, out.height), (verre.width, verre.height));
    let found = lines_found(&out, &verre);
    let kept = lines_found(&verre, &out);
    // Décalage local, carré par carré (8 × 8 carrés de 200 px).
    let mut shift: f64 = 0.0;
    let mut patches = 0;
    let step = (verre.width as usize - 8) / 8;
    for y in 0..8 {
        for x in 0..8 {
            if let Some((dx, dy)) = local_shift(&out, &verre, 4 + x * step, 4 + y * step, step - 8)
            {
                shift = shift.max(dx.hypot(dy));
                patches += 1;
            }
        }
    }
    assert!(patches > 20, "{patches} carrés seulement");
    let mean = out
        .data
        .iter()
        .zip(&verre.data)
        .map(|(&a, &b)| f64::from(a.abs_diff(b)))
        .sum::<f64>()
        / out.data.len() as f64;
    println!(
        "aller-retour de Verre-A ({} px) : {:.2} % des traits exportés sont sur un trait de Verre-A, {:.2} % des traits de Verre-A sont retrouvés (à 1 px près) ; décalage local au pire {shift:.2} px ({patches} carrés) ; écart moyen {mean:.2} niveaux",
        verre.width,
        100.0 * found,
        100.0 * kept
    );
    // Dans le réduit, les traits n'ont qu'un ou deux pixels : le double
    // rééchantillonnage en fait passer une partie juste sous le seuil.
    assert!(
        found > 0.995 && kept > 0.9,
        "traits : {found:.4} / {kept:.4}"
    );
    assert!(shift < 0.25, "décalage : {shift:.2} px");
    assert!(mean < 3.0, "écart moyen : {mean:.2}");

    // Le miroir retourne l'image gauche-droite, rien d'autre.
    let mirrored = render_slide(
        &fresco,
        &FRAME,
        &warp,
        "",
        &ExportSettings {
            mirror: true,
            ..settings(dpi)
        },
        0,
    )
    .unwrap();
    let w = out.width as usize;
    let mut worst = 0;
    for (row, mirror_row) in out.data.chunks_exact(w).zip(mirrored.data.chunks_exact(w)) {
        for i in 0..w {
            worst = worst.max(row[i].abs_diff(mirror_row[w - 1 - i]));
        }
    }
    assert!(worst <= 3, "miroir : écart de {worst} niveaux");
}

#[test]
fn margin_negative_and_one_bit_png() {
    let warp = slide_to_facade(&pairs()).unwrap();
    let fresco = Raster::new(600, 500, 1, vec![255; 600 * 500]);
    let s = ExportSettings {
        dpi: 100.0,
        margin: true,
        negative: true,
        one_bit: true,
        ..ExportSettings::default()
    };
    let plate = render_slide(&fresco, &FRAME, &warp, "ESSAI - A", &s, 0).unwrap();
    let (side, margin) = sizes_px(100.0);
    assert_eq!(plate.width, side + 2 * margin);
    let n = plate.width as usize;
    let at = |i: usize, j: usize| plate.data[j * n + i];
    // Négatif : la fresque blanche devient noire, la marge vide devient blanche.
    assert_eq!(at(n / 2, n / 2), 0);
    assert_eq!(at(1, 1), 255);

    let bytes = encode_png(&plate, &s).unwrap();
    let decoder = png::Decoder::new(std::io::Cursor::new(&bytes));
    let reader = decoder.read_info().unwrap();
    let info = reader.info();
    assert_eq!(info.bit_depth, png::BitDepth::One);
    let dims = info.pixel_dims.unwrap();
    let size_mm = f64::from(info.width) / f64::from(dims.xppu) * 1000.0;
    assert!((size_mm - 300.0).abs() < 0.3, "{size_mm} mm");
    let back = image::load_from_memory(&bytes).unwrap().into_luma8();
    assert_eq!(back.as_raw(), &plate.data);
}

#[test]
fn a_project_exports_its_registered_projectors() {
    let temp = TempDir::new("export-diapositives");
    let (dir, mut project) =
        ProjectDir::create(temp.path().join("Épinal.insitu"), Some("Épinal")).unwrap();
    project.add_projector("A").registration = pairs();
    project.add_projector("B").registration = pairs()[..3].to_vec();
    dir.save(&project).unwrap();

    // Pas de fresque : rien à exporter.
    let out = temp.path().join("export");
    let none = export_slides(&dir, &out, &settings(100.0), None, 0, &|_| {});
    assert!(matches!(none, Err(ExportError::NoFresco)), "{none:?}");

    // Sans façade à l'échelle, la fresque a besoin d'un rectangle.
    let image = temp.path().join("fresque.png");
    let verre = verre_a_reduit();
    let fresco = fresco_from(&verre, &slide_to_facade(&pairs()).unwrap(), 1_500);
    image::save_buffer(
        &image,
        &fresco.data,
        fresco.width,
        fresco.height,
        image::ExtendedColorType::L8,
    )
    .unwrap();
    assert!(matches!(
        import_fresco(&dir, &image, None),
        Err(ExportError::NoFacadeScale)
    ));
    let project = import_fresco(&dir, &image, Some(FRAME)).unwrap();
    assert_eq!(
        project.fresco.as_ref().unwrap().source.as_str(),
        "fresque/fresque.png"
    );

    let steps = std::sync::Mutex::new(Vec::new());
    let s = ExportSettings {
        dpi: 200.0,
        mirror: true,
        ..ExportSettings::default()
    };
    let report = export_slides(&dir, &out, &s, None, 0, &|p| {
        steps.lock().unwrap().push(p.done)
    })
    .unwrap();
    assert_eq!(*steps.lock().unwrap(), vec![0, 1]);
    assert_eq!(report.slides.len(), 1);
    assert_eq!(report.skipped.len(), 1);
    assert!(report.skipped[0].reason.contains("3 paire"));
    let slide = &report.slides[0];
    assert_eq!(slide.path, out.join("Épinal-A.png"));
    assert!((slide.size_mm - 280.0).abs() < 0.2, "{}", slide.size_mm);
    let (w, h) = image::image_dimensions(&slide.path).unwrap();
    assert_eq!((w, h), (slide.size_px, slide.size_px));
    // Les réglages deviennent ceux du projet.
    assert_eq!(dir.load().unwrap().project.export, s);

    // Un projecteur inconnu est refusé.
    let unknown = export_slides(&dir, &out, &s, Some(&["Z".to_string()]), 0, &|_| {});
    assert!(matches!(unknown, Err(ExportError::UnknownProjector(_))));
}

#[test]
fn file_names_and_captions() {
    assert_eq!(slide_file_name("Épinal 2025", "A/1"), "Épinal 2025-A-1.png");
    let s = ExportSettings {
        mirror: true,
        negative: true,
        ..ExportSettings::default()
    };
    assert_eq!(
        caption("Épinal", "A", &s),
        "Épinal - A - 1400 DPI - MIROIR - NEGATIF"
    );
}

#[test]
fn slide_previews_are_computed_once_and_follow_the_registration() {
    use super::preview::slide_views;
    let temp = TempDir::new("vue-diapositive");
    let (dir, mut project) =
        ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).unwrap();
    project.add_projector("A").registration = pairs();
    project.add_projector("B").registration = pairs()[..2].to_vec();
    dir.save(&project).unwrap();
    let cache = temp.path().join("cache");

    // Sans fresque : chaque projecteur dit ce qui manque.
    let views = slide_views(&dir, &cache, 600).unwrap();
    assert!(views
        .iter()
        .all(|v| v.preview.is_none() && v.problem.is_some()));

    let image = temp.path().join("fresque.png");
    let verre = verre_a_reduit();
    let fresco = fresco_from(&verre, &slide_to_facade(&pairs()).unwrap(), 1_500);
    image::save_buffer(
        &image,
        &fresco.data,
        fresco.width,
        fresco.height,
        image::ExtendedColorType::L8,
    )
    .unwrap();
    import_fresco(&dir, &image, Some(FRAME)).unwrap();

    let views = slide_views(&dir, &cache, 600).unwrap();
    let a = &views[0];
    assert!(a.problem.is_none());
    let request = a.preview.clone().unwrap();
    assert!(request.starts_with("diapositives/"));
    assert!(views[1].problem.as_deref().unwrap().contains("2 paire"));
    // Blanc à opacité variable : l'opacité est le gris de la diapositive.
    let png = image::open(cache.join(&request)).unwrap().into_rgba8();
    assert_eq!(png.dimensions(), (600, 600));
    let warp = slide_to_facade(&pairs()).unwrap();
    let fresco =
        read_fresco(&dir.resolve(&dir.load().unwrap().project.fresco.unwrap().source)).unwrap();
    let gray = render_slide(
        &fresco,
        &FRAME,
        &warp,
        "",
        &settings(600.0 / SLIDE_SIZE_MM * units::MM_PER_INCH),
        0,
    )
    .unwrap();
    assert!(png
        .pixels()
        .zip(&gray.data)
        .all(|(p, &g)| p.0 == [255, 255, 255, g]));

    // Rien n'a changé : le même fichier est resservi, sans recalcul.
    let written = std::fs::metadata(cache.join(&request))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(
        slide_views(&dir, &cache, 600).unwrap()[0]
            .preview
            .as_deref(),
        Some(request.as_str())
    );
    assert_eq!(
        std::fs::metadata(cache.join(&request))
            .unwrap()
            .modified()
            .unwrap(),
        written
    );

    // Un recalage modifié donne un nouvel aperçu.
    let mut project = dir.load().unwrap().project;
    project.projectors[0].registration[4].facade.x += 50.0;
    dir.save(&project).unwrap();
    let moved = slide_views(&dir, &cache, 600).unwrap()[0]
        .preview
        .clone()
        .unwrap();
    assert_ne!(moved, request);
}

/// Projet avec un projecteur recalé, sans fresque, et le verre A réduit en
/// dessin de bibliothèque, posé droit sur le rectangle `frame` de la façade.
fn project_with_placed_drawing(label: &str, frame: &RectMm) -> (TempDir, ProjectDir, Raster) {
    use crate::project::{Drawing, Placement, RelativePath};
    let temp = TempDir::new(label);
    let (dir, mut project) =
        ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).unwrap();
    project.add_projector("A").registration = pairs();
    let verre = verre_a_reduit();
    let file = dir.resolve(&RelativePath::new("bibliotheque/verre-a.png").unwrap());
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    image::save_buffer(
        &file,
        &verre.data,
        verre.width,
        verre.height,
        image::ExtendedColorType::L8,
    )
    .unwrap();
    project.library.push(Drawing {
        id: "verre".into(),
        name: "verre-a".into(),
        source: RelativePath::new("bibliotheque/verre-a.png").unwrap(),
        width: f64::from(verre.width),
        height: f64::from(verre.height),
        vector: None,
        extra: Default::default(),
    });
    project.placements.push(Placement {
        id: "p1".into(),
        drawing: "verre".into(),
        center: PointMm::new(frame.x + frame.width / 2.0, frame.y + frame.height / 2.0),
        width: frame.width,
        height: frame.height,
        rotation: 0.0,
        mirrored: false,
        group: None,
        layer: None,
        deformation: None,
        projector: None,
        extra: Default::default(),
    });
    dir.save(&project).unwrap();
    (temp, dir, verre)
}

/// Un dessin posé sort dans la diapositive exactement comme la même image
/// importée en fresque sur le même rectangle : même chaîne, mêmes filtres.
#[test]
fn a_placed_drawing_is_exported_like_the_same_image_as_a_fresco() {
    let frame = RectMm {
        x: 1_800.0,
        y: 1_200.0,
        width: 2_400.0,
        height: 2_400.0 * 0.9,
    };
    let (temp, dir, verre) = project_with_placed_drawing("export-dessins", &frame);
    let frame = RectMm {
        height: 2_400.0 * f64::from(verre.height) / f64::from(verre.width),
        ..frame
    };
    // Le dessin garde ses proportions : on le pose sur le bon rectangle.
    let mut project = dir.load().unwrap().project;
    project.placements[0].height = frame.height;
    project.placements[0].center.y = frame.y + frame.height / 2.0;
    dir.save(&project).unwrap();

    let out = temp.path().join("export");
    let s = settings(100.0);
    let report = export_slides(&dir, &out, &s, None, 0, &|_| {}).unwrap();
    let exported = to_gray(tiles::read_raster(&report.slides[0].path).unwrap());
    let warp = slide_to_facade(&pairs()).unwrap();
    let reference = render_slide(&verre, &frame, &warp, "", &s, 0).unwrap();
    assert_eq!(
        (exported.width, exported.height),
        (reference.width, reference.height)
    );
    let lit = reference.data.iter().filter(|&&v| v >= 128).count();
    assert!(
        lit > 1_000,
        "le dessin doit tomber dans la diapositive : {lit}"
    );
    let worst = exported
        .data
        .iter()
        .zip(&reference.data)
        .map(|(&a, &b)| a.abs_diff(b))
        .max()
        .unwrap();
    assert!(worst <= 2, "écart maximal {worst} niveaux de gris");

    // Retourné en miroir, l'export suit aussi la fresque.
    let m = ExportSettings {
        mirror: true,
        ..s.clone()
    };
    let report = export_slides(&dir, &out, &m, None, 0, &|_| {}).unwrap();
    let exported = to_gray(tiles::read_raster(&report.slides[0].path).unwrap());
    let reference = render_slide(&verre, &frame, &warp, "", &m, 0).unwrap();
    assert!(exported
        .data
        .iter()
        .zip(&reference.data)
        .all(|(&a, &b)| a.abs_diff(b) <= 2));

    // Le dessin sur un calque masqué : plus rien à exporter.
    let mut project = dir.load().unwrap().project;
    project.layers.push(crate::project::Layer {
        id: "fond".into(),
        name: "Fond".into(),
        visible: false,
        locked: false,
        extra: Default::default(),
    });
    dir.save(&project).unwrap();
    let none = export_slides(&dir, &out, &s, None, 0, &|_| {});
    assert!(matches!(none, Err(ExportError::NoFresco)), "{none:?}");
}

/// Dessins et fresque se superposent en gardant le plus clair ; un dessin
/// hors de la diapositive n'est pas lu.
#[test]
fn drawings_add_to_the_fresco_and_outside_ones_are_ignored() {
    let frame = RectMm {
        x: 1_800.0,
        y: 1_200.0,
        width: 2_400.0,
        height: 2_400.0,
    };
    let (temp, dir, _) = project_with_placed_drawing("export-dessins-fresque", &frame);
    let warp = slide_to_facade(&pairs()).unwrap();
    // Fresque : le verre A sur toute l'emprise.
    let image = temp.path().join("fresque.png");
    let verre = verre_a_reduit();
    let fresco = fresco_from(&verre, &warp, 800);
    image::save_buffer(
        &image,
        &fresco.data,
        fresco.width,
        fresco.height,
        image::ExtendedColorType::L8,
    )
    .unwrap();
    import_fresco(&dir, &image, Some(FRAME)).unwrap();
    let mut project = dir.load().unwrap().project;
    // Un second dessin, loin de la diapositive, dont le fichier manque :
    // il n'est pas lu, donc pas d'erreur.
    let mut far = project.placements[0].clone();
    far.id = "p2".into();
    far.drawing = "absent".into();
    far.center = PointMm::new(50_000.0, 50_000.0);
    project.library.push(crate::project::Drawing {
        id: "absent".into(),
        source: crate::project::RelativePath::new("bibliotheque/absent.png").unwrap(),
        ..project.library[0].clone()
    });
    project.placements.push(far);
    dir.save(&project).unwrap();

    let s = settings(100.0);
    let out = temp.path().join("export");
    let report = export_slides(&dir, &out, &s, None, 0, &|_| {}).unwrap();
    let both = to_gray(tiles::read_raster(&report.slides[0].path).unwrap());
    let fresco_image = read_fresco(&dir.resolve(&project.fresco.unwrap().source)).unwrap();
    let alone = render_slide(&fresco_image, &FRAME, &warp, "", &s, 0).unwrap();
    // Jamais plus sombre que la fresque seule, et plus clair quelque part.
    assert!(both.data.iter().zip(&alone.data).all(|(&b, &a)| b >= a));
    let brighter = both
        .data
        .iter()
        .zip(&alone.data)
        .filter(|(&b, &a)| b > a.saturating_add(64))
        .count();
    assert!(brighter > 1_000, "{brighter}");

    // Le dessin lointain, rapproché, est lu : son fichier manque.
    let mut project = dir.load().unwrap().project;
    project.placements[1].center = project.placements[0].center;
    dir.save(&project).unwrap();
    let missing = export_slides(&dir, &out, &s, None, 0, &|_| {});
    assert!(
        matches!(&missing, Err(ExportError::Image(m)) if m.contains("verre-a")),
        "{missing:?}"
    );
}

/// La vue diapositive montre aussi les dessins posés, sans fresque, et se
/// refait quand un dessin bouge.
#[test]
fn slide_previews_show_placed_drawings() {
    use super::preview::slide_views;
    let frame = RectMm {
        x: 1_800.0,
        y: 1_200.0,
        width: 2_400.0,
        height: 2_400.0,
    };
    let (temp, dir, _) = project_with_placed_drawing("vue-diapositive-dessins", &frame);
    let cache = temp.path().join("cache");
    let views = slide_views(&dir, &cache, 600).unwrap();
    assert!(views[0].problem.is_none(), "{:?}", views[0].problem);
    let first = views[0].preview.clone().unwrap();
    let png = image::open(cache.join(&first)).unwrap().into_rgba8();
    assert!(png.pixels().filter(|p| p.0[3] >= 128).count() > 100);

    let mut project = dir.load().unwrap().project;
    project.placements[0].center.x += 100.0;
    dir.save(&project).unwrap();
    let moved = slide_views(&dir, &cache, 600).unwrap()[0]
        .preview
        .clone()
        .unwrap();
    assert_ne!(moved, first);
}

/// Un dessin déformé (tâche 4.4) : au repos, la grille ne change rien à
/// l'export ; ramenée à la moitié de sa taille par ses quatre coins, le
/// dessin occupe le quart de la place.
#[test]
fn a_deformed_drawing_is_exported_through_its_mesh() {
    use crate::placement::{deformation_frame, mesh};
    let frame = RectMm {
        x: 1_800.0,
        y: 1_200.0,
        width: 2_400.0,
        height: 2_400.0,
    };
    let (temp, dir, _) = project_with_placed_drawing("export-deforme", &frame);
    let s = settings(100.0);
    let out = temp.path().join("export");
    let lit = |dir: &ProjectDir| {
        let report = export_slides(dir, &out, &s, None, 0, &|_| {}).unwrap();
        to_gray(tiles::read_raster(&report.slides[0].path).unwrap())
    };
    let rigid = lit(&dir);

    let mut project = dir.load().unwrap().project;
    let p = project.placements[0].clone();
    project.placements[0].deformation =
        Some(mesh::identity(deformation_frame(&[&p]).unwrap(), 3, 3));
    dir.save(&project).unwrap();
    let rest = lit(&dir);
    assert!(rest
        .data
        .iter()
        .zip(&rigid.data)
        .all(|(&a, &b)| a.abs_diff(b) <= 2));

    let mut d = mesh::identity(deformation_frame(&[&p]).unwrap(), 1, 1);
    for n in &mut d.nodes {
        *n = [0.25 + n[0] * 0.5, 0.25 + n[1] * 0.5];
    }
    project.placements[0].deformation = Some(d);
    dir.save(&project).unwrap();
    let small = lit(&dir);
    let count = |r: &Raster| r.data.iter().filter(|&&v| v >= 128).count() as f64;
    let ratio = count(&small) / count(&rigid);
    assert!((0.15..0.4).contains(&ratio), "{ratio}");
}

/// Tâche 4.5 : un dessin dans la zone couverte par deux projecteurs ne part
/// que dans une diapositive, celle que propose le moteur ou celle choisie à
/// la main.
#[test]
fn a_drawing_in_an_overlap_goes_to_one_slide_only() {
    let frame = RectMm {
        x: 3_800.0,
        y: 2_000.0,
        width: 1_000.0,
        height: 900.0,
    };
    let (temp, dir, _) = project_with_placed_drawing("export-affectation", &frame);
    let mut project = dir.load().unwrap().project;
    // B : le même recalage décalé de 1,5 m vers la droite ; le dessin est
    // dans les deux emprises, plus loin du bord de B que du bord de A.
    let b: Vec<RegistrationPair> = pairs()
        .into_iter()
        .map(|p| RegistrationPair::new(p.slide, PointMm::new(p.facade.x + 1_500.0, p.facade.y)))
        .collect();
    project.add_projector("B").registration = b;
    dir.save(&project).unwrap();
    let ids = [
        project.projectors[0].id.clone(),
        project.projectors[1].id.clone(),
    ];

    let lit = |out: &Path| -> Vec<usize> {
        let s = settings(60.0);
        let report = export_slides(&dir, out, &s, None, 0, &|_| {}).unwrap();
        report
            .slides
            .iter()
            .map(|slide| {
                to_gray(tiles::read_raster(&slide.path).unwrap())
                    .data
                    .iter()
                    .filter(|&&v| v >= 128)
                    .count()
            })
            .collect()
    };
    let auto = lit(&temp.path().join("auto"));
    assert!(auto[0] == 0 && auto[1] > 50, "proposé à B seul : {auto:?}");

    crate::placement::assign(&dir, &["p1".to_string()], Some("A")).unwrap();
    let manual = lit(&temp.path().join("main"));
    assert!(
        manual[0] > 50 && manual[1] == 0,
        "choisi à la main : A seul : {manual:?}"
    );
    let a = crate::placement::assign::assignments(&dir.load().unwrap().project);
    assert_eq!(a[0].projector.as_deref(), Some(ids[0].as_str()));
    assert!(a[0].manual && a[0].whole);
    assert_eq!(a[0].candidates, ids.to_vec());
}

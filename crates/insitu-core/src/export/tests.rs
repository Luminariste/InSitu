use super::*;
use crate::project::test_support::TempDir;
use crate::project::{PointMm, RegistrationPair};

/// Recalage d'essai : la diapositive tombe, agrandie environ 14 fois et en
/// trapèze, sur la façade ; deux points de plus tombent sur le même trapèze
/// (la photo est posée par un trapèze seul, D-3.2-l).
fn pairs() -> Vec<RegistrationPair> {
    [
        ((0.0, 0.0), (1_000.0, 800.0)),
        ((280.0, 0.0), (5_100.0, 600.0)),
        ((280.0, 280.0), (5_300.0, 4_700.0)),
        ((0.0, 280.0), (900.0, 4_500.0)),
        ((140.0, 140.0), (2_965.158_371, 2_582.352_941)),
        ((70.0, 210.0), (1_924.083_770, 3_549.065_071)),
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

    // La fresque se retire, puis se réimporte (demande de Benjamin).
    let removed = remove_fresco(&dir).unwrap();
    assert!(removed.fresco.is_none());
    assert!(dir.load().unwrap().project.fresco.is_none());
    assert!(dir.root().join("fresque/fresque.png").exists());
    assert!(matches!(remove_fresco(&dir), Err(ExportError::NoFresco)));
    import_fresco(&dir, &image, Some(FRAME)).unwrap();

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

    // Fresque cachée (D-3.6-d) : un autre aperçu, vide puisqu'aucun dessin
    // n'est posé, et pas de « rien à montrer ».
    let hidden = &super::preview::slide_views_with(
        &dir,
        &cache,
        600,
        super::preview::SlideContent {
            fresco: false,
            plan: false,
        },
    )
    .unwrap()[0];
    assert!(hidden.problem.is_none(), "{:?}", hidden.problem);
    let empty = hidden.preview.clone().unwrap();
    assert_ne!(empty, moved);
    let png = image::open(cache.join(&empty)).unwrap().into_rgba8();
    assert!(png.pixels().all(|p| p.0[3] == 0));
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
        silhouette: None,
        retouch: Vec::new(),
        folder: None,
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
        adjustment: None,
        hidden: false,
        locked: false,
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
    // Sans silhouette (tâche 4.7), un dessin ne cache rien : il ajoute ses
    // traits à la fresque.
    project.library[0].silhouette = Some(Vec::new());
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

    // Case « Fresque » décochée (D-3.5-n) : les dessins seuls, exactement
    // comme si le projet n'avait pas de fresque ; le réglage est gardé.
    let without = ExportSettings {
        fresco: false,
        ..s.clone()
    };
    let out_drawings = temp.path().join("export-dessins-seuls");
    let report = export_slides(&dir, &out_drawings, &without, None, 0, &|_| {}).unwrap();
    let drawings_only = to_gray(tiles::read_raster(&report.slides[0].path).unwrap());
    assert!(!dir.load().unwrap().project.export.fresco);
    let mut bare = dir.load().unwrap().project;
    let kept_fresco = bare.fresco.take();
    dir.save(&bare).unwrap();
    let out_bare = temp.path().join("export-sans-fresque");
    let report = export_slides(&dir, &out_bare, &s, None, 0, &|_| {}).unwrap();
    let reference = to_gray(tiles::read_raster(&report.slides[0].path).unwrap());
    assert_eq!(drawings_only.data, reference.data);
    assert!(drawings_only.data != both.data);
    // Sans dessin ni fresque gravée, l'erreur le dit.
    bare = dir.load().unwrap().project;
    bare.fresco = kept_fresco;
    let placements = std::mem::take(&mut bare.placements);
    dir.save(&bare).unwrap();
    let nothing = export_slides(&dir, &out_bare, &without, None, 0, &|_| {});
    assert!(
        matches!(&nothing, Err(ExportError::Nothing(m)) if m.contains("Fresque")),
        "{nothing:?}"
    );
    bare.placements = placements;
    dir.save(&bare).unwrap();

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

/// Tâche 4.6 : la retouche d'un dessin dans une diapositive apparaît à
/// l'export de cette diapositive, et se retire d'un clic.
#[test]
fn a_slide_adjustment_shows_in_the_export_and_resets() {
    use crate::placement::adjust::{adjust, set_adjustment, slide_drawings};
    let frame = RectMm {
        x: 2_000.0,
        y: 1_500.0,
        width: 1_600.0,
        height: 1_400.0,
    };
    let (temp, dir, _) = project_with_placed_drawing("export-retouche", &frame);
    let s = settings(60.0);
    let px_mm = units::px_size_mm(s.dpi);
    let export = |name: &str| -> Raster {
        let report = export_slides(&dir, &temp.path().join(name), &s, None, 0, &|_| {}).unwrap();
        to_gray(tiles::read_raster(&report.slides[0].path).unwrap())
    };
    // Centre des pixels allumés, en mm de diapositive.
    let centroid = |r: &Raster| -> (f64, f64, usize) {
        let (mut sx, mut sy, mut n) = (0.0, 0.0, 0usize);
        for (k, &v) in r.data.iter().enumerate() {
            if v >= 128 {
                sx += (k % r.width as usize) as f64;
                sy += (k / r.width as usize) as f64;
                n += 1;
            }
        }
        (sx / n as f64 * px_mm, sy / n as f64 * px_mm, n)
    };
    let before = export("avant");
    let (x0, y0, lit) = centroid(&before);
    assert!(lit > 200, "le dessin doit être dans la diapositive : {lit}");

    // Une grille au repos ne change rien.
    let project = adjust(&dir, "p1", 1, 1).unwrap();
    let id = project.projectors[0].id.clone();
    let rest = export("repos");
    let worst = rest
        .data
        .iter()
        .zip(&before.data)
        .map(|(&a, &b)| a.abs_diff(b))
        .max()
        .unwrap();
    assert!(worst <= 2, "grille au repos : écart {worst}");

    // Tous les nœuds décalés de 12 mm vers la droite dans la diapositive :
    // le dessin exporté se décale d'autant, la façade ne bouge pas.
    let mut a = project.placements[0].adjustment.clone().unwrap();
    assert_eq!(a.projector, id);
    let w = a.deformation.frame.width;
    for node in &mut a.deformation.nodes {
        node[0] += 12.0 / w;
    }
    let project = set_adjustment(&dir, "p1", Some(a.clone())).unwrap();
    assert_eq!(project.placements[0].center.x, frame.x + frame.width / 2.0);
    let moved = export("decale");
    let (x1, y1, _) = centroid(&moved);
    assert!(
        (x1 - x0 - 12.0).abs() < 0.5 && (y1 - y0).abs() < 0.5,
        "décalage ({}, {})",
        x1 - x0,
        y1 - y0
    );
    // La vue Diapositive le situe avant sa retouche.
    let seen = slide_drawings(&project, &id);
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].points.len(), 17 * 17);
    // Le cadre d'une retouche neuve est le rectangle de ces points, comme
    // le calcule l'interface (`restFrame` dans src/slide/adjust.ts).
    let xs = seen[0].points.iter().map(|p| p.x);
    let (lo, hi) = (
        xs.clone().fold(f64::INFINITY, f64::min),
        xs.fold(f64::NEG_INFINITY, f64::max),
    );
    assert!((a.deformation.frame.width - (hi - lo)).abs() < 1e-9);

    // Pour un autre projecteur, la retouche ne compte pas.
    a.projector = "autre".into();
    set_adjustment(&dir, "p1", Some(a)).unwrap();
    assert_eq!(export("autre").data, before.data);

    // Un clic la retire : l'export redevient celui d'avant.
    adjust(&dir, "p1", 1, 1).unwrap();
    adjust(&dir, "p1", 0, 0).unwrap();
    assert!(dir.load().unwrap().project.placements[0]
        .adjustment
        .is_none());
    assert_eq!(export("apres").data, before.data);
}

/// Tâche 4.7 : de deux dessins superposés, celui de devant cache l'autre
/// sous sa silhouette, et cache aussi la fresque.
#[test]
fn the_front_drawing_hides_the_one_behind_under_its_silhouette() {
    use crate::project::{Drawing, RelativePath};
    let frame = RectMm {
        x: 2_000.0,
        y: 1_500.0,
        width: 2_000.0,
        height: 2_000.0,
    };
    let (temp, dir, _) = project_with_placed_drawing("export-silhouettes", &frame);
    // Derrière : des rayures sur tout le carré. Devant : un anneau ouvert
    // d'un petit trou (le trait d'un vrai dessin n'est pas toujours fermé).
    let side = 600u32;
    let stripes: Vec<u8> = (0..side * side)
        .map(|k| if (k / side) % 20 < 4 { 255 } else { 0 })
        .collect();
    let ring: Vec<u8> = (0..side * side)
        .map(|k| {
            let (x, y) = (f64::from(k % side) - 300.0, f64::from(k / side) - 300.0);
            let r = (x * x + y * y).sqrt();
            let gap = y.abs() < 4.0 && x > 0.0;
            if (r - 180.0).abs() < 5.0 && !gap {
                255
            } else {
                0
            }
        })
        .collect();
    let mut project = dir.load().unwrap().project;
    for (name, data) in [("rayures", &stripes), ("anneau", &ring)] {
        let source = RelativePath::new(format!("bibliotheque/{name}.png")).unwrap();
        image::save_buffer(
            dir.resolve(&source),
            data,
            side,
            side,
            image::ExtendedColorType::L8,
        )
        .unwrap();
        project.library.push(Drawing {
            id: name.into(),
            name: name.into(),
            source,
            width: f64::from(side),
            height: f64::from(side),
            vector: None,
            silhouette: None,
            retouch: Vec::new(),
            folder: None,
            extra: Default::default(),
        });
    }
    // Deux placements sur le même carré : les rayures, puis l'anneau devant.
    let mut front = project.placements[0].clone();
    project.placements[0].drawing = "rayures".into();
    front.id = "p2".into();
    front.drawing = "anneau".into();
    project.placements.push(front);
    dir.save(&project).unwrap();

    let s = settings(100.0);
    let px_mm = units::px_size_mm(s.dpi);
    let warp = slide_to_facade(&pairs()).unwrap();
    // Valeur moyenne de l'export autour d'un point du dessin (u, v).
    let export = |name: &str| -> Raster {
        let report = export_slides(&dir, &temp.path().join(name), &s, None, 0, &|_| {}).unwrap();
        to_gray(tiles::read_raster(&report.slides[0].path).unwrap())
    };
    let mean = |r: &Raster, u: f64, v: f64| -> f64 {
        let facade = Point::new(frame.x + u * frame.width, frame.y + v * frame.height);
        let p = warp.apply_inverse(facade).unwrap();
        let (cx, cy) = ((p.x / px_mm) as i64, (p.y / px_mm) as i64);
        let mut sum = 0.0;
        for y in cy - 8..=cy + 8 {
            for x in cx - 8..=cx + 8 {
                sum += f64::from(r.data[(y * i64::from(r.width) + x) as usize]);
            }
        }
        sum / 289.0
    };
    let shown = export("devant");
    // Rayures : environ un cinquième de blanc. Dans l'anneau, plus rien ;
    // hors de l'anneau, elles restent.
    let outside = mean(&shown, 0.08, 0.5);
    assert!(outside > 30.0, "rayures hors de l'anneau : {outside}");
    for (u, v) in [(0.5, 0.5), (0.35, 0.4), (0.62, 0.6), (0.7, 0.5)] {
        let inside = mean(&shown, u, v);
        assert!(
            inside < 2.0,
            "rayures dans l'anneau en ({u}, {v}) : {inside}"
        );
    }
    // Le trait de l'anneau lui-même est là.
    let edge = mean(&shown, 0.5, 0.2);
    assert!(edge > 20.0, "trait de l'anneau : {edge}");

    // Silhouette retouchée à vide : l'anneau ne cache plus rien.
    let mut project = dir.load().unwrap().project;
    project
        .library
        .iter_mut()
        .find(|d| d.id == "anneau")
        .unwrap()
        .silhouette = Some(Vec::new());
    dir.save(&project).unwrap();
    let plain = export("sans");
    assert!(mean(&plain, 0.5, 0.5) > 30.0);

    // L'ordre inversé : les rayures devant cachent l'anneau en entier
    // (leur silhouette est tout le carré). L'export est celui des rayures
    // seules.
    let mut project = dir.load().unwrap().project;
    project.library.iter_mut().for_each(|d| d.silhouette = None);
    project.placements.swap(0, 1);
    dir.save(&project).unwrap();
    let swapped = export("derriere");
    project.placements.remove(0);
    dir.save(&project).unwrap();
    let alone = export("seules");
    let worst = swapped
        .data
        .iter()
        .zip(&alone.data)
        .map(|(&a, &b)| a.abs_diff(b))
        .max()
        .unwrap();
    assert!(
        worst <= 2,
        "anneau visible derrière les rayures : écart {worst}"
    );
}

/// Avec l'option SVG, chaque diapositive a son SVG en millimètres à côté
/// du PNG, avec un chemin par dessin posé ; sans l'option, aucun.
#[test]
fn the_svg_option_writes_one_svg_per_slide() {
    let frame = RectMm {
        x: 1_800.0,
        y: 1_200.0,
        width: 2_400.0,
        height: 2_400.0 * 0.9,
    };
    let (temp, dir, _) = project_with_placed_drawing("export-svg", &frame);
    let out = temp.path().join("export");
    let s = ExportSettings {
        svg: true,
        ..settings(100.0)
    };
    let report = export_slides(&dir, &out, &s, None, 0, &|_| {}).unwrap();
    let slide = &report.slides[0];
    let svg_path = slide.svg.as_ref().expect("SVG écrit");
    assert_eq!(svg_path.with_extension("png"), slide.path);
    let svg = std::fs::read_to_string(svg_path).unwrap();
    assert!(svg.contains("width=\"280mm\"") && svg.contains("viewBox=\"0 0 280 280\""));
    assert_eq!(svg.matches("<path ").count(), 1);
    assert!(svg.contains("id=\"p1\""));

    let report = export_slides(&dir, &out, &settings(100.0), None, 0, &|_| {}).unwrap();
    assert!(report.slides[0].svg.is_none());
}

// ---------------------------------------------------------------------------
// Fresque vectorielle : PDF, Illustrator, SVG
// ---------------------------------------------------------------------------

/// Page de 600 × 500 points (les proportions de `FRAME`), noire sauf un
/// rectangle blanc : u de 150 à 450, v de 100 à 300 (origine en haut à
/// gauche). Sur `FRAME`, 1 point = 10 mm : le rectangle couvre
/// x de 2 000 à 5 000 mm et y de 1 300 à 3 300 mm.
fn fresco_pdf() -> Vec<u8> {
    use pdf_writer::{Content, Pdf, Rect, Ref};
    let mut pdf = Pdf::new();
    let (catalog, tree, page, content) = (Ref::new(1), Ref::new(2), Ref::new(3), Ref::new(4));
    pdf.catalog(catalog).pages(tree);
    pdf.pages(tree).kids([page]).count(1);
    {
        let mut p = pdf.page(page);
        p.media_box(Rect::new(0.0, 0.0, 600.0, 500.0));
        p.parent(tree);
        p.contents(content);
    }
    let mut c = Content::new();
    c.set_fill_rgb(1.0, 1.0, 1.0);
    // Le PDF compte y vers le haut : v de 100 à 300 → y de 200 à 400.
    c.rect(150.0, 200.0, 300.0, 200.0);
    c.fill_nonzero();
    pdf.stream(content, &c.finish());
    pdf.finish()
}

const WHITE_RECT: (f64, f64, f64, f64) = (2_000.0, 1_300.0, 5_000.0, 3_300.0);

/// Distance signée (mm) d'un point de façade au bord du rectangle blanc :
/// positive dedans, négative dehors.
fn inside_white_rect(p: Point) -> f64 {
    let (x0, y0, x1, y1) = WHITE_RECT;
    (p.x - x0).min(x1 - p.x).min(p.y - y0).min(y1 - p.y)
}

/// Chaque pixel de la diapositive est blanc dans le rectangle et noir
/// dehors, à `tolerance_px` pixels près du bord ; rend le nombre de pixels
/// blancs.
fn check_white_rect(slide: &Raster, warp: &PointWarp, dpi: f64, tolerance_px: f64) -> usize {
    let px_mm = units::px_size_mm(dpi);
    let mut white = 0;
    for j in (0..slide.height).step_by(3) {
        for i in (0..slide.width).step_by(3) {
            let s = Point::new((f64::from(i) + 0.5) * px_mm, (f64::from(j) + 0.5) * px_mm);
            let f = warp.apply(s).unwrap();
            // Taille locale d'un pixel de diapositive sur la façade.
            let g = warp.apply(Point::new(s.x + px_mm, s.y)).unwrap();
            let local = (g.x - f.x).hypot(g.y - f.y);
            let d = inside_white_rect(f) / local;
            let v = slide.data[(j * slide.width + i) as usize];
            if d > tolerance_px {
                assert!(v >= 250, "pixel ({i}, {j}) gris {v} au lieu de blanc");
                white += 1;
            } else if d < -tolerance_px {
                assert!(v <= 5, "pixel ({i}, {j}) gris {v} au lieu de noir");
            }
        }
    }
    white
}

#[test]
fn an_illustrator_file_is_imported_and_exported_sharp() {
    let temp = TempDir::new("fresque-ai");
    let (dir, mut project) =
        ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).unwrap();
    project.add_projector("A").registration = pairs();
    dir.save(&project).unwrap();
    // Un .ai « compatible PDF » est un PDF : le nom ne change rien.
    let ai = temp.path().join("Composition.ai");
    std::fs::write(&ai, fresco_pdf()).unwrap();

    let project = import_fresco(&dir, &ai, Some(FRAME)).unwrap();
    let fresco = project.fresco.as_ref().unwrap();
    assert_eq!(fresco.source.as_str(), "fresque/Composition-apercu.png");
    let vector = fresco.vector.as_ref().unwrap();
    assert_eq!(vector.source.as_str(), "fresque/Composition.ai");
    assert_eq!(vector.page, None);
    assert_eq!(fresco.frame, FRAME);

    // L'aperçu : 8 192 px de large, le rectangle blanc à sa place.
    let display = read_gray(&dir.resolve(&fresco.source));
    assert_eq!((display.width, display.height), (8_192, 6_827));
    let at = |u: f64, v: f64| {
        let k = 8_192.0 / 600.0;
        display.data[((v * k) as u32 * display.width + (u * k) as u32) as usize]
    };
    assert_eq!(at(300.0, 200.0), 255);
    assert_eq!(at(100.0, 200.0), 0);
    assert_eq!(at(300.0, 400.0), 0);

    // Le projet relu est valide et garde le fichier d'origine.
    let reloaded = dir.load().unwrap().project;
    assert_eq!(reloaded.fresco, project.fresco);

    // L'export relit le vectoriel : bords nets, à leur place.
    let out = temp.path().join("export");
    let report = export_slides(&dir, &out, &settings(200.0), None, 0, &|_| {}).unwrap();
    let slide = read_gray(&report.slides[0].path);
    let warp = slide_to_facade(&pairs()).unwrap();
    let white = check_white_rect(&slide, &warp, 200.0, 1.5);
    assert!(white > 10_000, "{white} pixels blancs");
}

#[test]
fn the_vector_crop_has_the_resolution_of_the_slide() {
    let temp = TempDir::new("fresque-decoupe");
    let pdf = temp.path().join("f.pdf");
    std::fs::write(&pdf, fresco_pdf()).unwrap();
    let vector = fresco::VectorFresco::open(&pdf, 1).unwrap();
    assert_eq!(vector.size(), (600.0, 500.0));
    let warp = slide_to_facade(&pairs()).unwrap();
    let px_mm = units::px_size_mm(1_400.0);
    let (raster, frame) = vector.crop_for_slide(&FRAME, &warp, px_mm, 0).unwrap();
    // Le morceau couvre l'emprise (≈ 900 à 5 300 mm de large)...
    assert!(
        frame.x < 900.0 && frame.x + frame.width > 5_300.0,
        "{frame:?}"
    );
    // ... à un pixel de fresque par pixel de diapositive, au plus fin :
    // la diapositive, agrandie 14 à 15 fois, fait ≈ 0,26 mm par pixel.
    let mm_per_px = frame.width / f64::from(raster.width);
    assert!(mm_per_px > 0.2 && mm_per_px < 0.27, "{mm_per_px}");
    assert!(raster.width <= fresco::MAX_CROP_SIDE_PX);

    // Une diapositive hors de la fresque n'en rend rien.
    let far: Vec<_> = pairs()
        .iter()
        .map(|p| RegistrationPair::new(p.slide, PointMm::new(p.facade.x + 50_000.0, p.facade.y)))
        .collect();
    let far = slide_to_facade(&far).unwrap();
    assert!(vector.crop_for_slide(&FRAME, &far, px_mm, 0).is_none());
}

#[test]
fn an_svg_fresco_and_a_pdf_page_choice() {
    let temp = TempDir::new("fresque-svg");
    let (dir, mut project) =
        ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).unwrap();
    project.add_projector("A").registration = pairs();
    dir.save(&project).unwrap();
    let svg = temp.path().join("fresque.svg");
    std::fs::write(
        &svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="600" height="500">
             <rect x="150" y="100" width="300" height="200" fill="white"/>
           </svg>"#,
    )
    .unwrap();
    let project = import_fresco(&dir, &svg, Some(FRAME)).unwrap();
    let vector = project.fresco.as_ref().unwrap().vector.as_ref().unwrap();
    assert_eq!(vector.source.as_str(), "fresque/fresque.svg");
    let out = temp.path().join("export");
    let report = export_slides(&dir, &out, &settings(200.0), None, 0, &|_| {}).unwrap();
    let slide = read_gray(&report.slides[0].path);
    let white = check_white_rect(&slide, &slide_to_facade(&pairs()).unwrap(), 200.0, 1.5);
    assert!(white > 10_000, "{white} pixels blancs");

    // Une page qui n'existe pas est refusée ; la page 1 est notée.
    let pdf = temp.path().join("f.pdf");
    std::fs::write(&pdf, fresco_pdf()).unwrap();
    assert!(matches!(
        import_fresco_page(&dir, &pdf, Some(FRAME), Some(2)),
        Err(ExportError::BadSettings(_))
    ));
    let project = import_fresco_page(&dir, &pdf, Some(FRAME), Some(1)).unwrap();
    assert_eq!(project.fresco.unwrap().vector.unwrap().page, Some(1));
}

#[test]
fn an_old_illustrator_file_gets_a_clear_message() {
    let temp = TempDir::new("fresque-ps");
    let ai = temp.path().join("ancien.ai");
    std::fs::write(&ai, b"%!PS-Adobe-3.0\n%%Creator: Adobe Illustrator\n").unwrap();
    let message = fresco::format_of(&ai).unwrap_err().to_string();
    assert!(message.contains("compatible PDF"), "{message}");
    let fake = temp.path().join("faux.pdf");
    std::fs::write(&fake, b"pas un pdf").unwrap();
    assert!(fresco::format_of(&fake).is_err());
    let png = temp.path().join("image.png");
    std::fs::write(&png, b"\x89PNG\r\n").unwrap();
    assert_eq!(
        fresco::format_of(&png).unwrap(),
        fresco::FrescoFormat::Raster
    );
}

/// Le plan dans la vue diapositive (D-3.6-e) : demandé, il est ramené dans
/// le carré de la diapositive par le recalage ; un trait du plan tombe là où
/// le recalage l'attend. Non demandé, il n'est pas calculé.
#[test]
fn the_facade_plan_is_seen_through_the_registration() {
    plan_seen_through_registration(false);
}

/// Même chose avec un plan vectoriel (SVG), rendu à la demande.
#[test]
fn a_vector_plan_is_seen_through_the_registration() {
    plan_seen_through_registration(true);
}

fn plan_seen_through_registration(vector: bool) {
    use super::preview::{slide_views_with, SlideContent};
    use crate::facade::import::{import_facade, set_scale};
    use crate::project::SourcePoint;
    let temp = TempDir::new("vue-diapositive-plan");
    let (dir, mut project) =
        ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).unwrap();
    project.add_projector("A").registration = pairs();
    dir.save(&project).unwrap();
    // Plan de 7 000 × 6 000 mm (1 px = 10 mm) : un trait noir vertical à
    // x = 3 080 mm, là où tombe le centre de la diapositive.
    let plan_path = if vector {
        let path = temp.path().join("plan.svg");
        std::fs::write(
            &path,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="700" height="600" viewBox="0 0 700 600"><rect x="307" y="0" width="3" height="600" fill="black"/></svg>"#,
        )
        .unwrap();
        path
    } else {
        let plan = image::GrayImage::from_fn(700, 600, |x, _| {
            image::Luma([if (307..=309).contains(&x) { 0 } else { 255 }])
        });
        let path = temp.path().join("plan.png");
        plan.save(&path).unwrap();
        path
    };
    import_facade(&dir, &plan_path, None).unwrap();
    set_scale(
        &dir,
        SourcePoint { x: 0.0, y: 0.0 },
        SourcePoint { x: 700.0, y: 0.0 },
        7_000.0,
    )
    .unwrap();
    let cache = temp.path().join("cache");

    let without = slide_views_with(&dir, &cache, 600, SlideContent::default()).unwrap();
    assert!(without[0].plan.is_none());

    let content = SlideContent {
        fresco: true,
        plan: true,
    };
    let views = slide_views_with(&dir, &cache, 600, content).unwrap();
    let request = views[0].plan.clone().expect("plan vu par A");
    assert!(request.starts_with("diapositives/plan-"));
    let png = image::open(cache.join(&request)).unwrap().into_rgba8();
    assert_eq!(png.dimensions(), (600, 600));
    // Le centre de la diapositive (140 mm → 300 px) est sur le trait.
    let row = 300;
    let lit: Vec<u32> = (0..600)
        .filter(|&x| png.get_pixel(x, row).0[3] >= 128)
        .collect();
    assert!(!lit.is_empty(), "le trait manque");
    assert!(lit.iter().all(|&x| (280..=320).contains(&x)), "{lit:?}");
    // Le reste est transparent : seul le trait se voit.
    let opaque = png.pixels().filter(|p| p.0[3] >= 128).count();
    assert!(opaque < 600 * 600 / 10, "{opaque}");
    // Resservi depuis le cache.
    let again = slide_views_with(&dir, &cache, 600, content).unwrap();
    assert_eq!(again[0].plan.as_deref(), Some(request.as_str()));

    // Le plan de contrôle à graver (D-3.5-m) : la même image que l'aperçu,
    // traits blancs sur noir, dans son propre fichier ; les réglages
    // d'export du projet ne changent pas.
    let before = dir.load().unwrap().project.export;
    let out = temp.path().join("export");
    let flat = ExportSettings {
        dpi: 600.0 / SLIDE_SIZE_MM * units::MM_PER_INCH,
        margin: false,
        mirror: false,
        negative: false,
        one_bit: false,
        ..ExportSettings::default()
    };
    let report =
        export_control_plans(&dir, &out, &flat, Some(&["A".to_string()]), 1, &|_| {}).unwrap();
    assert_eq!(report.slides.len(), 1);
    let file = &report.slides[0].path;
    assert_eq!(
        file.file_name().unwrap().to_string_lossy(),
        "Essai-A - PLAN DE CONTROLE.png"
    );
    let engraved = image::open(file).unwrap().into_luma8();
    assert_eq!(engraved.dimensions(), (600, 600));
    let worst = engraved
        .pixels()
        .zip(png.pixels())
        .map(|(e, p)| e.0[0].abs_diff(p.0[3]))
        .max()
        .unwrap();
    if vector {
        // Plan vectoriel rendu plus finement pour la gravure : le même trait,
        // au même endroit.
        let lit: Vec<u32> = (0..600)
            .filter(|&x| engraved.get_pixel(x, 300).0[0] >= 128)
            .collect();
        assert!(
            !lit.is_empty() && lit.iter().all(|&x| (280..=320).contains(&x)),
            "{lit:?}"
        );
    } else {
        assert!(worst <= 1, "écart {worst}");
    }
    assert_eq!(dir.load().unwrap().project.export, before);
}

/// Sans plan à l'échelle, pas de plan de contrôle.
#[test]
fn no_control_plan_without_a_scaled_facade() {
    let temp = TempDir::new("plan-controle-absent");
    let (dir, mut project) =
        ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).unwrap();
    project.add_projector("A").registration = pairs();
    dir.save(&project).unwrap();
    let error = export_control_plans(
        &dir,
        &temp.path().join("x"),
        &settings(300.0),
        None,
        1,
        &|_| {},
    )
    .unwrap_err();
    assert!(matches!(error, ExportError::NoFacadeScale), "{error}");
}

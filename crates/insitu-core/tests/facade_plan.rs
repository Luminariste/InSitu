//! Tâche 3.1 : import du plan de façade.
//!
//! Critère d'acceptation du plan : « Un plan PDF d'architecte s'affiche net
//! à tout zoom, à l'échelle. » Le jeu d'essai ne contient pas encore de vrai
//! plan d'architecte (les PDF d'Épinal sont restés dans le Dropbox) : le
//! plan est fabriqué ici, avec ce qu'un plan d'architecte contient (traits
//! fins de 0,18 à 0,5 mm, textes, courbes, format A1 couché, deux pages).
//! On vérifie, à chaque niveau de la pyramide :
//! - qu'un trait fin garde la largeur qu'il doit avoir à ce zoom (il n'est
//!   ni épaissi ni flouté, contrairement à une image agrandie) ;
//! - et, après mise à l'échelle par deux points et une cote, que les
//!   dimensions en mm sont celles du bâtiment.

use std::fs;
use std::path::{Path, PathBuf};

use insitu_core::facade::{self, SourceKind, VECTOR_LEVEL0_SIDE};
use insitu_core::project::{ProjectDir, SourcePoint};
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};

/// Dossier provisoire propre à un test.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("insitu-facade-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Format A1 couché, en points.
const PAGE_W: f32 = 2384.0;
const PAGE_H: f32 = 1684.0;
/// Trait vertical de 0,5 pt (0,18 mm sur le papier), à x = 1000,3 pt.
const LINE_X: f64 = 1000.3;
const LINE_WIDTH: f64 = 0.5;
/// Cote horizontale de 1 000 pt, à 200 pt du bas de la page.
const DIM_FROM: f32 = 200.0;
const DIM_TO: f32 = 1200.0;
const DIM_Y: f32 = 200.0;

/// Un plan de façade en PDF, sur deux pages : la page 1 est un cartouche,
/// la page 2 la façade.
fn architect_pdf() -> Vec<u8> {
    let mut pdf = Pdf::new();
    let catalog = Ref::new(1);
    let pages = Ref::new(2);
    let font = Ref::new(3);
    let page_ids = [Ref::new(4), Ref::new(5)];
    let content_ids = [Ref::new(6), Ref::new(7)];
    pdf.catalog(catalog).pages(pages);
    pdf.pages(pages).kids(page_ids).count(2);
    pdf.type1_font(font).base_font(Name(b"Helvetica"));

    for (index, (&page_id, &content_id)) in page_ids.iter().zip(&content_ids).enumerate() {
        let mut page = pdf.page(page_id);
        page.media_box(Rect::new(0.0, 0.0, PAGE_W, PAGE_H));
        page.parent(pages);
        page.contents(content_id);
        page.resources().fonts().pair(Name(b"F1"), font);
        page.finish();

        let mut content = Content::new();
        if index == 0 {
            content
                .begin_text()
                .set_font(Name(b"F1"), 48.0)
                .next_line(100.0, 1500.0);
            content.show(Str(b"Basilique - cartouche")).end_text();
        } else {
            // Contour de la façade, fenêtres, arc du portail : traits fins.
            content.set_line_width(0.25);
            content.rect(150.0, 150.0, 2000.0, 1300.0).stroke();
            for i in 0..12 {
                let x = 250.0 + 150.0 * i as f32;
                content.rect(x, 900.0, 60.0, 180.0).stroke();
            }
            content.move_to(800.0, 300.0);
            content.cubic_to(800.0, 700.0, 1400.0, 700.0, 1400.0, 300.0);
            content.stroke();
            // Le trait mesuré par le test.
            content.set_line_width(LINE_WIDTH as f32);
            content
                .move_to(LINE_X as f32, 100.0)
                .line_to(LINE_X as f32, 1600.0)
                .stroke();
            // Ligne de cote et son texte.
            content.set_line_width(0.25);
            content
                .move_to(DIM_FROM, DIM_Y)
                .line_to(DIM_TO, DIM_Y)
                .stroke();
            content
                .begin_text()
                .set_font(Name(b"F1"), 10.0)
                .next_line(650.0, DIM_Y + 4.0);
            content.show(Str(b"35,28 m")).end_text();
            content
                .begin_text()
                .set_font(Name(b"F1"), 24.0)
                .next_line(150.0, 1550.0);
            content.show(Str(b"FACADE OUEST - Ech. 1:100")).end_text();
        }
        pdf.stream(content_id, &content.finish());
    }
    pdf.finish()
}

fn decode_gray(bytes: &[u8]) -> image::GrayImage {
    image::load_from_memory(bytes)
        .expect("tuile PNG")
        .into_luma8()
}

#[test]
fn an_architect_pdf_is_sharp_at_every_zoom_and_to_scale() {
    let scratch = Scratch::new("pdf");
    let plan_path = scratch.path("Facade Ouest.pdf");
    fs::write(&plan_path, architect_pdf()).unwrap();

    // Lecture : deux pages, la deuxième retenue.
    let info = facade::inspect(&plan_path, Some(2)).unwrap();
    assert_eq!(info.kind, SourceKind::Pdf);
    assert_eq!(info.page_count, Some(2));
    assert_eq!(
        (info.width, info.height),
        (f64::from(PAGE_W), f64::from(PAGE_H))
    );
    assert!(matches!(
        facade::inspect(&plan_path, Some(3)),
        Err(facade::FacadeError::BadPage {
            page: 3,
            page_count: 2
        })
    ));

    // Import dans un projet, puis mise à l'échelle sur la ligne de cote :
    // 1 000 pt à l'échelle 1:100, soit 352,78 mm × 100.
    let (dir, _) = ProjectDir::create(scratch.path("Epinal.insitu"), Some("Épinal")).unwrap();
    let project = facade::import_facade(&dir, &plan_path, Some(2)).unwrap();
    let imported = project.facade.as_ref().unwrap();
    assert_eq!(imported.source.as_str(), "facade/Facade Ouest.pdf");
    assert_eq!(imported.page, Some(2));
    assert!(imported.scale.is_none());
    assert!(dir.resolve(&imported.source).is_file());

    let dimension_mm = 1000.0 / 72.0 * 25.4 * 100.0;
    let y = f64::from(PAGE_H - DIM_Y); // repère du fichier : y vers le bas
    facade::set_scale(
        &dir,
        SourcePoint::new(f64::from(DIM_FROM), y),
        SourcePoint::new(f64::from(DIM_TO), y),
        dimension_mm,
    )
    .unwrap();
    let reopened = dir.load().unwrap().project;
    let scaled = reopened.facade.as_ref().unwrap();
    let (width_mm, height_mm) = scaled.size_mm().unwrap();
    // La page A1 (841 × 594 mm) représente un bâtiment de 84,1 × 59,4 m.
    assert!((width_mm - 84_102.2).abs() < 0.1, "largeur {width_mm}");
    assert!((height_mm - 59_407.8).abs() < 0.1, "hauteur {height_mm}");
    let point = scaled.to_mm(SourcePoint::new(LINE_X, 842.0)).unwrap();
    assert!((point.x - LINE_X / 72.0 * 25.4 * 100.0).abs() < 1e-6);

    // Affichage : pyramide virtuelle, niveaux grossiers rendus d'avance.
    let cache = scratch.path("cache");
    let started = std::time::Instant::now();
    let shown =
        facade::open_for_display(&dir.resolve(&scaled.source), scaled.page, 0, &cache).unwrap();
    eprintln!(
        "préparation de l'affichage : {:.1} s",
        started.elapsed().as_secs_f64()
    );
    assert_eq!(shown.kind, SourceKind::Pdf);
    assert_eq!(shown.pyramid.width, VECTOR_LEVEL0_SIDE);
    assert_eq!(
        shown.pyramid.channels, 1,
        "plan en noir et blanc : tuiles en gris"
    );
    // Le plan placé en mm a la taille du bâtiment.
    let mm_per_pixel =
        shown.source_units_per_pixel * scaled.scale.as_ref().unwrap().mm_per_source_unit().unwrap();
    assert!((f64::from(shown.pyramid.width) * mm_per_pixel - width_mm).abs() < width_mm * 1e-4);
    // Une seconde ouverture réutilise le cache.
    let again =
        facade::open_for_display(&dir.resolve(&scaled.source), scaled.page, 0, &cache).unwrap();
    assert_eq!(again.key, shown.key);

    // À chaque niveau, du plus fin au plus grossier, le trait de 0,5 pt.
    let scale0 = 1.0 / shown.source_units_per_pixel;
    let row_y = 842.0; // milieu de la page, en points
    for level in &shown.pyramid.levels {
        let scale = scale0 / f64::from(1u32 << level.level);
        let center = LINE_X * scale;
        let expected = LINE_WIDTH * scale;
        let tile = shown.pyramid.tile_size;
        let column = (center / f64::from(tile)).floor() as u32;
        let row = (row_y * scale / f64::from(tile)).floor() as u32;
        let request = format!("{}/{}/{column}_{row}.png", shown.key, level.level);
        let lazy = !display_is_eager(level);
        let tile_file = cache.join(&request);
        assert_eq!(
            tile_file.exists(),
            !lazy,
            "niveau {} : rendu d'avance ?",
            level.level
        );
        let (content_type, bytes) =
            facade::read_or_render(&cache, &request).expect("tuile fournie");
        assert_eq!(content_type, "image/png");
        assert!(tile_file.exists(), "la tuile rendue est gardée");
        let image = decode_gray(&bytes);
        let local_y = (row_y * scale) as u32 - row * tile;
        let local_x = center - f64::from(column * tile);
        let window = (expected + 6.0).ceil() as i64;
        let (mut dark, mut partial, mut darkest) = (0, 0, 255u8);
        for dx in -window..=window {
            let x = local_x.floor() as i64 + dx;
            if x < 0 || x >= i64::from(image.width()) {
                continue;
            }
            let value = image.get_pixel(x as u32, local_y).0[0];
            darkest = darkest.min(value);
            if value < 235 {
                dark += 1;
            }
            if (20..235).contains(&value) {
                partial += 1;
            }
        }
        let allowed = expected.ceil() as i64 + 2;
        eprintln!(
            "niveau {} : trait attendu {expected:.2} px, {dark} px marqués dont {partial} gris, le plus sombre {darkest}",
            level.level
        );
        assert!(
            dark <= allowed,
            "niveau {} : trait étalé sur {dark} px",
            level.level
        );
        if expected >= 1.0 {
            // Trait d'au moins un pixel : noir franc, bords nets (au plus
            // deux pixels gris de chaque côté).
            assert!(
                darkest < 40,
                "niveau {} : trait pâle ({darkest})",
                level.level
            );
            assert!(
                partial <= 4,
                "niveau {} : bords flous ({partial} px gris)",
                level.level
            );
        } else if expected >= 0.3 {
            assert!(
                darkest < 200,
                "niveau {} : trait invisible ({darkest})",
                level.level
            );
        }
    }
}

fn display_is_eager(level: &insitu_core::tiles::LevelInfo) -> bool {
    u64::from(level.width) * u64::from(level.height) <= facade::VECTOR_EAGER_PIXELS
}

#[test]
fn bad_tile_requests_are_refused() {
    let scratch = Scratch::new("refus");
    let plan_path = scratch.path("plan.pdf");
    fs::write(&plan_path, architect_pdf()).unwrap();
    let cache = scratch.path("cache");
    let shown = facade::open_for_display(&plan_path, Some(2), 0, &cache).unwrap();
    let key = &shown.key;
    for request in [
        format!("{key}/0/9999_0.png"),
        format!("{key}/42/0_0.png"),
        format!("{key}/../0/0_0.png"),
        format!("{key}/0/0_0.jpg"),
        "inconnu/0/0_0.png".to_string(),
    ] {
        assert!(
            facade::read_or_render(&cache, &request).is_none(),
            "{request}"
        );
    }
    // Le descripteur se lit comme pour une image.
    let (content_type, _) = facade::read_or_render(&cache, &format!("{key}/pyramid.json")).unwrap();
    assert_eq!(content_type, "application/json");
}

#[test]
fn an_svg_plan_is_rendered_from_its_vectors() {
    let scratch = Scratch::new("svg");
    let plan_path = scratch.path("plan.svg");
    fs::write(
        &plan_path,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="500">
             <rect x="10" y="10" width="980" height="480" fill="none" stroke="black" stroke-width="0.5"/>
             <line x1="500.25" y1="0" x2="500.25" y2="500" stroke="black" stroke-width="0.2"/>
           </svg>"#,
    )
    .unwrap();
    let info = facade::inspect(&plan_path, None).unwrap();
    assert_eq!(
        (info.kind, info.width, info.height),
        (SourceKind::Svg, 1000.0, 500.0)
    );
    let shown = facade::open_for_display(&plan_path, None, 0, &scratch.path("cache")).unwrap();
    assert_eq!(shown.pyramid.width, VECTOR_LEVEL0_SIDE);
    assert_eq!(shown.pyramid.height, VECTOR_LEVEL0_SIDE / 2);
    // Niveau 0 : 65,5 px par unité ; le trait de 0,2 fait 13 px, net.
    let scale = 1.0 / shown.source_units_per_pixel;
    let x = 500.25 * scale;
    let column = (x / 512.0) as u32;
    let request = format!("{}/0/{column}_10.png", shown.key);
    let (_, bytes) = facade::read_or_render(&scratch.path("cache"), &request).unwrap();
    let image = decode_gray(&bytes);
    let dark = (0..image.width())
        .filter(|&i| image.get_pixel(i, 100).0[0] < 128)
        .count();
    assert!((12..=14).contains(&dark), "{dark} px");
}

#[test]
fn a_dxf_plan_is_imported_with_its_origin() {
    use dxf::entities::{Circle, Entity, EntityType, Line};
    use dxf::{Drawing, Point};

    let scratch = Scratch::new("dxf");
    let plan_path = scratch.path("facade.dxf");
    let mut drawing = Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2000;
    drawing.header.default_drawing_units = dxf::enums::Units::Millimeters;
    // Une façade de 12 m × 8 m, dessinée en mm, loin de l'origine du DXF.
    for (a, b) in [
        ((5000.0, 2000.0), (17000.0, 2000.0)),
        ((17000.0, 2000.0), (17000.0, 10000.0)),
        ((17000.0, 10000.0), (5000.0, 10000.0)),
        ((5000.0, 10000.0), (5000.0, 2000.0)),
    ] {
        drawing.add_entity(Entity::new(EntityType::Line(Line::new(
            Point::new(a.0, a.1, 0.0),
            Point::new(b.0, b.1, 0.0),
        ))));
    }
    drawing.add_entity(Entity::new(EntityType::Circle(Circle::new(
        Point::new(11000.0, 6000.0, 0.0),
        1500.0,
    ))));
    drawing.save_file(&plan_path).unwrap();

    let info = facade::inspect(&plan_path, None).unwrap();
    assert_eq!(info.kind, SourceKind::Dxf);
    assert_eq!((info.width, info.height), (12_000.0, 8_000.0));
    assert_eq!(info.origin, Some([5000.0, 10_000.0]));
    assert_eq!(info.declared_mm_per_unit, Some(1.0));

    let (dir, _) = ProjectDir::create(scratch.path("Essai.insitu"), None).unwrap();
    let project = facade::import_facade(&dir, &plan_path, None).unwrap();
    let imported = project.facade.unwrap();
    assert_eq!(imported.origin, Some(SourcePoint::new(5000.0, 10_000.0)));
    assert_eq!(imported.page, None);

    // Le cercle est tracé d'un trait d'un pixel, au centre de la façade.
    let cache = scratch.path("cache");
    let shown = facade::open_for_display(&dir.resolve(&imported.source), None, 0, &cache).unwrap();
    let coarsest = shown.pyramid.levels.last().unwrap();
    let (_, bytes) =
        facade::read_or_render(&cache, &format!("{}/{}/0_0.png", shown.key, coarsest.level))
            .unwrap();
    let image = decode_gray(&bytes);
    let scale = 1.0 / shown.source_units_per_pixel / f64::from(1u32 << coarsest.level);
    let y = (4000.0 * scale) as u32;
    let row: Vec<u8> = (0..image.width())
        .map(|x| image.get_pixel(x, y).0[0])
        .collect();
    let marked = row.iter().filter(|&&v| v < 200).count();
    // Deux bords de la façade et deux passages du cercle, d'un ou deux pixels chacun.
    assert!((4..=8).contains(&marked), "{marked} px marqués : {row:?}");
}

#[test]
fn an_image_facade_keeps_its_pixels() {
    let reduced = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../jeu-essai/epinal/reduits/facade/portail-deroule.png");
    let scratch = Scratch::new("image");
    let (dir, _) = ProjectDir::create(scratch.path("Epinal.insitu"), None).unwrap();
    let project = facade::import_facade(&dir, &reduced, None).unwrap();
    let imported = project.facade.unwrap();
    let image = image::image_dimensions(&reduced).unwrap();
    assert_eq!(
        (imported.source_width, imported.source_height),
        (f64::from(image.0), f64::from(image.1))
    );
    let shown = facade::open_for_display(
        &dir.resolve(&imported.source),
        None,
        0,
        &scratch.path("cache"),
    )
    .unwrap();
    assert_eq!(shown.kind, SourceKind::Image);
    assert_eq!(shown.source_units_per_pixel, 1.0);
    assert_eq!(shown.pyramid.width, image.0);
}

#[test]
fn an_iphone_heic_photo_can_be_the_facade() {
    // Une photo d'ensemble prise à l'iPhone, en HEIC (retour de Benjamin,
    // 12 novembre 2026) : ici la photo de mire du jeu d'essai, faute de
    // photo de façade en HEIC.
    let heic = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jeu-essai/heic/a-brut.heic");
    let scratch = Scratch::new("heic");
    let (dir, _) = ProjectDir::create(scratch.path("Heic.insitu"), None).unwrap();
    let project = facade::import_facade(&dir, &heic, None).unwrap();
    let imported = project.facade.unwrap();
    let size = insitu_core::photo::dimensions(&heic).unwrap();
    assert_eq!(
        (imported.source_width, imported.source_height),
        (f64::from(size.0), f64::from(size.1))
    );
    let shown = facade::open_for_display(
        &dir.resolve(&imported.source),
        None,
        0,
        &scratch.path("cache"),
    )
    .unwrap();
    assert_eq!(shown.kind, SourceKind::Image);
    assert_eq!((shown.pyramid.width, shown.pyramid.height), size);
}

#[test]
fn scaling_needs_two_distinct_points_and_a_positive_distance() {
    let scratch = Scratch::new("echelle");
    let plan_path = scratch.path("plan.pdf");
    fs::write(&plan_path, architect_pdf()).unwrap();
    let (dir, _) = ProjectDir::create(scratch.path("Essai.insitu"), None).unwrap();
    let a = SourcePoint::new(100.0, 100.0);
    let b = SourcePoint::new(1100.0, 100.0);
    assert!(matches!(
        facade::set_scale(&dir, a, b, 1000.0),
        Err(facade::FacadeError::NoFacade)
    ));
    facade::import_facade(&dir, &plan_path, None).unwrap();
    for distance in [0.0, -5.0, f64::NAN, f64::INFINITY] {
        assert!(facade::set_scale(&dir, a, b, distance).is_err());
    }
    assert!(facade::set_scale(&dir, a, SourcePoint::new(100.5, 100.0), 1000.0).is_err());
    let project = facade::set_scale(&dir, a, b, 10_000.0).unwrap();
    assert_eq!(
        project.facade.unwrap().scale.unwrap().mm_per_source_unit(),
        Some(10.0)
    );
}

#[test]
fn unsupported_formats_get_a_useful_message() {
    let message = |name: &str| {
        facade::inspect(Path::new(name), None)
            .unwrap_err()
            .to_string()
    };
    assert!(message("plan.dwg").contains("PDF ou en DXF"));
    assert!(message("plan.xyz").contains(".xyz"));
}

#[test]
fn an_ai_file_is_read_as_a_pdf() {
    // Un .ai « compatible PDF » se lit comme un PDF ; un .ai sans sa partie
    // PDF reçoit un conseil.
    let scratch = Scratch::new("plan-ai");
    let fake = scratch.path("ancien.ai");
    std::fs::write(
        &fake,
        b"%!PS-Adobe-3.0 EPSF\n%%Creator: Adobe Illustrator\n",
    )
    .unwrap();
    let message = facade::inspect(&fake, None).unwrap_err().to_string();
    assert!(message.contains("compatible PDF"), "{message}");
}

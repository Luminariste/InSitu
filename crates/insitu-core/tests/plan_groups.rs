//! Plusieurs plans dans un projet (demande de Benjamin, 13 novembre 2026) :
//! le plan de référence et des groupes de plans posés dessus, dont les
//! calques d'un même PDF ou `.ai` (D-PL-a à g).
//!
//! Le PDF d'essai a deux calques, comme un `.ai` d'Illustrator enregistré
//! « compatible PDF » : « Détaillé » remplit la moitié gauche de la page,
//! « Guides » la moitié droite ; un petit carré n'est sur aucun calque.

use std::fs;
use std::path::PathBuf;

use insitu_core::facade::{self, display::render_whole_layers, PlanChoice};
use insitu_core::project::{PointMm, ProjectDir, SourcePoint};
use pdf_writer::{Name, Pdf, Rect, Ref, Str};

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("insitu-plans-{name}-{}", std::process::id()));
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

const W: f32 = 400.0;
const H: f32 = 200.0;

/// Un PDF de 400 × 200 pt à deux calques.
fn layered_pdf() -> Vec<u8> {
    let mut pdf = Pdf::new();
    let catalog = Ref::new(1);
    let pages = Ref::new(2);
    let page = Ref::new(3);
    let content = Ref::new(4);
    let detailed = Ref::new(5);
    let guides = Ref::new(6);
    {
        let mut c = pdf.catalog(catalog);
        c.pages(pages);
        let mut oc = c.insert(Name(b"OCProperties")).dict();
        oc.insert(Name(b"OCGs")).array().items([detailed, guides]);
        let mut d = oc.insert(Name(b"D")).dict();
        // Dans le fichier, « Guides » est masqué par défaut, comme un
        // calque caché dans Illustrator.
        d.insert(Name(b"OFF")).array().item(guides);
    }
    pdf.pages(pages).kids([page]).count(1);
    {
        let mut p = pdf.page(page);
        p.media_box(Rect::new(0.0, 0.0, W, H));
        p.parent(pages);
        p.contents(content);
        let mut resources = p.resources();
        let mut properties = resources.insert(Name(b"Properties")).dict();
        properties.pair(Name(b"L1"), detailed);
        properties.pair(Name(b"L2"), guides);
    }
    for (id, name) in [(detailed, &b"D\xe9taill\xe9"[..]), (guides, b"Guides")] {
        let mut group = pdf.indirect(id).dict();
        group.pair(Name(b"Type"), Name(b"OCG"));
        group.pair(Name(b"Name"), Str(name));
    }
    let stream = b"/OC /L1 BDC 0 0 0 rg 0 0 200 200 re f EMC\n\
/OC /L2 BDC 0 0 0 rg 200 0 200 200 re f EMC\n\
0 0 0 rg 195 95 10 10 re f\n";
    pdf.stream(content, stream);
    pdf.finish()
}

/// Gris moyen d'un rectangle de l'image RVBA (0 : noir, 255 : blanc).
fn mean(rgba: &[u8], width: u32, x0: u32, x1: u32, y0: u32, y1: u32) -> f64 {
    let mut sum = 0u64;
    let mut n = 0u64;
    for y in y0..y1 {
        for x in x0..x1 {
            sum += u64::from(rgba[((y * width + x) * 4) as usize]);
            n += 1;
        }
    }
    sum as f64 / n as f64
}

#[test]
fn the_layers_of_a_pdf_are_listed_and_shown_one_by_one() {
    let scratch = Scratch::new("calques");
    let file = scratch.path("plan.pdf");
    fs::write(&file, layered_pdf()).unwrap();

    let info = facade::inspect(&file, None).unwrap();
    assert_eq!(
        info.layers,
        vec!["Détaillé".to_string(), "Guides".to_string()]
    );

    // 400 × 200 px : un pixel par point.
    let render = |layers: Option<&[String]>| {
        let (w, h, rgba) = render_whole_layers(&file, None, layers, 0, 400).unwrap();
        assert_eq!((w, h), (400, 200));
        let left = mean(&rgba, w, 10, 180, 10, 190);
        let right = mean(&rgba, w, 220, 390, 10, 190);
        let square = mean(&rgba, w, 197, 203, 97, 103);
        (left, right, square)
    };
    // Tel que le fichier le dit : « Guides » masqué.
    let (left, right, square) = render(None);
    assert!(left < 5.0 && right > 250.0, "défaut : {left} {right}");
    assert!(square < 5.0);
    // Les guides seuls.
    let (left, right, square) = render(Some(&["Guides".to_string()]));
    assert!(left > 250.0 && right < 5.0, "guides : {left} {right}");
    // Ce qui n'est sur aucun calque reste montré.
    assert!(square < 5.0);
    // Les deux.
    let both = ["Détaillé".to_string(), "Guides".to_string()];
    let (left, right, _) = render(Some(&both));
    assert!(left < 5.0 && right < 5.0);
    // Aucun calque.
    let (left, right, square) = render(Some(&[]));
    assert!(left > 250.0 && right > 250.0 && square < 5.0);
}

#[test]
fn a_group_from_the_reference_file_falls_exactly_on_it() {
    let scratch = Scratch::new("groupe");
    let file = scratch.path("Ecuries.pdf");
    fs::write(&file, layered_pdf()).unwrap();
    let (dir, _) = ProjectDir::create(scratch.path("Essai.insitu"), Some("Essai")).unwrap();

    // Sans plan de référence à l'échelle, rien ne se pose.
    let choice = |name: &str| PlanChoice {
        layers: Some(vec![name.to_string()]),
        ..PlanChoice::default()
    };
    assert!(facade::add_plan_group(&dir, &file, &[choice("Guides")]).is_err());
    facade::import_facade(&dir, &file, None).unwrap();
    assert!(matches!(
        facade::add_plan_group(&dir, &file, &[choice("Guides")]),
        Err(facade::FacadeError::ReferenceNotScaled)
    ));
    // 400 pt = 40 m : 100 mm par point.
    facade::set_scale(
        &dir,
        SourcePoint::new(0.0, 0.0),
        SourcePoint::new(400.0, 0.0),
        40_000.0,
    )
    .unwrap();

    let project =
        facade::add_plan_group(&dir, &file, &[choice("Détaillé"), choice("Guides")]).unwrap();
    let group = &project.plan_groups[0];
    assert_eq!(group.name, "Ecuries");
    assert_eq!(group.origin, PointMm::new(0.0, 0.0));
    assert!((group.mm_per_unit - 100.0).abs() < 1e-9);
    assert_eq!(group.plans.len(), 2);
    assert_eq!(group.plans[0].name, "Détaillé");
    assert_eq!(group.plans[1].layers, Some(vec!["Guides".to_string()]));
    assert_eq!(group.size_mm(), (40_000.0, 20_000.0));
    assert!(group.source.as_str().starts_with("facade/plans/"));

    // Un calque inconnu est refusé.
    assert!(facade::add_plans(&dir, &group.id, &[choice("Mobilier")]).is_err());

    // Montrer, cacher : enregistré dans le projet.
    let (gid, pid) = (group.id.clone(), group.plans[1].id.clone());
    facade::set_plan_hidden(&dir, &gid, Some(&pid), true).unwrap();
    let project = facade::set_plan_hidden(&dir, &gid, None, true).unwrap();
    assert!(project.plan_groups[0].hidden && project.plan_groups[0].plans[1].hidden);
    let reread = dir.load().unwrap().project;
    assert!(reread.plan_groups[0].plans[1].hidden);
    assert!(!reread.plan_groups[0].plans[0].hidden);

    // Affichage : une pyramide par vue, chacune avec ses calques.
    let cache = scratch.path("cache");
    let a =
        facade::open_plan_display(&dir, &gid, &project.plan_groups[0].plans[0].id, &cache).unwrap();
    let b = facade::open_plan_display(&dir, &gid, &pid, &cache).unwrap();
    assert_ne!(a.key, b.key);
    assert_eq!(a.source_width, 400.0);

    // Le plan de référence tourne : le groupe le suit (D15-k, D-PL-a).
    let project = facade::rotate_facade(&dir, 1).unwrap();
    let group = &project.plan_groups[0];
    assert_eq!(group.rotation, 1);
    assert_eq!(group.size_mm(), (20_000.0, 40_000.0));
    assert!(group.origin.x.abs() < 1e-9 && group.origin.y.abs() < 1e-9);
    // Et un nouveau groupe tiré du même fichier tourne pareil.
    let project = facade::add_plan_group(&dir, &file, &[]).unwrap();
    assert_eq!(project.plan_groups[1].rotation, 1);
    assert_eq!(project.plan_groups[1].plans[0].source_width, 200.0);

    // Retirer la dernière vue retire le groupe ; le fichier reste.
    let second = project.plan_groups[1].clone();
    let project = facade::remove_plan(&dir, &second.id, Some(&second.plans[0].id)).unwrap();
    assert_eq!(project.plan_groups.len(), 1);
    assert!(dir.resolve(&second.source).exists());
}

#[test]
fn another_file_is_placed_then_aligned_by_two_points() {
    let scratch = Scratch::new("calage");
    let reference = scratch.path("reference.pdf");
    fs::write(&reference, layered_pdf()).unwrap();
    // Un autre plan : une image de 100 × 50 px.
    let other = scratch.path("simplifie.png");
    image::RgbImage::from_pixel(100, 50, image::Rgb([255, 255, 255]))
        .save(&other)
        .unwrap();
    let (dir, _) = ProjectDir::create(scratch.path("Essai.insitu"), Some("Essai")).unwrap();
    facade::import_facade(&dir, &reference, None).unwrap();
    facade::set_scale(
        &dir,
        SourcePoint::new(0.0, 0.0),
        SourcePoint::new(400.0, 0.0),
        40_000.0,
    )
    .unwrap();

    // À la largeur du plan de référence : 40 m pour 100 px.
    let project = facade::add_plan_group(&dir, &other, &[]).unwrap();
    let group = project.plan_groups[0].clone();
    assert!((group.mm_per_unit - 400.0).abs() < 1e-9);
    assert_eq!(group.plans[0].name, "simplifie");

    // Déplacé à la main.
    let project =
        facade::place_plan_group(&dir, &group.id, PointMm::new(1_000.0, 2_000.0), 400.0).unwrap();
    assert_eq!(
        project.plan_groups[0].origin,
        PointMm::new(1_000.0, 2_000.0)
    );
    assert!(facade::place_plan_group(&dir, &group.id, PointMm::new(0.0, 0.0), 0.0).is_err());

    // Calé : les pixels (10, 10) et (60, 10) de l'image vont en (0, 0) et
    // (10 000, 0) de la façade.
    let from = [
        PointMm::new(1_000.0 + 4_000.0, 2_000.0 + 4_000.0),
        PointMm::new(1_000.0 + 24_000.0, 6_000.0),
    ];
    let to = [PointMm::new(0.0, 0.0), PointMm::new(10_000.0, 0.0)];
    let aligned = facade::align_plan_group(&dir, &group.id, from, to).unwrap();
    let placed = &aligned.project.plan_groups[0];
    assert!((placed.mm_per_unit - 200.0).abs() < 1e-9);
    assert!((placed.origin.x + 2_000.0).abs() < 1e-6 && (placed.origin.y + 2_000.0).abs() < 1e-6);
    assert!(aligned.turn_degrees.abs() < 1e-9);
    assert!((aligned.factor - 0.5).abs() < 1e-12);

    // Tourné sur place : le centre ne bouge pas.
    let (w, h) = placed.size_mm();
    let center = (placed.origin.x + w / 2.0, placed.origin.y + h / 2.0);
    let project = facade::rotate_plan_group(&dir, &group.id, -1).unwrap();
    let turned = &project.plan_groups[0];
    assert_eq!(turned.rotation, 3);
    let (w2, h2) = turned.size_mm();
    assert!((turned.origin.x + w2 / 2.0 - center.0).abs() < 1e-9);
    assert!((turned.origin.y + h2 / 2.0 - center.1).abs() < 1e-9);

    // Renommer, réordonner, retirer.
    facade::rename_plan(&dir, &group.id, None, "Plan simplifié").unwrap();
    facade::add_plan_group(&dir, &reference, &[]).unwrap();
    let project = facade::move_plan_group(&dir, &group.id, 5).unwrap();
    assert_eq!(project.plan_groups[1].name, "Plan simplifié");
    let project = facade::remove_plan(&dir, &group.id, None).unwrap();
    assert_eq!(project.plan_groups.len(), 1);
}

#[test]
fn a_project_without_added_plans_is_written_as_before() {
    let scratch = Scratch::new("compat");
    let (dir, _) = ProjectDir::create(scratch.path("Essai.insitu"), Some("Essai")).unwrap();
    let project = dir.load().unwrap().project;
    dir.save(&project).unwrap();
    let text = fs::read_to_string(dir.root().join("projet.json")).unwrap();
    assert!(!text.contains("planGroups"));
}

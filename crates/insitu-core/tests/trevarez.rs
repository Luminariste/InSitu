//! Le projet des écuries de Trévarez, monté par Benjamin avec la version
//! 0.1.28 : deuxième projet réel du jeu d'essai (`jeu-essai/trevarez`).
//!
//! Ce qu'il apporte par rapport à Épinal : une façade en PDF vectoriel (un
//! déroulé de toute la cour), douze projecteurs, un projet réel enregistré
//! par l'app elle-même, et deux diapositives exportées par l'app. Ces tests
//! vérifient que le moteur relit ce projet et refait les mêmes calculs.

use std::fs;
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::GrayImage;
use insitu_core::export::{export_slides, ExportProgress};
use insitu_core::geometry::{Homography, Point, Transform};
use insitu_core::project::{ExportSettings, ProjectDir};
use insitu_core::rectification::{self, DetectOptions};
use insitu_core::registration::auto::{adjust, adjust_region, render_plan};

fn trevarez() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jeu-essai/trevarez")
}

/// Dossier temporaire, effacé à la fin du test.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "insitu-test-trevarez-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        );
        let path = std::env::temp_dir().join(unique);
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("création du dossier temporaire");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy(from: &str, root: &Path, to: &str) {
    let dst = root.join(to);
    fs::create_dir_all(dst.parent().unwrap()).unwrap();
    fs::copy(trevarez().join(from), &dst).unwrap_or_else(|e| panic!("copie de {from} : {e}"));
}

/// Remonte le dossier `.insitu` de Benjamin avec ce que le jeu d'essai en
/// garde : le projet, la façade et les deux dessins posés. Les photos et les
/// 93 autres dessins manquent : ils ne servent pas à l'export.
fn mount_project(root: &Path) -> ProjectDir {
    copy("references/projet/projet.json", root, "projet.json");
    copy(
        "references/facade/ecuries-planbase.pdf",
        root,
        "facade/ECURIES-COMPOSITION PlanBase.pdf",
    );
    copy(
        "references/dessins/algue-h02.png",
        root,
        "bibliotheque/Algues/Algue-H02.png",
    );
    copy(
        "references/dessins/algues-horizontales.png",
        root,
        "bibliotheque/Algues/Algues Horizontales.png",
    );
    ProjectDir::open(root).expect("le projet de Benjamin s'ouvre")
}

#[test]
fn le_projet_de_benjamin_se_relit() {
    let tmp = TempDir::new("relire");
    let dir = mount_project(&tmp.0);
    let loaded = dir.load().expect("projet lisible");
    let project = loaded.project;
    assert_eq!(project.name, "Trevarez");
    let labels: Vec<&str> = project
        .projectors
        .iter()
        .map(|p| p.label.as_str())
        .collect();
    assert_eq!(
        labels,
        ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L"]
    );
    // B à G sont redressés ; seuls B et C sont recalés.
    let rectified: Vec<&str> = project
        .projectors
        .iter()
        .filter(|p| p.rectification.is_some())
        .map(|p| p.label.as_str())
        .collect();
    assert_eq!(rectified, ["B", "C", "D", "E", "F", "G"]);
    let registered: Vec<&str> = project
        .projectors
        .iter()
        .filter(|p| p.registration.len() >= 4)
        .map(|p| p.label.as_str())
        .collect();
    assert_eq!(registered, ["B", "C"]);
    assert_eq!(project.library.len(), 95);
    assert_eq!(project.placements.len(), 2);

    // La façade : la page carrée de 500 mm du PDF, mise à l'échelle par
    // 27,26 m entre deux points, couvre 50 m de côté.
    let facade = project.facade.as_ref().expect("façade");
    let (w, h) = facade.size_mm().expect("façade à l'échelle");
    assert!((w - 50_000.0).abs() < 50.0, "largeur {w} mm");
    assert!((h - 50_000.0).abs() < 50.0, "hauteur {h} mm");
}

/// Proportion des pixels allumés de `a` qui ont un pixel allumé de `b` à
/// `reach` pixels au plus.
fn covered(a: &GrayImage, b: &GrayImage, reach: i64) -> f64 {
    let (w, h) = (a.width() as i64, a.height() as i64);
    let lit = |img: &GrayImage, x: i64, y: i64| {
        x >= 0 && y >= 0 && x < w && y < h && img.get_pixel(x as u32, y as u32)[0] >= 64
    };
    let (mut total, mut found) = (0u64, 0u64);
    for y in 0..h {
        for x in 0..w {
            if !lit(a, x, y) {
                continue;
            }
            total += 1;
            let near = (-reach..=reach).any(|dy| (-reach..=reach).any(|dx| lit(b, x + dx, y + dy)));
            if near {
                found += 1;
            }
        }
    }
    assert!(total > 100, "image presque vide ({total} pixels allumés)");
    found as f64 / total as f64
}

/// Critère : refaite à 100 dpi depuis le projet enregistré, la diapositive
/// C retombe sur celle que Benjamin a exportée à 1 400 dpi (réduite ici à la
/// même taille), à 2 pixels (0,5 mm) près. B n'est pas comparée : son
/// dessin a été déformé autrement après l'export.
#[test]
fn la_diapositive_c_se_refait_a_l_identique() {
    let tmp = TempDir::new("export");
    let dir = mount_project(&tmp.0);
    let out = tmp.0.join("export");
    let settings = ExportSettings {
        dpi: 100.0,
        svg: false,
        ..ExportSettings::default()
    };
    let only = vec!["C".to_string()];
    let report = export_slides(
        &dir,
        &out,
        &settings,
        Some(&only),
        0,
        &|_: ExportProgress| {},
    )
    .expect("export");
    assert_eq!(report.slides.len(), 1);
    let (label, reference) = ("C", "trevarez-c.png");
    let slide = report.slides.iter().find(|s| s.label == label).unwrap();
    let ours = image::open(&slide.path).unwrap().to_luma8();
    let theirs = image::open(trevarez().join("reduits/verres").join(reference))
        .unwrap()
        .to_luma8();
    let theirs = if theirs.dimensions() == ours.dimensions() {
        theirs
    } else {
        image::imageops::resize(&theirs, ours.width(), ours.height(), FilterType::Triangle)
    };
    let ours_in_theirs = covered(&ours, &theirs, 2);
    let theirs_in_ours = covered(&theirs, &ours, 2);
    eprintln!(
        "{label} : {:.1} % de nos traits sur les siens, {:.1} % des siens sur les nôtres",
        100.0 * ours_in_theirs,
        100.0 * theirs_in_ours
    );
    assert!(ours_in_theirs > 0.95, "{label} : {ours_in_theirs:.3}");
    assert!(theirs_in_ours > 0.95, "{label} : {theirs_in_ours:.3}");
}

/// La photo B se redresse sans aide, comme dans l'app de Benjamin, et les
/// croisements de l'image redressée tombent à moins de 0,3 mm de leur place
/// au 95e centile (critère de la tâche 2.4a).
#[test]
fn la_photo_b_se_redresse_sans_aide() {
    let photo = rectification::read_photo(&trevarez().join("references/photos-brutes/b-brut.jpg"))
        .expect("photo lisible");
    let detection =
        rectification::detect(&photo, &DetectOptions::default()).unwrap_or_else(|e| panic!("{e}"));
    let rectified = rectification::rectify(&photo, &detection, 250.0);
    let check = rectification::check_rectified(&rectified).expect("grille relue");
    eprintln!(
        "B : {} croisements détectés, {} relus, p95 {:.3} mm, couverture {:.0} %",
        detection.nodes.len(),
        check.nodes.len(),
        check.deviation_mm.p95,
        100.0 * detection.coverage
    );
    assert!(detection.nodes.len() >= 500);
    assert!(check.nodes.len() >= 500);
    assert!(
        check.deviation_mm.p95 < 0.3,
        "p95 {:.3}",
        check.deviation_mm.p95
    );
}

/// La photo A : la grille ne se lit presque pas. La détection sans aide le
/// dit, au lieu de donner un faux redressement.
#[test]
fn la_photo_a_est_refusee_sans_aide() {
    let photo = rectification::read_photo(&trevarez().join("references/photos-brutes/a-brut.jpg"))
        .expect("photo lisible");
    if let Ok(detection) = rectification::detect(&photo, &DetectOptions::default()) {
        panic!(
            "A ne devrait pas se détecter sans aide ({} croisements)",
            detection.nodes.len()
        );
    }
}

/// Les fichiers versés au dépôt sont bien ceux que décrit le manifeste
/// (empreinte et dimensions), comme pour Épinal.
#[test]
fn le_jeu_d_essai_correspond_a_son_manifeste() {
    use sha2::{Digest, Sha256};
    let text = fs::read_to_string(trevarez().join("manifeste.json")).expect("manifeste lisible");
    let manifest: serde_json::Value = serde_json::from_str(&text).expect("manifeste en JSON");
    let mut checked = 0;
    for item in manifest["fichiers"].as_array().unwrap() {
        for key in ["reference", "reduit"] {
            let Some(info) = item.get(key).filter(|v| !v.is_null()) else {
                continue;
            };
            let path = trevarez().join(info["chemin"].as_str().unwrap());
            let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{} : {e}", path.display()));
            let sha: String = Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            assert_eq!(sha, info["sha256"].as_str().unwrap(), "{}", path.display());
            if let Some(width) = info["largeur"].as_u64() {
                let (w, h) = image::image_dimensions(&path).unwrap();
                assert_eq!(u64::from(w), width, "{}", path.display());
                assert_eq!(u64::from(h), info["hauteur"].as_u64().unwrap());
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 9, "fichiers vérifiés");
}

/// Photo redressée réduite (côté `side` px), tirée de la photo brute par le
/// trapèze qui envoie la diapositive sur les quatre coins `corners` de la
/// mire (pixels de la photo, haut-gauche, haut-droit, bas-droit, bas-gauche).
fn rectified_by(photo: &image::RgbImage, slide_to_photo: &Homography, side: u32) -> GrayImage {
    let s = 280.0;
    // Réduite d'abord (la mire y garde au moins `side` pixels de côté),
    // pour ne pas prendre un pixel sur trois au hasard.
    let k = 3.0;
    let gray = image::imageops::resize(
        &image::DynamicImage::ImageRgb8(photo.clone()).into_luma8(),
        (f64::from(photo.width()) / k) as u32,
        (f64::from(photo.height()) / k) as u32,
        FilterType::Triangle,
    );
    let h = slide_to_photo;
    let mm = s / f64::from(side);
    GrayImage::from_fn(side, side, |x, y| {
        let p = h
            .apply(Point::new(
                (f64::from(x) + 0.5) * mm,
                (f64::from(y) + 0.5) * mm,
            ))
            .unwrap();
        let p = Point::new(p.x / k, p.y / k);
        let (u, v) = (
            p.x.clamp(0.0, f64::from(gray.width() - 1)),
            p.y.clamp(0.0, f64::from(gray.height() - 1)),
        );
        *gray.get_pixel(u as u32, v as u32)
    })
}

/// Ajustement automatique (D-3.2-i) sur les deux photos réelles de
/// Trévarez qu'on sait redresser : A (par les quatre coins posés à la main
/// par Benjamin, sa grille ne se lit pas) et B (par les croisements trouvés
/// par l'app). Chaque photo est posée « à la main » en plusieurs endroits
/// faux d'un demi-mètre à un mètre, un peu trop grande ou trop petite.
///
/// Les places attendues ont été vérifiées à l'œil en superposant la photo et
/// le plan (étude `docs/etudes/recalage-automatique.md`) : pour A, le bord de
/// l'avant-toit et le bandeau ; pour B, l'arc de la travée du milieu.
#[test]
fn l_ajustement_cale_a_et_b_depuis_une_pose_approchee() {
    let root = TempDir::new("ajuster");
    let dir = mount_project(&root.0);
    let project = dir.load().unwrap().project;
    let facade = project.facade.as_ref().unwrap();
    let mm_per_unit = facade.scale.as_ref().unwrap().mm_per_source_unit().unwrap();
    let pt = |x: f64, y: f64| Point::new(x * mm_per_unit, y * mm_per_unit);

    // A : coins de la mire posés par Benjamin (projet du 13 novembre 2026),
    // sur la photo pleine résolution.
    let a =
        rectification::read_photo(&trevarez().join("references/photos-brutes/a-brut.jpg")).unwrap();
    let square = [
        Point::new(0.0, 0.0),
        Point::new(280.0, 0.0),
        Point::new(280.0, 280.0),
        Point::new(0.0, 280.0),
    ];
    let corners = [
        Point::new(2923.2, 604.8),
        Point::new(2815.128, 2929.575),
        Point::new(1128.664, 2626.743),
        Point::new(1191.319, 882.844),
    ];
    let a = rectified_by(
        &a,
        &Homography::from_four_pairs(square, corners).unwrap(),
        560,
    );
    // B : croisements de la mire trouvés par l'app, enregistrés dans le
    // projet de Benjamin.
    let b_points = &project.projectors[1].rectification.as_ref().unwrap().points;
    let slide: Vec<Point> = b_points
        .iter()
        .map(|p| Point::new(p.slide.x, p.slide.y))
        .collect();
    let photo: Vec<Point> = b_points
        .iter()
        .map(|p| Point::new(p.photo.x, p.photo.y))
        .collect();
    let b =
        rectification::read_photo(&trevarez().join("references/photos-brutes/b-brut.jpg")).unwrap();
    let b = rectified_by(&b, &Homography::fit(&slide, &photo).unwrap(), 560);

    for (label, photo, expected) in [("A", &a, pt(458.0, 325.0)), ("B", &b, pt(635.0, 342.0))] {
        let mut centers = Vec::new();
        for (dx, dy, side) in [
            (0.0, 0.0, 9_500.0),
            (800.0, 0.0, 9_000.0),
            (-800.0, 300.0, 10_000.0),
            (0.0, -600.0, 9_500.0),
            (500.0, 500.0, 10_500.0),
        ] {
            let h = side / 2.0;
            let (cx, cy) = (expected.x + dx, expected.y + dy);
            let start = [
                Point::new(cx - h, cy - h),
                Point::new(cx + h, cy - h),
                Point::new(cx + h, cy + h),
                Point::new(cx - h, cy + h),
            ];
            let plan = render_plan(
                &dir.resolve(&facade.source),
                facade.page,
                facade.rotation,
                mm_per_unit,
                Some(adjust_region(&start)),
                1024,
            )
            .unwrap();
            let adjusted = adjust(photo, &plan, start);
            let c = Point::new(
                adjusted.pairs.iter().map(|q| q.facade.x).sum::<f64>() / 4.0,
                adjusted.pairs.iter().map(|q| q.facade.y).sum::<f64>() / 4.0,
            );
            eprintln!(
                "{label} départ ({dx}, {dy}, {side}) : centre ({:.0}, {:.0}), écart {:.0} mm, accord {} → {}",
                c.x,
                c.y,
                c.distance(expected),
                adjusted.start_score,
                adjusted.score
            );
            assert!(adjusted.score >= adjusted.start_score);
            centers.push(c);
        }
        let near = centers
            .iter()
            .filter(|c| c.distance(expected) < 1_000.0)
            .count();
        // Mesuré le 13 novembre 2026 : 4 sur 5 pour A comme pour B ; la pose
        // décalée de 80 cm et trop petite reste à 1,0-1,2 m (accord égal à
        // celui de la bonne place : le plan n'en dit pas plus).
        assert!(
            near >= 4,
            "{label} : seulement {near} poses sur 5 calées à moins d'un mètre"
        );
    }
}

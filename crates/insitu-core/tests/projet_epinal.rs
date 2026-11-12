//! Critère d'acceptation de la tâche 1.4 : un projet enregistré puis rouvert
//! est identique.
//!
//! Le projet est monté comme le serait celui d'Épinal à la fin du jalon 3 :
//! la façade (le déroulé du portail) mise à l'échelle, quatre projecteurs A
//! à D avec leur photo brute, leur photo redressée, leur redressement et
//! leurs points de recalage, la fresque en une image et les réglages
//! d'export. Les fichiers viennent des réduits du jeu d'essai. Les
//! coordonnées sont plausibles mais inventées : ce test vérifie le format,
//! pas la géométrie.

use std::fs;
use std::path::{Path, PathBuf};

use insitu_core::project::{
    validate, AssetKind, ExportSettings, ExtraFields, Facade, FacadeScale, FrescoImage, LockMode,
    PointMm, Project, ProjectDir, ProjectLock, RectMm, Rectification, RectificationMethod,
    RectificationPoint, RegistrationPair, SourcePoint, CURRENT_FORMAT_VERSION, LOCK_FILE,
};
use insitu_core::units::SLIDE_SIZE_MM;

fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../jeu-essai/epinal/reduits")
        .join(relative)
}

/// Dossier temporaire, effacé à la fin du test.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "insitu-test-{label}-{}-{:?}",
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

/// Monte le projet d'Épinal dans `root` et l'enregistre.
fn build_epinal(root: &Path) -> (ProjectDir, Project) {
    let (dir, mut project) = ProjectDir::create(root, Some("Épinal")).expect("création du projet");

    // La façade : le déroulé du portail, mis à l'échelle par deux points et
    // une cote (ici 4,20 m entre les deux piédroits).
    let facade_file = fixture("facade/portail-deroule.png");
    let (width, height) = image::image_dimensions(&facade_file).expect("dimensions de la façade");
    let facade = Facade {
        source: dir
            .import_file(&facade_file, AssetKind::Facade)
            .expect("import de la façade"),
        source_width: f64::from(width),
        source_height: f64::from(height),
        scale: Some(FacadeScale {
            point_a: SourcePoint::new(412.5, 801.25),
            point_b: SourcePoint::new(873.0, 799.5),
            distance_mm: 4200.0,
            extra: ExtraFields::new(),
        }),
        page: None,
        origin: None,
        extra: ExtraFields::new(),
    };
    let (facade_width_mm, facade_height_mm) = facade.size_mm().expect("façade à l'échelle");
    project.facade = Some(facade);

    // Les quatre projecteurs. Chaque emprise couvre un quart de la largeur,
    // avec recouvrement.
    for (index, label) in ["A", "B", "C", "D"].into_iter().enumerate() {
        let lower = label.to_lowercase();
        let raw = dir
            .import_file(
                &fixture(&format!("photos-brutes/{lower}-brut.jpg")),
                AssetKind::RawPhoto,
            )
            .expect("import de la photo brute");
        let rectified = dir
            .import_file(
                &fixture(&format!("photos-redressees/{lower}.png")),
                AssetKind::RectifiedPhoto,
            )
            .expect("import de la photo redressée");
        let shift = index as f64;

        let projector = project.add_projector(label);
        projector.raw_photo = Some(raw);
        projector.rectified_photo = Some(rectified);
        // Redressement manuel : les quatre coins de la mire dans la photo
        // brute (1 008 × 756 px), dans l'ordre haut-gauche, haut-droit,
        // bas-droit, bas-gauche.
        projector.rectification = Some(Rectification {
            method: RectificationMethod::Manual,
            points: vec![
                RectificationPoint::new(
                    SourcePoint::new(214.3 + shift, 88.7),
                    PointMm::new(0.0, 0.0),
                ),
                RectificationPoint::new(
                    SourcePoint::new(791.6 + shift, 96.2),
                    PointMm::new(SLIDE_SIZE_MM, 0.0),
                ),
                RectificationPoint::new(
                    SourcePoint::new(783.9 + shift, 671.4),
                    PointMm::new(SLIDE_SIZE_MM, SLIDE_SIZE_MM),
                ),
                RectificationPoint::new(
                    SourcePoint::new(221.8 + shift, 664.1),
                    PointMm::new(0.0, SLIDE_SIZE_MM),
                ),
            ],
            residual_mm: Some(0.35 + shift / 10.0),
            extra: ExtraFields::new(),
        });
        // Recalage : quatre points pour le trapèze, deux de plus pour le
        // relief des voussures.
        let left = facade_width_mm * (0.05 + 0.22 * shift);
        let span = facade_width_mm * 0.3;
        projector.registration = [
            ((21.4, 18.9), (0.02, 0.06)),
            ((262.7, 24.3), (0.97, 0.03)),
            ((255.1, 259.8), (0.93, 0.95)),
            ((17.6, 251.2), (0.05, 0.98)),
            ((139.2, 71.5), (0.49, 0.21)),
            ((142.8, 198.4), (0.51, 0.74)),
        ]
        .into_iter()
        .map(|((sx, sy), (fx, fy))| {
            RegistrationPair::new(
                PointMm::new(sx + shift / 3.0, sy),
                PointMm::new(left + span * fx, facade_height_mm * fy),
            )
        })
        .collect();
    }

    // La fresque : une image composée sur la façade entière. Le jeu d'essai
    // n'en contient pas ; le déroulé en tient lieu, il a le bon cadre.
    project.fresco = Some(FrescoImage {
        source: dir
            .import_file(&facade_file, AssetKind::Fresco)
            .expect("import de la fresque"),
        frame: RectMm {
            x: 0.0,
            y: 0.0,
            width: facade_width_mm,
            height: facade_height_mm,
        },
        extra: ExtraFields::new(),
    });

    // Réglages du chantier d'Épinal : gravé à 1 000 dpi, sans miroir.
    project.export = ExportSettings {
        dpi: 1000.0,
        ..ExportSettings::default()
    };

    dir.save(&project).expect("enregistrement");
    (dir, project)
}

#[test]
fn the_epinal_project_saved_then_reopened_is_identical() {
    let temp = TempDir::new("epinal-aller-retour");
    let root = temp.0.join("Épinal.insitu");
    let (dir, project) = build_epinal(&root);

    let reopened = ProjectDir::open(&root).expect("réouverture");
    let loaded = reopened.load().expect("lecture");
    assert_eq!(loaded.project, project);
    assert_eq!(loaded.migrated_from, None);
    assert!(loaded.unknown_fields.is_empty());

    // Le contenu attendu est bien là : ce n'est pas une égalité entre deux
    // projets vides.
    let labels: Vec<&str> = loaded
        .project
        .projectors
        .iter()
        .map(|p| p.label.as_str())
        .collect();
    assert_eq!(labels, ["A", "B", "C", "D"]);
    assert_eq!(loaded.project.format_version, CURRENT_FORMAT_VERSION);
    assert_eq!(loaded.project.export.dpi, 1000.0);
    for projector in &loaded.project.projectors {
        assert_eq!(projector.registration.len(), 6);
        assert_eq!(
            projector.rectification.as_ref().map(|r| r.points.len()),
            Some(4)
        );
    }

    // Réenregistrer un projet relu redonne le même fichier, à l'octet près.
    let first = fs::read(dir.project_file()).expect("lecture");
    reopened.save(&loaded.project).expect("réenregistrement");
    assert_eq!(fs::read(dir.project_file()).expect("lecture"), first);
}

#[test]
fn the_epinal_project_is_valid_and_self_contained() {
    let temp = TempDir::new("epinal-controle");
    let root = temp.0.join("Épinal.insitu");
    let (dir, project) = build_epinal(&root);

    let report = validate(&dir);
    assert!(report.issues.is_empty(), "{:?}", report.issues);

    // Dix fichiers référencés (façade, 4 × 2 photos, fresque), tous relatifs
    // et tous présents dans le dossier.
    let files = project.referenced_files();
    assert_eq!(files.len(), 10);
    for (_, path) in &files {
        assert!(!Path::new(path.as_str()).is_absolute());
        assert!(dir.resolve(path).starts_with(&root));
        assert!(dir.resolve(path).is_file(), "{path} manque");
    }

    // Le fichier ne dit rien de l'endroit où il se trouve sur ce poste.
    let text = fs::read_to_string(dir.project_file()).expect("lecture");
    assert!(!text.contains(&temp.0.to_string_lossy().into_owned()));
    assert!(text.contains("\"rawPhoto\": \"photos-brutes/a-brut.jpg\""));
    assert!(text.contains("\"source\": \"facade/portail-deroule.png\""));

    // Copié ailleurs (comme sur l'autre Mac), le projet reste valide.
    let elsewhere = temp.0.join("autre-mac").join("Épinal.insitu");
    copy_dir(&root, &elsewhere);
    let copy = ProjectDir::open(&elsewhere).expect("ouverture de la copie");
    assert_eq!(copy.load().expect("lecture").project, project);
    assert!(validate(&copy).issues.is_empty());

    // Un fichier qui disparaît est signalé.
    fs::remove_file(elsewhere.join("photos-redressees").join("c.png")).expect("suppression");
    let report = validate(&copy);
    let errors: Vec<String> = report.errors().map(|issue| issue.to_string()).collect();
    assert_eq!(
        errors,
        ["Erreur (projectors[2].rectifiedPhoto) : fichier introuvable : photos-redressees/c.png"]
    );
}

#[test]
fn the_lock_follows_an_open_and_close_cycle() {
    let temp = TempDir::new("epinal-verrou");
    let root = temp.0.join("Épinal.insitu");
    let (dir, project) = build_epinal(&root);

    let (lock, _) = ProjectLock::acquire(&dir, LockMode::Normal).expect("verrou");
    assert!(root.join(LOCK_FILE).is_file());
    // Le verrou ne change rien au projet lui-même.
    assert_eq!(dir.load().expect("lecture").project, project);
    lock.release().expect("fermeture");
    assert!(!root.join(LOCK_FILE).exists());
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("création du dossier");
    for entry in fs::read_dir(from).expect("lecture du dossier").flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copie");
        }
    }
}

/// Extrait le premier bloc ```json qui suit `heading` dans la documentation
/// du format.
fn documented_json(heading: &str) -> String {
    let doc = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/format-projet.md"),
    )
    .expect("docs/format-projet.md est lisible");
    let section = doc
        .split_once(heading)
        .unwrap_or_else(|| panic!("rubrique « {heading} » absente"))
        .1;
    let block = section
        .split_once("```json\n")
        .expect("bloc json absent")
        .1
        .split_once("```")
        .expect("bloc json non fermé")
        .0;
    block.to_string()
}

#[test]
fn the_documented_example_is_exactly_what_the_app_writes() {
    let temp = TempDir::new("exemple-doc");
    let (dir, _) =
        ProjectDir::create(temp.0.join("Exemple.insitu"), None).expect("création du projet");
    let example = documented_json("## Exemple");
    fs::write(dir.project_file(), &example).expect("écriture de l'exemple");

    let loaded = dir.load().expect("l'exemple de la documentation se lit");
    assert_eq!(loaded.migrated_from, None);
    assert!(
        loaded.unknown_fields.is_empty(),
        "{:?}",
        loaded.unknown_fields
    );
    assert_eq!(loaded.project.projectors.len(), 2);

    dir.save(&loaded.project).expect("enregistrement");
    assert_eq!(
        fs::read_to_string(dir.project_file()).expect("lecture"),
        example
    );
}

#[test]
fn the_documented_lock_is_a_readable_lock() {
    let temp = TempDir::new("verrou-doc");
    let (dir, _) =
        ProjectDir::create(temp.0.join("Exemple.insitu"), None).expect("création du projet");
    fs::write(dir.root().join(LOCK_FILE), documented_json("## Verrou")).expect("écriture");
    let info = insitu_core::project::read_lock(&dir)
        .expect("lecture")
        .expect("verrou présent");
    assert_eq!(info.user, "benjamin");
    assert_eq!(info.session_id, "c41a7e09b35d2f68");
    assert_eq!(
        info.refreshed_at.seconds_since(info.opened_at),
        18 * 60 + 18
    );
}

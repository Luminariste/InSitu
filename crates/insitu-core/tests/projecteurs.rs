//! Gestes de base sur les projecteurs (système de design, D15-f) : ajouter,
//! renommer, retirer. Le renommage garde tout ce qui a été fait ; le retrait
//! n'efface aucune photo du disque.

use std::path::{Path, PathBuf};

use insitu_core::project::ProjectDir;
use insitu_core::rectification::projector::{
    add_projector, import_raw_photo, remove_projector, rename_projector, RectifyError,
};

fn test_set() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jeu-essai")
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("insitu-projecteurs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn renaming_keeps_the_projector_and_refuses_duplicates() {
    let scratch = Scratch::new("renommer");
    let (dir, _) = ProjectDir::create(scratch.0.join("Essai.insitu"), None).unwrap();
    let (_, a) = add_projector(&dir, "A").unwrap();
    add_projector(&dir, "B").unwrap();
    let photo = test_set().join("epinal/reduits/photos-brutes/a-brut.jpg");
    import_raw_photo(&dir, "A", &photo).unwrap();

    let project = rename_projector(&dir, "A", "  Portail nord ").unwrap();
    let renamed = project.projector(&a).unwrap();
    assert_eq!(renamed.label, "Portail nord");
    assert!(renamed.raw_photo.is_some(), "la photo reste au projecteur");

    // Même nom à la casse près : accepté pour lui-même, refusé pour un autre.
    rename_projector(&dir, &a, "PORTAIL NORD").unwrap();
    assert!(matches!(
        rename_projector(&dir, "B", "portail nord"),
        Err(RectifyError::BadLabel(_))
    ));
    assert!(matches!(
        rename_projector(&dir, "B", "   "),
        Err(RectifyError::BadLabel(_))
    ));

    // Relu depuis le disque.
    let reread = dir.load().unwrap().project;
    assert_eq!(reread.projector(&a).unwrap().label, "PORTAIL NORD");
}

#[test]
fn removing_keeps_the_photos_on_disk() {
    let scratch = Scratch::new("retirer");
    let (dir, _) = ProjectDir::create(scratch.0.join("Essai.insitu"), None).unwrap();
    let (_, a) = add_projector(&dir, "A").unwrap();
    let (_, b) = add_projector(&dir, "B").unwrap();
    let photo = test_set().join("epinal/reduits/photos-brutes/a-brut.jpg");
    let project = import_raw_photo(&dir, "A", &photo).unwrap();
    let copied = dir.resolve(project.projector(&a).unwrap().raw_photo.as_ref().unwrap());
    assert!(copied.is_file());

    let project = remove_projector(&dir, "A").unwrap();
    assert!(project.projector(&a).is_none());
    assert!(project.projector(&b).is_some(), "l'autre projecteur reste");
    assert!(copied.is_file(), "la photo n'est pas effacée du disque");
    assert_eq!(dir.load().unwrap().project.projectors.len(), 1);

    assert!(matches!(
        remove_projector(&dir, "A"),
        Err(RectifyError::UnknownProjector(_))
    ));
    // Le nom libéré se reprend.
    add_projector(&dir, "A").unwrap();
}

//! Outils communs aux tests du modèle de projet.

use std::fs;
use std::path::{Path, PathBuf};

use super::model::new_id;

/// Dossier temporaire, effacé à la fin du test.
pub(crate) struct TempDir(PathBuf);

impl TempDir {
    pub(crate) fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!("insitu-test-{label}-{}", new_id()));
        fs::create_dir_all(&path).expect("création du dossier temporaire");
        Self(path)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

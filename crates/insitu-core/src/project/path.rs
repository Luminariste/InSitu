//! Chemins relatifs au dossier projet.
//!
//! Le dossier `.insitu` voyage par Dropbox entre deux Mac : un chemin absolu
//! (`/Users/benjamin/…`) n'aurait aucun sens sur l'autre machine. Le modèle
//! ne peut donc contenir que des [`RelativePath`], dont la construction
//! refuse tout ce qui sortirait du dossier.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::error::ProjectError;

/// Chemin d'un fichier à l'intérieur du dossier projet, écrit avec des `/`,
/// par exemple `photos-brutes/a-brut.jpg`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RelativePath(String);

impl RelativePath {
    /// Vérifie et construit un chemin relatif.
    pub fn new(value: impl Into<String>) -> Result<Self, ProjectError> {
        let value = value.into();
        match Self::problem(&value) {
            Some(reason) => Err(ProjectError::InvalidPath { value, reason }),
            None => Ok(Self(value)),
        }
    }

    fn problem(value: &str) -> Option<&'static str> {
        if value.is_empty() {
            return Some("le chemin est vide");
        }
        if value.starts_with('/') || value.starts_with('~') {
            return Some(
                "un projet ne contient que des chemins relatifs à son dossier, pas de chemin absolu",
            );
        }
        if value.contains('\\') {
            return Some("les dossiers se séparent par « / », pas par « \\ »");
        }
        if value.contains(':') {
            return Some("le caractère « : » n'est pas admis dans un nom de fichier");
        }
        if value.chars().any(char::is_control) {
            return Some("le chemin contient un caractère invisible");
        }
        for segment in value.split('/') {
            match segment {
                "" => return Some("le chemin contient un nom vide (« // » ou « / » final)"),
                "." | ".." => {
                    return Some("le chemin ne doit pas contenir « . » ni « .. » : il doit rester dans le dossier projet")
                }
                _ => {}
            }
        }
        None
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Dernier élément du chemin : le nom du fichier.
    pub fn file_name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or(&self.0)
    }

    /// Emplacement du fichier sur cette machine, sous le dossier `root`.
    pub fn resolve(&self, root: &Path) -> PathBuf {
        let mut path = root.to_path_buf();
        path.extend(self.0.split('/'));
        path
    }
}

impl fmt::Display for RelativePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<RelativePath> for String {
    fn from(value: RelativePath) -> Self {
        value.0
    }
}

impl TryFrom<String> for RelativePath {
    type Error = ProjectError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_relative_paths_are_accepted() {
        for value in [
            "projet.json",
            "photos-brutes/a-brut.jpg",
            "façade/Portail déroulé.png",
            "a/b/c/d.tiff",
            ".cache/vignette.png",
        ] {
            assert!(RelativePath::new(value).is_ok(), "refusé : {value}");
        }
    }

    #[test]
    fn anything_leaving_the_project_folder_is_refused() {
        for value in [
            "",
            "/Users/benjamin/Dropbox/a.jpg",
            "~/Dropbox/a.jpg",
            "../autre-projet/a.jpg",
            "photos-brutes/../../a.jpg",
            "./a.jpg",
            "photos-brutes//a.jpg",
            "photos-brutes/",
            "C:\\Users\\a.jpg",
            "C:/Users/a.jpg",
            "photos-brutes\\a.jpg",
            "a\u{0}.jpg",
        ] {
            let error = RelativePath::new(value).expect_err(value);
            assert!(matches!(error, ProjectError::InvalidPath { .. }));
        }
    }

    #[test]
    fn a_path_resolves_under_the_project_folder() {
        let path = RelativePath::new("photos-brutes/a-brut.jpg").expect("chemin valide");
        let resolved = path.resolve(Path::new("/tmp/Epinal.insitu"));
        assert_eq!(
            resolved,
            Path::new("/tmp/Epinal.insitu")
                .join("photos-brutes")
                .join("a-brut.jpg")
        );
        assert_eq!(path.file_name(), "a-brut.jpg");
    }

    #[test]
    fn an_absolute_path_in_json_is_refused_in_french() {
        let error = serde_json::from_str::<RelativePath>("\"/Users/marc/a.jpg\"")
            .expect_err("chemin absolu");
        assert!(error.to_string().contains("chemins relatifs"), "{error}");
    }
}

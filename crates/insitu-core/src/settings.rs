//! Réglages de l'app (fenêtre « Réglages… » du menu InSitu, ⌘,), propres à
//! un Mac et communs à tous les projets (demande de Benjamin, 13 novembre
//! 2026, D-4.1-j). Rangés dans un fichier JSON du dossier de l'app
//! (`~/Library/Application Support/com.luminariste.insitu/reglages.json`),
//! jamais dans un projet. Un réglage inconnu (version plus récente de l'app)
//! est gardé tel quel à l'enregistrement.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::project::ExtraFields;

/// Nom du fichier des réglages dans le dossier de l'app.
pub const SETTINGS_FILE: &str = "reglages.json";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// Dossier de la bibliothèque commune (D-4.1-i), en général dans
    /// Dropbox ; absent tant qu'aucun n'a été choisi.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared_library: Option<PathBuf>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// Lit les réglages ; un fichier absent ou illisible donne les réglages par
/// défaut (l'app doit toujours démarrer).
pub fn load(path: &Path) -> AppSettings {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Enregistre les réglages, dossier compris.
pub fn save(path: &Path, settings: &AppSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    text.push('\n');
    crate::project::store::write_atomic(path, text.as_bytes()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::test_support::TempDir;

    #[test]
    fn les_reglages_se_relisent_et_gardent_l_inconnu() {
        let temp = TempDir::new("reglages");
        let path = temp.path().join("app").join(SETTINGS_FILE);
        assert_eq!(load(&path), AppSettings::default());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"sharedLibrary": "/Dropbox/Commune", "futur": 3}"#,
        )
        .unwrap();
        let mut settings = load(&path);
        assert_eq!(
            settings.shared_library.as_deref(),
            Some(Path::new("/Dropbox/Commune"))
        );
        settings.shared_library = Some("/Autre".into());
        save(&path, &settings).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"futur\": 3"));
        assert_eq!(
            load(&path).shared_library.as_deref(),
            Some(Path::new("/Autre"))
        );
        fs::write(&path, "pas du json").unwrap();
        assert_eq!(load(&path), AppSettings::default());
    }
}

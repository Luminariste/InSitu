//! Version du format et mises à niveau.
//!
//! `projet.json` porte un numéro `formatVersion`. À l'ouverture :
//!
//! - même numéro que l'app : lecture directe ;
//! - numéro plus ancien : les étapes de mise à niveau s'appliquent l'une
//!   après l'autre, en mémoire, sur le JSON brut. Le fichier n'est réécrit
//!   qu'au prochain enregistrement, qui garde une copie de l'ancien fichier
//!   (`projet.v<N>.json`) ;
//! - numéro plus récent : refus, sans rien modifier. Une app ancienne ne
//!   devine pas un format qu'elle ne connaît pas.
//!
//! Quand faut-il monter le numéro ? Seulement pour un changement qu'une app
//! plus ancienne lirait de travers : champ renommé, supprimé, changé de sens
//! ou d'unité, nouvelle valeur dans une énumération. L'ajout d'un champ
//! facultatif ne le demande pas : une app plus ancienne le conserve sans le
//! comprendre (voir `model.rs`, champs inconnus).
//!
//! Pour ajouter une version : écrire une fonction `vN_to_vN+1` qui transforme
//! le JSON, l'ajouter à la fin de [`MIGRATIONS`], monter
//! [`CURRENT_FORMAT_VERSION`], ajouter un test avec un fichier de l'ancienne
//! version, et décrire le changement dans `docs/format-projet.md`.

use std::path::Path;

use serde_json::Value;

use super::error::ProjectError;

/// Version du format écrite par cette version de l'app.
pub const CURRENT_FORMAT_VERSION: u32 = 1;

/// Plus ancienne version connue. Rien n'a existé avant.
pub const FIRST_FORMAT_VERSION: u32 = 1;

/// Une étape : transforme le JSON de la version N en version N + 1. Elle n'a
/// pas à toucher `formatVersion`, mis à jour par l'appelant.
pub(crate) type Migration = fn(&mut Value) -> Result<(), String>;

/// Étapes de mise à niveau, dans l'ordre : la première part de
/// [`FIRST_FORMAT_VERSION`]. Vide tant que le format n'a qu'une version.
pub(crate) const MIGRATIONS: &[Migration] = &[];

/// Lit le numéro de version d'un JSON de projet.
pub(crate) fn format_version_of(document: &Value, path: &Path) -> Result<u64, ProjectError> {
    document
        .get("formatVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| ProjectError::MissingFormatVersion {
            path: path.to_path_buf(),
        })
}

/// Met le JSON au format courant. Renvoie la version d'origine s'il a fallu
/// le mettre à niveau, `None` s'il était déjà à jour.
///
/// Les étapes sont fournies par l'appelant : `migrations[0]` part de la
/// version `first`, et la version courante est `first + migrations.len()`.
/// L'app passe [`FIRST_FORMAT_VERSION`] et [`MIGRATIONS`] ; les tests passent
/// des étapes fictives, pour éprouver le mécanisme alors que le format n'a
/// encore qu'une version.
pub(crate) fn upgrade_with(
    document: &mut Value,
    path: &Path,
    first: u32,
    migrations: &[Migration],
) -> Result<Option<u32>, ProjectError> {
    let current = first + migrations.len() as u32;
    let found = format_version_of(document, path)?;
    if found > u64::from(current) {
        return Err(ProjectError::FormatTooRecent {
            path: path.to_path_buf(),
            found,
            supported: current,
        });
    }
    let found = found as u32;
    if found < first {
        return Err(ProjectError::Migration {
            from: found,
            message: format!("le format {found} n'a jamais existé (le premier est le {first})"),
        });
    }
    if found == current {
        return Ok(None);
    }
    for (offset, step) in migrations.iter().enumerate() {
        let from = first + offset as u32;
        if from < found {
            continue;
        }
        step(document).map_err(|message| ProjectError::Migration { from, message })?;
    }
    match document.as_object_mut() {
        Some(object) => {
            object.insert("formatVersion".to_string(), Value::from(current));
        }
        None => {
            return Err(ProjectError::Migration {
                from: found,
                message: "le fichier ne contient pas un objet JSON".to_string(),
            })
        }
    }
    Ok(Some(found))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Deux étapes fictives, pour éprouver le mécanisme : le format réel n'a
    // encore qu'une version.

    /// v1 → v2 : `title` est renommé en `name`.
    fn rename_title(document: &mut Value) -> Result<(), String> {
        let object = document.as_object_mut().ok_or("objet attendu")?;
        let title = object.remove("title").ok_or("champ title absent")?;
        object.insert("name".to_string(), title);
        Ok(())
    }

    /// v2 → v3 : la résolution passe de `dpi` à `export.dpi`.
    fn nest_dpi(document: &mut Value) -> Result<(), String> {
        let object = document.as_object_mut().ok_or("objet attendu")?;
        let dpi = object.remove("dpi").unwrap_or(json!(1400.0));
        object.insert("export".to_string(), json!({ "dpi": dpi }));
        Ok(())
    }

    const STEPS: &[Migration] = &[rename_title, nest_dpi];
    const PATH: &str = "Essai.insitu/projet.json";

    #[test]
    fn an_old_project_goes_through_every_step() {
        let mut document = json!({ "formatVersion": 1, "title": "Épinal", "dpi": 1000.0 });
        let from = upgrade_with(&mut document, Path::new(PATH), 1, STEPS).expect("mise à niveau");
        assert_eq!(from, Some(1));
        assert_eq!(
            document,
            json!({ "formatVersion": 3, "name": "Épinal", "export": { "dpi": 1000.0 } })
        );
    }

    #[test]
    fn an_intermediate_version_skips_the_steps_already_done() {
        let mut document = json!({ "formatVersion": 2, "name": "Épinal", "dpi": 1000.0 });
        let from = upgrade_with(&mut document, Path::new(PATH), 1, STEPS).expect("mise à niveau");
        assert_eq!(from, Some(2));
        assert_eq!(
            document,
            json!({ "formatVersion": 3, "name": "Épinal", "export": { "dpi": 1000.0 } })
        );
    }

    #[test]
    fn an_up_to_date_project_is_left_alone() {
        let original = json!({ "formatVersion": 3, "name": "Épinal" });
        let mut document = original.clone();
        let from = upgrade_with(&mut document, Path::new(PATH), 1, STEPS).expect("lecture");
        assert_eq!(from, None);
        assert_eq!(document, original);
    }

    #[test]
    fn a_newer_project_is_refused_with_a_clear_message() {
        let original = json!({ "formatVersion": 4, "name": "Épinal" });
        let mut document = original.clone();
        let error = upgrade_with(&mut document, Path::new(PATH), 1, STEPS).expect_err("refus");
        assert!(matches!(
            error,
            ProjectError::FormatTooRecent {
                found: 4,
                supported: 3,
                ..
            }
        ));
        let message = error.to_string();
        assert!(message.contains("version plus récente"), "{message}");
        assert!(message.contains("Mettre l'app à jour"), "{message}");
        assert_eq!(document, original, "un projet refusé n'est pas modifié");
    }

    #[test]
    fn a_failing_step_names_the_version_it_started_from() {
        let mut document = json!({ "formatVersion": 1, "dpi": 1000.0 });
        let error = upgrade_with(&mut document, Path::new(PATH), 1, STEPS).expect_err("échec");
        assert!(matches!(error, ProjectError::Migration { from: 1, .. }));
    }

    #[test]
    fn a_missing_or_absurd_version_is_refused() {
        for mut document in [
            json!({ "name": "Épinal" }),
            json!({ "formatVersion": "1" }),
            json!({ "formatVersion": -1 }),
            json!({ "formatVersion": 1.5 }),
            json!([1, 2, 3]),
        ] {
            let error = upgrade_with(&mut document, Path::new(PATH), 1, STEPS).expect_err("refus");
            assert!(matches!(error, ProjectError::MissingFormatVersion { .. }));
        }
        let mut document = json!({ "formatVersion": 0 });
        let error = upgrade_with(&mut document, Path::new(PATH), 1, STEPS).expect_err("refus");
        assert!(matches!(error, ProjectError::Migration { from: 0, .. }));
    }

    #[test]
    fn the_real_format_is_consistent() {
        assert_eq!(
            CURRENT_FORMAT_VERSION,
            FIRST_FORMAT_VERSION + MIGRATIONS.len() as u32
        );
    }
}

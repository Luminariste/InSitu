//! Contrôle d'un projet : le JSON est-il lisible, cohérent, et les fichiers
//! qu'il cite sont-ils là ?
//!
//! Une **erreur** empêche de travailler correctement (fichier manquant,
//! identifiant en double). Un **avertissement** signale quelque chose
//! d'inhabituel mais sans danger (champ inconnu conservé, libellé en double).

use std::collections::BTreeMap;
use std::fmt;

use serde::Serialize;

use super::model::Project;
use super::store::{has_project_extension, ProjectDir};
use crate::units;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
}

/// Un problème relevé par le contrôle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub severity: Severity,
    /// Endroit concerné, par exemple `projectors[1].rawPhoto` ; vide si le
    /// problème concerne tout le projet.
    pub location: String,
    pub message: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.severity {
            Severity::Error => "Erreur",
            Severity::Warning => "Avertissement",
        };
        if self.location.is_empty() {
            write!(f, "{kind} : {}", self.message)
        } else {
            write!(f, "{kind} ({}) : {}", self.location, self.message)
        }
    }
}

/// Résultat du contrôle.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct ValidationReport {
    pub issues: Vec<Issue>,
}

impl ValidationReport {
    /// Vrai s'il n'y a aucune erreur (des avertissements sont permis).
    pub fn is_valid(&self) -> bool {
        self.errors().next().is_none()
    }

    pub fn errors(&self) -> impl Iterator<Item = &Issue> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == Severity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &Issue> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == Severity::Warning)
    }

    fn error(&mut self, location: impl Into<String>, message: impl Into<String>) {
        self.issues.push(Issue {
            severity: Severity::Error,
            location: location.into(),
            message: message.into(),
        });
    }

    fn warning(&mut self, location: impl Into<String>, message: impl Into<String>) {
        self.issues.push(Issue {
            severity: Severity::Warning,
            location: location.into(),
            message: message.into(),
        });
    }
}

/// Contrôle un dossier projet : lecture du JSON, cohérence du modèle,
/// présence des fichiers référencés. Ne modifie rien.
pub fn validate(dir: &ProjectDir) -> ValidationReport {
    let mut report = ValidationReport::default();
    if !has_project_extension(dir.root()) {
        report.warning("", "le nom du dossier ne se termine pas par .insitu");
    }
    let loaded = match dir.load() {
        Ok(loaded) => loaded,
        Err(error) => {
            report.error("", error.to_string());
            return report;
        }
    };
    if let Some(from) = loaded.migrated_from {
        report.warning(
            "formatVersion",
            format!(
                "projet au format {from}, plus ancien que celui de l'app : il sera mis à niveau au prochain enregistrement (une copie de l'ancien fichier sera gardée)"
            ),
        );
    }
    for field in &loaded.unknown_fields {
        report.warning(
            field.clone(),
            "champ inconnu de cette version de l'app ; il est conservé tel quel",
        );
    }
    check_model(&loaded.project, &mut report);
    for (location, path) in loaded.project.referenced_files() {
        let resolved = dir.resolve(path);
        if !resolved.is_file() {
            report.error(location, format!("fichier introuvable : {path}"));
        }
    }
    report
}

/// Contrôle la cohérence du modèle, sans regarder le disque.
pub fn check_model(project: &Project, report: &mut ValidationReport) {
    if project.name.trim().is_empty() {
        report.warning("name", "le projet n'a pas de nom");
    }
    if project.id.trim().is_empty() {
        report.error("id", "le projet n'a pas d'identifiant");
    }
    for (location, value) in project.numbers() {
        if !value.is_finite() {
            report.error(location, "la valeur n'est pas un nombre fini");
        }
    }

    if let Some(facade) = &project.facade {
        if facade.source_width <= 0.0 || facade.source_height <= 0.0 {
            report.error(
                "facade",
                "les dimensions du fichier source doivent être positives",
            );
        }
        if let Some(scale) = &facade.scale {
            if scale.mm_per_source_unit().is_none() {
                report.error(
                    "facade.scale",
                    "mise à l'échelle inutilisable : les deux points sont confondus ou la cote n'est pas positive",
                );
            }
        }
    }

    let mut ids: BTreeMap<&str, usize> = BTreeMap::new();
    let mut labels: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, projector) in project.projectors.iter().enumerate() {
        let here = format!("projectors[{i}]");
        if projector.id.trim().is_empty() {
            report.error(format!("{here}.id"), "le projecteur n'a pas d'identifiant");
        } else if let Some(first) = ids.insert(projector.id.as_str(), i) {
            report.error(
                format!("{here}.id"),
                format!(
                    "identifiant « {} » déjà utilisé par projectors[{first}]",
                    projector.id
                ),
            );
        }
        if projector.label.trim().is_empty() {
            report.warning(format!("{here}.label"), "le projecteur n'a pas de libellé");
        } else if let Some(first) = labels.insert(projector.label.as_str(), i) {
            report.warning(
                format!("{here}.label"),
                format!(
                    "libellé « {} » déjà utilisé par projectors[{first}]",
                    projector.label
                ),
            );
        }
        if let Some(rectification) = &projector.rectification {
            if rectification.points.len() < 4 {
                report.warning(
                    format!("{here}.rectification.points"),
                    "un redressement demande au moins quatre points",
                );
            }
            if rectification.residual_mm.is_some_and(|r| r < 0.0) {
                report.error(
                    format!("{here}.rectification.residualMm"),
                    "un écart résiduel ne peut pas être négatif",
                );
            }
            for (j, point) in rectification.points.iter().enumerate() {
                if !on_slide(point.slide.x, point.slide.y) {
                    report.warning(
                        format!("{here}.rectification.points[{j}].slide"),
                        "point hors de la diapositive de 280 mm",
                    );
                }
            }
        }
        for (j, pair) in projector.registration.iter().enumerate() {
            if !on_slide(pair.slide.x, pair.slide.y) {
                report.warning(
                    format!("{here}.registration[{j}].slide"),
                    "point hors de la diapositive de 280 mm",
                );
            }
        }
    }

    if let Some(fresco) = &project.fresco {
        if fresco.frame.width <= 0.0 || fresco.frame.height <= 0.0 {
            report.error(
                "fresco.frame",
                "les dimensions de la fresque doivent être positives",
            );
        }
    }
    if project.export.dpi <= 0.0 {
        report.error("export.dpi", "la résolution d'export doit être positive");
    }
}

fn on_slide(x: f64, y: f64) -> bool {
    let range = 0.0..=units::SLIDE_SIZE_MM;
    range.contains(&x) && range.contains(&y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::{
        ExtraFields, Facade, FacadeScale, PointMm, Rectification, RectificationMethod,
        RegistrationPair, SourcePoint,
    };
    use crate::project::path::RelativePath;
    use crate::project::store::AssetKind;
    use crate::project::test_support::TempDir;
    use std::fs;

    #[test]
    fn a_fresh_project_is_valid() {
        let temp = TempDir::new("controle-vide");
        let (dir, _) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
        let report = validate(&dir);
        assert!(report.issues.is_empty(), "{:?}", report.issues);
    }

    #[test]
    fn a_missing_file_is_an_error_naming_the_field_and_the_path() {
        let temp = TempDir::new("controle-manquant");
        let photo = temp.path().join("a-brut.jpg");
        fs::write(&photo, b"photo").expect("fichier");
        let (dir, mut project) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
        let present = dir
            .import_file(&photo, AssetKind::RawPhoto)
            .expect("import");
        let projector = project.add_projector("A");
        projector.raw_photo = Some(present);
        projector.rectified_photo =
            Some(RelativePath::new("photos-redressees/a.png").expect("chemin"));
        dir.save(&project).expect("enregistrement");

        let report = validate(&dir);
        assert!(!report.is_valid());
        let errors: Vec<&Issue> = report.errors().collect();
        assert_eq!(errors.len(), 1, "{:?}", report.issues);
        assert_eq!(errors[0].location, "projectors[0].rectifiedPhoto");
        assert_eq!(
            errors[0].to_string(),
            "Erreur (projectors[0].rectifiedPhoto) : fichier introuvable : photos-redressees/a.png"
        );
    }

    #[test]
    fn an_unreadable_project_gives_one_clear_error() {
        let temp = TempDir::new("controle-illisible");
        let (dir, _) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
        fs::write(dir.project_file(), "pas du JSON").expect("écriture");
        let report = validate(&dir);
        assert_eq!(report.errors().count(), 1);
        assert!(report.issues[0].message.contains("JSON valide"));
    }

    #[test]
    fn incoherent_models_are_caught() {
        let mut project = Project::new("Essai");
        project.facade = Some(Facade {
            source: RelativePath::new("facade/plan.png").expect("chemin"),
            source_width: 1000.0,
            source_height: 0.0,
            scale: Some(FacadeScale {
                point_a: SourcePoint::new(5.0, 5.0),
                point_b: SourcePoint::new(5.0, 5.0),
                distance_mm: 1000.0,
                extra: ExtraFields::new(),
            }),
            page: None,
            origin: None,
            extra: ExtraFields::new(),
        });
        let first = project.add_projector("A").id.clone();
        let second = project.add_projector("A");
        second.id = first;
        second.rectification = Some(Rectification {
            method: RectificationMethod::Manual,
            points: Vec::new(),
            residual_mm: Some(-1.0),
            extra: ExtraFields::new(),
        });
        second.registration.push(RegistrationPair::new(
            PointMm::new(300.0, 10.0),
            PointMm::new(0.0, 0.0),
        ));
        project.export.dpi = 0.0;

        let mut report = ValidationReport::default();
        check_model(&project, &mut report);
        let locations = |severity: Severity| -> Vec<&str> {
            report
                .issues
                .iter()
                .filter(|issue| issue.severity == severity)
                .map(|issue| issue.location.as_str())
                .collect()
        };
        assert_eq!(
            locations(Severity::Error),
            [
                "facade",
                "facade.scale",
                "projectors[1].id",
                "projectors[1].rectification.residualMm",
                "export.dpi",
            ]
        );
        assert_eq!(
            locations(Severity::Warning),
            [
                "projectors[1].label",
                "projectors[1].rectification.points",
                "projectors[1].registration[0].slide",
            ]
        );
    }

    #[test]
    fn unknown_fields_are_warnings_not_errors() {
        let temp = TempDir::new("controle-inconnus");
        let (dir, _) =
            ProjectDir::create(temp.path().join("Essai.insitu"), None).expect("création");
        let text = fs::read_to_string(dir.project_file()).expect("lecture");
        let text = text.replacen('{', "{\n  \"placements\": [],", 1);
        fs::write(dir.project_file(), text).expect("écriture");
        let report = validate(&dir);
        assert!(report.is_valid());
        let warnings: Vec<&Issue> = report.warnings().collect();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].location, "placements");
    }
}

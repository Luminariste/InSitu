//! Dessins posés sur la façade (tâche 4.2).
//!
//! L'interface manipule les placements à la souris et au clavier ; à la fin
//! de chaque geste, elle envoie la liste complète au moteur, qui la contrôle
//! et l'enregistre ([`set_placements`]). Les fonctions d'ajout et de retrait
//! servent à la ligne de commande et aux tests.

use std::collections::BTreeSet;
use std::fmt;

use crate::project::model::new_id;
use crate::project::validate::placement_errors;
use crate::project::{Layer, Placement, PointMm, Project, ProjectDir, ProjectError};

#[derive(Debug)]
pub enum PlacementError {
    /// Placements incohérents : chaque problème avec son emplacement.
    Invalid(Vec<(String, String)>),
    /// Un nombre infini ou indéfini.
    NotFinite(String),
    /// Aucun dessin de la bibliothèque ne porte ce nom ou cet identifiant.
    UnknownDrawing(String),
    /// Aucun placement ne porte cet identifiant.
    UnknownPlacement(String),
    /// Une largeur nulle ou négative demandée.
    InvalidWidth(f64),
    Project(ProjectError),
}

impl fmt::Display for PlacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(problems) => {
                write!(f, "Placements refusés :")?;
                for (location, message) in problems {
                    write!(f, " {location} : {message}.")?;
                }
                Ok(())
            }
            Self::NotFinite(field) => write!(f, "{field} n'est pas un nombre fini."),
            Self::UnknownDrawing(name) => {
                write!(f, "Aucun dessin « {name} » dans la bibliothèque.")
            }
            Self::UnknownPlacement(id) => write!(f, "Aucun dessin posé « {id} »."),
            Self::InvalidWidth(width) => {
                write!(
                    f,
                    "Largeur impossible : {width} mm (elle doit être positive)."
                )
            }
            Self::Project(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PlacementError {}

impl From<ProjectError> for PlacementError {
    fn from(e: ProjectError) -> Self {
        Self::Project(e)
    }
}

/// Contrôle des placements d'un projet ; un placement sans identifiant en
/// reçoit un.
pub fn check(project: &mut Project) -> Result<(), PlacementError> {
    for placement in &mut project.placements {
        if placement.id.trim().is_empty() {
            placement.id = new_id();
        }
    }
    if let Some((field, _)) = project
        .numbers()
        .into_iter()
        .filter(|(field, _)| field.starts_with("placements"))
        .find(|(_, value)| !value.is_finite())
    {
        return Err(PlacementError::NotFinite(field));
    }
    let problems = placement_errors(project);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(PlacementError::Invalid(problems))
    }
}

/// Remplace tous les placements du projet et l'enregistre. Rien n'est
/// enregistré si un placement est incorrect.
pub fn set_placements(
    dir: &ProjectDir,
    placements: Vec<Placement>,
) -> Result<Project, PlacementError> {
    let mut project = dir.load()?.project;
    project.placements = placements;
    check(&mut project)?;
    dir.save(&project)?;
    Ok(project)
}

/// Remplace les calques et les dessins posés d'un coup (tâche 4.3) ; un
/// calque sans identifiant en reçoit un. Rien n'est enregistré si l'ensemble
/// est incohérent.
pub fn set_composition(
    dir: &ProjectDir,
    mut layers: Vec<Layer>,
    placements: Vec<Placement>,
) -> Result<Project, PlacementError> {
    let mut project = dir.load()?.project;
    for layer in &mut layers {
        if layer.id.trim().is_empty() {
            layer.id = new_id();
        }
    }
    project.layers = layers;
    project.placements = placements;
    check(&mut project)?;
    dir.save(&project)?;
    Ok(project)
}

/// Un nouveau placement du dessin `drawing` (nom ou identifiant), centré en
/// `center`, large de `width` mm ; la hauteur suit les proportions du
/// fichier.
pub fn new_placement(
    project: &Project,
    drawing: &str,
    center: PointMm,
    width: f64,
) -> Result<Placement, PlacementError> {
    if !(width > 0.0 && width.is_finite()) {
        return Err(PlacementError::InvalidWidth(width));
    }
    let found = project
        .drawing(drawing)
        .or_else(|| project.library.iter().find(|d| d.name == drawing))
        .ok_or_else(|| PlacementError::UnknownDrawing(drawing.to_string()))?;
    let aspect = if found.width > 0.0 && found.height > 0.0 {
        found.height / found.width
    } else {
        1.0
    };
    Ok(Placement {
        id: new_id(),
        drawing: found.id.clone(),
        center,
        width,
        height: width * aspect,
        rotation: 0.0,
        mirrored: false,
        group: None,
        layer: None,
        extra: Default::default(),
    })
}

/// Pose un dessin au premier plan et enregistre le projet ; renvoie aussi
/// l'identifiant du nouveau placement.
pub fn add(
    dir: &ProjectDir,
    mut placement: Placement,
) -> Result<(Project, String), PlacementError> {
    let mut project = dir.load()?.project;
    if placement.id.trim().is_empty() {
        placement.id = new_id();
    }
    let id = placement.id.clone();
    project.placements.push(placement);
    check(&mut project)?;
    dir.save(&project)?;
    Ok((project, id))
}

/// Retire des dessins posés (la bibliothèque n'est pas touchée).
pub fn remove(dir: &ProjectDir, ids: &[String]) -> Result<Project, PlacementError> {
    let mut project = dir.load()?.project;
    let wanted: BTreeSet<&str> = ids.iter().map(String::as_str).collect();
    if let Some(missing) = wanted
        .iter()
        .find(|id| !project.placements.iter().any(|p| p.id == **id))
    {
        return Err(PlacementError::UnknownPlacement(missing.to_string()));
    }
    project
        .placements
        .retain(|p| !wanted.contains(p.id.as_str()));
    dir.save(&project)?;
    Ok(project)
}

#[cfg(test)]
mod tests;

//! Dessins posés sur la façade (tâche 4.2).
//!
//! L'interface manipule les placements à la souris et au clavier ; à la fin
//! de chaque geste, elle envoie la liste complète au moteur, qui la contrôle
//! et l'enregistre ([`set_placements`]). Les fonctions d'ajout et de retrait
//! servent à la ligne de commande et aux tests.

pub mod assign;
pub mod mesh;

use std::collections::BTreeSet;
use std::fmt;

use crate::project::model::new_id;
use crate::project::validate::placement_errors;
use crate::project::{
    DeformationFrame, Layer, Placement, PointMm, Project, ProjectDir, ProjectError,
};

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
    /// Aucun projecteur ne porte ce libellé ou cet identifiant.
    UnknownProjector(String),
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
            Self::UnknownProjector(key) => write!(f, "Aucun projecteur « {key} »."),
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
        deformation: None,
        projector: None,
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

/// Le cadre d'une nouvelle déformation : le dessin lui-même s'il est seul,
/// sinon le rectangle droit qui contient tous les dessins.
pub fn deformation_frame(placements: &[&Placement]) -> Option<DeformationFrame> {
    match placements {
        [] => None,
        [p] => Some(DeformationFrame {
            center: p.center,
            width: p.width,
            height: p.height,
            rotation: p.rotation,
        }),
        many => {
            let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
            let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
            for p in many {
                for (u, v) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
                    let c = p.rigid_point(u, v);
                    x0 = x0.min(c.x);
                    y0 = y0.min(c.y);
                    x1 = x1.max(c.x);
                    y1 = y1.max(c.y);
                }
            }
            Some(DeformationFrame {
                center: PointMm::new((x0 + x1) / 2.0, (y0 + y1) / 2.0),
                width: x1 - x0,
                height: y1 - y0,
                rotation: 0.0,
            })
        }
    }
}

/// Tend une grille au repos de `columns × rows` cases (1 × 1 : quatre
/// coins) sur des dessins posés, ensemble (tâche 4.4) ; `columns` à 0 retire
/// leur déformation. Une déformation précédente est remplacée.
pub fn deform(
    dir: &ProjectDir,
    ids: &[String],
    columns: u32,
    rows: u32,
) -> Result<Project, PlacementError> {
    let mut project = dir.load()?.project;
    let wanted: BTreeSet<&str> = ids.iter().map(String::as_str).collect();
    if let Some(missing) = wanted
        .iter()
        .find(|id| !project.placements.iter().any(|p| p.id == **id))
    {
        return Err(PlacementError::UnknownPlacement(missing.to_string()));
    }
    let deformation = if columns == 0 {
        None
    } else {
        let chosen: Vec<&Placement> = project
            .placements
            .iter()
            .filter(|p| wanted.contains(p.id.as_str()))
            .collect();
        deformation_frame(&chosen).map(|frame| mesh::identity(frame, columns, rows.max(1)))
    };
    for p in &mut project.placements {
        if wanted.contains(p.id.as_str()) {
            p.deformation = deformation.clone();
        }
    }
    check(&mut project)?;
    dir.save(&project)?;
    Ok(project)
}

/// Choisit à la main la diapositive de dessins posés (tâche 4.5) :
/// `projector` est le libellé ou l'identifiant d'un projecteur ; aucun rend
/// ces dessins à la proposition du moteur.
pub fn assign(
    dir: &ProjectDir,
    ids: &[String],
    projector: Option<&str>,
) -> Result<Project, PlacementError> {
    let mut project = dir.load()?.project;
    let wanted: BTreeSet<&str> = ids.iter().map(String::as_str).collect();
    if let Some(missing) = wanted
        .iter()
        .find(|id| !project.placements.iter().any(|p| p.id == **id))
    {
        return Err(PlacementError::UnknownPlacement(missing.to_string()));
    }
    let chosen = match projector {
        None => None,
        Some(key) => Some(
            project
                .projectors
                .iter()
                .find(|p| p.id == key || p.label == key)
                .map(|p| p.id.clone())
                .ok_or_else(|| PlacementError::UnknownProjector(key.to_string()))?,
        ),
    };
    for p in &mut project.placements {
        if wanted.contains(p.id.as_str()) {
            p.projector = chosen.clone();
        }
    }
    check(&mut project)?;
    dir.save(&project)?;
    Ok(project)
}

#[cfg(test)]
mod tests;

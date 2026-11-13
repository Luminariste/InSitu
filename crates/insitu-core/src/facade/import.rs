//! Import du plan dans le projet, et mise à l'échelle.

use std::path::Path;

use super::{inspect, FacadeError, SourceKind};
use crate::project::{
    AssetKind, ExtraFields, Facade, FacadeScale, PointMm, Project, ProjectDir, SourcePoint,
};

/// Copie le plan `source` dans le dossier `facade/` du projet et en fait la
/// façade du projet, qui est enregistré. `page` (à partir de 1) ne sert
/// qu'au PDF ; 1 par défaut.
///
/// Le fichier est lu avant d'être copié : un plan illisible n'entre pas
/// dans le projet. Une façade précédente est remplacée, et sa mise à
/// l'échelle oubliée (elle valait pour l'ancien fichier).
pub fn import_facade(
    dir: &ProjectDir,
    source: &Path,
    page: Option<u32>,
) -> Result<Project, FacadeError> {
    let info = inspect(source, page)?;
    let mut project = dir.load()?.project;
    let relative = dir.import_file(source, AssetKind::Facade)?;
    project.facade = Some(Facade {
        source: relative,
        source_width: info.width,
        source_height: info.height,
        scale: None,
        page: info.page.filter(|_| info.kind == SourceKind::Pdf),
        origin: info.origin.map(|[x, y]| SourcePoint::new(x, y)),
        rotation: 0,
        extra: ExtraFields::new(),
    });
    dir.save(&project)?;
    Ok(project)
}

/// Retire le plan de la façade, et sa mise à l'échelle, et enregistre le
/// projet. Le fichier reste dans `facade/`. Ce qui est en millimètres de
/// façade (dessins posés, points de recalage, fresque) est gardé : il
/// retrouve sa place sur le prochain plan, une fois mis à l'échelle (D15-h).
pub fn remove_facade(dir: &ProjectDir) -> Result<Project, FacadeError> {
    let mut project = dir.load()?.project;
    if project.facade.take().is_none() {
        return Err(FacadeError::NoFacade);
    }
    dir.save(&project)?;
    Ok(project)
}

/// Met la façade à l'échelle : deux points du fichier (dans son unité) et
/// la distance réelle qui les sépare, en mm. Le projet est enregistré.
pub fn set_scale(
    dir: &ProjectDir,
    point_a: SourcePoint,
    point_b: SourcePoint,
    distance_mm: f64,
) -> Result<Project, FacadeError> {
    if !(distance_mm.is_finite() && distance_mm > 0.0) {
        return Err(FacadeError::BadScale(
            "la cote doit être un nombre de millimètres positif",
        ));
    }
    if ![point_a.x, point_a.y, point_b.x, point_b.y]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err(FacadeError::BadScale("un des points n'a pas de position"));
    }
    let mut project = dir.load()?.project;
    let facade = project.facade.as_mut().ok_or(FacadeError::NoFacade)?;
    // Deux points trop proches donneraient une échelle très imprécise :
    // au moins un millième du plan.
    let length = (point_b.x - point_a.x).hypot(point_b.y - point_a.y);
    if length < facade.source_width.max(facade.source_height) / 1000.0 {
        return Err(FacadeError::BadScale(
            "les deux points sont confondus ou trop proches",
        ));
    }
    facade.scale = Some(FacadeScale {
        point_a,
        point_b,
        distance_mm,
        extra: ExtraFields::new(),
    });
    dir.save(&project)?;
    Ok(project)
}

/// Tourne le plan de la façade de `quarter_turns` quarts de tour (positif :
/// sens des aiguilles d'une montre ; négatif : sens inverse) et enregistre
/// le projet (D15-k).
///
/// Le repère de la façade est celui du plan affiché : il tourne avec lui.
/// La mise à l'échelle est gardée (ses deux points tournent avec le plan) ;
/// ce qui est déjà posé en mm de façade tourne aussi, pour rester au même
/// endroit du plan : dessins posés, cadres de déformation, points de
/// recalage côté façade. Une fresque importée (une image dans un cadre
/// droit) ne sait pas tourner : il faut la retirer avant.
pub fn rotate_facade(dir: &ProjectDir, quarter_turns: i32) -> Result<Project, FacadeError> {
    let mut project = dir.load()?.project;
    let turns = quarter_turns.rem_euclid(4) as u8;
    let facade = project.facade.as_ref().ok_or(FacadeError::NoFacade)?;
    if turns == 0 {
        return Ok(project);
    }
    let size_mm = facade.size_mm();
    if size_mm.is_some() && project.fresco.is_some() {
        return Err(FacadeError::CannotRotate(
            "une fresque est posée sur la façade et elle ne tourne pas ; la retirer d'abord",
        ));
    }
    for _ in 0..turns {
        quarter_turn(&mut project, size_mm.is_some());
    }
    dir.save(&project)?;
    Ok(project)
}

/// Un quart de tour dans le sens des aiguilles d'une montre (y vers le
/// bas) : un point (x, y) d'un plan de hauteur h va en (h − y, x).
/// `carry_mm` : faire suivre ce qui est en mm de façade.
fn quarter_turn(project: &mut Project, carry_mm: bool) {
    let facade = project.facade.as_mut().expect("façade vérifiée");
    let height_mm = facade.size_mm().map(|(_, h)| h);
    let height = facade.source_height;
    let turn = |p: SourcePoint| SourcePoint::new(height - p.y, p.x);
    if let Some(scale) = facade.scale.as_mut() {
        scale.point_a = turn(scale.point_a);
        scale.point_b = turn(scale.point_b);
    }
    (facade.source_width, facade.source_height) = (facade.source_height, facade.source_width);
    facade.rotation = (facade.rotation + 1) % 4;

    let Some(h) = height_mm.filter(|_| carry_mm) else {
        return;
    };
    let turn_mm = |p: PointMm| PointMm::new(h - p.y, p.x);
    let turn_angle = |a: f64| {
        let turned = (a + 90.0) % 360.0;
        if turned > 180.0 {
            turned - 360.0
        } else {
            turned
        }
    };
    // Un dessin posé dans une diapositive (D-3.6-g) ne tourne pas avec le plan.
    for placement in project.placements.iter_mut().filter(|p| p.slide.is_none()) {
        placement.center = turn_mm(placement.center);
        placement.rotation = turn_angle(placement.rotation);
        if let Some(deformation) = placement.deformation.as_mut() {
            deformation.frame.center = turn_mm(deformation.frame.center);
            deformation.frame.rotation = turn_angle(deformation.frame.rotation);
        }
    }
    for projector in &mut project.projectors {
        for pair in &mut projector.registration {
            pair.facade = turn_mm(pair.facade);
        }
    }
    // Les plans ajoutés suivent aussi (D-PL-a).
    for group in &mut project.plan_groups {
        super::plans::turn_group_with_facade(group, h);
    }
}

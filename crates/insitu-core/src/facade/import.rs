//! Import du plan dans le projet, et mise à l'échelle.

use std::path::Path;

use super::{inspect, FacadeError, SourceKind};
use crate::project::{
    AssetKind, ExtraFields, Facade, FacadeScale, Project, ProjectDir, SourcePoint,
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
        extra: ExtraFields::new(),
    });
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

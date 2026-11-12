//! Ajustement d'un dessin par diapositive (tâche 4.6).
//!
//! Dans la vue Diapositive, on retouche un dessin sur la photo redressée
//! d'un projecteur sans toucher à la façade : une grille de déformation,
//! comme la déformation libre (tâche 4.4), mais tendue en mm de diapositive.
//! L'export de cette diapositive passe par la grille ; les autres
//! diapositives et la façade ne changent pas. Un clic la retire.

use serde::Serialize;

use super::assign::{assignments, Assignment};
use super::{check, mesh, PlacementError};
use crate::geometry::{Point, Transform};
use crate::project::{
    Deformation, DeformationFrame, Placement, PointMm, Project, ProjectDir, SlideAdjustment,
};
use crate::registration::slide_to_facade;

/// Cases de la maille qui situe un dessin dans la diapositive, par côté.
pub const SLIDE_MESH_DIVISIONS: usize = 16;

/// Un dessin posé vu dans la diapositive d'un projecteur, avant sa
/// retouche : de quoi l'afficher et le retoucher à l'écran.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideDrawing {
    /// Identifiant du placement.
    pub placement: String,
    /// Identifiant du dessin de la bibliothèque.
    pub drawing: String,
    /// Cases par côté : `(divisions + 1)²` points.
    pub divisions: usize,
    /// Position dans la diapositive (mm de diapositive), avant retouche, du
    /// point `(i, j) / divisions` du dessin, ligne par ligne.
    pub points: Vec<PointMm>,
}

/// Points du dessin dans la diapositive : la maille d'affichage, avant
/// retouche. `None` si un point n'a pas d'antécédent par le recalage.
fn slide_mesh(p: &Placement, warp: &crate::geometry::PointWarp) -> Option<Vec<PointMm>> {
    let n = SLIDE_MESH_DIVISIONS;
    let mut points = Vec::with_capacity((n + 1) * (n + 1));
    for j in 0..=n {
        for i in 0..=n {
            let f = p.facade_point(i as f64 / n as f64, j as f64 / n as f64);
            let s = warp.apply_inverse(Point::new(f.x, f.y))?;
            points.push(PointMm::new(s.x, s.y));
        }
    }
    Some(points)
}

/// Les dessins posés visibles que porte la diapositive du projecteur
/// `projector` (tâche 4.5), situés dans cette diapositive.
pub fn slide_drawings(project: &Project, projector: &str) -> Vec<SlideDrawing> {
    slide_drawings_in(project, &assignments(project), projector)
}

/// Comme [`slide_drawings`], d'après des affectations déjà calculées.
pub fn slide_drawings_in(
    project: &Project,
    all: &[Assignment],
    projector: &str,
) -> Vec<SlideDrawing> {
    let Some(warp) = project
        .projector(projector)
        .and_then(|p| slide_to_facade(&p.registration))
    else {
        return Vec::new();
    };
    super::assign::slide_placements_in(project, all, projector)
        .into_iter()
        .filter_map(|p| {
            Some(SlideDrawing {
                placement: p.id.clone(),
                drawing: p.drawing.clone(),
                divisions: SLIDE_MESH_DIVISIONS,
                points: slide_mesh(p, &warp)?,
            })
        })
        .collect()
}

/// Le cadre d'une nouvelle retouche : le rectangle droit, en mm de
/// diapositive, qui contient le dessin tel que le projecteur le montre.
pub fn adjustment_frame(
    project: &Project,
    placement: &Placement,
    projector: &str,
) -> Option<DeformationFrame> {
    let warp = slide_to_facade(&project.projector(projector)?.registration)?;
    let points = slide_mesh(placement, &warp)?;
    let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
    let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for q in &points {
        x0 = x0.min(q.x);
        y0 = y0.min(q.y);
        x1 = x1.max(q.x);
        y1 = y1.max(q.y);
    }
    (x1 > x0 && y1 > y0).then(|| DeformationFrame {
        center: PointMm::new((x0 + x1) / 2.0, (y0 + y1) / 2.0),
        width: x1 - x0,
        height: y1 - y0,
        rotation: 0.0,
    })
}

/// Tend sur le dessin `id`, dans la diapositive qui le porte, une grille de
/// retouche au repos de `columns × rows` cases (1 × 1 : quatre coins) ;
/// `columns` à 0 retire la retouche. Une retouche précédente est remplacée.
pub fn adjust(
    dir: &ProjectDir,
    id: &str,
    columns: u32,
    rows: u32,
) -> Result<Project, PlacementError> {
    let mut project = dir.load()?.project;
    let index = project
        .placements
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| PlacementError::UnknownPlacement(id.to_string()))?;
    let adjustment = if columns == 0 {
        None
    } else {
        let projector = assignments(&project)
            .into_iter()
            .find(|a| a.placement == id)
            .and_then(|a| a.projector)
            .ok_or_else(|| PlacementError::NoSlide(id.to_string()))?;
        let frame = adjustment_frame(&project, &project.placements[index], &projector)
            .ok_or_else(|| PlacementError::NoSlide(id.to_string()))?;
        Some(SlideAdjustment {
            projector,
            deformation: mesh::identity(frame, columns, rows.max(1)),
            extra: Default::default(),
        })
    };
    project.placements[index].adjustment = adjustment;
    check(&mut project)?;
    dir.save(&project)?;
    Ok(project)
}

/// Enregistre la retouche du dessin `id` telle que l'interface l'a tirée
/// (aucune : retirée). Rien n'est enregistré si elle replie le dessin.
pub fn set_adjustment(
    dir: &ProjectDir,
    id: &str,
    adjustment: Option<SlideAdjustment>,
) -> Result<Project, PlacementError> {
    let mut project = dir.load()?.project;
    let p = project
        .placements
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(|| PlacementError::UnknownPlacement(id.to_string()))?;
    p.adjustment = adjustment;
    check(&mut project)?;
    dir.save(&project)?;
    Ok(project)
}

/// Où tombe dans la diapositive, retouche comprise, le point `s` (mm de
/// diapositive, avant retouche) ; sans retouche, `s` lui-même.
pub fn adjusted(adjustment: Option<&Deformation>, s: PointMm) -> PointMm {
    match adjustment {
        Some(d) => mesh::forward(d, s),
        None => s,
    }
}

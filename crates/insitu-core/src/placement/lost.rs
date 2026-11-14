//! Ce qui ne sera pas projeté (demande de Benjamin après l'essai du 13
//! novembre 2026, décision D-4.5-h).
//!
//! Un dessin posé ne part que dans une diapositive (D-4.5-a). La part du
//! dessin qui sort de l'emprise de cette diapositive n'est projetée par
//! personne, même si une diapositive voisine couvre cet endroit. On la
//! calcule ici pour la montrer en rouge sur la façade.
//!
//! La forme d'un dessin est sa silhouette (tâche 4.7), qui suit ses traits ;
//! un dessin sans silhouette compte pour son rectangle. La part perdue est
//! la silhouette posée sur la façade, moins l'emprise de la diapositive du
//! dessin. Un dessin hors de toute emprise est perdu en entier.

use std::path::Path;

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;
use serde::Serialize;

use super::assign::{shape_assignments, Assignment};
use super::mask;
use crate::library::silhouette::{silhouettes, DrawingSilhouette};
use crate::library::LibraryError;
use crate::project::{Placement, PointMm, Project, ProjectDir, SilhouettePolygon};
use crate::registration::{footprint, slide_to_facade};

/// Points du rectangle d'un dessin sans silhouette, par côté.
const RECT_POINTS_PER_SIDE: usize = 24;
/// Un côté de silhouette d'un dessin déformé est coupé en autant de
/// morceaux (la déformation courbe les droites).
const DEFORMED_EDGE_PIECES: usize = 4;
/// En dessous de cette surface (mm²), une part perdue est du bruit de
/// calcul le long du bord de l'emprise : on l'oublie.
const MIN_AREA_MM2: f64 = 25.0;

/// La part d'un dessin posé que sa diapositive ne projette pas.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LostPart {
    /// Identifiant du placement.
    pub placement: String,
    /// La diapositive du dessin ; aucune s'il ne tombe dans aucune emprise.
    pub projector: Option<String>,
    /// Les morceaux perdus, sur la façade (mm) : pour chacun, le contour
    /// extérieur puis ses trous.
    pub shapes: Vec<Vec<Vec<PointMm>>>,
    /// Surface perdue, en m².
    pub area_m2: f64,
}

/// La forme du dessin sur la façade : ses polygones de silhouette posés
/// (ou son rectangle), à remplir « pair-impair ».
pub(super) fn shape_on_facade(p: &Placement, polygons: &[SilhouettePolygon]) -> Vec<Vec<[f64; 2]>> {
    let at = |u: f64, v: f64| {
        let q = p.facade_point(u, v);
        [q.x, q.y]
    };
    let usable: Vec<&SilhouettePolygon> = polygons.iter().filter(|s| s.len() >= 3).collect();
    if usable.is_empty() {
        let n = RECT_POINTS_PER_SIDE;
        let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let mut out = Vec::with_capacity(4 * n);
        for k in 0..4 {
            let (a, b) = (corners[k], corners[(k + 1) % 4]);
            for step in 0..n {
                let t = step as f64 / n as f64;
                out.push(at(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
            }
        }
        return vec![out];
    }
    let pieces = if p.deformation.is_some() {
        DEFORMED_EDGE_PIECES
    } else {
        1
    };
    usable
        .into_iter()
        .map(|polygon| {
            let n = polygon.len();
            let mut out = Vec::with_capacity(n * pieces);
            for k in 0..n {
                let ([u0, v0], [u1, v1]) = (polygon[k], polygon[(k + 1) % n]);
                for step in 0..pieces {
                    let t = step as f64 / pieces as f64;
                    out.push(at(u0 + (u1 - u0) * t, v0 + (v1 - v0) * t));
                }
            }
            out
        })
        .collect()
}

/// La forme visible du dessin sur la façade : comme [`shape_on_facade`],
/// moins ce que cache son masque (D-4.2-u). Un dessin entièrement caché
/// n'a aucune forme.
pub(super) fn visible_on_facade(
    project: &Project,
    p: &Placement,
    polygons: &[SilhouettePolygon],
) -> Vec<Vec<[f64; 2]>> {
    if p.mask.is_empty() {
        return shape_on_facade(p, polygons);
    }
    let usable: Vec<SilhouettePolygon> =
        polygons.iter().filter(|s| s.len() >= 3).cloned().collect();
    let base = if usable.is_empty() {
        mask::whole_drawing()
    } else {
        usable
    };
    let aspect = project
        .drawing(&p.drawing)
        .map_or((1.0, 1.0), |d| mask::aspect(d.width, d.height));
    let visible = mask::visible_shape(&base, &p.mask, aspect);
    if visible.is_empty() {
        return Vec::new();
    }
    shape_on_facade(p, &visible)
}

/// Surface d'un contour (mm², positive).
fn area(contour: &[[f64; 2]]) -> f64 {
    let n = contour.len();
    let mut twice = 0.0;
    for k in 0..n {
        let (a, b) = (contour[k], contour[(k + 1) % n]);
        twice += a[0] * b[1] - b[0] * a[1];
    }
    (twice / 2.0).abs()
}

/// La part perdue de chaque dessin posé visible, d'après les affectations
/// `all` et les silhouettes des dessins (`silhouettes`, sinon le
/// rectangle). Les dessins projetés en entier n'y figurent pas.
pub fn lost_parts(
    project: &Project,
    all: &[Assignment],
    silhouettes: &[DrawingSilhouette],
) -> Vec<LostPart> {
    let mut out = Vec::new();
    for p in project.facade_placements() {
        let Some(a) = all.iter().find(|a| a.placement == p.id) else {
            continue;
        };
        if a.whole && a.projector.is_some() {
            continue;
        }
        let polygons = silhouettes
            .iter()
            .find(|s| s.drawing == p.drawing)
            .map_or(&[][..], |s| s.polygons.as_slice());
        let subject = visible_on_facade(project, p, polygons);
        if subject.is_empty() {
            continue;
        }
        let emprise: Vec<Vec<[f64; 2]>> = a
            .projector
            .as_deref()
            .and_then(|id| project.projector(id))
            .and_then(|q| slide_to_facade(&q.registration))
            .map(|warp| vec![footprint(&warp).iter().map(|q| [q.x, q.y]).collect()])
            .unwrap_or_default();
        let shapes = if emprise.is_empty() {
            subject.overlay(&emprise, OverlayRule::Subject, FillRule::EvenOdd)
        } else {
            subject.overlay(&emprise, OverlayRule::Difference, FillRule::EvenOdd)
        };
        let mut total = 0.0;
        let shapes: Vec<Vec<Vec<PointMm>>> = shapes
            .into_iter()
            .filter_map(|shape| {
                let outer = shape.first()?;
                let surface = area(outer) - shape[1..].iter().map(|h| area(h)).sum::<f64>();
                if surface < MIN_AREA_MM2 {
                    return None;
                }
                total += surface;
                Some(
                    shape
                        .iter()
                        .map(|c| c.iter().map(|q| PointMm::new(q[0], q[1])).collect())
                        .collect(),
                )
            })
            .collect();
        if shapes.is_empty() {
            continue;
        }
        out.push(LostPart {
            placement: p.id.clone(),
            projector: a.projector.clone(),
            shapes,
            area_m2: total / 1e6,
        });
    }
    out
}

/// Les parts perdues du projet enregistré dans `dir` ; les silhouettes
/// détectées sont rangées dans le cache `cache_root`, comme pour la
/// bibliothèque.
pub fn load_lost_parts(dir: &ProjectDir, cache_root: &Path) -> Result<Vec<LostPart>, LibraryError> {
    let project = dir.load()?.project;
    let all = shape_assignments(dir, &project, cache_root);
    project_lost_parts(dir, &project, &all, cache_root)
}

/// Comme [`load_lost_parts`], pour un projet déjà lu et ses affectations.
pub fn project_lost_parts(
    dir: &ProjectDir,
    project: &Project,
    all: &[Assignment],
    cache_root: &Path,
) -> Result<Vec<LostPart>, LibraryError> {
    let mut ids: Vec<String> = project
        .facade_placements()
        .into_iter()
        .filter(|p| {
            all.iter()
                .any(|a| a.placement == p.id && !(a.whole && a.projector.is_some()))
        })
        .map(|p| p.drawing.clone())
        .filter(|id| project.drawing(id).is_some())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    let found = if ids.is_empty() {
        Vec::new()
    } else {
        // Une silhouette illisible (fichier du dessin manquant…) ne doit pas
        // tout bloquer : ces dessins comptent alors pour leur rectangle.
        silhouettes(dir, project, &ids, cache_root).unwrap_or_else(|_| {
            ids.iter()
                .filter_map(|id| {
                    silhouettes(dir, project, std::slice::from_ref(id), cache_root).ok()
                })
                .flatten()
                .collect()
        })
    };
    Ok(lost_parts(project, all, &found))
}

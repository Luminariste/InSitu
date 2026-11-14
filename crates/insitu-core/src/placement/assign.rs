//! Affectation des dessins posés aux diapositives (tâche 4.5).
//!
//! Chaque dessin posé part dans une seule diapositive : il n'est jamais
//! coupé entre deux projecteurs, ni projeté deux fois là où deux emprises se
//! recouvrent. Le moteur propose une diapositive ; un choix fait à la main
//! (`Placement::projector`) l'emporte et reste tant qu'on ne le retire pas.
//!
//! Proposition : le contour du dessin (déformation comprise) est ramené dans
//! chaque diapositive par l'inverse du recalage. Parmi les diapositives qui
//! contiennent tout le contour, on prend celle où le dessin est le plus loin
//! du bord ; si aucune ne le contient en entier, celle qui en contient la
//! plus grande part, et le dessin est signalé « à cheval ».
//!
//! « À cheval » se juge sur le tracé du dessin, pas sur son rectangle
//! (D-4.5-j) : [`shape_assignments`] reprend les dessins dont le rectangle
//! dépasse avec leur silhouette (tâche 4.7), qui suit les traits. Le choix
//! de la diapositive, lui, reste celui du rectangle, pour que l'export et
//! toutes les vues donnent le même projecteur sans lire les images.

use std::path::Path;

use serde::Serialize;

use super::lost::visible_on_facade;
use crate::geometry::{Point, PointWarp, Transform};
use crate::library::silhouette::{silhouettes, DrawingSilhouette};
use crate::project::{Placement, Project, ProjectDir};
use crate::registration::slide_to_facade;
use crate::units::SLIDE_SIZE_MM;

/// Points calculés sur chaque bord du dessin pour le situer.
const OUTLINE_POINTS_PER_SIDE: usize = 24;

/// La diapositive d'un dessin posé.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    /// Identifiant du placement.
    pub placement: String,
    /// Le projecteur dont la diapositive porte le dessin ; aucun si le
    /// dessin ne tombe dans aucune emprise.
    pub projector: Option<String>,
    /// Vrai si ce choix a été fait à la main.
    pub manual: bool,
    /// Vrai si le dessin tient en entier dans cette diapositive.
    pub whole: bool,
    /// Les projecteurs dont la diapositive contient le dessin en entier,
    /// dans l'ordre du projet.
    pub candidates: Vec<String>,
}

/// Comment un dessin tombe dans une diapositive.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Fit {
    /// Part des points du contour qui tombent dans la diapositive (0 à 1).
    inside: f64,
    /// Plus petite distance d'un point du contour au bord de la
    /// diapositive, en mm de diapositive ; négative s'il dépasse.
    margin: f64,
}

/// Points du contour du dessin sur la façade.
fn outline(p: &Placement) -> Vec<Point> {
    let n = OUTLINE_POINTS_PER_SIDE;
    let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
    let mut out = Vec::with_capacity(4 * n);
    for k in 0..4 {
        let (a, b) = (corners[k], corners[(k + 1) % 4]);
        for step in 0..n {
            let t = step as f64 / n as f64;
            let q = p.facade_point(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            out.push(Point::new(q.x, q.y));
        }
    }
    out
}

fn fit(outline: &[Point], warp: &PointWarp) -> Fit {
    let s = SLIDE_SIZE_MM;
    let mut inside = 0usize;
    let mut margin = f64::INFINITY;
    for &q in outline {
        let m = match warp.apply_inverse(q) {
            Some(p) => p.x.min(p.y).min(s - p.x).min(s - p.y),
            None => f64::NEG_INFINITY,
        };
        if m >= 0.0 {
            inside += 1;
        }
        margin = margin.min(m);
    }
    Fit {
        inside: inside as f64 / outline.len().max(1) as f64,
        margin,
    }
}

/// Les projecteurs recalés du projet, avec leur recalage.
fn registered(project: &Project) -> Vec<(&str, PointWarp)> {
    project
        .projectors
        .iter()
        .filter_map(|p| slide_to_facade(&p.registration).map(|w| (p.id.as_str(), w)))
        .collect()
}

fn assign_one(p: &Placement, warps: &[(&str, PointWarp)]) -> Assignment {
    let points = outline(p);
    // Posé dans une diapositive (D-3.6-g) : il y reste, recalée ou non.
    if let Some(slide) = &p.slide {
        let s = SLIDE_SIZE_MM;
        let whole = points
            .iter()
            .all(|q| q.x >= 0.0 && q.y >= 0.0 && q.x <= s && q.y <= s);
        return Assignment {
            placement: p.id.clone(),
            projector: Some(slide.clone()),
            manual: true,
            whole,
            candidates: vec![slide.clone()],
        };
    }
    let fits: Vec<(&str, Fit)> = warps.iter().map(|(id, w)| (*id, fit(&points, w))).collect();
    let candidates: Vec<String> = fits
        .iter()
        .filter(|(_, f)| f.margin >= 0.0)
        .map(|(id, _)| id.to_string())
        .collect();
    let fit_of = |id: &str| fits.iter().find(|(k, _)| *k == id).map(|(_, f)| *f);
    // Le choix à la main, s'il désigne un projecteur recalé.
    if let Some(chosen) = p.projector.as_deref() {
        if let Some(f) = fit_of(chosen) {
            return Assignment {
                placement: p.id.clone(),
                projector: Some(chosen.to_string()),
                manual: true,
                whole: f.margin >= 0.0,
                candidates,
            };
        }
    }
    // Le plus loin du bord parmi les diapositives qui le contiennent ; sinon
    // la plus grande part du contour. À égalité, le premier projecteur.
    let best =
        fits.iter()
            .filter(|(_, f)| f.inside > 0.0)
            .fold(None::<(&str, Fit)>, |best, &(id, f)| match best {
                Some((_, b)) if (b.inside, b.margin) >= (f.inside, f.margin) => best,
                _ => Some((id, f)),
            });
    Assignment {
        placement: p.id.clone(),
        projector: best.map(|(id, _)| id.to_string()),
        manual: false,
        whole: best.is_some_and(|(_, f)| f.margin >= 0.0),
        candidates,
    }
}

/// La diapositive de chaque dessin posé, dans l'ordre de `placements`.
pub fn assignments(project: &Project) -> Vec<Assignment> {
    let warps = registered(project);
    project
        .placements
        .iter()
        .map(|p| assign_one(p, &warps))
        .collect()
}

/// Reprend `all` avec le tracé des dessins (D-4.5-j) : un dessin dont le
/// rectangle dépasse de sa diapositive mais dont la silhouette (sinon le
/// rectangle) y tient en entier n'est plus « à cheval », et ses diapositives
/// candidates sont celles qui contiennent sa silhouette. Le projecteur
/// choisi ne change pas.
pub fn refine_with_silhouettes(
    project: &Project,
    all: &mut [Assignment],
    silhouettes: &[DrawingSilhouette],
) {
    refine_for(project, all, silhouettes, None);
}

/// L'affectation `a` est-elle à revoir pour la seule diapositive `only`
/// (toutes si `None`) ? Hors de toute emprise, un dessin ne tient nulle
/// part : rien à revoir.
fn to_refine(a: &Assignment, only: Option<&str>) -> bool {
    !a.whole && a.projector.is_some() && only.is_none_or(|id| a.projector.as_deref() == Some(id))
}

/// Comme [`refine_with_silhouettes`], pour les seuls dessins de la
/// diapositive `only` (toutes si `None`, D-3.6-i).
fn refine_for(
    project: &Project,
    all: &mut [Assignment],
    silhouettes: &[DrawingSilhouette],
    only: Option<&str>,
) {
    let warps = registered(project);
    for a in all.iter_mut().filter(|a| to_refine(a, only)) {
        let Some(p) = project.placements.iter().find(|p| p.id == a.placement) else {
            continue;
        };
        let polygons = silhouettes
            .iter()
            .find(|s| s.drawing == p.drawing)
            .map_or(&[][..], |s| s.polygons.as_slice());
        let points: Vec<Point> = visible_on_facade(project, p, polygons)
            .into_iter()
            .flatten()
            .map(|[x, y]| Point::new(x, y))
            .collect();
        if p.slide.is_some() {
            let s = SLIDE_SIZE_MM;
            a.whole = points
                .iter()
                .all(|q| q.x >= 0.0 && q.y >= 0.0 && q.x <= s && q.y <= s);
            continue;
        }
        let fits: Vec<(&str, Fit)> = warps.iter().map(|(id, w)| (*id, fit(&points, w))).collect();
        a.candidates = fits
            .iter()
            .filter(|(_, f)| f.margin >= 0.0)
            .map(|(id, _)| id.to_string())
            .collect();
        a.whole = a
            .projector
            .as_deref()
            .is_some_and(|chosen| a.candidates.iter().any(|c| c == chosen));
    }
}

/// Les affectations du projet, « à cheval » jugé sur le tracé (D-4.5-j) :
/// les silhouettes des seuls dessins dont le rectangle dépasse sont lues
/// (détectées et rangées dans le cache `cache_root` au besoin). Une
/// silhouette illisible (fichier du dessin manquant…) laisse ce dessin
/// jugé sur son rectangle.
pub fn shape_assignments(
    dir: &ProjectDir,
    project: &Project,
    cache_root: &Path,
) -> Vec<Assignment> {
    shape_assignments_for(dir, project, cache_root, None)
}

/// Comme [`shape_assignments`], « à cheval » jugé sur le tracé pour les
/// seuls dessins de la diapositive `only` (toutes si `None`) ; les autres
/// gardent le jugement sur leur rectangle. La diapositive de chaque dessin
/// est la même dans les deux cas (D-4.5-j) : la vue Diapositive n'en
/// calcule ainsi qu'une (D-3.6-i).
pub fn shape_assignments_for(
    dir: &ProjectDir,
    project: &Project,
    cache_root: &Path,
    only: Option<&str>,
) -> Vec<Assignment> {
    let mut all = assignments(project);
    let mut ids: Vec<String> = all
        .iter()
        .filter(|a| to_refine(a, only))
        .filter_map(|a| project.placements.iter().find(|p| p.id == a.placement))
        .map(|p| p.drawing.clone())
        .filter(|id| project.drawing(id).is_some())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return all;
    }
    let found = silhouettes(dir, project, &ids, cache_root).unwrap_or_else(|_| {
        ids.iter()
            .filter_map(|id| silhouettes(dir, project, std::slice::from_ref(id), cache_root).ok())
            .flatten()
            .collect()
    });
    refine_for(project, &mut all, &found, only);
    all
}

/// Les affectations du projet enregistré dans `dir`, « à cheval » jugé sur
/// le tracé (D-4.5-j).
pub fn load_shape_assignments(
    dir: &ProjectDir,
    cache_root: &Path,
) -> Result<Vec<Assignment>, crate::project::ProjectError> {
    let project = dir.load()?.project;
    Ok(shape_assignments(dir, &project, cache_root))
}

/// Les dessins posés visibles que porte la diapositive du projecteur
/// `projector`, dans l'ordre d'affichage.
pub fn slide_placements<'a>(project: &'a Project, projector: &str) -> Vec<&'a Placement> {
    slide_placements_in(project, &assignments(project), projector)
}

/// Comme [`slide_placements`], d'après des affectations déjà calculées.
pub fn slide_placements_in<'a>(
    project: &'a Project,
    all: &[Assignment],
    projector: &str,
) -> Vec<&'a Placement> {
    project
        .visible_placements()
        .into_iter()
        .filter(|p| {
            all.iter()
                .any(|a| a.placement == p.id && a.projector.as_deref() == Some(projector))
        })
        .collect()
}

/// Les affectations du projet enregistré dans `dir`.
pub fn load_assignments(
    dir: &crate::project::ProjectDir,
) -> Result<Vec<Assignment>, crate::project::ProjectError> {
    Ok(assignments(&dir.load()?.project))
}

//! Simulation (tâche 6.1) : tous les projecteurs ensemble sur la façade, et
//! les alertes qui en découlent.
//!
//! - **Dessin coupé** : un dessin posé qu'aucune diapositive ne contient en
//!   entier, ou que le choix à la main met dans une diapositive dont il
//!   dépasse ; seule la partie qui tombe dans la diapositive est projetée.
//! - **Hors emprise** : un dessin posé qui ne tombe dans aucune emprise ; il
//!   n'est pas projeté du tout.
//! - **Projeté deux fois** : là où deux emprises se recouvrent, un trait
//!   allumé dans les deux diapositives (la fresque importée en une image,
//!   découpée entre les diapositives, ou deux dessins qui se touchent). Le
//!   constat se fait sur les aperçus des diapositives, calculés comme
//!   l'export (`export::preview`).
//!
//! Chaque alerte donne son rectangle sur la façade, pour y mener la vue, et
//! le dessin posé concerné quand il y en a un.

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Serialize;

use crate::export::preview::{slide_views, SLIDE_PREVIEW_SIDE};
use crate::export::ExportError;
use crate::geometry::{Point, PointWarp, Transform};
use crate::placement::assign::{assignments, Assignment};
use crate::project::{Placement, PointMm, Project, ProjectDir, RectMm};
use crate::registration::{footprint, slide_to_facade};
use crate::tiles::{self, Raster};
use crate::units::SLIDE_SIZE_MM;

/// Un pixel d'aperçu est « allumé » à partir de ce niveau (sur 255).
pub const LIT_LEVEL: u8 = 128;

/// Côté des blocs qui regroupent les points projetés deux fois en une
/// alerte, en pixels d'aperçu de diapositive.
const DOUBLED_BLOCK_PX: f64 = 48.0;

/// Moins de points projetés deux fois que ceci dans un groupe : bruit
/// (un pixel d'antialiasing), pas d'alerte.
const DOUBLED_MIN_POINTS: usize = 4;

/// Points pris sur chaque bord d'un dessin pour son rectangle sur la façade.
const OUTLINE_POINTS_PER_SIDE: usize = 16;

/// La nature d'une alerte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AlertKind {
    /// Dessin coupé par le bord de sa diapositive.
    Cut,
    /// Dessin hors de toute emprise.
    Outside,
    /// Lumière projetée deux fois dans un recouvrement.
    Doubled,
}

/// Une alerte de la simulation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub kind: AlertKind,
    /// Le dessin posé concerné (identifiant du placement), s'il y en a un.
    pub placement: Option<String>,
    /// Les projecteurs en cause, par identifiant.
    pub projectors: Vec<String>,
    /// Ce qu'on dit, en français.
    pub message: String,
    /// Où regarder sur la façade (mm).
    pub bounds: RectMm,
}

/// Un projecteur dans la simulation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulatedSlide {
    pub projector_id: String,
    /// Aperçu de la diapositive dans le cache (`diapositives/<clé>.png`), à
    /// tendre sur le maillage de la vue façade.
    pub preview: Option<String>,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Simulation {
    pub slides: Vec<SimulatedSlide>,
    pub alerts: Vec<Alert>,
}

/// L'aperçu d'une diapositive, en niveaux de 0 à 255 (carré, mm de
/// diapositive).
pub struct LitSlide {
    pub side: u32,
    pub data: Vec<u8>,
}

impl LitSlide {
    /// D'après un aperçu du cache : le blanc à opacité variable donne son
    /// opacité ; une image grise, sa valeur.
    pub fn from_raster(raster: &Raster) -> Self {
        let c = raster.channels as usize;
        let data = if c == 4 {
            raster
                .data
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| p[3])
                .collect()
        } else {
            raster.data.chunks_exact(c).map(|p| p[0]).collect()
        };
        Self {
            side: raster.width.min(raster.height),
            data,
        }
    }

    fn px_mm(&self) -> f64 {
        SLIDE_SIZE_MM / f64::from(self.side)
    }

    /// Le niveau au point `p` (mm de diapositive) ; `None` hors de la
    /// diapositive.
    fn level(&self, p: Point) -> Option<u8> {
        let n = f64::from(self.side);
        let (x, y) = (p.x / self.px_mm(), p.y / self.px_mm());
        if !(x >= 0.0 && y >= 0.0 && x < n && y < n) {
            return None;
        }
        Some(self.data[y as usize * self.side as usize + x as usize])
    }

    /// Le plus haut niveau autour du point (3 × 3 pixels) : un trait fin
    /// d'une diapositive retrouve celui de l'autre malgré l'arrondi.
    fn level_around(&self, p: Point) -> Option<u8> {
        let n = self.side as i64;
        let (x, y) = (
            (p.x / self.px_mm()).floor() as i64,
            (p.y / self.px_mm()).floor() as i64,
        );
        if x < 0 || y < 0 || x >= n || y >= n {
            return None;
        }
        let mut best = 0;
        for j in (y - 1).max(0)..=(y + 1).min(n - 1) {
            for i in (x - 1).max(0)..=(x + 1).min(n - 1) {
                best = best.max(self.data[(j * n + i) as usize]);
            }
        }
        Some(best)
    }
}

/// Calcule la simulation du projet : l'aperçu de chaque diapositive (dans
/// le cache, fabriqué s'il manque) et les alertes.
pub fn simulate(dir: &ProjectDir, cache_root: &Path) -> Result<Simulation, ExportError> {
    let project = dir.load()?.project;
    let views = slide_views(dir, cache_root, SLIDE_PREVIEW_SIDE)?;
    let mut lit = HashMap::new();
    let mut slides = Vec::new();
    for view in views {
        if let Some(request) = &view.preview {
            let raster = tiles::read_raster(&cache_root.join(request))
                .map_err(|e| ExportError::Image(e.to_string()))?;
            lit.insert(view.projector_id.clone(), LitSlide::from_raster(&raster));
        }
        slides.push(SimulatedSlide {
            projector_id: view.projector_id,
            preview: view.preview,
            problem: view.problem,
        });
    }
    Ok(Simulation {
        slides,
        alerts: alerts(&project, &lit),
    })
}

/// Toutes les alertes : celles des dessins posés, dans l'ordre des
/// placements, puis la lumière projetée deux fois, couple de projecteurs par
/// couple de projecteurs.
pub fn alerts(project: &Project, lit: &HashMap<String, LitSlide>) -> Vec<Alert> {
    let all = assignments(project);
    let mut out = drawing_alerts(project, &all);
    out.extend(doubled_alerts(project, &all, lit));
    out
}

fn label(project: &Project, id: &str) -> String {
    project
        .projector(id)
        .map_or_else(|| id.to_string(), |p| p.label.clone())
}

fn drawing_name(project: &Project, p: &Placement) -> String {
    project
        .library
        .iter()
        .find(|d| d.id == p.drawing)
        .map_or_else(|| p.drawing.clone(), |d| d.name.clone())
}

/// Le rectangle d'un dessin posé sur la façade, déformation comprise.
pub fn placement_bounds(p: &Placement) -> RectMm {
    let n = OUTLINE_POINTS_PER_SIDE;
    let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
    let points = (0..4).flat_map(|k| {
        let (a, b) = (corners[k], corners[(k + 1) % 4]);
        (0..n).map(move |step| {
            let t = step as f64 / n as f64;
            p.facade_point(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
        })
    });
    bounds_of(points).unwrap_or(RectMm {
        x: p.center.x,
        y: p.center.y,
        width: 0.0,
        height: 0.0,
    })
}

fn bounds_of(points: impl Iterator<Item = PointMm>) -> Option<RectMm> {
    let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
    let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in points {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    (x0 <= x1 && y0 <= y1).then_some(RectMm {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
    })
}

fn intersects(a: &RectMm, b: &RectMm) -> bool {
    a.x <= b.x + b.width && b.x <= a.x + a.width && a.y <= b.y + b.height && b.y <= a.y + a.height
}

/// Les dessins coupés et hors emprise. Les dessins d'un calque masqué ne
/// partent pas à l'export : pas d'alerte pour eux.
fn drawing_alerts(project: &Project, all: &[Assignment]) -> Vec<Alert> {
    let mut out = Vec::new();
    for p in project.visible_placements() {
        let Some(a) = all.iter().find(|a| a.placement == p.id) else {
            continue;
        };
        let name = drawing_name(project, p);
        let names = |ids: &[String]| {
            ids.iter()
                .map(|id| label(project, id))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let (kind, projectors, message) = match &a.projector {
            None => (
                AlertKind::Outside,
                Vec::new(),
                format!("« {name} » ne tombe dans aucune emprise : il ne sera pas projeté."),
            ),
            Some(chosen) if !a.whole => {
                let here = label(project, chosen);
                let message = if a.candidates.is_empty() {
                    format!(
                        "« {name} » est coupé : aucune diapositive ne le contient en entier ; \
                         {here} n'en projette qu'une partie."
                    )
                } else {
                    format!(
                        "« {name} » dépasse de la diapositive {here}, choisie à la main ; \
                         {} le contient en entier.",
                        names(&a.candidates)
                    )
                };
                (AlertKind::Cut, vec![chosen.clone()], message)
            }
            Some(_) => continue,
        };
        out.push(Alert {
            kind,
            placement: Some(p.id.clone()),
            projectors,
            message,
            bounds: placement_bounds(p),
        });
    }
    out
}

/// La lumière projetée deux fois : pour chaque couple de projecteurs dont
/// les emprises se recouvrent, chaque pixel allumé de l'aperçu du premier
/// est porté sur la façade puis dans le second ; s'il y est allumé aussi, ce
/// point de la façade reçoit deux fois la lumière. Les points sont groupés
/// par blocs voisins : une alerte par groupe.
fn doubled_alerts(
    project: &Project,
    all: &[Assignment],
    lit: &HashMap<String, LitSlide>,
) -> Vec<Alert> {
    let registered: Vec<(&str, PointWarp, RectMm)> = project
        .projectors
        .iter()
        .filter(|p| lit.contains_key(&p.id))
        .filter_map(|p| {
            let warp = slide_to_facade(&p.registration)?;
            let area = bounds_of(footprint(&warp).into_iter())?;
            Some((p.id.as_str(), warp, area))
        })
        .collect();
    let placements = project.visible_placements();
    let mut out = Vec::new();
    for (i, (a, warp_a, area_a)) in registered.iter().enumerate() {
        for (b, warp_b, area_b) in &registered[i + 1..] {
            if !intersects(area_a, area_b) {
                continue;
            }
            let (slide_a, slide_b) = (&lit[*a], &lit[*b]);
            let points = doubled_points(slide_a, warp_a, area_b, slide_b, warp_b);
            if points.is_empty() {
                continue;
            }
            // Taille d'un bloc sur la façade : quelques dizaines de pixels
            // d'aperçu, d'après l'échelle moyenne de la première diapositive.
            let scale = (area_a.width * area_a.height).sqrt() / SLIDE_SIZE_MM;
            let block = DOUBLED_BLOCK_PX * slide_a.px_mm() * scale;
            for group in groups(&points, block) {
                if group.len() < DOUBLED_MIN_POINTS {
                    continue;
                }
                let bounds = bounds_of(group.iter().map(|p| PointMm::new(p.x, p.y)))
                    .expect("groupe non vide");
                // Un dessin posé de l'une ou l'autre diapositive qui touche
                // la zone : c'est lui que l'alerte montre.
                let concerned = placements
                    .iter()
                    .filter(|p| {
                        all.iter().any(|x| {
                            x.placement == p.id
                                && matches!(x.projector.as_deref(), Some(id) if id == *a || id == *b)
                        })
                    })
                    .find(|p| intersects(&placement_bounds(p), &bounds));
                let what = match concerned {
                    Some(p) => format!("« {} »", drawing_name(project, p)),
                    None if project.fresco.is_some() => "la fresque".to_string(),
                    None => "un trait".to_string(),
                };
                out.push(Alert {
                    kind: AlertKind::Doubled,
                    placement: concerned.map(|p| p.id.clone()),
                    projectors: vec![a.to_string(), b.to_string()],
                    message: format!(
                        "Lumière projetée deux fois par {} et {} : {what} est dans les deux \
                         diapositives, sur {:.0} × {:.0} mm.",
                        label(project, a),
                        label(project, b),
                        bounds.width,
                        bounds.height
                    ),
                    bounds,
                });
            }
        }
    }
    out
}

/// Les points de la façade (mm) allumés dans les deux diapositives.
fn doubled_points(
    slide_a: &LitSlide,
    warp_a: &PointWarp,
    area_b: &RectMm,
    slide_b: &LitSlide,
    warp_b: &PointWarp,
) -> Vec<Point> {
    let n = slide_a.side as usize;
    let px = slide_a.px_mm();
    let mut out = Vec::new();
    for (k, &v) in slide_a.data.iter().enumerate() {
        if v < LIT_LEVEL {
            continue;
        }
        let s = Point::new(((k % n) as f64 + 0.5) * px, ((k / n) as f64 + 0.5) * px);
        let Some(f) = warp_a.apply(s) else { continue };
        let inside = f.x >= area_b.x
            && f.y >= area_b.y
            && f.x <= area_b.x + area_b.width
            && f.y <= area_b.y + area_b.height;
        if !inside {
            continue;
        }
        let Some(t) = warp_b.apply_inverse(f) else {
            continue;
        };
        if slide_b.level_around(t).is_some_and(|w| w >= LIT_LEVEL) {
            out.push(f);
        }
    }
    out
}

/// Groupes de points dont les blocs (`block` mm de côté) se touchent, bords
/// et coins compris.
fn groups(points: &[Point], block: f64) -> Vec<Vec<Point>> {
    let block = block.max(1e-6);
    let key = |p: &Point| ((p.x / block).floor() as i64, (p.y / block).floor() as i64);
    let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, p) in points.iter().enumerate() {
        cells.entry(key(p)).or_default().push(i);
    }
    let mut keys: Vec<(i64, i64)> = cells.keys().copied().collect();
    keys.sort_unstable();
    let mut seen: HashSet<(i64, i64)> = HashSet::new();
    let mut out = Vec::new();
    for start in keys {
        if seen.contains(&start) {
            continue;
        }
        seen.insert(start);
        let mut stack = vec![start];
        let mut group = Vec::new();
        while let Some((x, y)) = stack.pop() {
            group.extend(cells[&(x, y)].iter().map(|&i| points[i]));
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let next = (x + dx, y + dy);
                    if cells.contains_key(&next) && !seen.contains(&next) {
                        seen.insert(next);
                        stack.push(next);
                    }
                }
            }
        }
        out.push(group);
    }
    out
}

/// Le niveau d'un point de la façade dans une diapositive : utile aux
/// tests et à la ligne de commande.
pub fn level_at(slide: &LitSlide, warp: &PointWarp, facade: Point) -> Option<u8> {
    slide.level(warp.apply_inverse(facade)?)
}

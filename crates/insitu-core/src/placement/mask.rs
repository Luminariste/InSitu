//! Masque d'un dessin posé (demande de Benjamin du 14 novembre 2026,
//! décision D-4.2-u).
//!
//! Le masque cache des parties d'un dessin posé sans toucher au dessin ni à
//! la bibliothèque. Il s'écrit comme la gomme de la bibliothèque (tâche
//! 4.8) : des coups de pinceau ronds ou des zones fermées, en fractions du
//! dessin, dans l'ordre (`erase` cache, `draw` fait réapparaître). Étant en
//! fractions du dessin, il suit le dessin dans toutes ses transformations.
//!
//! Deux formes du même masque :
//! - une image de visibilité (255 : visible, 0 : caché), peinte par le même
//!   calcul que la gomme, pour l'export PNG et l'aperçu ;
//! - des polygones de la partie cachée, pour l'export SVG, la part non
//!   projetée et la détection « à cheval » ([`hidden_region`],
//!   [`visible_shape`]).
//!
//! Une partie cachée ne cache plus rien derrière elle : la silhouette du
//! dessin y est retirée aussi.

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

use crate::library::retouch;
use crate::project::{RetouchStroke, RetouchTool, SilhouettePolygon};
use crate::tiles::Raster;

/// Côtés du polygone qui approche un bout de pinceau rond.
const ROUND_SIDES: usize = 24;

/// Un contour fermé ; une forme : son contour extérieur, puis ses trous.
pub type Contour = Vec<[f64; 2]>;
pub type Shape = Vec<Contour>;

/// L'image de visibilité du masque, `width × height` : 255 là où le dessin
/// se voit, 0 là où il est caché, bord adouci sur un pixel.
pub fn visibility(strokes: &[RetouchStroke], width: u32, height: u32) -> Raster {
    let mut raster = Raster::new(
        width,
        height,
        1,
        vec![255; width as usize * height as usize],
    );
    retouch::paint(&mut raster, strokes);
    raster
}

/// Taille de l'image de visibilité pour un dessin de `width × height`
/// pixels : la même, ramenée à `side` pixels au plus.
pub fn visibility_size(width: u32, height: u32, side: u32) -> (u32, u32) {
    let scale = (f64::from(side) / f64::from(width.max(height).max(1))).min(1.0);
    (
        ((f64::from(width) * scale).round() as u32).max(1),
        ((f64::from(height) * scale).round() as u32).max(1),
    )
}

/// Proportions du dessin, le plus grand côté valant 1 : l'épaisseur d'un
/// coup de pinceau est en fraction du plus grand côté, comme pour la gomme.
pub fn aspect(width: f64, height: f64) -> (f64, f64) {
    let side = width.max(height);
    if !(side > 0.0 && side.is_finite()) {
        return (1.0, 1.0);
    }
    (width / side, height / side)
}

fn signed_area(c: &[[f64; 2]]) -> f64 {
    let n = c.len();
    let mut twice = 0.0;
    for k in 0..n {
        let (a, b) = (c[k], c[(k + 1) % n]);
        twice += a[0] * b[1] - b[0] * a[1];
    }
    twice / 2.0
}

/// Les formes avec leur contour extérieur de surface positive (les trous
/// dans l'autre sens), pour qu'on puisse les combiner « non nul ».
fn oriented(shapes: Vec<Shape>) -> Vec<Shape> {
    shapes
        .into_iter()
        .filter(|s| s.first().is_some_and(|c| c.len() >= 3))
        .map(|mut shape| {
            if signed_area(&shape[0]) < 0.0 {
                for c in &mut shape {
                    c.reverse();
                }
            }
            shape
        })
        .collect()
}

fn flat(shapes: &[Shape]) -> Vec<Contour> {
    shapes.iter().flatten().cloned().collect()
}

/// Un disque de rayon `r` autour de `c`.
fn disc(c: [f64; 2], r: f64) -> Contour {
    (0..ROUND_SIDES)
        .map(|k| {
            let a = std::f64::consts::TAU * k as f64 / ROUND_SIDES as f64;
            [c[0] + r * a.cos(), c[1] + r * a.sin()]
        })
        .collect()
}

/// Le trait de `a` à `b`, d'épaisseur `2 r`, à bouts ronds.
fn capsule(a: [f64; 2], b: [f64; 2], r: f64) -> Contour {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    if dx.hypot(dy) < 1e-12 {
        return disc(a, r);
    }
    let theta = dy.atan2(dx);
    let half = ROUND_SIDES / 2;
    let mut out = Vec::with_capacity(2 * (half + 1));
    for (centre, start) in [
        (b, -std::f64::consts::FRAC_PI_2),
        (a, std::f64::consts::FRAC_PI_2),
    ] {
        for k in 0..=half {
            let t = theta + start + std::f64::consts::PI * k as f64 / half as f64;
            out.push([centre[0] + r * t.cos(), centre[1] + r * t.sin()]);
        }
    }
    out
}

/// La surface couverte par un coup, dans le repère où le plus grand côté
/// du dessin vaut 1 (`aspect`).
fn stroke_shapes(stroke: &RetouchStroke, aspect: (f64, f64)) -> Vec<Shape> {
    let pts: Vec<[f64; 2]> = stroke
        .points
        .iter()
        .map(|&[u, v]| [u * aspect.0, v * aspect.1])
        .collect();
    if stroke.fill {
        if pts.len() < 3 {
            return Vec::new();
        }
        return oriented(vec![pts].overlay(
            &Vec::<Contour>::new(),
            OverlayRule::Subject,
            FillRule::EvenOdd,
        ));
    }
    let r = stroke.size / 2.0;
    if r.is_nan() || r <= 0.0 || pts.is_empty() {
        return Vec::new();
    }
    // Des points plus proches qu'un quart de rayon n'ajoutent rien.
    let mut kept: Vec<[f64; 2]> = Vec::with_capacity(pts.len());
    for (k, p) in pts.iter().enumerate() {
        let last = k + 1 == pts.len();
        match kept.last() {
            Some(q) if !last && (p[0] - q[0]).hypot(p[1] - q[1]) < r / 4.0 => {}
            _ => kept.push(*p),
        }
    }
    let pieces: Vec<Contour> = if kept.len() == 1 {
        vec![disc(kept[0], r)]
    } else {
        kept.windows(2)
            .map(|w| capsule(w[0], w[1], r))
            .map(|mut c| {
                if signed_area(&c) < 0.0 {
                    c.reverse();
                }
                c
            })
            .collect()
    };
    oriented(pieces.overlay(
        &Vec::<Contour>::new(),
        OverlayRule::Subject,
        FillRule::NonZero,
    ))
}

/// La partie cachée par le masque, en fractions du dessin : des formes
/// (contour extérieur de surface positive, puis ses trous). Vide si rien
/// n'est caché.
pub fn hidden_region(strokes: &[RetouchStroke], aspect: (f64, f64)) -> Vec<Shape> {
    let mut region: Vec<Shape> = Vec::new();
    for stroke in strokes {
        let piece = flat(&stroke_shapes(stroke, aspect));
        if piece.is_empty() {
            continue;
        }
        let subject = flat(&region);
        region = match stroke.tool {
            RetouchTool::Erase => {
                oriented(subject.overlay(&piece, OverlayRule::Union, FillRule::NonZero))
            }
            RetouchTool::Draw if region.is_empty() => continue,
            RetouchTool::Draw => {
                oriented(subject.overlay(&piece, OverlayRule::Difference, FillRule::NonZero))
            }
        };
    }
    let back = |c: &Contour| {
        c.iter()
            .map(|&[x, y]| [x / aspect.0, y / aspect.1])
            .collect()
    };
    region
        .iter()
        .map(|shape| shape.iter().map(back).collect())
        .collect()
}

/// Une forme à trous en un seul polygone à remplir « pair-impair » : chaque
/// trou est relié au contour par un aller-retour sans épaisseur.
pub fn bridged(shape: &[Contour]) -> Contour {
    let Some(outer) = shape.first() else {
        return Vec::new();
    };
    let mut poly = outer.clone();
    for hole in shape.iter().skip(1).filter(|h| h.len() >= 3) {
        let (mut best, mut at, mut from) = (f64::INFINITY, 0, 0);
        for (i, p) in poly.iter().enumerate() {
            for (k, q) in hole.iter().enumerate() {
                let d = (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2);
                if d < best {
                    (best, at, from) = (d, i, k);
                }
            }
        }
        let mut next = Vec::with_capacity(poly.len() + hole.len() + 2);
        next.extend_from_slice(&poly[..=at]);
        next.extend_from_slice(&hole[from..]);
        next.extend_from_slice(&hole[..=from]);
        next.extend_from_slice(&poly[at..]);
        poly = next;
    }
    poly
}

/// La forme visible d'un dessin masqué : `base` (sa silhouette, ou son
/// rectangle ; chaque polygone rempli pair-impair, réunis entre eux) moins
/// la partie cachée, en polygones à remplir pair-impair (un trou est relié
/// à son contour, voir [`bridged`]). Sans masque, `base` telle quelle ;
/// dessin entièrement caché : aucun polygone.
pub fn visible_shape(
    base: &[SilhouettePolygon],
    strokes: &[RetouchStroke],
    aspect: (f64, f64),
) -> Vec<SilhouettePolygon> {
    if strokes.is_empty() {
        return base.to_vec();
    }
    let hidden = hidden_region(strokes, aspect);
    if hidden.is_empty() {
        return base.to_vec();
    }
    let parts: Vec<Contour> = base
        .iter()
        .filter(|p| p.len() >= 3)
        .flat_map(|p| {
            flat(&oriented(vec![p.clone()].overlay(
                &Vec::<Contour>::new(),
                OverlayRule::Subject,
                FillRule::EvenOdd,
            )))
        })
        .collect();
    let base = oriented(parts.overlay(
        &Vec::<Contour>::new(),
        OverlayRule::Subject,
        FillRule::NonZero,
    ));
    let shapes =
        oriented(flat(&base).overlay(&flat(&hidden), OverlayRule::Difference, FillRule::NonZero));
    shapes
        .iter()
        .map(|s| bridged(s))
        .filter(|p| p.len() >= 3)
        .collect()
}

/// La réunion de polygones remplis chacun pair-impair : des contours sans
/// recouvrement, extérieurs de surface positive et trous dans l'autre sens
/// (à remplir « non nul », ou pair-impair, c'est pareil).
pub fn union_even_odd(polygons: &[Vec<[f64; 2]>]) -> Vec<Vec<[f64; 2]>> {
    let parts: Vec<Contour> = polygons
        .iter()
        .filter(|p| p.len() >= 3)
        .flat_map(|p| {
            flat(&oriented(vec![p.clone()].overlay(
                &Vec::<Contour>::new(),
                OverlayRule::Subject,
                FillRule::EvenOdd,
            )))
        })
        .collect();
    flat(&oriented(parts.overlay(
        &Vec::<Contour>::new(),
        OverlayRule::Subject,
        FillRule::NonZero,
    )))
}

/// Le rectangle entier du dessin, en fractions.
pub fn whole_drawing() -> Vec<SilhouettePolygon> {
    vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]]
}

#[cfg(test)]
mod tests;

//! Dessins posés dans l'export des diapositives (demande de Benjamin après
//! la tâche 4.3, décision D16).
//!
//! Chaque dessin posé sur la façade (tâche 4.2) est calculé dans la
//! diapositive à partir de son fichier d'origine en pleine résolution, par la
//! même chaîne que la fresque : pixel de la diapositive → mm de diapositive
//! → mm de façade (recalage) → position dans le dessin (inverse de son
//! placement) → pixel du dessin. Seul le rectangle de la diapositive que le
//! dessin peut toucher est calculé.
//!
//! Les dessins sont des traits blancs sur fond noir, posés du fond vers
//! l'avant. Chacun cache d'abord, sous sa silhouette (tâche 4.7), ce qui est
//! déjà dans la diapositive (la fresque, les dessins de derrière), puis
//! ajoute ses traits en gardant, pixel par pixel, le plus clair.

use std::collections::HashMap;

use super::{to_gray, ExportError};
use crate::geometry::{Point, PointWarp, Transform};
use crate::library::silhouette;
use crate::library::{flatten_on_black, read::decode};
use crate::placement::mesh;
use crate::project::{Deformation, Placement, PointMm, Project, ProjectDir};
use crate::resample::{warp_raster, WarpOptions};
use crate::tiles::Raster;

/// Points calculés sur chaque bord d'un dessin pour trouver le rectangle
/// qu'il occupe dans la diapositive.
const OUTLINE_POINTS_PER_SIDE: usize = 32;
/// Marge ajoutée autour de ce rectangle, en pixels de diapositive.
const OUTLINE_MARGIN_PX: f64 = 3.0;
/// Côté de rendu d'un SVG : borné pour garder la mémoire raisonnable.
const SVG_SIDE_RANGE: (u32, u32) = (512, 16_384);
/// Plus grand côté du masque de silhouette d'un dessin, en pixels.
const MASK_SIDE: u32 = 2048;

/// Un dessin posé, son image et le masque de sa silhouette (255 dedans ;
/// aucun : le dessin ne cache rien).
#[derive(Clone, Copy)]
pub struct Posed<'a> {
    pub placement: &'a Placement,
    pub image: &'a Raster,
    pub mask: Option<&'a Raster>,
}

/// Rectangle de pixels `[x0, x1[ × [y0, y1[` de la diapositive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

/// Où l'on pose les dessins : la diapositive (`side` pixels de `px_mm` mm,
/// retournée si `mirror`), son recalage, et son projecteur, dont les
/// retouches de dessins s'appliquent (tâche 4.6).
pub struct Placing<'a> {
    pub warp: &'a PointWarp,
    pub px_mm: f64,
    pub side: u32,
    pub mirror: bool,
    pub projector: Option<&'a str>,
}

impl Placing<'_> {
    fn adjustment<'p>(&self, placement: &'p Placement) -> Option<&'p Deformation> {
        self.projector.and_then(|id| placement.adjustment_for(id))
    }
}

/// Le rectangle de la diapositive que le dessin posé peut toucher ; `None`
/// s'il n'y tombe pas.
pub fn slide_rect(placement: &Placement, place: &Placing) -> Option<PixelRect> {
    let Placing {
        warp,
        px_mm,
        side,
        mirror,
        ..
    } = *place;
    let adjustment = place.adjustment(placement);
    let n = OUTLINE_POINTS_PER_SIDE;
    let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
    let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
    let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for k in 0..4 {
        let (a, b) = (corners[k], corners[(k + 1) % 4]);
        for step in 0..n {
            let t = step as f64 / n as f64;
            let facade = placement.facade_point(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            // Un point sans antécédent : on ne sait pas borner, toute la
            // diapositive est calculée pour ce dessin.
            let Some(s) = warp.apply_inverse(Point::new(facade.x, facade.y)) else {
                return Some(PixelRect {
                    x0: 0,
                    y0: 0,
                    x1: side,
                    y1: side,
                });
            };
            // La retouche déplace le dessin dans la diapositive.
            let s = match adjustment {
                Some(d) => {
                    let q = mesh::forward(d, PointMm::new(s.x, s.y));
                    Point::new(q.x, q.y)
                }
                None => s,
            };
            let x = s.x / px_mm;
            let x = if mirror { f64::from(side) - x } else { x };
            let y = s.y / px_mm;
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    let clamp = |v: f64| v.clamp(0.0, f64::from(side));
    let rect = PixelRect {
        x0: clamp((x0 - OUTLINE_MARGIN_PX).floor()) as u32,
        y0: clamp((y0 - OUTLINE_MARGIN_PX).floor()) as u32,
        x1: clamp((x1 + OUTLINE_MARGIN_PX).ceil()) as u32,
        y1: clamp((y1 + OUTLINE_MARGIN_PX).ceil()) as u32,
    };
    (rect.x0 < rect.x1 && rect.y0 < rect.y1).then_some(rect)
}

/// Pose les dessins sur l'image de la diapositive `slide` (niveaux de gris,
/// `side × side`), du fond vers l'avant : sous la silhouette de chacun, ce
/// qui est déjà là est caché ; ses traits gardent le plus clair.
pub fn draw(
    slide: &mut Raster,
    drawings: &[Posed],
    place: &Placing,
    threads: usize,
) -> Result<(), ExportError> {
    let Placing {
        warp,
        px_mm,
        side,
        mirror,
        ..
    } = *place;
    debug_assert_eq!(side, slide.width);
    let options = WarpOptions {
        threads,
        ..WarpOptions::default()
    };
    for posed in drawings {
        let (placement, image) = (posed.placement, posed.image);
        let Some(rect) = slide_rect(placement, place) else {
            continue;
        };
        let adjustment = place.adjustment(placement);
        let (w, h) = (rect.x1 - rect.x0, rect.y1 - rect.y0);
        // Position (u, v) dans le dessin du pixel (x, y) du rectangle.
        let to_drawing = |q: Point| {
            let x = q.x + f64::from(rect.x0);
            let x = if mirror { f64::from(side) - x } else { x };
            let y = q.y + f64::from(rect.y0);
            // La retouche se défait avant de revenir sur la façade.
            let s = match adjustment {
                Some(d) => mesh::inverse(d, PointMm::new(x * px_mm, y * px_mm))?,
                None => PointMm::new(x * px_mm, y * px_mm),
            };
            let facade = warp.apply(Point::new(s.x, s.y))?;
            placement.drawing_point(PointMm::new(facade.x, facade.y))
        };
        let in_pixels = |source: &Raster| {
            let (sw, sh) = (f64::from(source.width), f64::from(source.height));
            move |q: Point| to_drawing(q).map(|(u, v)| Point::new(u * sw, v * sh))
        };
        let stride = side as usize;
        let rows = || (rect.y0 as usize..rect.y1 as usize).map(|j| j * stride + rect.x0 as usize);
        // La silhouette d'abord, et seulement s'il y a quelque chose à cacher.
        let covered = rows().any(|s| slide.data[s..s + w as usize].iter().any(|&v| v > 0));
        if let (Some(mask), true) = (posed.mask, covered) {
            let part = warp_raster(mask, w, h, in_pixels(mask), &options)
                .map_err(|e| ExportError::BadSettings(e.to_string()))?;
            for (start, line) in rows().zip(part.data.chunks_exact(w as usize)) {
                for (out, &m) in slide.data[start..start + w as usize].iter_mut().zip(line) {
                    *out = ((u32::from(*out) * (255 - u32::from(m)) + 127) / 255) as u8;
                }
            }
        }
        let part = warp_raster(image, w, h, in_pixels(image), &options)
            .map_err(|e| ExportError::BadSettings(e.to_string()))?;
        for (start, line) in rows().zip(part.data.chunks_exact(w as usize)) {
            for (out, &v) in slide.data[start..start + w as usize].iter_mut().zip(line) {
                *out = (*out).max(v);
            }
        }
    }
    Ok(())
}

/// Dessins de la bibliothèque lus en pleine résolution, en niveaux de gris,
/// gardés le temps d'un export (un dessin posé plusieurs fois n'est lu
/// qu'une fois).
#[derive(Default)]
pub struct DrawingCache {
    images: HashMap<String, (Raster, Option<Raster>)>,
}

impl DrawingCache {
    /// Lit, si ce n'est déjà fait, le dessin de chaque placement donné. Un
    /// SVG est rendu à la taille du plus grand de ses placements, à `px_mm`.
    pub fn load(
        &mut self,
        dir: &ProjectDir,
        project: &Project,
        placements: &[&Placement],
        px_mm: f64,
    ) -> Result<(), ExportError> {
        for p in placements {
            if self.images.contains_key(&p.drawing) {
                continue;
            }
            let drawing = project
                .library
                .iter()
                .find(|d| d.id == p.drawing)
                .ok_or_else(|| {
                    ExportError::Image(format!(
                        "dessin « {} » absent de la bibliothèque",
                        p.drawing
                    ))
                })?;
            let largest = placements
                .iter()
                .filter(|q| q.drawing == p.drawing)
                .map(|q| q.width.max(q.height) / px_mm)
                .fold(0.0, f64::max);
            let svg_side = (largest.ceil() as u32).clamp(SVG_SIDE_RANGE.0, SVG_SIDE_RANGE.1);
            let path = dir.resolve(&drawing.source);
            let decoded = decode(&path, svg_side)
                .map_err(|e| ExportError::Image(format!("dessin « {} » : {e}", drawing.name)))?;
            let image = to_gray(flatten_on_black(decoded.raster));
            // La silhouette retouchée, sinon celle détectée sur l'image.
            let polygons = match &drawing.silhouette {
                Some(polygons) => polygons.clone(),
                None => silhouette::detect(&image),
            };
            let mask = (!polygons.is_empty()).then(|| {
                let scale =
                    (f64::from(MASK_SIDE) / f64::from(image.width.max(image.height))).min(1.0);
                let mw = ((f64::from(image.width) * scale).round() as u32).max(1);
                let mh = ((f64::from(image.height) * scale).round() as u32).max(1);
                silhouette::rasterize(&polygons, mw, mh)
            });
            self.images.insert(p.drawing.clone(), (image, mask));
        }
        Ok(())
    }

    /// Les placements avec leur image et leur silhouette (ceux dont le
    /// dessin n'est pas lu sont omis).
    pub fn pair<'a>(&'a self, placements: &[&'a Placement]) -> Vec<Posed<'a>> {
        placements
            .iter()
            .filter_map(|&placement| {
                self.images
                    .get(&placement.drawing)
                    .map(|(image, mask)| Posed {
                        placement,
                        image,
                        mask: mask.as_ref(),
                    })
            })
            .collect()
    }
}

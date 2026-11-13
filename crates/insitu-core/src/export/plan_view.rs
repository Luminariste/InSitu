//! Le plan de la façade vu par un projecteur (D-3.6-e) : la partie du plan
//! que couvre l'emprise, rendue puis ramenée dans le carré de la
//! diapositive par le recalage, comme la fresque. Le plan sert de repère
//! dans la vue diapositive ; il n'est jamais exporté.
//!
//! Les traits noirs du plan deviennent blancs sur noir, puis le noir devient
//! transparent (comme l'aperçu de la diapositive) : l'interface les colore.

use std::path::Path;

use crate::facade::display::rotate_plan;
use crate::facade::{open_vector, SourceKind};
use crate::geometry::PointWarp;
use crate::project::model::RectMm;
use crate::project::{Facade, ProjectDir};
use crate::registration::footprint;
use crate::tiles::Raster;

/// Plus grand côté, en pixels, de la partie du plan rendue pour une
/// diapositive : un peu plus fin que l'aperçu de 2 048 px.
pub const PLAN_WINDOW_PX: u32 = 3_072;

/// La partie du plan sous l'emprise du recalage `warp`, en niveaux de gris
/// négatifs (traits blancs sur noir), et le rectangle de façade (mm) qu'elle
/// couvre. `None` sans plan à l'échelle, si l'emprise tombe hors du plan, ou
/// si le plan est illisible.
pub fn plan_under_footprint(
    dir: &ProjectDir,
    facade: &Facade,
    warp: &PointWarp,
) -> Option<(Raster, RectMm)> {
    let mm_per_unit = facade.scale.as_ref()?.mm_per_source_unit()?;
    let (plan_w, plan_h) = facade.size_mm()?;
    let outline = footprint(warp);
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in &outline {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    // Seulement ce qui est sur le plan.
    let (x0, y0, x1, y1) = (x0.max(0.0), y0.max(0.0), x1.min(plan_w), y1.min(plan_h));
    if !(x1 > x0 && y1 > y0) {
        return None;
    }
    // Pixels par mm de façade : le plus grand côté fait PLAN_WINDOW_PX.
    let k = f64::from(PLAN_WINDOW_PX) / (x1 - x0).max(y1 - y0);
    let path = dir.resolve(&facade.source);
    let (gray, frame) = match SourceKind::of(&path).ok()? {
        SourceKind::Image => image_window(&path, facade, mm_per_unit, (x0, y0, x1, y1), k)?,
        kind => {
            let plan = rotate_plan(open_vector(&path, kind, facade.page).ok()?, facade.rotation);
            let w = (((x1 - x0) * k).ceil() as u32).max(1);
            let h = (((y1 - y0) * k).ceil() as u32).max(1);
            let rgba = plan.render(k * mm_per_unit, x0 * k, y0 * k, w, h);
            let frame = RectMm {
                x: x0,
                y: y0,
                width: f64::from(w) / k,
                height: f64::from(h) / k,
            };
            (negative_gray(&rgba, w, h), frame)
        }
    };
    Some((gray, frame))
}

/// La fenêtre d'un plan image : découpée dans le fichier (tourné comme à
/// l'écran), réduite si elle dépasse `PLAN_WINDOW_PX`.
fn image_window(
    path: &Path,
    facade: &Facade,
    mm_per_unit: f64,
    (x0, y0, x1, y1): (f64, f64, f64, f64),
    k: f64,
) -> Option<(Raster, RectMm)> {
    let image = crate::photo::read(path).ok()?;
    let image = match facade.rotation % 4 {
        1 => image.rotate90(),
        2 => image.rotate180(),
        3 => image.rotate270(),
        _ => image,
    };
    let px = |mm: f64| (mm / mm_per_unit).max(0.0);
    let left = (px(x0).floor() as u32).min(image.width().saturating_sub(1));
    let top = (px(y0).floor() as u32).min(image.height().saturating_sub(1));
    let right = (px(x1).ceil() as u32).clamp(left + 1, image.width());
    let bottom = (px(y1).ceil() as u32).clamp(top + 1, image.height());
    let mut window = image.crop_imm(left, top, right - left, bottom - top);
    // Pas plus fin qu'il ne faut : k pixels par mm au plus.
    let wanted = ((f64::from(right - left) * mm_per_unit * k).ceil() as u32).max(1);
    if wanted < window.width() {
        let ratio = f64::from(wanted) / f64::from(window.width());
        let h = ((f64::from(window.height()) * ratio).round() as u32).max(1);
        window = window.resize_exact(wanted, h, image::imageops::FilterType::Triangle);
    }
    let rgba = window.into_rgba8();
    let frame = RectMm {
        x: f64::from(left) * mm_per_unit,
        y: f64::from(top) * mm_per_unit,
        width: f64::from(right - left) * mm_per_unit,
        height: f64::from(bottom - top) * mm_per_unit,
    };
    Some((
        negative_gray(rgba.as_raw(), rgba.width(), rgba.height()),
        frame,
    ))
}

/// Niveaux de gris en négatif, posés sur du blanc : un trait noir devient
/// blanc, le fond blanc (ou transparent) devient noir.
/// `pixels` : RVBA, ligne par ligne.
fn negative_gray(pixels: &[u8], width: u32, height: u32) -> Raster {
    let data = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| {
            let (r, g, b, a) = (
                u32::from(p[0]),
                u32::from(p[1]),
                u32::from(p[2]),
                u32::from(p[3]),
            );
            let lum = (299 * r + 587 * g + 114 * b) / 1000;
            let on_white = (lum * a + 255 * (255 - a)) / 255;
            (255 - on_white) as u8
        })
        .collect();
    Raster::new(width, height, 1, data)
}

//! Vue façade (tâche 3.3) : tous les projecteurs recalés, posés ensemble sur
//! le plan de la façade.
//!
//! Pour chaque projecteur recalé (au moins 4 paires), le moteur fournit :
//!
//! - son **emprise** : le contour de la diapositive envoyé sur la façade ;
//! - un **maillage** : une grille régulière de la diapositive (17 × 17
//!   points) et la position de chaque point sur la façade. L'interface y
//!   tend la photo redressée, qui épouse ainsi le recalage, relief compris ;
//! - un **aperçu** de la photo redressée (1 024 px au plus), à tendre sur ce
//!   maillage, rangé dans le cache de l'app.
//!
//! Et, pour chaque couple de projecteurs qui se chevauchent, la **zone de
//! recouvrement** : son contour et sa surface. Les emprises ne sont pas
//! forcément convexes (relief) : la zone est calculée sur une grille fine
//! (400 cases sur le plus grand côté de la boîte commune), puis détourée.

#[cfg(test)]
mod tests;

use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::UNIX_EPOCH;

use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder};
use serde::Serialize;

use crate::geometry::{Point, Transform};
use crate::project::model::new_id;
use crate::project::{PointMm, Project, ProjectDir, ProjectError};
use crate::registration::{footprint, slide_to_facade};
use crate::tiles::{self, Raster};
use crate::units::SLIDE_SIZE_MM;
use crate::vectorize::contour::{trace, Field};

/// Cases du maillage sur chaque côté de la diapositive (17 × 17 points).
pub const MESH_DIVISIONS: usize = 16;

/// Plus grand côté de l'aperçu d'une photo redressée, en pixels.
pub const PHOTO_PREVIEW_SIDE: u32 = 1024;

/// Cases de la grille de calcul des recouvrements, sur le plus grand côté.
const OVERLAP_GRID: f64 = 400.0;

/// Sous-dossier du cache où sont rangés les aperçus des photos.
pub const PREVIEW_DIR: &str = "apercus";

/// La photo redressée posée sur la façade : les points d'une grille
/// régulière de la diapositive, ligne par ligne (de haut en bas), et leur
/// position sur la façade.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideMesh {
    /// Cases par côté : la grille a `(divisions + 1)²` points.
    pub divisions: usize,
    /// Position sur la façade (mm) du point `(i, j)` de la diapositive, à
    /// l'indice `j × (divisions + 1) + i`. Ce point est à
    /// `(i, j) × 280 / divisions` mm sur la diapositive.
    pub facade: Vec<PointMm>,
}

/// Un projecteur dans la vue façade.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectorOverview {
    pub projector_id: String,
    pub label: String,
    /// Nombre de paires de points du recalage.
    pub pairs: usize,
    /// Emprise sur la façade (mm), fermée ; vide si le projecteur n'est pas
    /// recalé.
    pub footprint: Vec<PointMm>,
    /// Surface de l'emprise, en m².
    pub area_m2: Option<f64>,
    pub mesh: Option<SlideMesh>,
    /// Aperçu de la photo redressée dans le cache (`apercus/<clé>.jpg`).
    pub photo_preview: Option<String>,
    /// Ce qui manque pour le montrer, ou ce qui a échoué.
    pub problem: Option<String>,
}

/// Zone couverte par deux projecteurs à la fois.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overlap {
    /// Les deux projecteurs, par identifiant, dans l'ordre du projet.
    pub projectors: [String; 2],
    pub area_m2: f64,
    /// Contours de la zone (mm), fermés ; plusieurs si elle est en morceaux.
    pub outlines: Vec<Vec<PointMm>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacadeOverview {
    pub projectors: Vec<ProjectorOverview>,
    pub overlaps: Vec<Overlap>,
}

/// Calcule la vue façade d'un projet. Avec un cache, l'aperçu de chaque
/// photo redressée y est fabriqué s'il n'y est pas encore.
pub fn overview(dir: &ProjectDir, project: &Project, cache_root: Option<&Path>) -> FacadeOverview {
    let mut projectors = Vec::new();
    for projector in &project.projectors {
        let mut item = ProjectorOverview {
            projector_id: projector.id.clone(),
            label: projector.label.clone(),
            pairs: projector.registration.len(),
            footprint: Vec::new(),
            area_m2: None,
            mesh: None,
            photo_preview: None,
            problem: None,
        };
        match slide_to_facade(&projector.registration) {
            None if projector.registration.len() < 4 => {
                item.problem = Some("pas encore recalé (4 points au moins)".into());
            }
            None => {
                item.problem = Some("recalage impossible à calculer : vérifier ses points".into())
            }
            Some(warp) => {
                item.footprint = footprint(&warp);
                item.area_m2 = Some(polygon_area(&item.footprint) / 1e6);
                item.mesh = mesh(&warp);
            }
        }
        if let (Some(cache), Some(photo)) = (cache_root, &projector.rectified_photo) {
            if item.mesh.is_some() {
                match ensure_photo_preview(&dir.resolve(photo), cache, PHOTO_PREVIEW_SIDE) {
                    Ok(request) => item.photo_preview = Some(request),
                    Err(error) => item.problem = Some(error),
                }
            }
        }
        projectors.push(item);
    }
    let mut overlaps = Vec::new();
    for (a, first) in projectors.iter().enumerate() {
        for second in &projectors[a + 1..] {
            if first.footprint.len() < 3 || second.footprint.len() < 3 {
                continue;
            }
            if let Some((area_mm2, outlines)) = intersection(&first.footprint, &second.footprint) {
                overlaps.push(Overlap {
                    projectors: [first.projector_id.clone(), second.projector_id.clone()],
                    area_m2: area_mm2 / 1e6,
                    outlines,
                });
            }
        }
    }
    FacadeOverview {
        projectors,
        overlaps,
    }
}

/// Comme [`overview`], en lisant le projet.
pub fn project_overview(
    dir: &ProjectDir,
    cache_root: Option<&Path>,
) -> Result<FacadeOverview, ProjectError> {
    let project = dir.load()?.project;
    Ok(overview(dir, &project, cache_root))
}

fn mesh(warp: &impl Transform) -> Option<SlideMesh> {
    let n = MESH_DIVISIONS;
    let step = SLIDE_SIZE_MM / n as f64;
    let mut facade = Vec::with_capacity((n + 1) * (n + 1));
    for j in 0..=n {
        for i in 0..=n {
            let p = warp.apply(Point::new(i as f64 * step, j as f64 * step))?;
            facade.push(PointMm::new(p.x, p.y));
        }
    }
    Some(SlideMesh {
        divisions: n,
        facade,
    })
}

/// Surface d'un polygone (mm²), quel que soit son sens.
pub fn polygon_area(points: &[PointMm]) -> f64 {
    let n = points.len();
    if n < 3 {
        return 0.0;
    }
    let twice: f64 = (0..n)
        .map(|k| {
            let (a, b) = (points[k], points[(k + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    (twice / 2.0).abs()
}

fn bounds(points: &[PointMm]) -> (f64, f64, f64, f64) {
    points.iter().fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |(x0, y0, x1, y1), p| (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y)),
    )
}

/// Remplit un polygone sur une grille (règle pair-impair) : une case est
/// pleine si son centre est dedans.
fn rasterize(
    polygon: &[PointMm],
    x0: f64,
    y0: f64,
    cell: f64,
    width: usize,
    height: usize,
    mask: &mut [u8],
) {
    let n = polygon.len();
    let mut crossings = Vec::new();
    for j in 0..height {
        let y = y0 + (j as f64 + 0.5) * cell;
        crossings.clear();
        for k in 0..n {
            let (a, b) = (polygon[k], polygon[(k + 1) % n]);
            if (a.y <= y && b.y > y) || (b.y <= y && a.y > y) {
                crossings.push(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y));
            }
        }
        crossings.sort_by(f64::total_cmp);
        for span in crossings.chunks_exact(2) {
            // Cases dont le centre x0 + (i + 0,5) × cell est entre les deux.
            let first = ((span[0] - x0) / cell - 0.5).ceil().max(0.0) as usize;
            let last = ((span[1] - x0) / cell - 0.5).floor();
            if last < 0.0 {
                continue;
            }
            let last = (last as usize).min(width - 1);
            for i in first..=last {
                mask[j * width + i] = mask[j * width + i].wrapping_add(1);
            }
        }
    }
}

/// Partie commune de deux polygones : surface (mm²) et contours, ou `None`
/// s'ils ne se chevauchent pas.
fn intersection(a: &[PointMm], b: &[PointMm]) -> Option<(f64, Vec<Vec<PointMm>>)> {
    let (ax0, ay0, ax1, ay1) = bounds(a);
    let (bx0, by0, bx1, by1) = bounds(b);
    let (x0, y0, x1, y1) = (ax0.max(bx0), ay0.max(by0), ax1.min(bx1), ay1.min(by1));
    if !(x1 > x0 && y1 > y0) {
        return None;
    }
    let cell = (x1 - x0).max(y1 - y0) / OVERLAP_GRID;
    let width = ((x1 - x0) / cell).ceil().max(1.0) as usize;
    let height = ((y1 - y0) / cell).ceil().max(1.0) as usize;
    let mut mask = vec![0u8; width * height];
    rasterize(a, x0, y0, cell, width, height, &mut mask);
    rasterize(b, x0, y0, cell, width, height, &mut mask);
    // Les deux remplissages ajoutent 1 : 2 = dans les deux.
    let mut count = 0usize;
    for v in &mut mask {
        *v = if *v == 2 {
            count += 1;
            255
        } else {
            0
        };
    }
    if count == 0 {
        return None;
    }
    let field = Field {
        width,
        height,
        data: &mask,
        threshold: 128,
    };
    let outlines = trace(&field)
        .into_iter()
        .map(|contour| {
            let mut points: Vec<PointMm> = contour
                .into_iter()
                .map(|p| PointMm::new(x0 + p.x * cell, y0 + p.y * cell))
                .collect();
            if let Some(&first) = points.first() {
                points.push(first);
            }
            points
        })
        .collect();
    Some((count as f64 * cell * cell, outlines))
}

/// Renvoie l'adresse dans le cache de l'aperçu (JPEG, `side` px au plus) de
/// la photo `path`, en le fabriquant s'il n'existe pas encore. La clé
/// dépend du chemin, de la taille et de la date du fichier.
pub fn ensure_photo_preview(path: &Path, cache_root: &Path, side: u32) -> Result<String, String> {
    let describe = |e: std::io::Error| format!("photo illisible « {} » : {e}", path.display());
    let canonical = fs::canonicalize(path).map_err(describe)?;
    let metadata = fs::metadata(&canonical).map_err(describe)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    let identity = format!(
        "{}|{}|{modified}|{side}|p1",
        canonical.display(),
        metadata.len()
    );
    let request = format!(
        "{PREVIEW_DIR}/{:016x}.jpg",
        tiles::fnv1a_64(identity.as_bytes())
    );
    let target = cache_root.join(&request);
    if target.is_file() {
        return Ok(request);
    }
    let raster = tiles::read_raster(path).map_err(|e| e.to_string())?;
    let small = shrink(raster, side);
    let rgb = match small.channels {
        1 => small.data.iter().flat_map(|&v| [v, v, v]).collect(),
        3 => small.data,
        _ => small
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|&[r, g, b, _]| [r, g, b])
            .collect(),
    };
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, 85)
        .write_image(&rgb, small.width, small.height, ExtendedColorType::Rgb8)
        .map_err(|e| e.to_string())?;
    let folder = target.parent().expect("apercus/<clé>.jpg");
    fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let temp = folder.join(format!(".~{}", new_id()));
    let written = fs::File::create(&temp)
        .and_then(|mut f| f.write_all(&bytes))
        .and_then(|()| fs::rename(&temp, &target));
    if let Err(error) = written {
        let _ = fs::remove_file(&temp);
        return Err(format!("aperçu impossible à écrire : {error}"));
    }
    Ok(request)
}

/// Réduit de moitié en moitié (moyenne en lumière) jusqu'à tenir dans `side`.
fn shrink(mut raster: Raster, side: u32) -> Raster {
    while raster.width.max(raster.height) > side.max(1) {
        raster = tiles::downsample(&raster);
    }
    raster
}

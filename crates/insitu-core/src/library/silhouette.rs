//! Silhouettes des dessins (tâche 4.7).
//!
//! La silhouette d'un dessin est son contour extérieur, rempli : posée en
//! noir sous le dessin, elle cache ce qui est derrière (la fresque, un autre
//! dessin), comme une découpe de papier noir.
//!
//! Détection automatique, sur une grille réduite d'environ 1 024 cases :
//!
//! 1. une case est « encrée » si un pixel au moins y est plus clair que le
//!    seuil (une réduction par maximum : un trait fin n'est jamais perdu) ;
//! 2. l'encre est épaissie de [`CLOSING_RADIUS`] cases, pour fermer les
//!    petites ouvertures d'un trait discontinu ;
//! 3. le fond est rempli depuis les bords : tout ce qu'il n'atteint pas est
//!    dans la silhouette (l'intérieur d'un anneau aussi) ;
//! 4. la silhouette est amincie d'autant, moins [`MARGIN`] cases : elle
//!    déborde un peu du trait extérieur ;
//! 5. ses contours sont suivis case par case puis simplifiés.
//!
//! Les polygones sont en fractions du dessin : (0, 0) est son coin
//! haut-gauche, (1, 1) son coin bas-droit. Ils suivent ainsi le dessin dans
//! toutes ses transformations et déformations, comme ses pixels.
//!
//! Une silhouette retouchée à la main est écrite dans `projet.json` (champ
//! `silhouette` du dessin) ; une silhouette détectée est rangée dans le
//! cache de l'app (`silhouettes/`), comme les vignettes.

use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::geometry::Point;
use crate::project::{Project, ProjectDir, SilhouettePolygon};
use crate::tiles::{self, Raster};
use crate::vectorize::contour::{self, Field};
use crate::vectorize::fit::douglas_peucker;

use super::read::decode;
use super::{flatten_on_black, parallel_each, thread_count, unreadable, LibraryError};

/// Sous-dossier du cache où sont rangées les silhouettes détectées.
pub const SILHOUETTE_DIR: &str = "silhouettes";

/// Plus grand côté de la grille de détection, en cases.
pub const DETECTION_SIDE: u32 = 1024;

/// Un pixel plus clair que ce niveau (0 à 255) est de l'encre.
pub const INK_THRESHOLD: u8 = 48;

/// Épaississement qui ferme les ouvertures du trait, en cases (environ 1,2 %
/// du plus grand côté du dessin).
pub const CLOSING_RADIUS: f64 = 12.0;

/// Débord de la silhouette autour du trait extérieur, en cases.
pub const MARGIN: f64 = 2.0;

/// Écart toléré à la simplification des contours, en cases.
const SIMPLIFY_TOLERANCE: f64 = 0.75;

/// Un morceau de silhouette plus petit que cette aire (cases²) est une
/// poussière : il est oublié.
const MIN_AREA: f64 = 16.0;

/// Côté de rendu d'un SVG pour la détection.
const SVG_SIDE: u32 = 2048;

/// Version du calcul : à augmenter si la détection change.
const SILHOUETTE_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Détection
// ---------------------------------------------------------------------------

/// La silhouette détectée d'un dessin (traits clairs sur fond noir ; une
/// image avec transparence est d'abord posée sur du noir).
pub fn detect(raster: &Raster) -> Vec<SilhouettePolygon> {
    let (w, h) = (raster.width as usize, raster.height as usize);
    if w == 0 || h == 0 {
        return Vec::new();
    }
    // 1. Grille réduite, encre par maximum.
    let k = (w.max(h)).div_ceil(DETECTION_SIDE as usize).max(1);
    let (gw, gh) = (w.div_ceil(k), h.div_ceil(k));
    let pad = (CLOSING_RADIUS.ceil() as usize) + 2;
    let (pw, ph) = (gw + 2 * pad, gh + 2 * pad);
    let mut ink = vec![false; pw * ph];
    let c = raster.channels as usize;
    let alpha = c == 4;
    for y in 0..h {
        let row = &raster.data[y * w * c..(y + 1) * w * c];
        let gy = y / k + pad;
        for x in 0..w {
            let px = &row[x * c..x * c + c];
            let mut v = match c {
                1 => px[0],
                _ => px[0].max(px[1]).max(px[2]),
            };
            if alpha {
                v = (u16::from(v) * u16::from(px[3]) / 255) as u8;
            }
            if v > INK_THRESHOLD {
                ink[gy * pw + x / k + pad] = true;
            }
        }
    }
    // 2. Épaississement : distance à l'encre.
    let to_ink = distance(&ink, pw, ph);
    let thick: Vec<bool> = to_ink.iter().map(|&d| d <= CLOSING_RADIUS).collect();
    // 3. Fond rempli depuis le bord (la bordure ajoutée n'a jamais d'encre).
    let mut outside = vec![false; pw * ph];
    let mut queue = VecDeque::from([0usize]);
    outside[0] = true;
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i % pw, i / pw);
        let mut visit = |j: usize| {
            if !outside[j] && !thick[j] {
                outside[j] = true;
                queue.push_back(j);
            }
        };
        if x > 0 {
            visit(i - 1);
        }
        if x + 1 < pw {
            visit(i + 1);
        }
        if y > 0 {
            visit(i - pw);
        }
        if y + 1 < ph {
            visit(i + pw);
        }
    }
    // 4. Amincissement : distance au fond.
    let to_outside = distance(&outside, pw, ph);
    let keep = CLOSING_RADIUS - MARGIN;
    let data: Vec<u8> = to_outside
        .iter()
        .zip(&outside)
        .map(|(&d, &out)| if !out && d > keep { 255 } else { 0 })
        .collect();
    // 5. Contours.
    let field = Field {
        width: pw,
        height: ph,
        data: &data,
        threshold: 128,
    };
    let (sx, sy) = (k as f64 / w as f64, k as f64 / h as f64);
    contour::trace(&field)
        .into_iter()
        .filter(|c| contour::signed_area(c) > MIN_AREA)
        .map(|c| {
            simplify(&c)
                .into_iter()
                .map(|p| {
                    [
                        ((p.x - pad as f64) * sx).clamp(0.0, 1.0),
                        ((p.y - pad as f64) * sy).clamp(0.0, 1.0),
                    ]
                })
                .collect::<Vec<_>>()
        })
        .filter(|p| p.len() >= 3)
        .collect()
}

/// Distance (en cases, chanfrein 3-4 ramené à l'unité) de chaque case à la
/// plus proche case marquée.
fn distance(marked: &[bool], w: usize, h: usize) -> Vec<f64> {
    const FAR: u32 = u32::MAX / 2;
    let mut d: Vec<u32> = marked.iter().map(|&m| if m { 0 } else { FAR }).collect();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut best = d[i];
            if x > 0 {
                best = best.min(d[i - 1] + 3);
            }
            if y > 0 {
                best = best.min(d[i - w] + 3);
                if x > 0 {
                    best = best.min(d[i - w - 1] + 4);
                }
                if x + 1 < w {
                    best = best.min(d[i - w + 1] + 4);
                }
            }
            d[i] = best;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            let mut best = d[i];
            if x + 1 < w {
                best = best.min(d[i + 1] + 3);
            }
            if y + 1 < h {
                best = best.min(d[i + w] + 3);
                if x + 1 < w {
                    best = best.min(d[i + w + 1] + 4);
                }
                if x > 0 {
                    best = best.min(d[i + w - 1] + 4);
                }
            }
            d[i] = best;
        }
    }
    d.into_iter().map(|v| f64::from(v) / 3.0).collect()
}

/// Contour fermé simplifié (Douglas-Peucker).
fn simplify(points: &[Point]) -> Vec<Point> {
    let n = points.len();
    if n < 4 {
        return points.to_vec();
    }
    // Départ au point le plus éloigné du premier : les deux moitiés du
    // contour se simplifient chacune.
    let far = (1..n)
        .max_by(|&a, &b| {
            points[a]
                .distance(points[0])
                .total_cmp(&points[b].distance(points[0]))
        })
        .unwrap_or(n / 2);
    let mut kept = vec![0, far];
    douglas_peucker(points, 0, far, SIMPLIFY_TOLERANCE, &mut kept);
    douglas_peucker(points, far, n, SIMPLIFY_TOLERANCE, &mut kept);
    kept.sort_unstable();
    kept.dedup();
    kept.into_iter().map(|i| points[i]).collect()
}

// ---------------------------------------------------------------------------
// Masque
// ---------------------------------------------------------------------------

/// Le masque de la silhouette (`width × height`, un canal) : 255 dedans,
/// 0 dehors. Les polygones se réunissent ; chacun est rempli « pair-impair ».
/// Un pixel est dedans si son centre l'est.
pub fn rasterize(polygons: &[SilhouettePolygon], width: u32, height: u32) -> Raster {
    let (w, h) = (width as usize, height as usize);
    let mut data = vec![0u8; w * h];
    let mut crossings = Vec::new();
    for polygon in polygons {
        let n = polygon.len();
        if n < 3 {
            continue;
        }
        let pts: Vec<(f64, f64)> = polygon
            .iter()
            .map(|&[u, v]| (u * width as f64, v * height as f64))
            .collect();
        let y_min = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let y_max = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        let j0 = (y_min - 0.5).ceil().max(0.0) as usize;
        let j1 = ((y_max - 0.5).floor() + 1.0).clamp(0.0, h as f64) as usize;
        for j in j0..j1 {
            let y = j as f64 + 0.5;
            crossings.clear();
            for k in 0..n {
                let (a, b) = (pts[k], pts[(k + 1) % n]);
                if (a.1 <= y) != (b.1 <= y) {
                    crossings.push(a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            crossings.sort_by(f64::total_cmp);
            // Pair-impair dans un polygone, union entre polygones.
            for &[a, b] in crossings.as_chunks::<2>().0 {
                let i0 = (a - 0.5).ceil().clamp(0.0, w as f64) as usize;
                let i1 = (b - 0.5).ceil().clamp(0.0, w as f64) as usize;
                data[j * w + i0..j * w + i1.max(i0)].fill(255);
            }
        }
    }
    Raster::new(width, height, 1, data)
}

// ---------------------------------------------------------------------------
// Silhouettes du projet
// ---------------------------------------------------------------------------

/// La silhouette d'un dessin, telle que l'interface l'affiche.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawingSilhouette {
    pub drawing: String,
    /// Polygones en fractions du dessin.
    pub polygons: Vec<SilhouettePolygon>,
    /// Vrai si la silhouette a été retouchée à la main (écrite dans le
    /// projet) ; faux si elle est détectée.
    pub manual: bool,
}

/// Adresse dans le cache de la silhouette détectée de `path`.
fn cache_request(path: &Path) -> Result<String, LibraryError> {
    let canonical = fs::canonicalize(path).map_err(|e| unreadable(path, e))?;
    let metadata = fs::metadata(&canonical).map_err(|e| unreadable(path, e))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    let identity = format!(
        "{}|{}|{modified}|s{SILHOUETTE_VERSION}",
        canonical.display(),
        metadata.len()
    );
    Ok(format!(
        "{SILHOUETTE_DIR}/{:016x}.json",
        tiles::fnv1a_64(identity.as_bytes())
    ))
}

/// La silhouette détectée du dessin `path`, lue dans le cache ou calculée
/// (et rangée dans le cache).
pub fn ensure_detected(
    path: &Path,
    cache_root: &Path,
) -> Result<Vec<SilhouettePolygon>, LibraryError> {
    let request = cache_request(path)?;
    let target = cache_root.join(&request);
    if let Some(polygons) = fs::read_to_string(&target)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
    {
        return Ok(polygons);
    }
    let raster = flatten_on_black(decode(path, SVG_SIDE)?.raster);
    let polygons = detect(&raster);
    let json = serde_json::to_vec(&polygons).expect("sérialisable");
    super::vectors::write_atomic(cache_root, &request, &json)?;
    Ok(polygons)
}

/// La silhouette de chacun des dessins `ids` (tous si la liste est vide) :
/// celle retouchée à la main, sinon celle détectée. Plusieurs dessins à la
/// fois ; un dessin illisible a une silhouette vide.
pub fn silhouettes(
    dir: &ProjectDir,
    project: &Project,
    ids: &[String],
    cache_root: &Path,
) -> Result<Vec<DrawingSilhouette>, LibraryError> {
    for id in ids {
        if project.drawing(id).is_none() {
            return Err(LibraryError::UnknownDrawing(id.clone()));
        }
    }
    let chosen: Vec<_> = project
        .library
        .iter()
        .filter(|d| ids.is_empty() || ids.contains(&d.id))
        .collect();
    Ok(parallel_each(
        &chosen,
        thread_count(0),
        |drawing| match &drawing.silhouette {
            Some(polygons) => DrawingSilhouette {
                drawing: drawing.id.clone(),
                polygons: polygons.clone(),
                manual: true,
            },
            None => DrawingSilhouette {
                drawing: drawing.id.clone(),
                polygons: ensure_detected(&dir.resolve(&drawing.source), cache_root)
                    .unwrap_or_default(),
                manual: false,
            },
        },
        |_| {},
    ))
}

/// Les silhouettes des dessins du projet enregistré dans `dir`.
pub fn load_silhouettes(
    dir: &ProjectDir,
    ids: &[String],
    cache_root: &Path,
) -> Result<Vec<DrawingSilhouette>, LibraryError> {
    let project = dir.load()?.project;
    silhouettes(dir, &project, ids, cache_root)
}

/// Retouche la silhouette d'un dessin (`None` : revenir à la détection
/// automatique), enregistre le projet et renvoie la silhouette en vigueur.
pub fn set_silhouette(
    dir: &ProjectDir,
    id: &str,
    polygons: Option<Vec<SilhouettePolygon>>,
    cache_root: &Path,
) -> Result<(Project, DrawingSilhouette), LibraryError> {
    if let Some(polygons) = &polygons {
        if let Some(problem) = crate::project::silhouette_problem(polygons) {
            return Err(LibraryError::Silhouette(problem));
        }
    }
    let mut project = dir.load()?.project;
    let drawing = project
        .library
        .iter_mut()
        .find(|d| d.id == id)
        .ok_or_else(|| LibraryError::UnknownDrawing(id.to_string()))?;
    if drawing.silhouette != polygons {
        drawing.silhouette = polygons;
        dir.save(&project)?;
    }
    let current = silhouettes(dir, &project, &[id.to_string()], cache_root)?
        .pop()
        .expect("un dessin demandé");
    Ok((project, current))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un cercle au trait blanc (épaisseur 6 px), ouvert sur `gap` degrés.
    fn ring(side: u32, radius: f64, gap: f64) -> Raster {
        let mut data = vec![0u8; (side * side) as usize];
        let c = f64::from(side) / 2.0;
        for y in 0..side {
            for x in 0..side {
                let (dx, dy) = (f64::from(x) + 0.5 - c, f64::from(y) + 0.5 - c);
                let angle = dy.atan2(dx).to_degrees().abs();
                if ((dx * dx + dy * dy).sqrt() - radius).abs() < 3.0 && angle > gap / 2.0 {
                    data[(y * side + x) as usize] = 255;
                }
            }
        }
        Raster::new(side, side, 1, data)
    }

    fn inside(polygons: &[SilhouettePolygon], u: f64, v: f64) -> bool {
        let mask = rasterize(polygons, 1000, 1000);
        mask.data[(v * 1000.0) as usize * 1000 + (u * 1000.0) as usize] == 255
    }

    #[test]
    fn a_ring_is_filled_even_with_a_small_gap() {
        // 2 000 px : la grille est réduite de moitié. Ouverture de 2° sur un
        // rayon de 800 px, soit 28 px (14 cases) : moins que deux fois
        // l'épaississement.
        let polygons = detect(&ring(2000, 800.0, 2.0));
        assert_eq!(polygons.len(), 1, "un seul contour");
        assert!(inside(&polygons, 0.5, 0.5), "le centre est caché");
        // Le trait est dedans, le dehors reste libre.
        assert!(inside(&polygons, 0.1, 0.5));
        assert!(!inside(&polygons, 0.08, 0.5));
        assert!(!inside(&polygons, 0.02, 0.02));
        // Le débord autour du trait est d'environ deux cases (4 px).
        let left = polygons[0]
            .iter()
            .map(|p| p[0])
            .fold(f64::INFINITY, f64::min);
        let ink_left = (1000.0 - 803.0) / 2000.0;
        assert!(
            (ink_left - left) * 2000.0 > 1.0 && (ink_left - left) * 2000.0 < 8.0,
            "débord {} px",
            (ink_left - left) * 2000.0
        );
    }

    #[test]
    fn a_wide_opening_is_not_closed() {
        // Ouverture de 90° : le fond entre dans l'anneau.
        let polygons = detect(&ring(1000, 400.0, 90.0));
        assert!(!inside(&polygons, 0.5, 0.5));
    }

    #[test]
    fn separate_shapes_give_separate_contours_and_dust_is_ignored() {
        let side = 1000u32;
        let mut data = vec![0u8; (side * side) as usize];
        let mut square = |x0: u32, y0: u32, s: u32| {
            for y in y0..y0 + s {
                for x in x0..x0 + s {
                    data[(y * side + x) as usize] = 200;
                }
            }
        };
        square(100, 100, 200);
        square(600, 600, 300);
        square(900, 100, 1); // une poussière
        let polygons = detect(&Raster::new(side, side, 1, data));
        assert_eq!(polygons.len(), 2);
        // Un carré se simplifie en peu de points.
        assert!(
            polygons.iter().all(|p| p.len() < 40),
            "{:?}",
            polygons.iter().map(Vec::len).collect::<Vec<_>>()
        );
        assert!(inside(&polygons, 0.2, 0.2));
        assert!(inside(&polygons, 0.75, 0.75));
        assert!(!inside(&polygons, 0.5, 0.5));
    }

    #[test]
    fn a_black_drawing_has_no_silhouette() {
        assert!(detect(&Raster::new(64, 64, 1, vec![0; 64 * 64])).is_empty());
    }

    #[test]
    fn rasterize_fills_pixel_centres() {
        let square = vec![[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]];
        let mask = rasterize(&[square], 8, 8);
        let lit: usize = mask.data.iter().filter(|&&v| v == 255).count();
        assert_eq!(lit, 16);
        assert_eq!(mask.data[2 * 8 + 2], 255);
        assert_eq!(mask.data[8 + 1], 0);
    }
}

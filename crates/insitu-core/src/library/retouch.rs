//! Gomme et crayon sur les dessins (tâche 4.8).
//!
//! Une retouche efface des traits du dessin d'origine (gomme : du noir est
//! peint) ou en ajoute (crayon : du blanc est peint), soit le long d'un
//! trait d'une épaisseur donnée, soit dans une zone fermée remplie. Les
//! retouches sont écrites dans `projet.json` (champ `retouch` du dessin), en
//! fractions du dessin comme la silhouette : elles s'appliquent à toute
//! résolution et le fichier importé n'est jamais modifié (règle de la 4.1).
//!
//! Le dessin retouché est fabriqué une fois, à pleine résolution, dans le
//! dossier `retouches/` du projet, sous un nom qui dépend des retouches
//! ([`retouched_file`]). Tout le reste de l'app (vignettes, aperçus,
//! vecteurs, silhouette détectée, export, simulation, fiche) lit ce fichier
//! à la place de l'original : les caches, rangés par fichier, se refont
//! d'eux-mêmes. S'il manque (autre Mac, dossier effacé), il est refait.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::project::model::new_id;
use crate::project::{Drawing, Project, ProjectDir, RetouchStroke, RetouchTool};
use crate::tiles::{self, Raster};

use super::read::{decode, DrawingFormat};
use super::vectors::SVG_VECTOR_SIDE;
use super::{encode_png, entries, flatten_on_black, LibraryEntry, LibraryError};

/// Dossier du projet où sont rangés les dessins retouchés.
pub const RETOUCH_DIR: &str = "retouches";

/// Version du calcul : à augmenter si le rendu des retouches change, pour
/// que les dessins retouchés soient refaits.
const RETOUCH_VERSION: u32 = 1;

/// Sous-échantillons par pixel, en hauteur, pour le bord des zones.
const ZONE_SUBROWS: usize = 4;

/// Peint les retouches sur une image (1 ou 3 canaux, traits clairs sur
/// fond noir), dans l'ordre. Le bord est adouci : un pixel à moitié couvert
/// prend la moitié de la nouvelle valeur.
pub fn paint(raster: &mut Raster, strokes: &[RetouchStroke]) {
    let (w, h) = (raster.width as usize, raster.height as usize);
    if w == 0 || h == 0 {
        return;
    }
    let channels = raster.channels as usize;
    for stroke in strokes {
        let Some((x0, y0, cover, cw)) = coverage(stroke, w, h) else {
            continue;
        };
        let value = match stroke.tool {
            RetouchTool::Erase => 0.0,
            RetouchTool::Draw => 255.0,
        };
        for (j, line) in cover.chunks_exact(cw).enumerate() {
            let row = (y0 + j) * w;
            for (i, &c) in line.iter().enumerate() {
                if c <= 0.0 {
                    continue;
                }
                let at = (row + x0 + i) * channels;
                for v in &mut raster.data[at..at + channels] {
                    let old = f32::from(*v);
                    *v = (old + (value - old) * c.min(1.0)).round() as u8;
                }
            }
        }
    }
}

/// Couverture (0 à 1) d'une retouche sur l'image `w × h`, dans le rectangle
/// qu'elle touche : coin haut-gauche, valeurs ligne par ligne, largeur.
fn coverage(stroke: &RetouchStroke, w: usize, h: usize) -> Option<(usize, usize, Vec<f32>, usize)> {
    let pts: Vec<(f64, f64)> = stroke
        .points
        .iter()
        .map(|&[u, v]| (u * w as f64, v * h as f64))
        .collect();
    let reach = if stroke.fill {
        0.0
    } else {
        stroke.size * w.max(h) as f64 / 2.0
    };
    let x_min = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min) - reach - 1.0;
    let x_max = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max) + reach + 1.0;
    let y_min = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min) - reach - 1.0;
    let y_max = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max) + reach + 1.0;
    let x0 = x_min.floor().clamp(0.0, w as f64) as usize;
    let x1 = x_max.ceil().clamp(0.0, w as f64) as usize;
    let y0 = y_min.floor().clamp(0.0, h as f64) as usize;
    let y1 = y_max.ceil().clamp(0.0, h as f64) as usize;
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let (cw, ch) = (x1 - x0, y1 - y0);
    let mut cover = vec![0f32; cw * ch];
    if stroke.fill {
        zone_coverage(&pts, x0, y0, cw, ch, &mut cover);
    } else {
        line_coverage(&pts, reach, x0, y0, cw, ch, &mut cover);
    }
    Some((x0, y0, cover, cw))
}

/// Trait rond d'épaisseur `2 × radius` : chaque pixel est couvert selon la
/// distance de son centre à la ligne brisée (bord d'un pixel de large).
fn line_coverage(
    pts: &[(f64, f64)],
    radius: f64,
    x0: usize,
    y0: usize,
    cw: usize,
    ch: usize,
    cover: &mut [f32],
) {
    // Un trait plus fin qu'un pixel reste visible, à demi-teinte.
    let radius = radius.max(0.5);
    let segments: Vec<((f64, f64), (f64, f64))> = if pts.len() == 1 {
        vec![(pts[0], pts[0])]
    } else {
        pts.windows(2).map(|p| (p[0], p[1])).collect()
    };
    for &(a, b) in &segments {
        let i0 = ((a.0.min(b.0) - radius - 1.0).floor() as isize - x0 as isize).max(0) as usize;
        let i1 = ((a.0.max(b.0) + radius + 1.0).ceil() as isize - x0 as isize).clamp(0, cw as isize)
            as usize;
        let j0 = ((a.1.min(b.1) - radius - 1.0).floor() as isize - y0 as isize).max(0) as usize;
        let j1 = ((a.1.max(b.1) + radius + 1.0).ceil() as isize - y0 as isize).clamp(0, ch as isize)
            as usize;
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length2 = dx * dx + dy * dy;
        for j in j0..j1 {
            let py = (y0 + j) as f64 + 0.5;
            for i in i0..i1 {
                let px = (x0 + i) as f64 + 0.5;
                let t = if length2 > 0.0 {
                    (((px - a.0) * dx + (py - a.1) * dy) / length2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (qx, qy) = (a.0 + t * dx - px, a.1 + t * dy - py);
                let d = (qx * qx + qy * qy).sqrt();
                let c = (radius + 0.5 - d).clamp(0.0, 1.0) as f32;
                let slot = &mut cover[j * cw + i];
                *slot = slot.max(c);
            }
        }
    }
}

/// Zone fermée, remplie « pair-impair » : chaque ligne de pixels est
/// coupée en [`ZONE_SUBROWS`] sous-lignes, et chaque sous-ligne couvre
/// exactement la partie de pixel qu'elle traverse.
fn zone_coverage(
    pts: &[(f64, f64)],
    x0: usize,
    y0: usize,
    cw: usize,
    ch: usize,
    cover: &mut [f32],
) {
    let n = pts.len();
    let share = 1.0 / ZONE_SUBROWS as f32;
    let mut crossings = Vec::new();
    for j in 0..ch {
        for s in 0..ZONE_SUBROWS {
            let y = (y0 + j) as f64 + (s as f64 + 0.5) / ZONE_SUBROWS as f64;
            crossings.clear();
            for k in 0..n {
                let (a, b) = (pts[k], pts[(k + 1) % n]);
                if (a.1 <= y) != (b.1 <= y) {
                    crossings.push(a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            crossings.sort_by(f64::total_cmp);
            for &[left, right] in crossings.as_chunks::<2>().0 {
                let left = (left - x0 as f64).clamp(0.0, cw as f64);
                let right = (right - x0 as f64).clamp(0.0, cw as f64);
                if right <= left {
                    continue;
                }
                let (first, last) = (left.floor() as usize, (right.ceil() as usize).min(cw));
                for i in first..last {
                    let inside = (right.min(i as f64 + 1.0) - left.max(i as f64)).max(0.0);
                    cover[j * cw + i] += inside as f32 * share;
                }
            }
        }
    }
}

/// Nom du dessin retouché dans le projet : `retouches/<id>-<clé>.png`. La
/// clé dépend du fichier d'origine (chemin et taille) et des retouches.
pub fn retouched_name(drawing: &Drawing, original: &Path) -> String {
    let length = fs::metadata(original).map_or(0, |m| m.len());
    let strokes = serde_json::to_string(&drawing.retouch).expect("sérialisable");
    let identity = format!(
        "{}|{length}|{strokes}|t{RETOUCH_VERSION}",
        drawing.source.as_str()
    );
    format!(
        "{RETOUCH_DIR}/{}-{:016x}.png",
        safe_id(&drawing.id),
        tiles::fnv1a_64(identity.as_bytes())
    )
}

fn safe_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Le fichier à lire pour ce dessin : l'original s'il n'a pas de retouche,
/// sinon le dessin retouché, fabriqué s'il n'existe pas encore.
pub fn retouched_file(dir: &ProjectDir, drawing: &Drawing) -> Result<PathBuf, LibraryError> {
    let original = dir.resolve(&drawing.source);
    if drawing.retouch.is_empty() {
        return Ok(original);
    }
    let name = retouched_name(drawing, &original);
    let target = dir.root().join(&name);
    if target.is_file() {
        return Ok(target);
    }
    // Un SVG est rendu comme pour la vectorisation (4 096 px) ; une image
    // est lue telle quelle.
    let mut raster = flatten_on_black(decode(&original, SVG_VECTOR_SIDE)?.raster);
    paint(&mut raster, &drawing.retouch);
    let bytes = encode_png(&raster).map_err(|reason| LibraryError::Cache {
        path: target.clone(),
        reason,
    })?;
    write_atomic(&target, &bytes)?;
    Ok(target)
}

/// Le fichier à lire pour le dessin `id` du projet (voir [`retouched_file`]).
pub fn drawing_file(
    dir: &ProjectDir,
    project: &Project,
    id: &str,
) -> Result<PathBuf, LibraryError> {
    let drawing = project
        .drawing(id)
        .ok_or_else(|| LibraryError::UnknownDrawing(id.to_string()))?;
    retouched_file(dir, drawing)
}

/// Le fichier d'un dessin pour une lecture qui ne doit pas échouer
/// (vignettes, silhouettes) : le dessin retouché, sinon l'original.
pub fn readable_file(dir: &ProjectDir, drawing: &Drawing) -> PathBuf {
    retouched_file(dir, drawing).unwrap_or_else(|_| dir.resolve(&drawing.source))
}

fn write_atomic(target: &Path, bytes: &[u8]) -> Result<(), LibraryError> {
    let failed = |reason: String| LibraryError::Cache {
        path: target.to_path_buf(),
        reason,
    };
    let folder = target.parent().expect("retouches/<fichier>");
    fs::create_dir_all(folder).map_err(|e| failed(e.to_string()))?;
    let temp = folder.join(format!(".~{}.{}", new_id(), std::process::id()));
    let written = fs::File::create(&temp)
        .and_then(|mut file| file.write_all(bytes))
        .and_then(|()| fs::rename(&temp, target));
    if let Err(error) = written {
        let _ = fs::remove_file(&temp);
        return Err(failed(error.to_string()));
    }
    Ok(())
}

/// Remplace les retouches d'un dessin (liste vide : revenir au dessin
/// d'origine), enregistre le projet, fabrique le dessin retouché et rend
/// la nouvelle entrée de bibliothèque (vignette refaite). Des retouches
/// refusées ne modifient pas le projet. Les anciens fichiers retouchés de
/// ce dessin sont effacés.
pub fn set_retouch(
    dir: &ProjectDir,
    id: &str,
    strokes: Vec<RetouchStroke>,
    cache_root: Option<&Path>,
) -> Result<(Project, LibraryEntry), LibraryError> {
    if let Some(problem) = crate::project::retouch_problem(&strokes) {
        return Err(LibraryError::Retouch(problem));
    }
    let mut project = dir.load()?.project;
    let drawing = project
        .library
        .iter_mut()
        .find(|d| d.id == id)
        .ok_or_else(|| LibraryError::UnknownDrawing(id.to_string()))?;
    if DrawingFormat::of(&dir.resolve(&drawing.source)).is_none() {
        return Err(LibraryError::Retouch("format de dessin inconnu".into()));
    }
    let changed = drawing.retouch != strokes;
    drawing.retouch = strokes;
    let drawing = drawing.clone();
    // Fabriqué avant d'enregistrer : un dessin illisible ne laisse pas de
    // retouche impossible à appliquer dans le projet.
    let current = retouched_file(dir, &drawing)?;
    if changed {
        dir.save(&project)?;
    }
    remove_stale(dir, &drawing, &current);
    let only = Project {
        library: vec![drawing],
        ..project.clone()
    };
    let entry = entries(dir, &only, cache_root, 1)
        .pop()
        .expect("un dessin demandé");
    Ok((project, entry))
}

/// Efface les dessins retouchés de `drawing` autres que `current`.
fn remove_stale(dir: &ProjectDir, drawing: &Drawing, current: &Path) {
    let prefix = format!("{}-", safe_id(&drawing.id));
    let Ok(listing) = fs::read_dir(dir.root().join(RETOUCH_DIR)) else {
        return;
    };
    for entry in listing.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&prefix) && name.ends_with(".png") && path != current {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests;

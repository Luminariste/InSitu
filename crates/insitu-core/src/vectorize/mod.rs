//! Vectorisation d'un dessin (tâche 5.1) : du trait blanc sur fond noir aux
//! courbes.
//!
//! Le dessin est lu à pleine résolution, en niveaux de gris (une image en
//! couleur est ramenée à sa luminance ; la transparence compte comme du noir).
//! Il est vectorisé en deux temps :
//!
//! 1. **contours bruts** ([`contour`]) : la ligne de niveau du seuil, suivie
//!    avec une précision inférieure au pixel grâce aux gris du bord du trait ;
//! 2. **lissage** ([`fit`]) : chaque contour devient une suite de droites et
//!    de cubiques de Bézier, à moins de la tolérance choisie des contours
//!    bruts, avec des angles vifs là où le dessin en a.
//!
//! [`compare`] mesure ensuite, indépendamment de l'ajustement, l'écart entre
//! le résultat et l'image source : c'est le critère d'acceptation de la
//! tâche (moins d'un demi-pixel).
//!
//! Les coordonnées sont en pixels de l'image source, origine au coin
//! haut-gauche de l'image, y vers le bas (le centre du premier pixel est en
//! (0,5 ; 0,5), comme D-2.4-c). Le passage en millimètres se fera au
//! transport vers la diapositive (tâche 5.3).

mod check;
mod contour;
mod fit;

use std::fmt::{self, Write as _};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use serde::Serialize;

use crate::geometry::Point;
use crate::tiles::Raster;

pub use check::{compare, Comparison};
pub use fit::{Segment, CORNER_SCALE_PX};

/// Réglages de la vectorisation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorizeOptions {
    /// Seuil, de 1 à 255 : un pixel au moins aussi clair est dans le trait.
    pub threshold: u8,
    /// Écart maximal toléré entre les courbes et le contour brut, en pixels.
    /// Plus grand, le tracé est plus lisse et a moins de points, mais
    /// s'éloigne de l'image.
    pub tolerance_px: f64,
    /// Un sommet où la direction tourne de plus de cet angle (en degrés) est
    /// un angle vif ; ailleurs la courbe reste lisse.
    pub corner_angle_deg: f64,
    /// Les taches (ou les trous) plus petites que cette aire, en pixels
    /// carrés, sont ignorées. 0 : tout est gardé.
    pub min_area_px: f64,
    /// Nombre de fils de calcul ; 0 : tous les cœurs.
    pub threads: usize,
}

impl Default for VectorizeOptions {
    fn default() -> Self {
        Self {
            threshold: 128,
            tolerance_px: 0.3,
            corner_angle_deg: 70.0,
            min_area_px: 0.0,
            threads: 0,
        }
    }
}

/// Ce qui peut empêcher la vectorisation.
#[derive(Debug, Clone, PartialEq)]
pub enum VectorizeError {
    Threshold,
    Tolerance(f64),
    CornerAngle(f64),
    MinArea(f64),
}

impl fmt::Display for VectorizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VectorizeError::Threshold => write!(f, "le seuil doit aller de 1 à 255"),
            VectorizeError::Tolerance(t) => write!(
                f,
                "la tolérance doit être comprise entre 0,01 et 10 pixels (reçu {t})"
            ),
            VectorizeError::CornerAngle(a) => write!(
                f,
                "l'angle des coins doit être compris entre 1 et 179 degrés (reçu {a})"
            ),
            VectorizeError::MinArea(a) => {
                write!(f, "l'aire minimale doit être positive ou nulle (reçu {a})")
            }
        }
    }
}

impl std::error::Error for VectorizeError {}

/// Un contour fermé : départ, puis morceaux jusqu'au retour au départ.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Contour {
    pub start: Point,
    pub segments: Vec<Segment>,
    /// Aire en pixels carrés : positive pour le bord extérieur d'un trait,
    /// négative pour un trou.
    pub area: f64,
}

/// Le dessin vectorisé.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Vectorization {
    pub width: u32,
    pub height: u32,
    pub options: VectorizeOptions,
    pub contours: Vec<Contour>,
    pub stats: VectorizeStats,
}

/// Chiffres d'une vectorisation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorizeStats {
    pub contours: usize,
    /// Contours écartés par l'aire minimale.
    pub removed_contours: usize,
    /// Points des contours bruts gardés.
    pub raw_points: usize,
    pub lines: usize,
    pub cubics: usize,
    /// Écart maximal mesuré pendant l'ajustement, en pixels (voir
    /// [`compare`] pour la mesure indépendante).
    pub max_fit_error_px: f64,
}

/// Une image en niveaux de gris, prête à vectoriser.
#[derive(Debug, Clone, PartialEq)]
pub struct GrayImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl GrayImage {
    /// Luminance de l'image (Rec. 709, comme le module `image`) ; la
    /// transparence est posée sur du noir.
    pub fn from_raster(raster: &Raster) -> Self {
        let channels = usize::from(raster.channels);
        let data = match channels {
            1 => raster.data.clone(),
            _ => raster
                .data
                .chunks_exact(channels)
                .map(|px| {
                    let luma = 0.2126 * f32::from(px[0])
                        + 0.7152 * f32::from(px[1])
                        + 0.0722 * f32::from(px[2]);
                    let alpha = if channels == 4 {
                        f32::from(px[3]) / 255.0
                    } else {
                        1.0
                    };
                    (luma * alpha).round().clamp(0.0, 255.0) as u8
                })
                .collect(),
        };
        Self {
            width: raster.width,
            height: raster.height,
            data,
        }
    }

    fn field(&self, threshold: u8) -> contour::Field<'_> {
        contour::Field {
            width: self.width as usize,
            height: self.height as usize,
            data: &self.data,
            threshold,
        }
    }
}

fn validate(options: &VectorizeOptions) -> Result<(), VectorizeError> {
    if options.threshold == 0 {
        return Err(VectorizeError::Threshold);
    }
    if !(0.01..=10.0).contains(&options.tolerance_px) {
        return Err(VectorizeError::Tolerance(options.tolerance_px));
    }
    if !(1.0..=179.0).contains(&options.corner_angle_deg) {
        return Err(VectorizeError::CornerAngle(options.corner_angle_deg));
    }
    if !(options.min_area_px >= 0.0 && options.min_area_px.is_finite()) {
        return Err(VectorizeError::MinArea(options.min_area_px));
    }
    Ok(())
}

/// Vectorise un dessin.
pub fn vectorize(
    image: &GrayImage,
    options: &VectorizeOptions,
) -> Result<Vectorization, VectorizeError> {
    validate(options)?;
    let raw = contour::trace(&image.field(options.threshold));
    let total = raw.len();
    let raw: Vec<(Vec<Point>, f64)> = raw
        .into_iter()
        .map(|points| {
            let area = contour::signed_area(&points);
            (points, area)
        })
        .filter(|(_, area)| area.abs() >= options.min_area_px)
        .collect();
    let removed_contours = total - raw.len();
    let raw_points = raw.iter().map(|(p, _)| p.len()).sum();

    let params = fit::FitParams {
        tolerance: options.tolerance_px,
        corner_cos: options.corner_angle_deg.to_radians().cos(),
    };
    let fitted = parallel_map(&raw, options.threads, |(points, area)| {
        let (start, segments, error) = fit::fit_closed(points, params);
        (
            Contour {
                start,
                segments,
                area: *area,
            },
            error,
        )
    });
    let mut stats = VectorizeStats {
        contours: fitted.len(),
        removed_contours,
        raw_points,
        lines: 0,
        cubics: 0,
        max_fit_error_px: 0.0,
    };
    let mut contours = Vec::with_capacity(fitted.len());
    for (contour, error) in fitted {
        for segment in &contour.segments {
            match segment {
                Segment::Line { .. } => stats.lines += 1,
                Segment::Cubic { .. } => stats.cubics += 1,
            }
        }
        stats.max_fit_error_px = stats.max_fit_error_px.max(error);
        contours.push(contour);
    }
    Ok(Vectorization {
        width: image.width,
        height: image.height,
        options: *options,
        contours,
        stats,
    })
}

/// Applique `f` à chaque élément, sur plusieurs fils, dans l'ordre.
fn parallel_map<T: Sync, R: Send>(
    items: &[T],
    threads: usize,
    f: impl Fn(&T) -> R + Sync,
) -> Vec<R> {
    let threads = if threads == 0 {
        std::thread::available_parallelism().map_or(1, |n| n.get())
    } else {
        threads
    };
    if threads <= 1 || items.len() < 64 {
        return items.iter().map(f).collect();
    }
    const CHUNK: usize = 256;
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(usize, Vec<R>)>> = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let start = next.fetch_add(CHUNK, Ordering::Relaxed);
                if start >= items.len() {
                    break;
                }
                let end = (start + CHUNK).min(items.len());
                let chunk: Vec<R> = items[start..end].iter().map(&f).collect();
                results.lock().expect("verrou").push((start, chunk));
            });
        }
    });
    let mut chunks = results.into_inner().expect("verrou");
    chunks.sort_by_key(|(start, _)| *start);
    chunks.into_iter().flat_map(|(_, chunk)| chunk).collect()
}

impl Vectorization {
    /// SVG du dessin, en pixels de l'image source : traits blancs remplis
    /// (règle « nonzero ») sur fond noir, pour comparer à l'œil avec l'image.
    pub fn to_svg(&self) -> String {
        let (w, h) = (self.width, self.height);
        let mut svg = String::new();
        let _ = writeln!(
            svg,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#
        );
        let _ = writeln!(svg, r#"<rect width="{w}" height="{h}" fill="black"/>"#);
        svg.push_str(r#"<path fill="white" fill-rule="nonzero" d=""#);
        for contour in &self.contours {
            let _ = write!(svg, "M{} {}", num(contour.start.x), num(contour.start.y));
            for segment in &contour.segments {
                match *segment {
                    Segment::Line { to } => {
                        let _ = write!(svg, "L{} {}", num(to.x), num(to.y));
                    }
                    Segment::Cubic { c1, c2, to } => {
                        let _ = write!(
                            svg,
                            "C{} {} {} {} {} {}",
                            num(c1.x),
                            num(c1.y),
                            num(c2.x),
                            num(c2.y),
                            num(to.x),
                            num(to.y)
                        );
                    }
                }
            }
            svg.push('Z');
        }
        svg.push_str("\"/>\n</svg>\n");
        svg
    }
}

/// Nombre écrit au millième de pixel, sans zéros inutiles.
fn num(v: f64) -> String {
    let text = format!("{v:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".to_string()
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests;

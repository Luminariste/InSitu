//! Ajustement automatique de la photo redressée sur la façade (demande de
//! Benjamin du 13 novembre 2026, décision D-3.2-i).
//!
//! On pose la photo à peu près à la main (« Poser la photo sur la façade »,
//! coins, poignée) ; « Ajuster » la cale ensuite sur les lignes du plan,
//! autour de cette position. Le résultat est un trapèze donné par quatre
//! paires aux coins de la diapositive : le format du projet ne change pas,
//! et on retouche à la main comme avant (coins, épingles).
//!
//! La photo (prise de nuit, à travers la mire) et le plan (des traits ou des
//! aplats) ne se ressemblent pas pixel à pixel. Ce qu'ils partagent, ce sont
//! les grandes lignes : bord de l'avant-toit, bandeau, tableaux des portes,
//! arcs. On compare donc l'**orientation des contours**, sans tenir compte du
//! sens du contraste (un bord clair sur sombre vaut un bord sombre sur
//! clair) :
//!
//! 1. la grille de la mire, faite de traits fins et sombres, est effacée par
//!    une fermeture (maximum puis minimum sur un petit voisinage) ;
//! 2. **recherche locale** : autour de la pose à la main, la photo réduite,
//!    tournée (±6°) et mise à quelques échelles voisines, est corrélée avec
//!    le plan par transformée de Fourier ; les meilleurs pics proches de la
//!    pose donnent d'autres points de départ ;
//! 3. **ajustement** : depuis la pose à la main et depuis ces points de
//!    départ, le trapèze (les quatre coins sur la façade) est poussé coin par
//!    coin tant que les contours s'accordent mieux ; le meilleur gagne, et
//!    seulement s'il fait mieux que la pose à la main.
//!
//! Limites, mesurées dans `docs/etudes/recalage-automatique.md` : la pose à
//! la main doit être à moins d'une demi-travée environ de la bonne place
//! (sinon une travée voisine, presque identique, peut l'emporter). Une
//! façade non plane n'est pas suivie par un trapèze : le relief se corrige
//! ensuite par des épingles.

use std::f64::consts::PI;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use image::GrayImage;
use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use serde::{Deserialize, Serialize};

use super::{footprint, projector_id, slide_to_facade, RegistrationError};
use crate::facade::display::rotate_plan;
use crate::facade::{open_vector, SourceKind};
use crate::geometry::{Homography, Point, Transform};
use crate::project::{PointMm, ProjectDir, RegistrationPair};
use crate::units::SLIDE_SIZE_MM;

/// Côté de la photo pendant la recherche, en pixels.
const SEARCH_PHOTO_PX: usize = 96;
/// Côté de la photo pendant l'ajustement, en pixels.
const REFINE_PHOTO_PX: usize = 128;
/// Côté de la photo réduite avant d'effacer la grille de la mire, en pixels
/// (un trait de la mire y fait un ou deux pixels).
const CLEAN_PHOTO_PX: usize = 560;
/// Demi-côté du voisinage qui efface la grille de la mire, en pixels de la
/// photo réduite à [`CLEAN_PHOTO_PX`] (soit 1,5 mm de diapositive).
const CLEAN_RADIUS_PX: usize = 3;
/// Plus grand côté du plan pendant l'ajustement, en pixels.
const REFINE_PLAN_PX: u32 = 1024;
/// Plus grand côté du plan pendant la recherche, en pixels.
const SEARCH_PLAN_PX: usize = 448;
/// Rotations essayées, en degrés : la photo redressée est déjà à peu près
/// droite (la mire est posée d'aplomb sur le projecteur).
const ANGLES_DEG: [f64; 5] = [-6.0, -3.0, 0.0, 3.0, 6.0];
/// Échelles essayées autour de la pose à la main (fraction de son côté).
const ADJUST_SCALES: [f64; 5] = [0.8, 0.89, 1.0, 1.12, 1.25];
/// Pics retenus par échelle et rotation.
const PEAKS_PER_MAP: usize = 8;
/// Points de départ tirés de la recherche locale, en plus de la pose.
const LOCAL_STARTS: usize = 3;
/// Marge autour de la pose où l'on regarde le plan, en fraction de son côté.
const ADJUST_MARGIN: f64 = 0.5;
/// Un point de départ de la recherche locale doit avoir son centre à moins
/// de cette fraction du côté du centre de la pose à la main.
const ADJUST_REACH: f64 = 0.25;
/// Deux points de départ dont les centres sont plus proches que cette
/// fraction du côté sont le même.
const SAME_PLACE: f64 = 0.3;
/// Lissage avant de mesurer les contours, en pixels.
const EDGE_SIGMA: f64 = 1.0;

/// Un rectangle de la façade, en mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionMm {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Résultat de l'ajustement d'une pose.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Adjustment {
    /// Les quatre paires aux coins de la diapositive (haut-gauche,
    /// haut-droit, bas-droit, bas-gauche) : ce qu'on enregistre. Ce sont
    /// celles de la pose de départ si rien de mieux n'a été trouvé.
    pub pairs: Vec<RegistrationPair>,
    /// Contour de la diapositive posée ainsi (emprise), fermé.
    pub footprint: Vec<PointMm>,
    /// Accord des contours de la photo et du plan, de 0 (aucun) à 1
    /// (parfait), après et avant l'ajustement. 0,3 à 0,4 est déjà bon entre
    /// une photo de nuit et un plan schématique.
    pub score: f64,
    pub start_score: f64,
    /// Plus grand déplacement d'un coin, en mm (0 : pose gardée telle quelle).
    pub moved_mm: f64,
}

/// Compte rendu de l'ajustement d'un projecteur.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjustReport {
    pub projector_id: String,
    #[serde(flatten)]
    pub adjustment: Adjustment,
    /// Zone de la façade regardée, en mm.
    pub region: RegionMm,
    pub seconds: f64,
}

// --- Images en nombres flottants ------------------------------------------

/// Image en niveaux de gris, en nombres flottants (0 à 255).
#[derive(Debug, Clone)]
pub struct Gray {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
}

impl Gray {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0.0; width * height],
        }
    }

    pub fn from_image(image: &GrayImage) -> Self {
        Self {
            width: image.width() as usize,
            height: image.height() as usize,
            data: image.as_raw().iter().map(|&v| f32::from(v)).collect(),
        }
    }

    fn at(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x]
    }

    /// Valeur interpolée au point (x, y) en pixels (centre du premier pixel
    /// en 0,5) ; bords prolongés.
    fn sample(&self, x: f64, y: f64) -> f32 {
        let fx = (x - 0.5).clamp(0.0, (self.width - 1) as f64);
        let fy = (y - 0.5).clamp(0.0, (self.height - 1) as f64);
        let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (tx, ty) = ((fx - x0 as f64) as f32, (fy - y0 as f64) as f32);
        let top = self.at(x0, y0) * (1.0 - tx) + self.at(x1, y0) * tx;
        let bottom = self.at(x0, y1) * (1.0 - tx) + self.at(x1, y1) * tx;
        top * (1.0 - ty) + bottom * ty
    }

    /// Réduction par moyenne des pixels couverts (ou agrandissement
    /// interpolé si l'image est plus petite).
    pub fn resized(&self, width: usize, height: usize) -> Gray {
        let (width, height) = (width.max(1), height.max(1));
        let mut out = Gray::new(width, height);
        let kx = self.width as f64 / width as f64;
        let ky = self.height as f64 / height as f64;
        if kx <= 1.0 && ky <= 1.0 {
            for y in 0..height {
                for x in 0..width {
                    out.data[y * width + x] =
                        self.sample((x as f64 + 0.5) * kx, (y as f64 + 0.5) * ky);
                }
            }
            return out;
        }
        for y in 0..height {
            let (y0, y1) = (y as f64 * ky, (y + 1) as f64 * ky);
            for x in 0..width {
                let (x0, x1) = (x as f64 * kx, (x + 1) as f64 * kx);
                let (mut sum, mut weight) = (0.0f64, 0.0f64);
                let mut j = y0.floor() as usize;
                while (j as f64) < y1 && j < self.height {
                    let wy = (y1.min((j + 1) as f64) - y0.max(j as f64)).max(0.0);
                    let mut i = x0.floor() as usize;
                    while (i as f64) < x1 && i < self.width {
                        let wx = (x1.min((i + 1) as f64) - x0.max(i as f64)).max(0.0);
                        sum += f64::from(self.at(i, j)) * wx * wy;
                        weight += wx * wy;
                        i += 1;
                    }
                    j += 1;
                }
                out.data[y * width + x] = if weight > 0.0 {
                    (sum / weight) as f32
                } else {
                    0.0
                };
            }
        }
        out
    }

    /// Maximum ou minimum sur un carré de demi-côté `radius`.
    fn extremum(&self, radius: usize, max: bool) -> Gray {
        let pick = |a: f32, b: f32| if max { a.max(b) } else { a.min(b) };
        let mut rows = Gray::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let (a, b) = (x.saturating_sub(radius), (x + radius).min(self.width - 1));
                let mut v = self.at(a, y);
                for i in a + 1..=b {
                    v = pick(v, self.at(i, y));
                }
                rows.data[y * self.width + x] = v;
            }
        }
        let mut out = Gray::new(self.width, self.height);
        for y in 0..self.height {
            let (a, b) = (y.saturating_sub(radius), (y + radius).min(self.height - 1));
            for x in 0..self.width {
                let mut v = rows.at(x, a);
                for j in a + 1..=b {
                    v = pick(v, rows.at(x, j));
                }
                out.data[y * self.width + x] = v;
            }
        }
        out
    }

    /// Fermeture : efface les traits sombres plus fins que le voisinage
    /// (la grille de la mire).
    pub fn closed(&self, radius: usize) -> Gray {
        self.extremum(radius, true).extremum(radius, false)
    }

    /// Flou gaussien d'écart type `sigma` pixels.
    fn blurred(&self, sigma: f64) -> Gray {
        let reach = (3.0 * sigma).ceil() as isize;
        let kernel: Vec<f32> = (-reach..=reach)
            .map(|i| (-(i * i) as f64 / (2.0 * sigma * sigma)).exp() as f32)
            .collect();
        let total: f32 = kernel.iter().sum();
        let kernel: Vec<f32> = kernel.iter().map(|k| k / total).collect();
        let (w, h) = (self.width as isize, self.height as isize);
        let mut rows = Gray::new(self.width, self.height);
        for y in 0..h {
            for x in 0..w {
                let mut v = 0.0;
                for (k, weight) in kernel.iter().enumerate() {
                    let i = (x + k as isize - reach).clamp(0, w - 1);
                    v += weight * self.data[(y * w + i) as usize];
                }
                rows.data[(y * w + x) as usize] = v;
            }
        }
        let mut out = Gray::new(self.width, self.height);
        for y in 0..h {
            for x in 0..w {
                let mut v = 0.0;
                for (k, weight) in kernel.iter().enumerate() {
                    let j = (y + k as isize - reach).clamp(0, h - 1);
                    v += weight * rows.data[(j * w + x) as usize];
                }
                out.data[(y * w + x) as usize] = v;
            }
        }
        out
    }
}

/// Champ des contours : en chaque pixel, l'orientation du contour doublée
/// (un bord et son inverse donnent le même nombre) et sa force, comprimée.
#[derive(Debug, Clone)]
struct Field {
    width: usize,
    height: usize,
    data: Vec<Complex32>,
}

impl Field {
    fn of(image: &Gray, sigma: f64) -> Field {
        let g = image.blurred(sigma);
        let (w, h) = (g.width, g.height);
        let at = |x: isize, y: isize| {
            g.data[(y.clamp(0, h as isize - 1) as usize) * w + x.clamp(0, w as isize - 1) as usize]
        };
        let mut raw = vec![Complex32::new(0.0, 0.0); w * h];
        let mut magnitudes = Vec::with_capacity(w * h);
        for y in 0..h as isize {
            for x in 0..w as isize {
                // Sobel.
                let gx = (at(x + 1, y - 1) + 2.0 * at(x + 1, y) + at(x + 1, y + 1))
                    - (at(x - 1, y - 1) + 2.0 * at(x - 1, y) + at(x - 1, y + 1));
                let gy = (at(x - 1, y + 1) + 2.0 * at(x, y + 1) + at(x + 1, y + 1))
                    - (at(x - 1, y - 1) + 2.0 * at(x, y - 1) + at(x + 1, y - 1));
                let z = Complex32::new(gx, gy);
                raw[y as usize * w + x as usize] = z;
                magnitudes.push(z.norm());
            }
        }
        // Force normalisée par le 99e centile, puis racine : quelques bords
        // très contrastés ne doivent pas écraser les autres.
        let mut sorted = magnitudes.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let p99 = sorted[(sorted.len() * 99 / 100).min(sorted.len() - 1)].max(1e-6);
        let data = raw
            .iter()
            .zip(&magnitudes)
            .map(|(&z, &m)| {
                if m < 1e-6 {
                    Complex32::new(0.0, 0.0)
                } else {
                    let unit = z / m;
                    unit * unit * (m / p99).min(1.0).sqrt()
                }
            })
            .collect();
        Field {
            width: w,
            height: h,
            data,
        }
    }

    fn energy(&self) -> f32 {
        self.data.iter().map(|z| z.norm_sqr()).sum()
    }

    /// Accord de deux champs de même taille : cosinus, de -1 à 1.
    fn agreement(&self, other: &Field) -> f64 {
        let dot: f32 = self
            .data
            .iter()
            .zip(&other.data)
            .map(|(a, b)| (a * b.conj()).re)
            .sum();
        let norm = (self.energy() * other.energy()).sqrt();
        if norm > 0.0 {
            f64::from(dot / norm)
        } else {
            0.0
        }
    }
}

// --- Le plan en image -----------------------------------------------------

/// Le plan de la façade rendu en image sur une zone, avec son repère.
#[derive(Debug, Clone)]
pub struct PlanRaster {
    pub image: Gray,
    /// Point de la façade (mm) au coin haut-gauche de l'image.
    pub origin: PointMm,
    pub mm_per_px: f64,
}

impl PlanRaster {
    fn to_mm(&self, x: f64, y: f64) -> Point {
        Point::new(
            self.origin.x + x * self.mm_per_px,
            self.origin.y + y * self.mm_per_px,
        )
    }

    fn to_px(&self, p: Point) -> (f64, f64) {
        (
            (p.x - self.origin.x) / self.mm_per_px,
            (p.y - self.origin.y) / self.mm_per_px,
        )
    }

    fn region(&self) -> RegionMm {
        RegionMm {
            x: self.origin.x,
            y: self.origin.y,
            width: self.image.width as f64 * self.mm_per_px,
            height: self.image.height as f64 * self.mm_per_px,
        }
    }

    fn reduced(&self, max_side: usize) -> PlanRaster {
        let long = self.image.width.max(self.image.height);
        if long <= max_side {
            return self.clone();
        }
        let k = max_side as f64 / long as f64;
        let w = ((self.image.width as f64 * k).round() as usize).max(1);
        let h = ((self.image.height as f64 * k).round() as usize).max(1);
        PlanRaster {
            image: self.image.resized(w, h),
            origin: self.origin,
            mm_per_px: self.mm_per_px * self.image.width as f64 / w as f64,
        }
    }
}

fn luminance(rgba: &[u8]) -> f32 {
    let [r, g, b, a] = [rgba[0], rgba[1], rgba[2], rgba[3]].map(f32::from);
    // Sur fond blanc, comme l'affiche la visionneuse.
    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
    (lum * a + 255.0 * (255.0 - a)) / 255.0
}

/// Rend le plan de la façade (fichier `path`, tourné de `rotation` quarts de
/// tour, à `mm_per_unit` mm par unité du fichier) sur la zone `region` (mm),
/// en une image dont le plus grand côté fait au plus `max_side` pixels. La
/// zone est ramenée à la façade.
pub fn render_plan(
    path: &Path,
    page: Option<u32>,
    rotation: u8,
    mm_per_unit: f64,
    region: Option<RegionMm>,
    max_side: u32,
) -> Result<PlanRaster, RegistrationError> {
    let unreadable = |e: String| RegistrationError::BadOptions(format!("plan illisible : {e}"));
    let kind = SourceKind::of(path).map_err(|e| unreadable(e.to_string()))?;
    enum Source {
        Vector(Arc<dyn crate::facade::VectorPlan>),
        Image(Gray),
    }
    let source = match kind {
        SourceKind::Image => {
            let image = crate::photo::read(path).map_err(|e| unreadable(e.to_string()))?;
            let image = match rotation % 4 {
                1 => image.rotate90(),
                2 => image.rotate180(),
                3 => image.rotate270(),
                _ => image,
            };
            Source::Image(Gray::from_image(&image.into_luma8()))
        }
        _ => Source::Vector(rotate_plan(
            open_vector(path, kind, page).map_err(|e| unreadable(e.to_string()))?,
            rotation,
        )),
    };
    let (units_w, units_h) = match &source {
        Source::Vector(plan) => plan.size(),
        Source::Image(image) => (image.width as f64, image.height as f64),
    };
    let (facade_w, facade_h) = (units_w * mm_per_unit, units_h * mm_per_unit);
    let whole = RegionMm {
        x: 0.0,
        y: 0.0,
        width: facade_w,
        height: facade_h,
    };
    let r = region.unwrap_or(whole);
    let (x0, y0) = (r.x.max(0.0), r.y.max(0.0));
    let (x1, y1) = (
        (r.x + r.width).min(facade_w),
        (r.y + r.height).min(facade_h),
    );
    if !(x1 - x0 > 0.0 && y1 - y0 > 0.0) {
        return Err(RegistrationError::BadOptions(
            "la zone de recherche est hors de la façade".into(),
        ));
    }
    let mm_per_px = (x1 - x0).max(y1 - y0) / f64::from(max_side.max(16));
    let w = ((x1 - x0) / mm_per_px).round().max(1.0) as usize;
    let h = ((y1 - y0) / mm_per_px).round().max(1.0) as usize;
    let image = match &source {
        Source::Vector(plan) => {
            let scale = mm_per_unit / mm_per_px;
            let rgba = plan.render(scale, x0 / mm_per_px, y0 / mm_per_px, w as u32, h as u32);
            Gray {
                width: w,
                height: h,
                data: rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|p| luminance(p))
                    .collect(),
            }
        }
        Source::Image(image) => {
            // Recadre (en pixels du fichier), puis réduit.
            let (u0, v0) = (x0 / mm_per_unit, y0 / mm_per_unit);
            let (u1, v1) = (x1 / mm_per_unit, y1 / mm_per_unit);
            let (a, b) = (u0.floor() as usize, v0.floor() as usize);
            let (c, d) = (
                (u1.ceil() as usize).min(image.width),
                (v1.ceil() as usize).min(image.height),
            );
            let mut crop = Gray::new(c - a, d - b);
            for y in 0..d - b {
                crop.data[y * (c - a)..(y + 1) * (c - a)].copy_from_slice(
                    &image.data[(y + b) * image.width + a..(y + b) * image.width + c],
                );
            }
            crop.resized(w, h)
        }
    };
    Ok(PlanRaster {
        image,
        origin: PointMm::new(x0, y0),
        mm_per_px,
    })
}

// --- Recherche par transformée de Fourier ---------------------------------

/// Une place candidate : la diapositive posée en similitude.
#[derive(Debug, Clone, Copy)]
struct Candidate {
    score: f64,
    /// Centre de la diapositive sur la façade (mm).
    center: Point,
    /// Côté de la diapositive sur la façade (mm).
    side: f64,
    /// Rotation (radians, sens des aiguilles d'une montre à l'écran).
    angle: f64,
}

impl Candidate {
    /// Coins de la diapositive sur la façade (mm) : haut-gauche, haut-droit,
    /// bas-droit, bas-gauche.
    fn corners(&self) -> [Point; 4] {
        let (c, s) = (self.angle.cos(), self.angle.sin());
        let half = self.side / 2.0;
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(u, v)| {
            let (x, y) = (u * half, v * half);
            Point::new(self.center.x + c * x - s * y, self.center.y + s * x + c * y)
        })
    }
}

/// Transformée de Fourier 2D, en place, ligne par ligne puis colonne par
/// colonne.
struct Fft2 {
    width: usize,
    height: usize,
    rows: Arc<dyn Fft<f32>>,
    cols: Arc<dyn Fft<f32>>,
    rows_inv: Arc<dyn Fft<f32>>,
    cols_inv: Arc<dyn Fft<f32>>,
}

impl Fft2 {
    fn new(width: usize, height: usize) -> Self {
        let mut planner = FftPlanner::new();
        Self {
            width,
            height,
            rows: planner.plan_fft_forward(width),
            cols: planner.plan_fft_forward(height),
            rows_inv: planner.plan_fft_inverse(width),
            cols_inv: planner.plan_fft_inverse(height),
        }
    }

    fn run(&self, data: &mut [Complex32], inverse: bool) {
        let (rows, cols) = if inverse {
            (&self.rows_inv, &self.cols_inv)
        } else {
            (&self.rows, &self.cols)
        };
        rows.process(data);
        let mut column = vec![Complex32::new(0.0, 0.0); self.height];
        for x in 0..self.width {
            for (y, v) in column.iter_mut().enumerate() {
                *v = data[y * self.width + x];
            }
            cols.process(&mut column);
            for (y, v) in column.iter().enumerate() {
                data[y * self.width + x] = *v;
            }
        }
    }
}

/// Taille commode pour la transformée : produit de 2, 3 et 5.
fn fft_size(n: usize) -> usize {
    let mut m = n.max(1);
    loop {
        let mut k = m;
        for p in [2, 3, 5] {
            while k.is_multiple_of(p) {
                k /= p;
            }
        }
        if k == 1 {
            return m;
        }
        m += 1;
    }
}

/// La photo (champ des contours) tournée de `angle` et mise à l'échelle `s`,
/// dans un carré de côté `t` : ce qu'on cherche sur le plan.
fn template(photo: &Field, s: f64, angle: f64, t: usize) -> Vec<Complex32> {
    let (c, sn) = (angle.cos(), angle.sin());
    let half = t as f64 / 2.0;
    let side = photo.width as f64;
    // L'orientation doublée tourne de deux fois l'angle.
    let turn = Complex32::new((2.0 * angle).cos() as f32, (2.0 * angle).sin() as f32);
    let mut out = vec![Complex32::new(0.0, 0.0); t * t];
    for y in 0..t {
        for x in 0..t {
            let (dx, dy) = (x as f64 + 0.5 - half, y as f64 + 0.5 - half);
            // Rotation inverse, puis mise à l'échelle inverse.
            let u = (c * dx + sn * dy) / s + side / 2.0;
            let v = (-sn * dx + c * dy) / s + side / 2.0;
            if u < 0.0 || v < 0.0 || u >= side || v >= side {
                continue;
            }
            let (i, j) = (u as usize, v as usize);
            out[y * t + x] = photo.data[j * photo.width + i] * turn;
        }
    }
    out
}

fn search(photo: &Field, plan: &PlanRaster, sides_mm: &[f64]) -> Vec<Candidate> {
    let field = Field::of(&plan.image, EDGE_SIGMA);
    let (w, h) = (field.width, field.height);
    let n = photo.width as f64;
    let biggest = sides_mm.iter().copied().fold(0.0, f64::max) / plan.mm_per_px;
    let t_max = (biggest * 1.12).ceil() as usize + 2;
    let (fw, fh) = (fft_size(w + t_max), fft_size(h + t_max));
    let fft = Fft2::new(fw, fh);
    let mut plan_spectrum = vec![Complex32::new(0.0, 0.0); fw * fh];
    for y in 0..h {
        plan_spectrum[y * fw..y * fw + w].copy_from_slice(&field.data[y * w..(y + 1) * w]);
    }
    fft.run(&mut plan_spectrum, false);
    // Énergie du plan cumulée, pour normaliser la corrélation sur la
    // fenêtre couverte par la photo.
    let mut integral = vec![0.0f64; (w + 1) * (h + 1)];
    for y in 0..h {
        let mut row = 0.0;
        for x in 0..w {
            row += f64::from(field.data[y * w + x].norm_sqr());
            integral[(y + 1) * (w + 1) + x + 1] = integral[y * (w + 1) + x + 1] + row;
        }
    }
    let window_energy = |x0: isize, y0: isize, x1: isize, y1: isize| {
        let cx = |v: isize| v.clamp(0, w as isize) as usize;
        let cy = |v: isize| v.clamp(0, h as isize) as usize;
        let (a, b, c, d) = (cx(x0), cy(y0), cx(x1), cy(y1));
        integral[d * (w + 1) + c] - integral[b * (w + 1) + c] - integral[d * (w + 1) + a]
            + integral[b * (w + 1) + a]
    };

    let jobs: Vec<(f64, f64)> = sides_mm
        .iter()
        .flat_map(|&side| ANGLES_DEG.iter().map(move |&a| (side, a * PI / 180.0)))
        .collect();
    let next = Mutex::new(0usize);
    let found = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, 8);
    std::thread::scope(|scope| {
        for _ in 0..threads.min(jobs.len()) {
            scope.spawn(|| {
                let mut buffer = vec![Complex32::new(0.0, 0.0); fw * fh];
                loop {
                    let k = {
                        let mut guard = next.lock().expect("verrou");
                        let k = *guard;
                        *guard += 1;
                        k
                    };
                    let Some(&(side_mm, angle)) = jobs.get(k) else {
                        break;
                    };
                    let side_px = side_mm / plan.mm_per_px;
                    let s = side_px / n;
                    let t = (side_px * (angle.cos().abs() + angle.sin().abs())).ceil() as usize + 1;
                    if t < 8 || t > t_max {
                        continue;
                    }
                    let tpl = template(photo, s, angle, t);
                    let tpl_energy: f32 = tpl.iter().map(|z| z.norm_sqr()).sum();
                    if tpl_energy <= 0.0 {
                        continue;
                    }
                    buffer
                        .iter_mut()
                        .for_each(|v| *v = Complex32::new(0.0, 0.0));
                    for y in 0..t {
                        buffer[y * fw..y * fw + t].copy_from_slice(&tpl[y * t..(y + 1) * t]);
                    }
                    fft.run(&mut buffer, false);
                    for (b, p) in buffer.iter_mut().zip(&plan_spectrum) {
                        *b = p * b.conj();
                    }
                    fft.run(&mut buffer, true);
                    let scale = 1.0 / (fw * fh) as f32;
                    // Décalage (coin haut-gauche du gabarit) de -t/2 à w - t/2.
                    let half = (t / 2) as isize;
                    let (gw, gh) = (w + 1, h + 1);
                    let mut scores = vec![f32::NEG_INFINITY; gw * gh];
                    let floor = 0.05 * f64::from(tpl_energy);
                    for gy in 0..gh {
                        let oy = gy as isize - half;
                        let iy = oy.rem_euclid(fh as isize) as usize;
                        for gx in 0..gw {
                            let ox = gx as isize - half;
                            let ix = ox.rem_euclid(fw as isize) as usize;
                            let corr = buffer[iy * fw + ix].re * scale;
                            let e =
                                window_energy(ox, oy, ox + t as isize, oy + t as isize).max(floor);
                            scores[gy * gw + gx] = (f64::from(corr)
                                / (f64::from(tpl_energy).sqrt() * e.sqrt()))
                                as f32;
                        }
                    }
                    // Pics locaux (voisinage de 7 × 7).
                    let mut peaks = Vec::new();
                    for gy in 0..gh {
                        for gx in 0..gw {
                            let v = scores[gy * gw + gx];
                            if v <= 0.0 {
                                continue;
                            }
                            let mut top = true;
                            'scan: for j in gy.saturating_sub(3)..(gy + 4).min(gh) {
                                for i in gx.saturating_sub(3)..(gx + 4).min(gw) {
                                    if scores[j * gw + i] > v {
                                        top = false;
                                        break 'scan;
                                    }
                                }
                            }
                            if top {
                                peaks.push((v, gx, gy));
                            }
                        }
                    }
                    peaks.sort_by(|a, b| b.0.total_cmp(&a.0));
                    let mut local = Vec::new();
                    for &(v, gx, gy) in peaks.iter().take(PEAKS_PER_MAP) {
                        let cx = gx as f64 - half as f64 + t as f64 / 2.0;
                        let cy = gy as f64 - half as f64 + t as f64 / 2.0;
                        local.push(Candidate {
                            score: f64::from(v),
                            center: plan.to_mm(cx, cy),
                            side: side_mm,
                            angle,
                        });
                    }
                    found.lock().expect("verrou").extend(local);
                }
            });
        }
    });
    let mut all = found.into_inner().expect("verrou");
    all.sort_by(|a, b| b.score.total_cmp(&a.score));
    all
}

/// Garde la meilleure de chaque groupe de places voisines.
fn distinct<T: Copy>(items: &[T], center: impl Fn(&T) -> (Point, f64), keep: usize) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    for item in items {
        let (c, side) = center(item);
        let near = out.iter().any(|o| {
            let (d, other_side) = center(o);
            c.distance(d) < SAME_PLACE * side.max(other_side)
        });
        if !near {
            out.push(*item);
        }
        if out.len() >= keep {
            break;
        }
    }
    out
}

// --- Ajustement du trapèze ------------------------------------------------

fn slide_corners() -> [Point; 4] {
    let s = SLIDE_SIZE_MM;
    [
        Point::new(0.0, 0.0),
        Point::new(s, 0.0),
        Point::new(s, s),
        Point::new(0.0, s),
    ]
}

fn homography(corners: &[Point; 4]) -> Option<Homography> {
    Homography::from_four_pairs(slide_corners(), *corners).ok()
}

/// Le trapèze est-il plausible : convexe, ni retourné ni écrasé ?
fn plausible(corners: &[Point; 4], reference_area: f64) -> bool {
    let mut sign = 0.0;
    for k in 0..4 {
        let (a, b, c) = (corners[k], corners[(k + 1) % 4], corners[(k + 2) % 4]);
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if cross.abs() < 1e-9 || (sign != 0.0 && cross.signum() != sign) {
            return false;
        }
        sign = cross.signum();
    }
    let area = polygon_area(corners);
    area > 0.4 * reference_area && area < 2.5 * reference_area
}

fn polygon_area(points: &[Point]) -> f64 {
    let n = points.len();
    (0..n)
        .map(|k| {
            let (a, b) = (points[k], points[(k + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

/// Accord des contours de la photo et du plan pour ce trapèze : le plan est
/// ramené dans le repère de la photo, puis les deux champs sont comparés.
fn refine_score(photo: &Field, plan: &PlanRaster, corners: &[Point; 4]) -> f64 {
    let Some(h) = homography(corners) else {
        return -1.0;
    };
    let n = photo.width;
    let mut warped = Gray::new(n, n);
    let mm = SLIDE_SIZE_MM / n as f64;
    for y in 0..n {
        for x in 0..n {
            let slide = Point::new((x as f64 + 0.5) * mm, (y as f64 + 0.5) * mm);
            let Some(p) = h.apply(slide) else { return -1.0 };
            let (px, py) = plan.to_px(p);
            warped.data[y * n + x] = plan.image.sample(px, py);
        }
    }
    photo.agreement(&Field::of(&warped, EDGE_SIGMA))
}

/// Pousse les coins tant que l'accord augmente (recherche par motifs : un
/// coin à la fois, puis toute la photo, le pas divisé par deux quand plus
/// rien n'améliore).
fn refine(photo: &Field, plan: &PlanRaster, start: [Point; 4]) -> ([Point; 4], f64) {
    let reference = polygon_area(&start);
    let side = reference.sqrt();
    let reach = 0.3 * side;
    let mut best = start;
    let mut value = refine_score(photo, plan, &best);
    let mut step = 0.04 * side;
    let min_step = (0.002 * side).max(plan.mm_per_px * 0.25);
    let within = |c: &[Point; 4]| (0..4).all(|k| c[k].distance(start[k]) <= reach);
    let mut evaluations = 0;
    while step >= min_step && evaluations < 1500 {
        let mut improved = false;
        let mut moves: Vec<[Point; 4]> = Vec::new();
        for k in 0..4 {
            for (dx, dy) in [(step, 0.0), (-step, 0.0), (0.0, step), (0.0, -step)] {
                let mut c = best;
                c[k] = Point::new(c[k].x + dx, c[k].y + dy);
                moves.push(c);
            }
        }
        for (dx, dy) in [(step, 0.0), (-step, 0.0), (0.0, step), (0.0, -step)] {
            moves.push(best.map(|p| Point::new(p.x + dx, p.y + dy)));
        }
        for c in moves {
            if !within(&c) || !plausible(&c, reference) {
                continue;
            }
            evaluations += 1;
            let v = refine_score(photo, plan, &c);
            if v > value {
                best = c;
                value = v;
                improved = true;
            }
        }
        if !improved {
            step /= 2.0;
        }
    }
    (best, value)
}

// --- Assemblage -----------------------------------------------------------

/// Prépare la photo redressée : réduite, grille de la mire effacée.
fn prepared_photo(photo: &GrayImage) -> Gray {
    Gray::from_image(photo)
        .resized(CLEAN_PHOTO_PX, CLEAN_PHOTO_PX)
        .closed(CLEAN_RADIUS_PX)
}

fn corner_pairs(corners: &[Point; 4]) -> Vec<RegistrationPair> {
    slide_corners()
        .iter()
        .zip(corners)
        .map(|(s, f)| RegistrationPair::new(PointMm::new(s.x, s.y), PointMm::new(f.x, f.y)))
        .collect()
}

fn centroid(c: &[Point; 4]) -> Point {
    Point::new(
        (c[0].x + c[1].x + c[2].x + c[3].x) / 4.0,
        (c[0].y + c[1].y + c[2].y + c[3].y) / 4.0,
    )
}

/// Zone du plan à regarder pour ajuster la pose `start` : son cadre, élargi
/// d'une demi-emprise de chaque côté.
pub fn adjust_region(start: &[Point; 4]) -> RegionMm {
    let side = polygon_area(start).sqrt();
    let margin = ADJUST_MARGIN * side;
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in start {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    RegionMm {
        x: x0 - margin,
        y: y0 - margin,
        width: x1 - x0 + 2.0 * margin,
        height: y1 - y0 + 2.0 * margin,
    }
}

/// Cale la photo redressée `photo` sur le plan `plan` à partir de la pose
/// à la main `start` (coins de la diapositive sur la façade, en mm :
/// haut-gauche, haut-droit, bas-droit, bas-gauche). Le plan doit couvrir la
/// pose et ses environs ([`adjust_region`]).
pub fn adjust(photo: &GrayImage, plan: &PlanRaster, start: [Point; 4]) -> Adjustment {
    let cleaned = prepared_photo(photo);
    let refine_photo = Field::of(
        &cleaned.resized(REFINE_PHOTO_PX, REFINE_PHOTO_PX),
        EDGE_SIGMA,
    );
    let side = polygon_area(&start).sqrt();
    let center = centroid(&start);
    let search_photo = Field::of(
        &cleaned.resized(SEARCH_PHOTO_PX, SEARCH_PHOTO_PX),
        EDGE_SIGMA,
    );
    let candidates = search(
        &search_photo,
        &plan.reduced(SEARCH_PLAN_PX),
        &ADJUST_SCALES.map(|k| k * side),
    );
    let near: Vec<Candidate> = candidates
        .into_iter()
        .filter(|c| c.center.distance(center) <= ADJUST_REACH * side)
        .collect();
    let mut starts = vec![start];
    starts.extend(
        distinct(&near, |c| (c.center, c.side), LOCAL_STARTS)
            .iter()
            .map(Candidate::corners),
    );
    let start_score = refine_score(&refine_photo, plan, &start);
    let (mut best, mut score) = (start, start_score);
    for s in starts {
        let (corners, value) = refine(&refine_photo, plan, s);
        // Une place trouvée loin de la pose n'est pas un ajustement.
        if value > score && centroid(&corners).distance(center) <= ADJUST_REACH * side {
            best = corners;
            score = value;
        }
    }
    let pairs = corner_pairs(&best);
    let outline = slide_to_facade(&pairs)
        .map(|w| footprint(&w))
        .unwrap_or_default();
    let round = |v: f64| (v * 1000.0).round() / 1000.0;
    Adjustment {
        pairs,
        footprint: outline,
        score: round(score.max(0.0)),
        start_score: round(start_score.max(0.0)),
        moved_mm: (0..4)
            .map(|k| best[k].distance(start[k]))
            .fold(0.0, f64::max),
    }
}

/// Coins de la diapositive sur la façade pour ces paires (4 au moins).
pub fn pose_corners(pairs: &[RegistrationPair]) -> Option<[Point; 4]> {
    let warp = slide_to_facade(pairs)?;
    let c = slide_corners();
    Some([
        warp.apply(c[0])?,
        warp.apply(c[1])?,
        warp.apply(c[2])?,
        warp.apply(c[3])?,
    ])
}

/// Ajuste la pose d'un projecteur du projet : `pairs` (la pose affichée,
/// même pas encore enregistrée), ou à défaut ses paires enregistrées. Rien
/// n'est enregistré : on enregistre les paires rendues
/// ([`super::set_registration`]).
pub fn projector_adjustment(
    dir: &ProjectDir,
    projector: &str,
    pairs: Option<&[RegistrationPair]>,
) -> Result<AdjustReport, RegistrationError> {
    let started = Instant::now();
    let project = dir.load()?.project;
    let id = projector_id(&project, projector)?;
    let facade = project
        .facade
        .as_ref()
        .ok_or(RegistrationError::NoScaledFacade)?;
    let mm_per_unit = facade
        .scale
        .as_ref()
        .and_then(|s| s.mm_per_source_unit())
        .ok_or(RegistrationError::NoScaledFacade)?;
    let p = project.projector(&id).expect("projecteur trouvé");
    let rectified = p.rectified_photo.as_ref().ok_or_else(|| {
        RegistrationError::BadOptions(format!(
            "le projecteur {} n'a pas encore de photo redressée",
            p.label
        ))
    })?;
    let pairs = pairs.unwrap_or(&p.registration);
    let start = pose_corners(pairs).ok_or_else(|| {
        RegistrationError::BadOptions(
            "poser d'abord la photo sur la façade (4 points au moins)".into(),
        )
    })?;
    let photo = crate::photo::read(&dir.resolve(rectified))
        .map_err(|e| RegistrationError::BadOptions(format!("photo redressée illisible : {e}")))?
        .into_luma8();
    let plan = render_plan(
        &dir.resolve(&facade.source),
        facade.page,
        facade.rotation,
        mm_per_unit,
        Some(adjust_region(&start)),
        REFINE_PLAN_PX,
    )?;
    let adjustment = adjust(&photo, &plan, start);
    Ok(AdjustReport {
        projector_id: id,
        adjustment,
        region: plan.region(),
        seconds: started.elapsed().as_secs_f64(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un plan de façade inventé, en aplats (comme celui de Trévarez) :
    /// toit, bandeau, mur, et des ouvertures toutes différentes.
    fn synthetic_plan(width: usize, height: usize) -> Gray {
        let mut plan = Gray::new(width, height);
        plan.data.iter_mut().for_each(|v| *v = 60.0);
        let mut fill = |x0: usize, y0: usize, x1: usize, y1: usize, value: f32| {
            for y in y0..y1.min(height) {
                for x in x0..x1.min(width) {
                    plan.data[y * width + x] = value;
                }
            }
        };
        // Toit (de 60 à 200), bandeau (200 à 215), mur (215 à 420).
        fill(40, 60, 760, 200, 15.0);
        fill(40, 200, 760, 215, 170.0);
        fill(40, 215, 760, 420, 35.0);
        // Ouvertures de tailles et de places différentes.
        for (x, y, w, h, v) in [
            (70, 260, 40, 110, 120.0),
            (160, 300, 70, 120, 180.0),
            (290, 240, 30, 30, 90.0),
            (360, 250, 110, 170, 140.0),
            (520, 270, 45, 60, 200.0),
            (610, 255, 25, 120, 110.0),
            (680, 290, 50, 50, 160.0),
        ] {
            fill(x, y, x + w, y + h, v);
        }
        // Une lucarne dans le toit.
        fill(420, 120, 470, 170, 120.0);
        plan
    }

    /// La « photo redressée » qu'on aurait prise de cette place : le plan
    /// vu à travers `truth` (diapositive → façade), contraste inversé, avec
    /// du bruit et la grille sombre de la mire tous les 10 mm.
    fn synthetic_photo(plan: &PlanRaster, truth: &Homography, side: u32) -> GrayImage {
        let mut seed = 12345u64;
        let mut noise = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 33) as f32 / (1u64 << 31) as f32) - 0.5
        };
        let mm = SLIDE_SIZE_MM / f64::from(side);
        GrayImage::from_fn(side, side, |x, y| {
            let slide = Point::new((f64::from(x) + 0.5) * mm, (f64::from(y) + 0.5) * mm);
            let p = truth.apply(slide).unwrap();
            let (px, py) = plan.to_px(p);
            let v = 230.0 - 0.8 * plan.image.sample(px, py) + 30.0 * noise();
            let on_grid = |c: f64| (c % 10.0) < 0.35;
            let v = if on_grid(slide.x) || on_grid(slide.y) {
                v * 0.3
            } else {
                v
            };
            image::Luma([v.clamp(0.0, 255.0) as u8])
        })
    }

    #[test]
    fn l_ajustement_cale_une_pose_approchee_sur_un_plan_invente() {
        // Façade de 32 m × 18 m, rendue à 40 mm par pixel.
        let whole = PlanRaster {
            image: synthetic_plan(800, 450),
            origin: PointMm::new(0.0, 0.0),
            mm_per_px: 40.0,
        };
        // La diapositive couvre environ 11 m, en trapèze (projecteur bas,
        // qui vise vers le haut) et légèrement tournée.
        let truth_corners = [
            Point::new(11_200.0, 5_400.0),
            Point::new(22_600.0, 5_900.0),
            Point::new(21_900.0, 16_600.0),
            Point::new(11_700.0, 16_200.0),
        ];
        let truth = homography(&truth_corners).unwrap();
        let photo = synthetic_photo(&whole, &truth, 700);
        // Pose « à la main » : un carré décalé d'un mètre, 8 % trop grand.
        let start = Candidate {
            score: 0.0,
            center: Point::new(17_850.0 + 1_000.0, 11_025.0 - 700.0),
            side: 11_800.0,
            angle: 0.0,
        }
        .corners();
        let error = |c: &[Point; 4]| {
            (0..4)
                .map(|k| c[k].distance(truth_corners[k]))
                .fold(0.0, f64::max)
        };
        let adjusted = adjust(&photo, &whole, start);
        let corners: Vec<Point> = adjusted
            .pairs
            .iter()
            .map(|p| Point::new(p.facade.x, p.facade.y))
            .collect();
        let corners: [Point; 4] = corners.try_into().unwrap();
        // Moins de 2 % du côté de l'emprise (environ 11 m) au pire coin.
        assert!(
            error(&corners) < 220.0,
            "coin le plus loin à {:.0} mm (départ : {:.0}) ; {adjusted:?}",
            error(&corners),
            error(&start)
        );
        assert!(adjusted.score > adjusted.start_score && adjusted.score > 0.3);
        assert!(adjusted.moved_mm > 500.0);
    }

    #[test]
    fn une_pose_deja_juste_ne_bouge_presque_pas() {
        let whole = PlanRaster {
            image: synthetic_plan(800, 450),
            origin: PointMm::new(0.0, 0.0),
            mm_per_px: 40.0,
        };
        let truth_corners = [
            Point::new(11_200.0, 5_400.0),
            Point::new(22_600.0, 5_900.0),
            Point::new(21_900.0, 16_600.0),
            Point::new(11_700.0, 16_200.0),
        ];
        let photo = synthetic_photo(&whole, &homography(&truth_corners).unwrap(), 700);
        let adjusted = adjust(&photo, &whole, truth_corners);
        assert!(adjusted.moved_mm < 150.0, "{adjusted:?}");
    }

    #[test]
    fn la_zone_regardee_entoure_la_pose() {
        let start = [
            Point::new(1_000.0, 2_000.0),
            Point::new(11_000.0, 2_000.0),
            Point::new(11_000.0, 12_000.0),
            Point::new(1_000.0, 12_000.0),
        ];
        let r = adjust_region(&start);
        assert_eq!(
            r,
            RegionMm {
                x: -4_000.0,
                y: -3_000.0,
                width: 20_000.0,
                height: 20_000.0
            }
        );
    }

    #[test]
    fn la_fermeture_efface_la_grille_de_la_mire() {
        let mut image = Gray::new(60, 60);
        for y in 0..60 {
            for x in 0..60 {
                image.data[y * 60 + x] = if x % 20 == 0 || y % 20 == 0 {
                    20.0
                } else {
                    200.0
                };
            }
        }
        let closed = image.closed(2);
        assert!(closed.data.iter().all(|&v| v > 190.0));
    }
}

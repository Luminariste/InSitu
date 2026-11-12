//! Points remarquables et appariement automatique entre deux photos
//! (tâche 6.6, décision D-6.6-a).
//!
//! Méthode classique, écrite ici en Rust pur (pas de bibliothèque système) :
//!
//! 1. **Coins** : chaque photo est réduite à environ 2 000 px de côté, puis
//!    déclinée en une pyramide de 5 tailles (facteur 1,3), pour retrouver
//!    un même détail sur deux photos prises d'un peu plus près ou de plus
//!    loin. Sur chaque taille, test FAST (9 pixels consécutifs d'un cercle
//!    nettement plus clairs ou plus sombres que le centre), classement par
//!    la mesure de Harris, et au plus 2 coins par case (environ 42 cases sur
//!    le grand côté) pour qu'ils couvrent toute la photo.
//! 2. **Signature** : histogrammes des directions du relief lumineux sur
//!    4 × 4 cases autour du coin, tournés selon l'orientation dominante du
//!    détail (méthode « SIFT »).
//! 3. **Appariement** : pour chaque coin d'une photo, le plus ressemblant de
//!    l'autre, gardé s'il est nettement meilleur que le deuxième (rapport
//!    0,85) et si la réciproque est vraie.
//! 4. **Tri géométrique (RANSAC)** : on tire au hasard 4 appariements, on en
//!    déduit un trapèze, on compte les autres qui le suivent à 3 px près ;
//!    le meilleur tirage est affiné sur tous ceux qui le suivent. Les faux
//!    appariements (fenêtres répétées, ciel, relief hors du plan du mur)
//!    tombent d'eux-mêmes.
//! 5. **Affinage** : chaque appariement retenu est recalé au dixième de
//!    pixel par corrélation locale (Lucas-Kanade), et le trapèze recalculé.

use std::f64::consts::PI;

use image::GrayImage;

use crate::geometry::{Homography, Point, Transform};

/// Plus grand côté de l'image de travail, en pixels.
pub const WORK_SIDE: u32 = 2048;

/// Nombre de tailles de la pyramide, et rapport entre deux tailles.
const LEVELS: usize = 5;
const LEVEL_FACTOR: f64 = 1.3;

/// Seuil du test FAST, en niveaux de gris (0 à 255).
const FAST_THRESHOLD: f32 = 12.0;

/// Côté des cases de répartition des coins, en pixels de chaque taille :
/// environ 42 cases sur le grand côté de l'image de travail, entre 12 et
/// 48 px.
fn cell_size(width: usize, height: usize) -> usize {
    (width.max(height) / 42).clamp(12, 48)
}

/// Coins gardés au plus par case.
const PER_CELL: usize = 2;

/// Distance au bord sous laquelle un coin n'est pas gardé (sa signature
/// déborderait).
const BORDER: usize = 20;

/// Rapport maximal entre la meilleure et la deuxième ressemblance.
const RATIO: f64 = 0.85;

/// Écart (pixels de l'image de travail) sous lequel un appariement suit le
/// trapèze.
pub const RANSAC_THRESHOLD: f64 = 3.0;

/// Tirages au plus.
const RANSAC_ITERATIONS: usize = 4000;

/// Un coin, en pixels de l'image de travail (centre du pixel en `i + 0,5`).
#[derive(Debug, Clone, Copy)]
pub struct Keypoint {
    pub point: Point,
    /// Taille de la pyramide (0 : la plus fine).
    pub level: usize,
    pub response: f32,
}

/// Les coins d'une photo et leur signature.
#[derive(Debug, Clone)]
pub struct Features {
    pub keypoints: Vec<Keypoint>,
    pub descriptors: Vec<Descriptor>,
    /// Taille de l'image de travail.
    pub width: usize,
    pub height: usize,
    /// Pixels de la photo par pixel de l'image de travail.
    pub scale: f64,
    /// Image de travail légèrement lissée, pour l'affinage des positions.
    image: Plane,
}

impl Features {
    /// Position d'un coin en pixels de la photo d'origine.
    pub fn in_photo(&self, k: usize) -> Point {
        let p = self.keypoints[k].point;
        Point::new(p.x * self.scale, p.y * self.scale)
    }
}

/// Une image en niveaux de gris, en flottants.
#[derive(Debug, Clone)]
struct Plane {
    w: usize,
    h: usize,
    data: Vec<f32>,
}

impl Plane {
    fn at(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.w + x]
    }

    /// Valeur interpolée (bilinéaire), indices de pixels (centre en `i`).
    fn bilinear(&self, x: f64, y: f64) -> f32 {
        let x = x.clamp(0.0, (self.w - 1) as f64);
        let y = y.clamp(0.0, (self.h - 1) as f64);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (fx, fy) = ((x - x0 as f64) as f32, (y - y0 as f64) as f32);
        let top = self.at(x0, y0) * (1.0 - fx) + self.at(x1, y0) * fx;
        let bottom = self.at(x0, y1) * (1.0 - fx) + self.at(x1, y1) * fx;
        top * (1.0 - fy) + bottom * fy
    }

    /// Flou gaussien séparable d'écart type `sigma` (pixels).
    fn blur(&self, sigma: f64) -> Plane {
        if sigma < 0.3 {
            return self.clone();
        }
        let radius = (3.0 * sigma).ceil() as i64;
        let kernel: Vec<f32> = (-radius..=radius)
            .map(|i| (-(i * i) as f64 / (2.0 * sigma * sigma)).exp() as f32)
            .collect();
        let total: f32 = kernel.iter().sum();
        let kernel: Vec<f32> = kernel.iter().map(|k| k / total).collect();
        let (w, h) = (self.w as i64, self.h as i64);
        let mut tmp = vec![0f32; self.data.len()];
        for y in 0..h {
            for x in 0..w {
                let mut s = 0.0;
                for (k, kv) in kernel.iter().enumerate() {
                    let xx = (x + k as i64 - radius).clamp(0, w - 1);
                    s += kv * self.data[(y * w + xx) as usize];
                }
                tmp[(y * w + x) as usize] = s;
            }
        }
        let mut out = vec![0f32; self.data.len()];
        for y in 0..h {
            for x in 0..w {
                let mut s = 0.0;
                for (k, kv) in kernel.iter().enumerate() {
                    let yy = (y + k as i64 - radius).clamp(0, h - 1);
                    s += kv * tmp[(yy * w + x) as usize];
                }
                out[(y * w + x) as usize] = s;
            }
        }
        Plane {
            w: self.w,
            h: self.h,
            data: out,
        }
    }

    /// Réduction d'un facteur `factor` (> 1) : flou puis échantillonnage.
    fn shrink(&self, factor: f64) -> Plane {
        let w = ((self.w as f64 / factor).round() as usize).max(1);
        let h = ((self.h as f64 / factor).round() as usize).max(1);
        let blurred = self.blur(0.5 * (factor * factor - 1.0).sqrt());
        let (sx, sy) = (self.w as f64 / w as f64, self.h as f64 / h as f64);
        let mut data = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                data.push(
                    blurred.bilinear((x as f64 + 0.5) * sx - 0.5, (y as f64 + 0.5) * sy - 0.5),
                );
            }
        }
        Plane { w, h, data }
    }
}

/// Réduit la photo par moyenne de blocs entiers, jusqu'à `max_side` de côté
/// au plus. Rend l'image et le facteur (pixels de photo par pixel réduit).
fn work_image(gray: &GrayImage, max_side: u32) -> (Plane, f64) {
    let (w, h) = gray.dimensions();
    let k = w.max(h).div_ceil(max_side.max(1)).max(1) as usize;
    let (ow, oh) = ((w as usize / k).max(1), (h as usize / k).max(1));
    let mut data = vec![0f32; ow * oh];
    let raw = gray.as_raw();
    let norm = 1.0 / (k * k) as f32;
    for oy in 0..oh {
        for dy in 0..k {
            let row = &raw[(oy * k + dy) * w as usize..];
            for ox in 0..ow {
                let mut s = 0u32;
                for dx in 0..k {
                    s += u32::from(row[ox * k + dx]);
                }
                data[oy * ow + ox] += s as f32;
            }
        }
    }
    for v in &mut data {
        *v *= norm;
    }
    (Plane { w: ow, h: oh, data }, k as f64)
}

/// Les 16 pixels du cercle de rayon 3 du test FAST.
const CIRCLE: [(i32, i32); 16] = [
    (0, -3),
    (1, -3),
    (2, -2),
    (3, -1),
    (3, 0),
    (3, 1),
    (2, 2),
    (1, 3),
    (0, 3),
    (-1, 3),
    (-2, 2),
    (-3, 1),
    (-3, 0),
    (-3, -1),
    (-2, -2),
    (-1, -3),
];

/// Test FAST-9 : 9 pixels consécutifs du cercle tous plus clairs (ou tous
/// plus sombres) que le centre, de plus que le seuil.
fn is_fast_corner(img: &Plane, x: usize, y: usize) -> bool {
    let c = img.at(x, y);
    let at = |i: usize| {
        let (dx, dy) = CIRCLE[i % 16];
        img.at((x as i32 + dx) as usize, (y as i32 + dy) as usize)
    };
    // Rejet rapide : sur les 4 points cardinaux, au moins 2 consécutifs
    // doivent passer d'un même côté.
    let (hi, lo) = (c + FAST_THRESHOLD, c - FAST_THRESHOLD);
    let card = [at(0), at(4), at(8), at(12)];
    let brighter = card.iter().filter(|&&v| v > hi).count();
    let darker = card.iter().filter(|&&v| v < lo).count();
    if brighter < 2 && darker < 2 {
        return false;
    }
    let values: [f32; 16] = std::array::from_fn(at);
    for sign in [1.0f32, -1.0] {
        let mut run = 0;
        for i in 0..32 {
            let v = values[i % 16];
            if sign * (v - c) > FAST_THRESHOLD {
                run += 1;
                if run >= 9 {
                    return true;
                }
            } else {
                run = 0;
            }
        }
    }
    false
}

/// Mesure de Harris sur une fenêtre 7 × 7 (gradients centrés).
fn harris(img: &Plane, x: usize, y: usize) -> f32 {
    let (mut sxx, mut syy, mut sxy) = (0f32, 0f32, 0f32);
    for dy in -3i32..=3 {
        for dx in -3i32..=3 {
            let (px, py) = ((x as i32 + dx) as usize, (y as i32 + dy) as usize);
            let gx = img.at(px + 1, py) - img.at(px - 1, py);
            let gy = img.at(px, py + 1) - img.at(px, py - 1);
            sxx += gx * gx;
            syy += gy * gy;
            sxy += gx * gy;
        }
    }
    sxx * syy - sxy * sxy - 0.04 * (sxx + syy) * (sxx + syy)
}

/// Petit générateur pseudo-aléatoire déterministe (xorshift) : mêmes
/// résultats à chaque calcul.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    pub(crate) fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// Entier dans `0..n`.
    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Longueur d'une signature : 4 × 4 cases × 8 directions.
pub const DESCRIPTOR_LEN: usize = 128;

/// Une signature : histogrammes des directions du relief lumineux autour
/// du coin (méthode « SIFT »), sur 4 × 4 cases, en octets.
pub type Descriptor = [u8; DESCRIPTOR_LEN];

/// Demi-côté de la fenêtre de la signature, en pixels de la taille du coin.
const DESCRIPTOR_RADIUS: f64 = 12.0;

/// Gradient (x, y) en un point, par différences centrées.
fn gradient(img: &Plane, x: f64, y: f64) -> (f64, f64) {
    (
        0.5 * f64::from(img.bilinear(x + 1.0, y) - img.bilinear(x - 1.0, y)),
        0.5 * f64::from(img.bilinear(x, y + 1.0) - img.bilinear(x, y - 1.0)),
    )
}

/// Orientation dominante du relief lumineux autour du coin (histogramme
/// de 36 directions, pondéré par l'intensité du gradient), en radians.
fn orientation(img: &Plane, x: usize, y: usize) -> f64 {
    let mut hist = [0f64; 36];
    let r = 9i32;
    let sigma = 4.5;
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = f64::from(dx * dx + dy * dy);
            if d2 > f64::from(r * r) {
                continue;
            }
            let (gx, gy) = gradient(img, (x as i32 + dx) as f64, (y as i32 + dy) as f64);
            let weight = (gx * gx + gy * gy).sqrt() * (-d2 / (2.0 * sigma * sigma)).exp();
            let angle = gy.atan2(gx).rem_euclid(2.0 * PI);
            hist[((angle / (2.0 * PI) * 36.0) as usize).min(35)] += weight;
        }
    }
    // Lissage, puis sommet affiné par une parabole.
    let smooth: Vec<f64> = (0..36)
        .map(|i| 0.25 * hist[(i + 35) % 36] + 0.5 * hist[i] + 0.25 * hist[(i + 1) % 36])
        .collect();
    let (best, _) = smooth
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .expect("36 cases");
    let (l, c, r) = (
        smooth[(best + 35) % 36],
        smooth[best],
        smooth[(best + 1) % 36],
    );
    let denom = l - 2.0 * c + r;
    let offset = if denom.abs() > 1e-12 {
        0.5 * (l - r) / denom
    } else {
        0.0
    };
    (best as f64 + 0.5 + offset) / 36.0 * 2.0 * PI
}

/// Signature d'un coin, tournée selon `angle`.
fn describe(img: &Plane, x: usize, y: usize, angle: f64) -> Descriptor {
    let mut hist = [0f64; DESCRIPTOR_LEN];
    let (s, c) = angle.sin_cos();
    let r = DESCRIPTOR_RADIUS;
    let step = 2.0 * r / 16.0;
    let sigma = r;
    for iy in 0..16 {
        for ix in 0..16 {
            // Position dans le repère tourné du coin.
            let u = -r + (f64::from(ix) + 0.5) * step;
            let v = -r + (f64::from(iy) + 0.5) * step;
            let px = x as f64 + c * u - s * v;
            let py = y as f64 + s * u + c * v;
            let (gx, gy) = gradient(img, px, py);
            let magnitude =
                (gx * gx + gy * gy).sqrt() * (-(u * u + v * v) / (2.0 * sigma * sigma)).exp();
            // Direction relative à l'orientation du coin.
            let theta = (gy.atan2(gx) - angle).rem_euclid(2.0 * PI);
            let bin = theta / (2.0 * PI) * 8.0;
            let b0 = (bin.floor() as usize) % 8;
            let frac = bin - bin.floor();
            // Répartition sur les cases voisines (position et direction).
            let fx = (f64::from(ix) + 0.5) / 4.0 - 0.5;
            let fy = (f64::from(iy) + 0.5) / 4.0 - 0.5;
            let (cx0, cy0) = (fx.floor(), fy.floor());
            let (wx, wy) = (fx - cx0, fy - cy0);
            for (cy, wyy) in [(cy0, 1.0 - wy), (cy0 + 1.0, wy)] {
                if !(0.0..4.0).contains(&cy) {
                    continue;
                }
                for (cx, wxx) in [(cx0, 1.0 - wx), (cx0 + 1.0, wx)] {
                    if !(0.0..4.0).contains(&cx) {
                        continue;
                    }
                    let cell = (cy as usize * 4 + cx as usize) * 8;
                    let w = magnitude * wxx * wyy;
                    hist[cell + b0] += w * (1.0 - frac);
                    hist[cell + (b0 + 1) % 8] += w * frac;
                }
            }
        }
    }
    // Normalisation, écrêtage à 0,2 (reflets, contrastes forts), puis
    // normalisation encore.
    let norm = hist.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-12);
    hist.iter_mut().for_each(|v| *v = (*v / norm).min(0.2));
    let norm = hist.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-12);
    std::array::from_fn(|i| ((hist[i] / norm) * 512.0).round().min(255.0) as u8)
}

/// Coins et signatures d'une photo en niveaux de gris.
pub fn detect(gray: &GrayImage) -> Features {
    let (base, scale) = work_image(gray, WORK_SIDE);
    let (width, height) = (base.w, base.h);
    let mut keypoints = Vec::new();
    let mut descriptors = Vec::new();
    let cell = cell_size(width, height);
    let mut level_img = base.blur(0.5);
    for level in 0..LEVELS {
        let factor = LEVEL_FACTOR.powi(level as i32);
        if level > 0 {
            level_img = base.shrink(factor);
        }
        let img = &level_img;
        if img.w <= 2 * BORDER || img.h <= 2 * BORDER {
            break;
        }
        // Coins FAST et mesure de Harris.
        let mut score = vec![0f32; img.w * img.h];
        for y in BORDER..img.h - BORDER {
            for x in BORDER..img.w - BORDER {
                if is_fast_corner(img, x, y) {
                    score[y * img.w + x] = harris(img, x, y).max(f32::MIN_POSITIVE);
                }
            }
        }
        // Le meilleur maximum local de chaque case.
        let (cols, rows) = (img.w.div_ceil(cell), img.h.div_ceil(cell));
        let mut best: Vec<Vec<(usize, usize, f32)>> = vec![Vec::new(); cols * rows];
        for y in BORDER..img.h - BORDER {
            for x in BORDER..img.w - BORDER {
                let v = score[y * img.w + x];
                if v <= 0.0 {
                    continue;
                }
                let local_max = (y - 1..=y + 1).all(|yy| {
                    (x - 1..=x + 1).all(|xx| (xx == x && yy == y) || score[yy * img.w + xx] < v)
                });
                if !local_max {
                    continue;
                }
                let k = (y / cell) * cols + x / cell;
                let list = &mut best[k];
                if list.len() < PER_CELL || list.last().is_some_and(|l| v > l.2) {
                    let at = list.partition_point(|l| l.2 >= v);
                    list.insert(at, (x, y, v));
                    list.truncate(PER_CELL);
                }
            }
        }
        let smooth = img.blur(1.0);
        for (x, y, v) in best.into_iter().flatten() {
            let angle = orientation(&smooth, x, y);
            descriptors.push(describe(&smooth, x, y, angle));
            keypoints.push(Keypoint {
                // Centre du pixel de cette taille, ramené à la plus fine.
                point: Point::new(
                    (x as f64 + 0.5) * base.w as f64 / img.w as f64,
                    (y as f64 + 0.5) * base.h as f64 / img.h as f64,
                ),
                level,
                response: v,
            });
        }
    }
    Features {
        keypoints,
        descriptors,
        width,
        height,
        scale,
        image: base.blur(0.8),
    }
}

fn distance(a: &Descriptor, b: &Descriptor) -> u32 {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| {
            let d = i32::from(x) - i32::from(y);
            (d * d) as u32
        })
        .sum()
}

/// Les deux plus proches, tenus à jour : (distance, indice), (distance).
#[derive(Clone, Copy)]
struct Best2 {
    d1: u32,
    i1: usize,
    d2: u32,
}

impl Best2 {
    const NONE: Self = Self {
        d1: u32::MAX,
        i1: usize::MAX,
        d2: u32::MAX,
    };
    fn offer(&mut self, d: u32, i: usize) {
        if d < self.d1 {
            self.d2 = self.d1;
            self.d1 = d;
            self.i1 = i;
        } else if d < self.d2 {
            self.d2 = d;
        }
    }
    /// Le plus proche, s'il l'est nettement plus que le deuxième (rapport
    /// des distances, donc des carrés au carré du rapport).
    fn accepted(&self) -> Option<usize> {
        (self.i1 != usize::MAX && (self.d1 as f64) < RATIO * RATIO * self.d2 as f64)
            .then_some(self.i1)
    }
}

/// Appariements candidats (indices dans `a`, dans `b`) : plus proches
/// voisins réciproques, nettement meilleurs que le deuxième.
pub fn candidate_matches(a: &Features, b: &Features) -> Vec<(usize, usize)> {
    let mut rows = vec![Best2::NONE; a.descriptors.len()];
    let mut cols = vec![Best2::NONE; b.descriptors.len()];
    for (i, da) in a.descriptors.iter().enumerate() {
        for (j, db) in b.descriptors.iter().enumerate() {
            let d = distance(da, db);
            rows[i].offer(d, j);
            cols[j].offer(d, i);
        }
    }
    rows.iter()
        .enumerate()
        .filter_map(|(i, r)| {
            r.accepted()
                .filter(|&j| cols[j].accepted() == Some(i))
                .map(|j| (i, j))
        })
        .collect()
}

/// Résultat du tri géométrique entre deux photos.
#[derive(Debug, Clone)]
pub struct PairMatch {
    /// Trapèze qui envoie l'image de travail de `a` sur celle de `b`.
    pub homography: Homography,
    /// Appariements retenus, positions affinées.
    pub inliers: Vec<Matched>,
}

/// Un appariement retenu, en pixels des images de travail.
#[derive(Debug, Clone, Copy)]
pub struct Matched {
    pub a: Point,
    pub b: Point,
    /// Écart au trapèze final, en pixels de l'image de travail de `b`.
    pub error: f64,
}

fn error(h: &Homography, p: Point, q: Point) -> f64 {
    h.apply(p).map_or(f64::INFINITY, |r| r.distance(q))
}

/// Vrai si le trapèze pose l'image `a` sur `b` de façon vraisemblable : sans
/// retournement ni écrasement.
pub fn plausible(h: &Homography, a: &Features) -> bool {
    let (w, ht) = (a.width as f64, a.height as f64);
    let corners = [
        Point::new(0.0, 0.0),
        Point::new(w, 0.0),
        Point::new(w, ht),
        Point::new(0.0, ht),
    ];
    let mut q = [Point::new(0.0, 0.0); 4];
    for (k, c) in corners.iter().enumerate() {
        match h.apply(*c) {
            Some(p) if p.is_finite() => q[k] = p,
            _ => return false,
        }
    }
    // Quadrilatère convexe, parcouru dans le même sens (pas de miroir).
    for k in 0..4 {
        let (p0, p1, p2) = (q[k], q[(k + 1) % 4], q[(k + 2) % 4]);
        let cross = (p1.x - p0.x) * (p2.y - p1.y) - (p1.y - p0.y) * (p2.x - p1.x);
        if cross <= 0.0 {
            return false;
        }
    }
    let area = 0.5
        * (0..4)
            .map(|k| q[k].x * q[(k + 1) % 4].y - q[(k + 1) % 4].x * q[k].y)
            .sum::<f64>();
    let ratio = area / (w * ht);
    (0.1..=10.0).contains(&ratio)
}

/// Tri géométrique (RANSAC) des appariements candidats. `None` si aucun
/// trapèze vraisemblable n'est suivi par au moins `min_inliers` d'entre eux.
pub fn ransac(
    a: &Features,
    b: &Features,
    candidates: &[(usize, usize)],
    min_inliers: usize,
) -> Option<PairMatch> {
    if candidates.len() < min_inliers.max(4) {
        return None;
    }
    let pa: Vec<Point> = candidates
        .iter()
        .map(|&(i, _)| a.keypoints[i].point)
        .collect();
    let pb: Vec<Point> = candidates
        .iter()
        .map(|&(_, j)| b.keypoints[j].point)
        .collect();
    let n = candidates.len();
    let count = |h: &Homography| {
        (0..n)
            .filter(|&k| error(h, pa[k], pb[k]) < RANSAC_THRESHOLD)
            .count()
    };
    let mut rng = Rng::new(0x5EED_0006 ^ n as u64);
    let mut best: Option<(Homography, usize)> = None;
    let mut limit = RANSAC_ITERATIONS;
    let mut iteration = 0;
    while iteration < limit {
        iteration += 1;
        let mut idx = [0usize; 4];
        let mut ok = true;
        for slot in 0..4 {
            let mut tries = 0;
            loop {
                let k = rng.below(n);
                tries += 1;
                if idx[..slot]
                    .iter()
                    .all(|&j| j != k && pa[j].distance(pa[k]) > 10.0)
                {
                    idx[slot] = k;
                    break;
                }
                if tries > 20 {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        let Ok(h) = Homography::from_four_pairs(idx.map(|k| pa[k]), idx.map(|k| pb[k])) else {
            continue;
        };
        if !plausible(&h, a) {
            continue;
        }
        let c = count(&h);
        if best.as_ref().is_none_or(|(_, bc)| c > *bc) {
            best = Some((h, c));
            // Nombre de tirages qui suffit, à 99,9 %, vu la part d'appariements justes.
            let w = c as f64 / n as f64;
            let needed = ((1.0 - 0.999f64).ln() / (1.0 - w.powi(4)).max(1e-12).ln()).ceil();
            if needed.is_finite() {
                limit = limit.min((needed as usize).max(50));
            }
        }
    }
    let (mut h, _) = best?;
    // Affinage : moindres carrés sur tous ceux qui suivent.
    for _ in 0..2 {
        let (src, dst): (Vec<Point>, Vec<Point>) = (0..n)
            .filter(|&k| error(&h, pa[k], pb[k]) < RANSAC_THRESHOLD)
            .map(|k| (pa[k], pb[k]))
            .unzip();
        if src.len() < 4 {
            return None;
        }
        h = Homography::fit(&src, &dst).ok()?;
    }
    // Positions affinées au dixième de pixel par corrélation locale, puis
    // trapèze recalculé sur elles (D-6.6-b).
    let mut refined: Vec<(Point, Point)> = (0..n)
        .filter(|&k| error(&h, pa[k], pb[k]) < RANSAC_THRESHOLD)
        .filter_map(|k| refine(a, b, &h, pa[k]).map(|q| (pa[k], q)))
        .collect();
    for _ in 0..2 {
        if refined.len() < min_inliers.max(4) {
            return None;
        }
        let (src, dst): (Vec<Point>, Vec<Point>) = refined.iter().copied().unzip();
        h = Homography::fit(&src, &dst).ok()?;
        refined.retain(|&(p, q)| error(&h, p, q) < REFINED_THRESHOLD);
    }
    if refined.len() < min_inliers || !plausible(&h, a) {
        return None;
    }
    let inliers = refined
        .into_iter()
        .map(|(p, q)| Matched {
            a: p,
            b: q,
            error: error(&h, p, q),
        })
        .collect();
    Some(PairMatch {
        homography: h,
        inliers,
    })
}

/// Écart au trapèze (pixels de travail) au-delà duquel un appariement
/// affiné est écarté.
const REFINED_THRESHOLD: f64 = 1.0;

/// Demi-côté de la fenêtre de corrélation, en pixels de travail.
const REFINE_RADIUS: i32 = 8;

/// Position affinée, dans `b`, du détail vu en `p` dans `a`. La fenêtre de
/// `a` autour de `p` est transportée dans `b` par le trapèze `h` (ce qui
/// compense le changement de point de vue), puis glissée sur `b` jusqu'à
/// coïncider au mieux (méthode de Lucas et Kanade, luminosité et contraste
/// compensés). `None` si la fenêtre est trop uniforme ou si la position ne
/// converge pas près de la prédiction.
fn refine(a: &Features, b: &Features, h: &Homography, p: Point) -> Option<Point> {
    let inverse = h.inverse().ok()?;
    let q0 = h.apply(p)?;
    let r = REFINE_RADIUS;
    let side = (2 * r + 1) as usize;
    // Fenêtre de `a` vue dans le repère de `b`, autour de q0.
    let mut template = Vec::with_capacity(side * side);
    for dy in -r..=r {
        for dx in -r..=r {
            let src = inverse.apply(Point::new(q0.x + f64::from(dx), q0.y + f64::from(dy)))?;
            if src.x < 1.0
                || src.y < 1.0
                || src.x > a.width as f64 - 1.0
                || src.y > a.height as f64 - 1.0
            {
                return None;
            }
            template.push(f64::from(a.image.bilinear(src.x - 0.5, src.y - 0.5)));
        }
    }
    let normalize = |v: &mut [f64]| -> Option<()> {
        let mean = v.iter().sum::<f64>() / v.len() as f64;
        let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / v.len() as f64;
        if var < 4.0 {
            return None;
        }
        let sd = var.sqrt();
        v.iter_mut().for_each(|x| *x = (*x - mean) / sd);
        Some(())
    };
    normalize(&mut template)?;
    let mut q = q0;
    let margin = f64::from(r) + 2.0;
    for _ in 0..10 {
        if q.x < margin
            || q.y < margin
            || q.x > b.width as f64 - margin
            || q.y > b.height as f64 - margin
        {
            return None;
        }
        let mut values = Vec::with_capacity(side * side);
        let mut gx = Vec::with_capacity(side * side);
        let mut gy = Vec::with_capacity(side * side);
        for dy in -r..=r {
            for dx in -r..=r {
                let (x, y) = (q.x + f64::from(dx) - 0.5, q.y + f64::from(dy) - 0.5);
                values.push(f64::from(b.image.bilinear(x, y)));
                gx.push(
                    0.5 * f64::from(b.image.bilinear(x + 1.0, y) - b.image.bilinear(x - 1.0, y)),
                );
                gy.push(
                    0.5 * f64::from(b.image.bilinear(x, y + 1.0) - b.image.bilinear(x, y - 1.0)),
                );
            }
        }
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let var = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
        if var < 4.0 {
            return None;
        }
        let sd = var.sqrt();
        let (mut axx, mut axy, mut ayy, mut bx, mut by) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for k in 0..values.len() {
            let (ix, iy) = (gx[k] / sd, gy[k] / sd);
            let diff = template[k] - (values[k] - mean) / sd;
            axx += ix * ix;
            axy += ix * iy;
            ayy += iy * iy;
            bx += ix * diff;
            by += iy * diff;
        }
        let det = axx * ayy - axy * axy;
        // Fenêtre sans détail dans une direction (un trait droit, un aplat).
        let trace = axx + ayy;
        if det <= 1e-6 * trace * trace || trace <= 0.0 {
            return None;
        }
        let step = Point::new((ayy * bx - axy * by) / det, (axx * by - axy * bx) / det);
        q = Point::new(q.x + step.x, q.y + step.y);
        if q.distance(q0) > 2.5 {
            return None;
        }
        if step.norm() < 0.01 {
            return Some(q);
        }
    }
    Some(q)
}

/// Appariement complet de deux photos.
pub fn match_pair(a: &Features, b: &Features, min_inliers: usize) -> Option<PairMatch> {
    let candidates = candidate_matches(a, b);
    ransac(a, b, &candidates, min_inliers)
}

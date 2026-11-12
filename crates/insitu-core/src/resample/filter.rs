//! Le filtre de rééchantillonnage : noyau, empreinte d'un pixel, pyramide.
//!
//! Pour chaque pixel de sortie, on connaît sa position dans l'image source et
//! la dérivée locale de la transformation (`J`, en pixels source par pixel de
//! sortie). La valeur du pixel est une moyenne pondérée des pixels source
//! voisins, avec un noyau cubique de Catmull-Rom :
//!
//! - **agrandissement ou taille égale** (la transformation n'étire nulle part
//!   le pixel de sortie au-delà d'un pixel source) : le noyau est posé sur la
//!   grille de l'image source ; c'est une interpolation bicubique classique,
//!   qui rend exactement l'image d'origine quand la transformation est
//!   l'identité ou une translation d'un nombre entier de pixels ;
//! - **réduction** : le noyau est étiré, dans chaque direction, de ce que la
//!   transformation réduit dans cette direction. Il couvre alors tout ce que
//!   le pixel de sortie « voit » de la source, ce qui évite le crénelage et
//!   le moiré des hachures fines.
//!
//! Au-delà d'une réduction de 2, on lit une version de la source réduite par
//! moitiés successives (pyramide), pour que le coût par pixel reste borné.

use std::sync::OnceLock;

use crate::geometry::Jacobian;
use crate::tiles::Raster;

/// Demi-largeur du noyau de Catmull-Rom, en pixels.
pub(super) const KERNEL_RADIUS: f64 = 2.0;

/// Étirement maximal du noyau dans la pyramide : au-delà, on passe au niveau
/// suivant. Avec 4, un pixel lit au plus une vingtaine de pixels source dans
/// la direction la plus réduite, même sous un trapèze très incliné.
const MAX_STRETCH: f64 = 4.0;

/// Noyau cubique de Catmull-Rom (Keys, a = −1/2) : vaut 1 en 0, 0 aux autres
/// entiers, pente continue. Il rend la netteté d'un trait mieux qu'une
/// interpolation bilinéaire (qui l'étale) et sonne moins qu'un Lanczos.
#[inline]
pub(super) fn catmull_rom(t: f64) -> f64 {
    let t = t.abs();
    if t < 1.0 {
        (1.5 * t - 2.5) * t * t + 1.0
    } else if t < 2.0 {
        ((-0.5 * t + 2.5) * t - 4.0) * t + 2.0
    } else {
        0.0
    }
}

/// Ce qu'il faut pour calculer un pixel de sortie, hors sa position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Footprint {
    /// Niveau de la pyramide à lire (0 = image source).
    pub level: usize,
    /// Facteur à appliquer à une position de niveau 0 pour ce niveau.
    pub level_scale: f64,
    /// Passage d'un écart en pixels du niveau lu aux coordonnées du noyau.
    pub inverse: [[f64; 2]; 2],
    /// Demi-étendue, en pixels du niveau lu, des pixels à lire autour de la
    /// position : en x puis en y.
    pub extent: (f64, f64),
    /// Vrai si le noyau est posé tel quel sur la grille source (pas de
    /// réduction) : c'est une interpolation bicubique, calculée plus vite.
    pub interpolating: bool,
}

/// Empreinte d'un pixel sans réduction : interpolation bicubique.
const INTERPOLATION: Footprint = Footprint {
    level: 0,
    level_scale: 1.0,
    inverse: [[1.0, 0.0], [0.0, 1.0]],
    extent: (KERNEL_RADIUS, KERNEL_RADIUS),
    interpolating: true,
};

impl Footprint {
    /// Empreinte d'un pixel de sortie, d'après la dérivée de la
    /// transformation (sortie vers source) en ce pixel. `None` si la dérivée
    /// n'est pas un nombre fini.
    pub fn from_jacobian(j: &Jacobian, level_count: usize) -> Option<Self> {
        if !j.iter().flatten().all(|v| v.is_finite()) {
            return None;
        }
        // M = J·Jᵀ décrit l'ellipse que couvre, dans la source, le cercle
        // d'un pixel de sortie : ses valeurs propres sont les carrés des
        // étirements, ses vecteurs propres leurs directions.
        let a = j[0][0] * j[0][0] + j[0][1] * j[0][1];
        let b = j[0][0] * j[1][0] + j[0][1] * j[1][1];
        let c = j[1][0] * j[1][0] + j[1][1] * j[1][1];
        // Somme des carrés des étirements ≤ 1 : aucun ne dépasse 1.
        if a + c <= 1.0 {
            return Some(INTERPOLATION);
        }
        let mean = 0.5 * (a + c);
        let spread = (0.25 * (a - c) * (a - c) + b * b).sqrt();
        let big = (mean + spread).max(0.0).sqrt();
        let small = (mean - spread).max(0.0).sqrt();
        // Direction de l'étirement le plus grand.
        let (ux, uy) = if b.abs() > 1e-12 * (a + c) {
            let (x, y) = (b, mean + spread - a);
            let n = x.hypot(y);
            (x / n, y / n)
        } else if a >= c {
            (1.0, 0.0)
        } else {
            (0.0, 1.0)
        };

        let mut level = 0usize;
        if small >= 2.0 {
            level = small.log2().floor() as usize;
        }
        if big > MAX_STRETCH {
            level = level.max((big / MAX_STRETCH).log2().ceil() as usize);
        }
        level = level.min(level_count.saturating_sub(1));
        let level_scale = 0.5f64.powi(level as i32);

        // Étirement du noyau : au moins un pixel du niveau lu.
        let s_big = (big * level_scale).max(1.0);
        let s_small = (small * level_scale).max(1.0);
        if level == 0 && s_big == 1.0 {
            return Some(INTERPOLATION);
        }
        // A = U·diag(s)·Uᵀ, et son inverse avec 1/s.
        let (vx, vy) = (-uy, ux);
        let (ib, is) = (1.0 / s_big, 1.0 / s_small);
        let inverse = [
            [ib * ux * ux + is * vx * vx, ib * ux * uy + is * vx * vy],
            [ib * ux * uy + is * vx * vy, ib * uy * uy + is * vy * vy],
        ];
        let forward = [
            [
                s_big * ux * ux + s_small * vx * vx,
                s_big * ux * uy + s_small * vx * vy,
            ],
            [
                s_big * ux * uy + s_small * vx * vy,
                s_big * uy * uy + s_small * vy * vy,
            ],
        ];
        let extent = (
            KERNEL_RADIUS * (forward[0][0].abs() + forward[0][1].abs()),
            KERNEL_RADIUS * (forward[1][0].abs() + forward[1][1].abs()),
        );
        Some(Self {
            level,
            level_scale,
            inverse,
            extent,
            interpolating: false,
        })
    }
}

/// L'image source et ses réductions par moitiés, fabriquées à la demande.
pub(super) struct Pyramid<'a> {
    base: &'a Raster,
    reduced: Vec<OnceLock<Raster>>,
}

impl<'a> Pyramid<'a> {
    pub fn new(base: &'a Raster) -> Self {
        let mut levels = 1;
        let (mut w, mut h) = (base.width, base.height);
        while w > 1 || h > 1 {
            w = w.div_ceil(2);
            h = h.div_ceil(2);
            levels += 1;
        }
        Self {
            base,
            reduced: (1..levels).map(|_| OnceLock::new()).collect(),
        }
    }

    pub fn level_count(&self) -> usize {
        self.reduced.len() + 1
    }

    pub fn level(&self, level: usize) -> &Raster {
        if level == 0 {
            return self.base;
        }
        self.reduced[level - 1].get_or_init(|| halve(self.level(level - 1)))
    }
}

/// Réduit une image de moitié : moyenne de chaque carré de 2 × 2 pixels, sur
/// les valeurs enregistrées (pas en lumière linéaire, voir D-3.4-c). Sur un
/// bord impair, la moyenne ne porte que sur les pixels existants.
fn halve(source: &Raster) -> Raster {
    let c = source.channels as usize;
    let (sw, sh) = (source.width as usize, source.height as usize);
    let (dw, dh) = (sw.div_ceil(2), sh.div_ceil(2));
    let mut data = vec![0u8; dw * dh * c];
    for y in 0..dh {
        let rows = [2 * y, (2 * y + 1).min(sh - 1)];
        let row_count = if 2 * y + 1 < sh { 2 } else { 1 };
        for x in 0..dw {
            let cols = [2 * x, (2 * x + 1).min(sw - 1)];
            let col_count = if 2 * x + 1 < sw { 2 } else { 1 };
            for k in 0..c {
                let mut sum = 0u32;
                for &r in &rows[..row_count] {
                    for &q in &cols[..col_count] {
                        sum += u32::from(source.data[(r * sw + q) * c + k]);
                    }
                }
                let n = (row_count * col_count) as u32;
                data[(y * dw + x) * c + k] = ((sum + n / 2) / n) as u8;
            }
        }
    }
    Raster::new(dw as u32, dh as u32, source.channels, data)
}

/// Calcule un pixel de sortie de `N` canaux : position `(px, py)` en pixels
/// du niveau lu (centres des pixels en `i + 0,5`). Hors de l'image, la source
/// vaut `background`.
#[inline]
pub(super) fn sample<const N: usize>(
    image: &Raster,
    px: f64,
    py: f64,
    footprint: &Footprint,
    background: u8,
    out: &mut [u8],
) {
    let (w, h) = (image.width as i64, image.height as i64);
    let (ex, ey) = footprint.extent;
    let x_lo = (px - ex - 0.5).ceil() as i64;
    let x_hi = (px + ex - 0.5).floor() as i64;
    let y_lo = (py - ey - 0.5).ceil() as i64;
    let y_hi = (py + ey - 0.5).floor() as i64;
    if x_hi < 0 || y_hi < 0 || x_lo >= w || y_lo >= h || !(px.is_finite() && py.is_finite()) {
        out[..N].fill(background);
        return;
    }
    if footprint.interpolating {
        bicubic::<N>(image, px, py, background, out);
        return;
    }
    let [[a00, a01], [a10, a11]] = footprint.inverse;
    let bg = f64::from(background);
    let mut acc = [0.0f64; N];
    let mut total = 0.0f64;
    let data = &image.data;
    for iy in y_lo..=y_hi {
        let dy = iy as f64 + 0.5 - py;
        let dx0 = x_lo as f64 + 0.5 - px;
        let mut u = a00 * dx0 + a01 * dy;
        let mut v = a10 * dx0 + a11 * dy;
        let row_inside = (0..h).contains(&iy);
        let row_start = if row_inside {
            iy as usize * w as usize
        } else {
            0
        };
        for ix in x_lo..=x_hi {
            let weight = catmull_rom(u) * catmull_rom(v);
            u += a00;
            v += a10;
            if weight == 0.0 {
                continue;
            }
            total += weight;
            if row_inside && (0..w).contains(&ix) {
                let base = (row_start + ix as usize) * N;
                for (k, slot) in acc.iter_mut().enumerate() {
                    *slot += weight * f64::from(data[base + k]);
                }
            } else {
                for slot in acc.iter_mut() {
                    *slot += weight * bg;
                }
            }
        }
    }
    if total.abs() < 1e-12 {
        out[..N].fill(background);
        return;
    }
    let norm = 1.0 / total;
    for (slot, value) in out.iter_mut().zip(acc) {
        *slot = (value * norm).round().clamp(0.0, 255.0) as u8;
    }
}

/// Interpolation bicubique de Catmull-Rom : 4 × 4 pixels source, poids
/// séparables. Même résultat que le cas général avec un noyau non étiré.
#[inline]
fn bicubic<const N: usize>(image: &Raster, px: f64, py: f64, background: u8, out: &mut [u8]) {
    let (w, h) = (image.width as i64, image.height as i64);
    // Premier des 4 pixels de chaque côté : son centre est à moins de 2 px.
    let (fx, fy) = ((px - 0.5).floor(), (py - 0.5).floor());
    let (x0, y0) = (fx as i64 - 1, fy as i64 - 1);
    let (tx, ty) = (px - 0.5 - fx, py - 0.5 - fy);
    let weights = |t: f64| {
        [
            catmull_rom(t + 1.0),
            catmull_rom(t),
            catmull_rom(1.0 - t),
            catmull_rom(2.0 - t),
        ]
    };
    let (wx, wy) = (weights(tx), weights(ty));
    let inside = x0 >= 0 && y0 >= 0 && x0 + 3 < w && y0 + 3 < h;
    let bg = f64::from(background);
    let mut acc = [0.0f64; N];
    for (r, wy) in wy.iter().enumerate() {
        let iy = y0 + r as i64;
        let mut row = [0.0f64; N];
        for (q, wx) in wx.iter().enumerate() {
            let ix = x0 + q as i64;
            if inside || (0..w).contains(&ix) && (0..h).contains(&iy) {
                let base = (iy as usize * w as usize + ix as usize) * N;
                for (k, slot) in row.iter_mut().enumerate() {
                    *slot += wx * f64::from(image.data[base + k]);
                }
            } else {
                for slot in row.iter_mut() {
                    *slot += wx * bg;
                }
            }
        }
        for (slot, value) in acc.iter_mut().zip(row) {
            *slot += wy * value;
        }
    }
    for (slot, value) in out.iter_mut().zip(acc) {
        *slot = value.round().clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_interpolates_and_sums_to_one() {
        assert_eq!(catmull_rom(0.0), 1.0);
        for k in [1.0, 2.0, -1.0, -2.0, 2.5] {
            assert_eq!(catmull_rom(k), 0.0);
        }
        for i in 0..20 {
            let t = i as f64 / 20.0;
            let sum: f64 = (-3..=3).map(|k| catmull_rom(t - k as f64)).sum();
            assert!((sum - 1.0).abs() < 1e-12, "somme {sum} en {t}");
        }
    }

    #[test]
    fn identity_reads_one_source_pixel_per_axis() {
        let f = Footprint::from_jacobian(&[[1.0, 0.0], [0.0, 1.0]], 10).unwrap();
        assert_eq!(f.level, 0);
        assert_eq!(f.inverse, [[1.0, 0.0], [0.0, 1.0]]);
        assert_eq!(f.extent, (2.0, 2.0));
    }

    #[test]
    fn enlargement_keeps_the_kernel_on_the_source_grid() {
        // Agrandissement de 4 avec rotation de 30° : le noyau reste celui de
        // l'interpolation bicubique dans la source.
        let (s, c) = (30f64.to_radians().sin(), 30f64.to_radians().cos());
        let j = [[0.25 * c, -0.25 * s], [0.25 * s, 0.25 * c]];
        let f = Footprint::from_jacobian(&j, 10).unwrap();
        assert_eq!(f.level, 0);
        for (row, expected) in f.inverse.iter().zip([[1.0, 0.0], [0.0, 1.0]]) {
            for (v, e) in row.iter().zip(expected) {
                assert!((v - e).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn reduction_picks_a_pyramid_level_and_stretches_the_kernel() {
        // Réduction de 3 : niveau 1 (÷ 2), noyau étiré de 1,5.
        let f = Footprint::from_jacobian(&[[3.0, 0.0], [0.0, 3.0]], 10).unwrap();
        assert_eq!(f.level, 1);
        assert!((f.inverse[0][0] - 1.0 / 1.5).abs() < 1e-12);
        assert!((f.extent.0 - 3.0).abs() < 1e-12);
        // Réduction de 6 dans une seule direction : étirement borné à 4.
        let f = Footprint::from_jacobian(&[[1.0, 0.0], [0.0, 6.0]], 10).unwrap();
        assert_eq!(f.level, 1);
        assert!((f.inverse[1][1] - 1.0 / 3.0).abs() < 1e-12);
        assert!((f.inverse[0][0] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn fast_bicubic_equals_the_general_filter() {
        let data: Vec<u8> = (0..40 * 30)
            .map(|i| ((i * 37 + i / 40 * 11) % 256) as u8)
            .collect();
        let image = Raster::new(40, 30, 1, data);
        let general = Footprint {
            interpolating: false,
            ..INTERPOLATION
        };
        // Intérieur, bords et dehors.
        for (px, py) in [
            (10.3, 7.8),
            (0.2, 0.9),
            (39.7, 29.4),
            (-1.5, 12.0),
            (20.5, 15.5),
        ] {
            let (mut fast, mut slow) = ([0u8], [0u8]);
            sample::<1>(&image, px, py, &INTERPOLATION, 17, &mut fast);
            sample::<1>(&image, px, py, &general, 17, &mut slow);
            assert!(
                fast[0].abs_diff(slow[0]) <= 1,
                "({px}, {py}) : {fast:?} / {slow:?}"
            );
        }
    }

    #[test]
    fn halving_averages_and_handles_odd_edges() {
        let image = Raster::new(3, 1, 1, vec![10, 20, 200]);
        let half = halve(&image);
        assert_eq!((half.width, half.height), (2, 1));
        assert_eq!(half.data, vec![15, 200]);
        let pyramid = Pyramid::new(&image);
        assert_eq!(pyramid.level_count(), 3);
        assert_eq!(pyramid.level(2).data, vec![108]);
    }
}

//! Fabrication de l'image redressée : chaque pixel de la diapositive va
//! chercher sa couleur dans la photo brute, par la déformation mm → px.

use image::{Rgb, RgbImage};

use crate::geometry::{GridWarp, Point, Transform};
use crate::units::SLIDE_SIZE_MM;

/// Poids de l'interpolation cubique de Catmull-Rom (a = -0,5).
fn cubic_weights(t: f32) -> [f32; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        -0.5 * t3 + t2 - 0.5 * t,
        1.5 * t3 - 2.5 * t2 + 1.0,
        -1.5 * t3 + 2.0 * t2 + 0.5 * t,
        0.5 * t3 - 0.5 * t2,
    ]
}

/// Couleur de la photo au point `(x, y)` (coordonnées continues), par
/// interpolation bicubique, bords répétés. Noir hors de la photo.
fn sample_bicubic(photo: &RgbImage, x: f64, y: f64) -> Rgb<u8> {
    let (w, h) = (photo.width() as i64, photo.height() as i64);
    if !(x >= 0.0 && y >= 0.0 && x <= w as f64 && y <= h as f64) {
        return Rgb([0, 0, 0]);
    }
    let u = x - 0.5;
    let v = y - 0.5;
    let i0 = u.floor();
    let j0 = v.floor();
    let wx = cubic_weights((u - i0) as f32);
    let wy = cubic_weights((v - j0) as f32);
    let (i0, j0) = (i0 as i64, j0 as i64);
    let mut acc = [0.0f32; 3];
    for (dj, wyj) in wy.iter().enumerate() {
        let jj = (j0 - 1 + dj as i64).clamp(0, h - 1) as u32;
        let mut row = [0.0f32; 3];
        for (di, wxi) in wx.iter().enumerate() {
            let ii = (i0 - 1 + di as i64).clamp(0, w - 1) as u32;
            let p = photo.get_pixel(ii, jj).0;
            for c in 0..3 {
                row[c] += wxi * f32::from(p[c]);
            }
        }
        for c in 0..3 {
            acc[c] += wyj * row[c];
        }
    }
    Rgb(acc.map(|v| v.round().clamp(0.0, 255.0) as u8))
}

/// Image redressée de `side_px` pixels de côté, couvrant les 280 mm de la
/// diapositive. `warp` va des mm de diapositive vers les px de la photo.
/// Le calcul est réparti sur tous les cœurs.
pub(crate) fn render(photo: &RgbImage, warp: &GridWarp, side_px: u32) -> RgbImage {
    let side = side_px as usize;
    let mm_per_px = SLIDE_SIZE_MM / side_px as f64;
    let mut out = vec![0u8; side * side * 3];
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(16);
    let rows_per_chunk = side.div_ceil(threads * 4).max(1);
    let chunks: Vec<(usize, &mut [u8])> = out
        .chunks_mut(rows_per_chunk * side * 3)
        .enumerate()
        .collect();
    let queue = std::sync::Mutex::new(chunks);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let next = queue.lock().expect("file de travail").pop();
                let Some((k, chunk)) = next else {
                    break;
                };
                let first_row = k * rows_per_chunk;
                for (r, row) in chunk.chunks_mut(side * 3).enumerate() {
                    let y = (first_row + r) as f64 + 0.5;
                    for (i, px) in row.chunks_mut(3).enumerate() {
                        let p = Point::new((i as f64 + 0.5) * mm_per_px, y * mm_per_px);
                        let colour = warp
                            .apply(p)
                            .map_or(Rgb([0, 0, 0]), |q| sample_bicubic(photo, q.x, q.y));
                        px.copy_from_slice(&colour.0);
                    }
                }
            });
        }
    });
    RgbImage::from_raw(side_px, side_px, out).expect("taille cohérente")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Homography, Rect};

    #[test]
    fn cubic_weights_sum_to_one_and_interpolate() {
        for t in [0.0f32, 0.25, 0.5, 0.9] {
            let w = cubic_weights(t);
            assert!((w.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        }
        assert_eq!(cubic_weights(0.0), [0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn an_identity_warp_copies_the_photo() {
        let mut photo = RgbImage::new(28, 28);
        for (x, y, p) in photo.enumerate_pixels_mut() {
            *p = Rgb([(x * 9) as u8, (y * 9) as u8, 77]);
        }
        // 10 mm par pixel de photo : 28 px couvrent 280 mm.
        let base = Homography::similarity(0.1, 0.0, 0.0, 0.0).unwrap();
        let region = Rect::new(Point::new(0.0, 0.0), Point::new(280.0, 280.0)).unwrap();
        let warp = GridWarp::sample(base, region, 3, 3, |_| Point::new(0.0, 0.0)).unwrap();
        let out = render(&photo, &warp, 28);
        assert_eq!(out, photo);
    }
}

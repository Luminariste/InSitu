//! Masque général d'une diapositive (demande de Benjamin du 14 novembre
//! 2026, décision D-3.6-h).
//!
//! Des formes tracées sur la photo redressée, en mm de diapositive (avant
//! miroir). À l'export, ce qui est dedans (ou dehors) devient noir, quels
//! que soient la fresque et les dessins : dans le PNG, pixel par pixel ;
//! dans le SVG, en découpant les contours.

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

use super::svg::{PathMm, SlidePaths};
use crate::geometry::Point;
use crate::placement::mask::union_even_odd;
use crate::project::SlideMask;
use crate::tiles::Raster;
use crate::units::SLIDE_SIZE_MM;
use crate::vectorize::Segment;

/// Les formes du masque dans la diapositive telle qu'elle est gravée (le
/// miroir appliqué), réunies.
fn region(mask: &SlideMask, mirror: bool) -> Vec<Vec<[f64; 2]>> {
    let shapes: Vec<Vec<[f64; 2]>> = mask
        .shapes
        .iter()
        .map(|s| {
            s.iter()
                .map(|&[x, y]| [if mirror { SLIDE_SIZE_MM - x } else { x }, y])
                .collect()
        })
        .collect();
    union_even_odd(&shapes)
}

/// Noircit, dans l'image de la diapositive (`side × side`, sans la marge,
/// pixels de `px_mm` mm, déjà retournée si `mirror`), ce que cache le masque.
pub fn apply(slide: &mut Raster, mask: &SlideMask, px_mm: f64, mirror: bool) {
    if !mask.active() {
        return;
    }
    let (w, h) = (slide.width as usize, slide.height as usize);
    let channels = slide.channels as usize;
    let contours: Vec<Vec<(f64, f64)>> = region(mask, mirror)
        .iter()
        .map(|c| c.iter().map(|p| (p[0] / px_mm, p[1] / px_mm)).collect())
        .collect();
    let mut crossings: Vec<f64> = Vec::new();
    for j in 0..h {
        let y = j as f64 + 0.5;
        crossings.clear();
        for c in &contours {
            let n = c.len();
            for k in 0..n {
                let (a, b) = (c[k], c[(k + 1) % n]);
                if (a.1 <= y) != (b.1 <= y) {
                    crossings.push(a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
        }
        crossings.sort_by(f64::total_cmp);
        // Les pixels dont le centre est dans le masque (pair-impair).
        let column = |x: f64| ((x - 0.5).ceil().clamp(0.0, w as f64)) as usize;
        let row = &mut slide.data[j * w * channels..(j + 1) * w * channels];
        let mut black = |i0: usize, i1: usize| {
            if i1 > i0 {
                row[i0 * channels..i1 * channels].fill(0);
            }
        };
        if mask.outside {
            let mut from = 0;
            for &[a, b] in crossings.as_chunks::<2>().0 {
                black(from, column(a));
                from = column(b);
            }
            black(from, w);
        } else {
            for &[a, b] in crossings.as_chunks::<2>().0 {
                black(column(a), column(b));
            }
        }
    }
}

/// Les contours de chaque dessin du SVG, moins ce que cache le masque.
pub fn clip(drawings: Vec<SlidePaths>, mask: &SlideMask, mirror: bool) -> Vec<SlidePaths> {
    if !mask.active() {
        return drawings;
    }
    let region = region(mask, mirror);
    let rule = if mask.outside {
        OverlayRule::Intersect
    } else {
        OverlayRule::Difference
    };
    drawings
        .into_iter()
        .map(|d| {
            let subject: Vec<Vec<[f64; 2]>> = d
                .paths
                .iter()
                .map(|p| {
                    super::svg::flatten(p, super::svg::TOLERANCE_MM)
                        .iter()
                        .map(|q| [q.x, q.y])
                        .collect()
                })
                .filter(|c: &Vec<[f64; 2]>| c.len() >= 3)
                .collect();
            // Un dessin découpé est rempli pair-impair (contours en segments) ;
            // un dessin entier, non nul : on garde sa règle pour le lire.
            let fill = if d.clipped {
                FillRule::EvenOdd
            } else {
                FillRule::NonZero
            };
            let shapes = if region.is_empty() {
                if mask.outside {
                    Vec::new()
                } else {
                    return d;
                }
            } else {
                subject.overlay(&region, rule, fill)
            };
            let paths = shapes
                .into_iter()
                .flatten()
                .filter(|c| c.len() >= 3)
                .map(|c| PathMm {
                    start: Point::new(c[0][0], c[0][1]),
                    segments: c[1..]
                        .iter()
                        .chain(std::iter::once(&c[0]))
                        .map(|q| Segment::Line {
                            to: Point::new(q[0], q[1]),
                        })
                        .collect(),
                })
                .collect();
            SlidePaths {
                placement: d.placement,
                paths,
                clipped: true,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(shapes: Vec<Vec<[f64; 2]>>, outside: bool) -> SlideMask {
        SlideMask {
            shapes,
            outside,
            hidden: false,
            extra: Default::default(),
        }
    }

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
        vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
    }

    #[test]
    fn inside_turns_black_and_outside_stays() {
        let mut slide = Raster::new(28, 28, 1, vec![255; 28 * 28]);
        // Pixels de 10 mm : le carré 0–140 mm couvre 14 × 14 pixels.
        apply(
            &mut slide,
            &mask(vec![square(0.0, 0.0, 140.0, 140.0)], false),
            10.0,
            false,
        );
        let black = slide.data.iter().filter(|&&v| v == 0).count();
        assert_eq!(black, 14 * 14);
        assert_eq!(slide.data[0], 0);
        assert_eq!(slide.data[27], 255);
    }

    #[test]
    fn outside_keeps_only_the_shape_and_follows_the_mirror() {
        let mut slide = Raster::new(28, 28, 1, vec![255; 28 * 28]);
        apply(
            &mut slide,
            &mask(vec![square(0.0, 0.0, 140.0, 140.0)], true),
            10.0,
            true,
        );
        let white = slide.data.iter().filter(|&&v| v == 255).count();
        assert_eq!(white, 14 * 14);
        // Retournée : la forme de gauche est gravée à droite.
        assert_eq!(slide.data[0], 0);
        assert_eq!(slide.data[27], 255);
    }

    #[test]
    fn a_hidden_mask_changes_nothing() {
        let mut slide = Raster::new(28, 28, 1, vec![255; 28 * 28]);
        let mut m = mask(vec![square(0.0, 0.0, 140.0, 140.0)], false);
        m.hidden = true;
        apply(&mut slide, &m, 10.0, false);
        assert!(slide.data.iter().all(|&v| v == 255));
    }

    #[test]
    fn the_svg_is_cut_like_the_png() {
        let path = PathMm {
            start: Point::new(100.0, 100.0),
            segments: vec![
                Segment::Line {
                    to: Point::new(200.0, 100.0),
                },
                Segment::Line {
                    to: Point::new(200.0, 200.0),
                },
                Segment::Line {
                    to: Point::new(100.0, 200.0),
                },
                Segment::Line {
                    to: Point::new(100.0, 100.0),
                },
            ],
        };
        let drawing = || SlidePaths {
            placement: "p".into(),
            paths: vec![path.clone()],
            clipped: false,
        };
        let area = |d: &SlidePaths| -> f64 {
            d.paths
                .iter()
                .map(|p| {
                    let pts = super::super::svg::flatten(p, 0.01);
                    let n = pts.len();
                    (0..n)
                        .map(|k| pts[k].x * pts[(k + 1) % n].y - pts[(k + 1) % n].x * pts[k].y)
                        .sum::<f64>()
                        / 2.0
                })
                .sum::<f64>()
                .abs()
        };
        let left = vec![square(0.0, 0.0, 150.0, 280.0)];
        let inside = clip(vec![drawing()], &mask(left.clone(), false), false);
        assert!((area(&inside[0]) - 5_000.0).abs() < 1e-6);
        let outside = clip(vec![drawing()], &mask(left, true), false);
        assert!((area(&outside[0]) - 5_000.0).abs() < 1e-6);
        // Le masque à gauche, retourné, tombe à droite : 130–280 mm, qui
        // couvre 130–200 mm du carré.
        let mirrored = clip(
            vec![drawing()],
            &mask(vec![square(0.0, 0.0, 150.0, 280.0)], false),
            true,
        );
        assert!((area(&mirrored[0]) - 3_000.0).abs() < 1e-6);
    }
}

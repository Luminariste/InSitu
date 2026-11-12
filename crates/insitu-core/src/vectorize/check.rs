//! Mesure indépendante de l'écart entre un dessin vectorisé et son image
//! source (critère de la tâche 5.1 : moins d'un demi-pixel).
//!
//! Deux mesures, qui ne réutilisent rien de l'ajustement :
//!
//! - **écart géométrique** : distance de Hausdorff entre les courbes et le
//!   contour brut de l'image (la ligne de niveau du seuil), dans les deux
//!   sens. Les courbes sont aplaties à 0,005 px près et parcourues tous les
//!   0,25 px ; le contour brut est pris à ses sommets et aux milieux de ses
//!   côtés ;
//! - **pixels** : les courbes sont remplies (règle « nonzero ») au centre de
//!   chaque pixel et comparées à l'image seuillée. Un pixel qui diffère n'est
//!   acceptable que s'il touche presque la courbe : sa distance à la courbe
//!   est relevée.

use serde::Serialize;

use super::contour::{self, Field};
use super::fit::{distance_to_segment, Segment};
use super::{GrayImage, Vectorization};
use crate::geometry::Point;

/// Résultat de [`compare`]. Distances en pixels de l'image source.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    /// Plus grand écart entre les courbes et le contour brut, dans un sens ou
    /// dans l'autre : le chiffre du critère.
    pub max_deviation_px: f64,
    /// Du contour brut vers les courbes.
    pub raw_to_vector_px: f64,
    /// Des courbes vers le contour brut.
    pub vector_to_raw_px: f64,
    /// Pixels dans le trait d'après l'image seuillée.
    pub source_pixels: u64,
    /// Pixels dans le trait d'après les courbes remplies.
    pub vector_pixels: u64,
    /// Pixels classés différemment.
    pub mismatched_pixels: u64,
    /// Plus grande distance d'un pixel mal classé à la courbe.
    pub max_mismatch_distance_px: f64,
}

/// Compare `vector` à l'image dont il est tiré.
pub fn compare(image: &GrayImage, vector: &Vectorization) -> Comparison {
    let field = Field {
        width: image.width as usize,
        height: image.height as usize,
        data: &image.data,
        threshold: vector.options.threshold,
    };
    let raw = contour::trace(&field);
    let mut raw_segments = Vec::new();
    for points in &raw {
        let n = points.len();
        for k in 0..n {
            raw_segments.push((points[k], points[(k + 1) % n]));
        }
    }
    drop(raw);
    let edges = flatten(vector);
    let (w, h) = (image.width as f64, image.height as f64);

    let vector_grid = SegmentGrid::new(edges.iter().map(|e| (e.0, e.1)).collect(), w, h);
    let raw_to_vector = parallel_max(&raw_segments, |&(p, q)| {
        vector_grid
            .nearest(p)
            .max(vector_grid.nearest((p + q) * 0.5))
    });
    let raw_grid = SegmentGrid::new(raw_segments, w, h);
    let vector_to_raw = parallel_max(&edges, |&(a, b, _)| {
        let steps = (a.distance(b) / 0.25).ceil().max(1.0) as usize;
        (0..steps)
            .map(|s| raw_grid.nearest(a + (b - a) * (s as f64 / steps as f64)))
            .fold(0.0, f64::max)
    });
    drop(raw_grid);

    let pixels = compare_pixels(image, vector.options.threshold, &edges, &vector_grid);
    Comparison {
        max_deviation_px: raw_to_vector.max(vector_to_raw),
        raw_to_vector_px: raw_to_vector,
        vector_to_raw_px: vector_to_raw,
        ..pixels
    }
}

/// Plus grande valeur de `f` sur les éléments, calculée sur tous les cœurs.
fn parallel_max<T: Sync>(items: &[T], f: impl Fn(&T) -> f64 + Sync) -> f64 {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let chunk = items.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(chunk)
            .map(|part| scope.spawn(|| part.iter().map(&f).fold(0.0, f64::max)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("fil de mesure"))
            .fold(0.0, f64::max)
    })
}

/// Côtés des courbes aplaties, avec leur sens vertical (+1 vers le bas).
fn flatten(vector: &Vectorization) -> Vec<(Point, Point, i32)> {
    const FLATNESS_PX: f64 = 0.005;
    let mut edges = Vec::new();
    let mut push = |a: Point, b: Point| {
        if a != b {
            let dir = if b.y > a.y { 1 } else { -1 };
            edges.push((a, b, dir));
        }
    };
    for contour in &vector.contours {
        let mut current = contour.start;
        for segment in &contour.segments {
            match *segment {
                Segment::Line { to } => {
                    push(current, to);
                    current = to;
                }
                Segment::Cubic { c1, c2, to } => {
                    let p0 = current;
                    let dd = (p0 - c1 * 2.0 + c2).norm().max((c1 - c2 * 2.0 + to).norm());
                    let n = (0.75 * dd / FLATNESS_PX).sqrt().ceil().clamp(1.0, 4096.0) as usize;
                    let mut prev = p0;
                    for s in 1..=n {
                        let t = s as f64 / n as f64;
                        let u = 1.0 - t;
                        let p = p0 * (u * u * u)
                            + c1 * (3.0 * u * u * t)
                            + c2 * (3.0 * u * t * t)
                            + to * (t * t * t);
                        push(prev, p);
                        prev = p;
                    }
                    current = to;
                }
            }
        }
        push(current, contour.start);
    }
    edges
}

fn compare_pixels(
    image: &GrayImage,
    threshold: u8,
    edges: &[(Point, Point, i32)],
    grid: &SegmentGrid,
) -> Comparison {
    let (w, h) = (image.width as usize, image.height as usize);
    // Chaque côté est rangé à la première ligne de centres qu'il traverse
    // (y = j + 0,5 dans [ymin, ymax[).
    let row_of = |y: f64| (y - 0.5).ceil();
    let mut starts: Vec<Vec<usize>> = vec![Vec::new(); h];
    for (k, &(a, b, _)) in edges.iter().enumerate() {
        let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
        let first = row_of(y0).max(0.0);
        if first < h as f64 && first < row_of(y1) {
            starts[first as usize].push(k);
        }
    }
    let mut active: Vec<usize> = Vec::new();
    let mut crossings: Vec<(f64, i32)> = Vec::new();
    let mut result = Comparison {
        max_deviation_px: 0.0,
        raw_to_vector_px: 0.0,
        vector_to_raw_px: 0.0,
        source_pixels: 0,
        vector_pixels: 0,
        mismatched_pixels: 0,
        max_mismatch_distance_px: 0.0,
    };
    for (j, starting) in starts.iter().enumerate() {
        let y = j as f64 + 0.5;
        active.extend(starting);
        active.retain(|&k| {
            let (a, b, _) = edges[k];
            a.y.max(b.y) > y
        });
        crossings.clear();
        for &k in &active {
            let (a, b, dir) = edges[k];
            let t = (y - a.y) / (b.y - a.y);
            crossings.push((a.x + t * (b.x - a.x), dir));
        }
        crossings.sort_by(|p, q| p.0.total_cmp(&q.0));
        let row = &image.data[j * w..(j + 1) * w];
        let mut winding = 0;
        let mut next = 0;
        for (i, &value) in row.iter().enumerate() {
            let x = i as f64 + 0.5;
            while next < crossings.len() && crossings[next].0 < x {
                winding += crossings[next].1;
                next += 1;
            }
            let in_vector = winding != 0;
            let in_source = value >= threshold;
            result.source_pixels += u64::from(in_source);
            result.vector_pixels += u64::from(in_vector);
            if in_vector != in_source {
                result.mismatched_pixels += 1;
                let d = grid.nearest(Point::new(x, y));
                result.max_mismatch_distance_px = result.max_mismatch_distance_px.max(d);
            }
        }
    }
    result
}

/// Segments rangés par cases carrées, pour trouver vite le plus proche.
struct SegmentGrid {
    segments: Vec<(Point, Point)>,
    cols: usize,
    rows: usize,
    offsets: Vec<u32>,
    items: Vec<u32>,
}

const CELL_PX: f64 = 4.0;
/// Au-delà, la distance est déclarée infinie : c'est déjà un échec franc.
const MAX_RING: usize = 8;

impl SegmentGrid {
    fn new(segments: Vec<(Point, Point)>, width: f64, height: f64) -> Self {
        let cols = ((width + 2.0) / CELL_PX).ceil() as usize + 1;
        let rows = ((height + 2.0) / CELL_PX).ceil() as usize + 1;
        let mut grid = Self {
            segments,
            cols,
            rows,
            offsets: vec![0; cols * rows + 1],
            items: Vec::new(),
        };
        for k in 0..grid.segments.len() {
            grid.for_cells(k, |grid, cell| grid.offsets[cell + 1] += 1);
        }
        for c in 0..cols * rows {
            grid.offsets[c + 1] += grid.offsets[c];
        }
        let mut fill = grid.offsets.clone();
        grid.items = vec![0; grid.offsets[cols * rows] as usize];
        for k in 0..grid.segments.len() {
            let mut cells = Vec::new();
            grid.for_cells(k, |_, cell| cells.push(cell));
            for cell in cells {
                grid.items[fill[cell] as usize] = k as u32;
                fill[cell] += 1;
            }
        }
        grid
    }

    fn cell_of(&self, p: Point) -> (usize, usize) {
        let cx = ((p.x + 1.0) / CELL_PX)
            .floor()
            .clamp(0.0, (self.cols - 1) as f64);
        let cy = ((p.y + 1.0) / CELL_PX)
            .floor()
            .clamp(0.0, (self.rows - 1) as f64);
        (cx as usize, cy as usize)
    }

    /// Cases que touche le segment `k` (au moins) : il est découpé en
    /// morceaux d'un quart de case, et chaque morceau est inscrit dans toutes
    /// les cases de son rectangle englobant.
    fn for_cells(&mut self, k: usize, mut f: impl FnMut(&mut Self, usize)) {
        let (a, b) = self.segments[k];
        let steps = (a.distance(b) / (CELL_PX / 4.0)).ceil().max(1.0) as usize;
        let mut last = (usize::MAX, usize::MAX, usize::MAX, usize::MAX);
        for s in 0..steps {
            let p = a + (b - a) * (s as f64 / steps as f64);
            let q = a + (b - a) * ((s + 1) as f64 / steps as f64);
            let (x0, y0) = self.cell_of(Point::new(p.x.min(q.x), p.y.min(q.y)));
            let (x1, y1) = self.cell_of(Point::new(p.x.max(q.x), p.y.max(q.y)));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    // Un même segment n'est pas inscrit deux fois de suite
                    // dans la même case.
                    if (last.0..=last.2).contains(&x) && (last.1..=last.3).contains(&y) {
                        continue;
                    }
                    f(self, y * self.cols + x);
                }
            }
            last = (x0, y0, x1, y1);
        }
    }

    /// Distance du point au segment le plus proche.
    fn nearest(&self, p: Point) -> f64 {
        let (cx, cy) = self.cell_of(p);
        // Distance garantie couverte par la case du point : jusqu'à son bord.
        let fx = (p.x + 1.0) / CELL_PX - cx as f64;
        let fy = (p.y + 1.0) / CELL_PX - cy as f64;
        let margin = fx.min(1.0 - fx).min(fy).min(1.0 - fy).max(0.0) * CELL_PX;
        let mut best = f64::INFINITY;
        for ring in 0..=MAX_RING {
            let (x0, x1) = (cx.saturating_sub(ring), (cx + ring).min(self.cols - 1));
            let (y0, y1) = (cy.saturating_sub(ring), (cy + ring).min(self.rows - 1));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if x.abs_diff(cx).max(y.abs_diff(cy)) != ring {
                        continue;
                    }
                    let cell = y * self.cols + x;
                    let range = self.offsets[cell] as usize..self.offsets[cell + 1] as usize;
                    for &k in &self.items[range] {
                        let (a, b) = self.segments[k as usize];
                        best = best.min(distance_to_segment(p, a, b));
                    }
                }
            }
            // Les cases jusqu'à l'anneau `ring` couvrent au moins le disque
            // de ce rayon autour du point : rien de plus proche ailleurs.
            if best <= margin + ring as f64 * CELL_PX {
                return best;
            }
        }
        best
    }
}

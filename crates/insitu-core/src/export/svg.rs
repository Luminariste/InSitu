//! Export SVG des diapositives (tâche 5.3).
//!
//! Les courbes de chaque dessin posé (vectorisé comme en bibliothèque,
//! tâche 5.2) sont transportées vers la diapositive sans repasser par des
//! pixels : position dans le dessin → façade (placement, déformation) →
//! diapositive (inverse du recalage) → retouche de la diapositive (4.6) →
//! miroir. C'est la chaîne de l'export PNG, prise dans l'autre sens.
//!
//! La chaîne n'est pas une transformation simple : chaque morceau est
//! transporté par ses extrémités, et ses points de contrôle suivent la
//! dérivée de la chaîne aux extrémités. Le morceau transporté est comparé à
//! des points du morceau d'origine transportés un par un ; s'il s'en écarte
//! de plus de [`TOLERANCE_MM`], il est coupé en deux et chaque moitié est
//! reprise.
//!
//! Silhouettes (tâche 4.7) : un dessin placé derrière un autre est découpé
//! par la silhouette de celui de devant (différence de polygones). Les
//! courbes d'un dessin découpé deviennent des segments, à la même
//! tolérance.
//!
//! Le SVG est en millimètres : 280 mm de côté, ou 300 mm avec la marge de
//! la plaque (les repères et le nom gravés dans la marge ne sont que dans le
//! PNG). Les traits sont des surfaces remplies de noir, sans fond.

use std::fmt::Write as _;

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

use crate::geometry::{Point, PointWarp};
use crate::placement::mesh;
use crate::project::{Placement, PointMm, SilhouettePolygon};
use crate::units::{PLATE_MARGIN_MM, SLIDE_SIZE_MM};
use crate::vectorize::{Segment, Vectorization};

/// Écart permis entre une courbe transportée et la chaîne exacte, en mm de
/// diapositive (le critère de la tâche est 0,02 mm).
pub const TOLERANCE_MM: f64 = 0.002;

/// Coupures au plus d'un morceau (2^16 morceaux).
const MAX_DEPTH: u32 = 16;

/// Un contour fermé, en mm de diapositive.
#[derive(Debug, Clone, PartialEq)]
pub struct PathMm {
    pub start: Point,
    pub segments: Vec<Segment>,
}

/// Le dessin d'une diapositive : ses contours, et la règle de remplissage.
#[derive(Debug, Clone, PartialEq)]
pub struct SlidePaths {
    pub placement: String,
    pub paths: Vec<PathMm>,
    /// Vrai si le dessin a été découpé par une silhouette (contours en
    /// segments, remplis « pair-impair »).
    pub clipped: bool,
}

/// Où l'on pose les courbes : le recalage du projecteur, le miroir, et le
/// projecteur (pour ses retouches).
pub struct SvgPlacing<'a> {
    /// Aucun : seuls les dessins posés dans la diapositive (D-3.6-g).
    pub warp: Option<&'a PointWarp>,
    pub mirror: bool,
    pub projector: Option<&'a str>,
}

/// La chaîne d'un point du dessin (fractions `u`, `v`) à la diapositive,
/// en mm.
pub fn slide_point(placement: &Placement, place: &SvgPlacing, u: f64, v: f64) -> Option<Point> {
    let f = placement.facade_point(u, v);
    let s = crate::placement::to_slide(placement, place.warp, Point::new(f.x, f.y))?;
    let s = match place.projector.and_then(|id| placement.adjustment_for(id)) {
        Some(d) => {
            let q = mesh::forward(d, PointMm::new(s.x, s.y));
            Point::new(q.x, q.y)
        }
        None => s,
    };
    let x = if place.mirror {
        SLIDE_SIZE_MM - s.x
    } else {
        s.x
    };
    (x.is_finite() && s.y.is_finite()).then_some(Point::new(x, s.y))
}

// ---------------------------------------------------------------------------
// Transport des courbes
// ---------------------------------------------------------------------------

fn lerp(a: Point, b: Point, t: f64) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

fn cubic_at(p0: Point, c1: Point, c2: Point, p3: Point, t: f64) -> Point {
    let a = lerp(p0, c1, t);
    let b = lerp(c1, c2, t);
    let c = lerp(c2, p3, t);
    lerp(lerp(a, b, t), lerp(b, c, t), t)
}

/// Les deux moitiés d'une cubique (de Casteljau).
type Cubic = (Point, Point, Point, Point);
fn split_cubic(p0: Point, c1: Point, c2: Point, p3: Point) -> (Cubic, Cubic) {
    let a = lerp(p0, c1, 0.5);
    let b = lerp(c1, c2, 0.5);
    let c = lerp(c2, p3, 0.5);
    let d = lerp(a, b, 0.5);
    let e = lerp(b, c, 0.5);
    let m = lerp(d, e, 0.5);
    ((p0, a, d, m), (m, e, c, p3))
}

/// Transporte les contours d'un dessin. `map` envoie un point du dessin
/// (pixels de l'image vectorisée) dans la diapositive (mm) ; un contour
/// dont un point n'a pas d'image est laissé de côté.
pub struct Transporter<F: Fn(Point) -> Option<Point>> {
    pub map: F,
    pub tolerance: f64,
    /// Pas des dérivées, en unités du dessin.
    pub step: f64,
}

impl<F: Fn(Point) -> Option<Point>> Transporter<F> {
    /// Dérivée de `map` en `p`, appliquée au vecteur `d`.
    fn push(&self, p: Point, d: Point) -> Option<Point> {
        let h = self.step;
        let ax = (self.map)(Point::new(p.x + h, p.y))?;
        let bx = (self.map)(Point::new(p.x - h, p.y))?;
        let ay = (self.map)(Point::new(p.x, p.y + h))?;
        let by = (self.map)(Point::new(p.x, p.y - h))?;
        let jx = Point::new((ax.x - bx.x) / (2.0 * h), (ax.y - bx.y) / (2.0 * h));
        let jy = Point::new((ay.x - by.x) / (2.0 * h), (ay.y - by.y) / (2.0 * h));
        Some(Point::new(jx.x * d.x + jy.x * d.y, jx.y * d.x + jy.y * d.y))
    }

    pub fn contour(&self, start: Point, segments: &[Segment]) -> Option<PathMm> {
        let mut out = Vec::with_capacity(segments.len());
        let first = (self.map)(start)?;
        let mut p = start;
        let mut q = first;
        for segment in segments {
            match *segment {
                Segment::Line { to } => {
                    let end = (self.map)(to)?;
                    self.line(p, to, q, end, 0, &mut out)?;
                    q = end;
                }
                Segment::Cubic { c1, c2, to } => {
                    let end = (self.map)(to)?;
                    self.cubic((p, c1, c2, to), q, end, 0, &mut out)?;
                    q = end;
                }
            }
            p = segment.end();
        }
        Some(PathMm {
            start: first,
            segments: out,
        })
    }

    fn line(
        &self,
        a: Point,
        b: Point,
        qa: Point,
        qb: Point,
        depth: u32,
        out: &mut Vec<Segment>,
    ) -> Option<()> {
        let mid = lerp(a, b, 0.5);
        let qm = (self.map)(mid)?;
        if depth >= MAX_DEPTH || qm.distance(lerp(qa, qb, 0.5)) <= self.tolerance {
            out.push(Segment::Line { to: qb });
            return Some(());
        }
        self.line(a, mid, qa, qm, depth + 1, out)?;
        self.line(mid, b, qm, qb, depth + 1, out)
    }

    fn cubic(
        &self,
        (p0, c1, c2, p3): Cubic,
        q0: Point,
        q3: Point,
        depth: u32,
        out: &mut Vec<Segment>,
    ) -> Option<()> {
        let d1 = self.push(p0, c1 - p0)?;
        let d2 = self.push(p3, c2 - p3)?;
        let (k1, k2) = (q0 + d1, q3 + d2);
        let fits = depth >= MAX_DEPTH
            || [0.25, 0.5, 0.75].iter().all(|&t| {
                (self.map)(cubic_at(p0, c1, c2, p3, t)).is_some_and(|exact| {
                    exact.distance(cubic_at(q0, k1, k2, q3, t)) <= self.tolerance
                })
            });
        if fits {
            out.push(Segment::Cubic {
                c1: k1,
                c2: k2,
                to: q3,
            });
            return Some(());
        }
        let (left, right) = split_cubic(p0, c1, c2, p3);
        let qm = (self.map)(left.3)?;
        self.cubic(left, q0, qm, depth + 1, out)?;
        self.cubic(right, qm, q3, depth + 1, out)
    }

    /// Un polygone du dessin (fractions) transporté en suivant ses bords :
    /// des points, assez serrés pour rester à la tolérance.
    pub fn polygon(&self, points: &[Point]) -> Option<Vec<Point>> {
        let n = points.len();
        let mut out = Vec::new();
        for k in 0..n {
            let (a, b) = (points[k], points[(k + 1) % n]);
            let (qa, qb) = ((self.map)(a)?, (self.map)(b)?);
            out.push(qa);
            let mut segs = Vec::new();
            self.line(a, b, qa, qb, 0, &mut segs)?;
            for s in &segs[..segs.len().saturating_sub(1)] {
                out.push(s.end());
            }
        }
        Some(out)
    }
}

/// Points d'un contour aplati à `tolerance` près (les cubiques coupées en
/// deux jusqu'à être presque droites).
pub fn flatten(path: &PathMm, tolerance: f64) -> Vec<Point> {
    let mut out = vec![path.start];
    let mut p = path.start;
    for segment in &path.segments {
        match *segment {
            Segment::Line { to } => out.push(to),
            Segment::Cubic { c1, c2, to } => flatten_cubic((p, c1, c2, to), tolerance, 0, &mut out),
        }
        p = segment.end();
    }
    if out.len() > 1 && out.last() == out.first() {
        out.pop();
    }
    out
}

fn flatten_cubic(c: Cubic, tolerance: f64, depth: u32, out: &mut Vec<Point>) {
    let (p0, c1, c2, p3) = c;
    // Écart des points de contrôle à la corde : borne de l'écart de la courbe.
    let flat = crate::vectorize::fit::distance_to_segment(c1, p0, p3)
        .max(crate::vectorize::fit::distance_to_segment(c2, p0, p3));
    if depth >= MAX_DEPTH || flat <= tolerance {
        out.push(p3);
        return;
    }
    let (left, right) = split_cubic(p0, c1, c2, p3);
    flatten_cubic(left, tolerance, depth + 1, out);
    flatten_cubic(right, tolerance, depth + 1, out);
}

fn signed_area(points: &[Point]) -> f64 {
    let n = points.len();
    (0..n)
        .map(|k| {
            let (p, q) = (points[k], points[(k + 1) % n]);
            p.x * q.y - q.x * p.y
        })
        .sum::<f64>()
        / 2.0
}

fn bounds(points: impl Iterator<Item = Point>) -> Option<[f64; 4]> {
    points.fold(None, |b, p| {
        let [x0, y0, x1, y1] = b.unwrap_or([p.x, p.y, p.x, p.y]);
        Some([x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y)])
    })
}

fn overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

// ---------------------------------------------------------------------------
// Une diapositive
// ---------------------------------------------------------------------------

/// Un dessin à poser : son placement, ses courbes, sa silhouette.
pub struct VectorDrawing<'a> {
    pub placement: &'a Placement,
    pub vector: &'a Vectorization,
    pub silhouette: &'a [SilhouettePolygon],
}

/// Les contours de chaque dessin (du fond vers l'avant) dans la
/// diapositive, découpés par les silhouettes des dessins de devant.
pub fn slide_paths(
    drawings: &[VectorDrawing],
    place: &SvgPlacing,
    threads: usize,
) -> Vec<SlidePaths> {
    // Courbes transportées, et silhouettes (polygones orientés dans le même
    // sens, pour que leur réunion se remplisse « non nul »).
    let transported: Vec<(Vec<PathMm>, Vec<Vec<Point>>)> = drawings
        .iter()
        .map(|d| {
            let (w, h) = (f64::from(d.vector.width), f64::from(d.vector.height));
            let t = Transporter {
                map: |p: Point| slide_point(d.placement, place, p.x / w, p.y / h),
                tolerance: TOLERANCE_MM,
                step: 0.5,
            };
            let paths = transport_all(&t, d.vector, threads);
            let s = Transporter {
                map: |p: Point| slide_point(d.placement, place, p.x, p.y),
                tolerance: TOLERANCE_MM,
                step: 0.5 / w.max(h),
            };
            let silhouettes = d
                .silhouette
                .iter()
                .filter_map(|poly| {
                    let pts: Vec<Point> = poly.iter().map(|&[u, v]| Point::new(u, v)).collect();
                    let mut out = s.polygon(&pts)?;
                    if signed_area(&out) < 0.0 {
                        out.reverse();
                    }
                    Some(out)
                })
                .collect();
            (paths, silhouettes)
        })
        .collect();
    let silhouette_bounds: Vec<Vec<[f64; 4]>> = transported
        .iter()
        .map(|(_, s)| s.iter().filter_map(|p| bounds(p.iter().copied())).collect())
        .collect();
    transported
        .iter()
        .enumerate()
        .map(|(i, (paths, _))| {
            let id = drawings[i].placement.id.clone();
            let mine =
                bounds(paths.iter().flat_map(|p| {
                    std::iter::once(p.start).chain(p.segments.iter().map(Segment::end))
                }));
            // Les silhouettes de devant qui peuvent toucher ce dessin.
            let front: Vec<Vec<[f64; 2]>> = match mine {
                Some(b) => (i + 1..transported.len())
                    .flat_map(|j| {
                        transported[j]
                            .1
                            .iter()
                            .zip(&silhouette_bounds[j])
                            .filter(move |(_, sb)| overlaps(b, **sb))
                            .map(|(poly, _)| poly.iter().map(|p| [p.x, p.y]).collect())
                    })
                    .collect(),
                None => Vec::new(),
            };
            if front.is_empty() {
                return SlidePaths {
                    placement: id,
                    paths: paths.clone(),
                    clipped: false,
                };
            }
            // Courbes aplaties, puis différence avec les silhouettes.
            let subject: Vec<Vec<[f64; 2]>> = paths
                .iter()
                .map(|p| {
                    flatten(p, TOLERANCE_MM)
                        .iter()
                        .map(|q| [q.x, q.y])
                        .collect()
                })
                .filter(|c: &Vec<[f64; 2]>| c.len() >= 3)
                .collect();
            let shapes = subject.overlay(&front, OverlayRule::Difference, FillRule::NonZero);
            let paths = shapes
                .into_iter()
                .flatten()
                .filter(|c| c.len() >= 3)
                .map(|c| PathMm {
                    start: Point::new(c[0][0], c[0][1]),
                    segments: c[1..]
                        .iter()
                        .map(|q| Segment::Line {
                            to: Point::new(q[0], q[1]),
                        })
                        .chain(std::iter::once(Segment::Line {
                            to: Point::new(c[0][0], c[0][1]),
                        }))
                        .collect(),
                })
                .collect();
            SlidePaths {
                placement: id,
                paths,
                clipped: true,
            }
        })
        .collect()
}

/// Tous les contours d'un dessin, sur `threads` fils (0 : tous).
fn transport_all<F>(t: &Transporter<F>, vector: &Vectorization, threads: usize) -> Vec<PathMm>
where
    F: Fn(Point) -> Option<Point> + Sync,
{
    let threads = if threads == 0 {
        std::thread::available_parallelism().map_or(1, |n| n.get())
    } else {
        threads
    };
    let chunk = vector.contours.len().div_ceil(threads.max(1)).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = vector
            .contours
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .filter_map(|c| t.contour(c.start, &c.segments))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("transport des courbes"))
            .collect()
    })
}

// ---------------------------------------------------------------------------
// Écriture
// ---------------------------------------------------------------------------

/// Nombre écrit au dix-millième de mm, sans zéros inutiles.
fn num(v: f64) -> String {
    let s = format!("{:.4}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

/// Le document SVG d'une diapositive. Avec `margin`, la plaque de 300 mm
/// et la diapositive décalée de la marge.
pub fn write_svg(drawings: &[SlidePaths], margin: bool, title: &str) -> String {
    let (side, offset) = if margin {
        (SLIDE_SIZE_MM + 2.0 * PLATE_MARGIN_MM, PLATE_MARGIN_MM)
    } else {
        (SLIDE_SIZE_MM, 0.0)
    };
    let mut s = String::new();
    let side = num(side);
    let _ = writeln!(
        s,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{side}mm\" height=\"{side}mm\" \
         viewBox=\"0 0 {side} {side}\">"
    );
    let _ = writeln!(s, "<title>{}</title>", escape(title));
    let shift = |p: Point| (num(p.x + offset), num(p.y + offset));
    for drawing in drawings {
        if drawing.paths.is_empty() {
            continue;
        }
        let mut d = String::new();
        for path in &drawing.paths {
            let (x, y) = shift(path.start);
            let _ = write!(d, "M{x} {y}");
            for segment in &path.segments {
                match *segment {
                    Segment::Line { to } => {
                        let (x, y) = shift(to);
                        let _ = write!(d, "L{x} {y}");
                    }
                    Segment::Cubic { c1, c2, to } => {
                        let (ax, ay) = shift(c1);
                        let (bx, by) = shift(c2);
                        let (x, y) = shift(to);
                        let _ = write!(d, "C{ax} {ay} {bx} {by} {x} {y}");
                    }
                }
            }
            d.push('Z');
        }
        let rule = if drawing.clipped {
            "evenodd"
        } else {
            "nonzero"
        };
        let _ = writeln!(
            s,
            "<path id=\"{}\" fill=\"#000000\" fill-rule=\"{rule}\" d=\"{d}\"/>",
            escape(&drawing.placement)
        );
    }
    // Cadre de la diapositive (D-5.3-f), demandé par Benjamin après l'essai
    // LightBurn : un carré de 280 mm, trait rouge sans remplissage, pour
    // situer la diapositive. Le rouge en fait un calque à part dans
    // LightBurn, qu'on peut ne pas graver.
    let (o, size) = (num(offset), num(SLIDE_SIZE_MM));
    let _ = writeln!(
        s,
        "<rect id=\"cadre-diapositive\" x=\"{o}\" y=\"{o}\" width=\"{size}\" height=\"{size}\" \
         fill=\"none\" stroke=\"#ff0000\" stroke-width=\"0.1\"/>"
    );
    s.push_str("</svg>\n");
    s
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::drawings::{draw, slide_rect, Placing, Posed};
    use crate::geometry::Transform;
    use crate::project::{Deformation, DeformationFrame, RegistrationPair};
    use crate::registration::slide_to_facade;
    use crate::tiles::Raster;
    use crate::units;
    use crate::vectorize::{self, compare, Contour, GrayImage, VectorizeOptions, VectorizeStats};

    /// Recalage d'essai (celui des tests d'export) : agrandi environ 14
    /// fois, en trapèze, avec du relief.
    fn warp() -> PointWarp {
        let pairs: Vec<RegistrationPair> = [
            ((0.0, 0.0), (1_000.0, 800.0)),
            ((280.0, 0.0), (5_100.0, 600.0)),
            ((280.0, 280.0), (5_300.0, 4_700.0)),
            ((0.0, 280.0), (900.0, 4_500.0)),
            ((140.0, 140.0), (3_080.0, 2_690.0)),
            ((70.0, 210.0), (1_990.0, 3_640.0)),
        ]
        .iter()
        .map(|&((sx, sy), (fx, fy))| {
            RegistrationPair::new(PointMm::new(sx, sy), PointMm::new(fx, fy))
        })
        .collect();
        slide_to_facade(&pairs).unwrap()
    }

    /// Un dessin de 480 px : un anneau, un disque, une barre en biais, bords
    /// adoucis (16 échantillons par pixel).
    fn drawing() -> Raster {
        let side = 480u32;
        let inside = |x: f64, y: f64| {
            let r = ((x - 240.0).powi(2) + (y - 240.0).powi(2)).sqrt();
            let ring = (r - 150.0).abs() < 14.0;
            let disk = (x - 380.0).powi(2) + (y - 90.0).powi(2) < 40.0 * 40.0;
            let d = ((x - 60.0) * 0.6 - (y - 400.0) * 0.8).abs();
            let bar = d < 5.0 && (60.0..300.0).contains(&x);
            ring || disk || bar
        };
        let mut data = vec![0u8; (side * side) as usize];
        for j in 0..side {
            for i in 0..side {
                let mut n = 0;
                for a in 0..4 {
                    for b in 0..4 {
                        let x = f64::from(i) + (f64::from(a) + 0.5) / 4.0;
                        let y = f64::from(j) + (f64::from(b) + 0.5) / 4.0;
                        n += u32::from(inside(x, y));
                    }
                }
                data[(j * side + i) as usize] = (n * 255 / 16) as u8;
            }
        }
        Raster::new(side, side, 1, data)
    }

    /// Le dessin posé près du point (`sx`, `sy`) de la diapositive, tourné,
    /// déformé en trapèze, à peu près un pixel du dessin par pixel de
    /// diapositive à 1 400 dpi.
    fn placement(warp: &PointWarp, sx: f64, sy: f64) -> Placement {
        let c = warp.apply(Point::new(sx, sy)).unwrap();
        let size = 480.0 * units::px_size_mm(1400.0) * 14.6;
        let center = PointMm::new(c.x, c.y);
        Placement {
            id: "p".into(),
            drawing: "d".into(),
            center,
            width: size,
            height: size,
            rotation: 17.0,
            mirrored: false,
            group: None,
            layer: None,
            deformation: Some(Deformation {
                frame: DeformationFrame {
                    center,
                    width: size,
                    height: size,
                    rotation: 17.0,
                },
                columns: 1,
                rows: 1,
                nodes: vec![[0.08, 0.0], [0.95, 0.05], [0.0, 0.9], [1.0, 1.0]],
                extra: Default::default(),
            }),
            projector: None,
            adjustment: None,
            slide: None,
            hidden: false,
            locked: false,
            extra: Default::default(),
        }
    }

    fn options() -> VectorizeOptions {
        VectorizeOptions {
            threads: 1,
            ..VectorizeOptions::default()
        }
    }

    /// Critère de la tâche 5.3 : le SVG posé sur le PNG de la même
    /// diapositive (1 400 dpi) coïncide à moins de 0,02 mm. Mesure : la
    /// mesure indépendante de la vectorisation (5.1), entre les courbes du
    /// SVG ramenées en pixels de la diapositive et le PNG.
    #[test]
    fn svg_and_png_of_a_slide_coincide_within_two_hundredths_of_a_mm() {
        let warp = warp();
        let image = drawing();
        let vector = vectorize::vectorize(&GrayImage::from_raster(&image), &options()).unwrap();
        for mirror in [false, true] {
            let p = placement(&warp, 60.0, 200.0);
            let px_mm = units::px_size_mm(1400.0);
            let (side, _) = crate::export::sizes_px(1400.0);
            let place = Placing {
                warp: Some(&warp),
                px_mm,
                side,
                mirror,
                projector: None,
            };
            let rect = slide_rect(&p, &place).unwrap();
            // L'image entière de la diapositive (pages à zéro, presque rien
            // en mémoire), dont seul le rectangle du dessin est calculé.
            let mut slide = Raster::new(side, side, 1, vec![0; side as usize * side as usize]);
            let posed = [Posed {
                placement: &p,
                image: &image,
                mask: None,
            }];
            draw(&mut slide, &posed, &place, 1).unwrap();
            let (w, h) = (rect.x1 - rect.x0, rect.y1 - rect.y0);
            let mut crop = Vec::with_capacity((w * h) as usize);
            for y in rect.y0..rect.y1 {
                let s = (y * side + rect.x0) as usize;
                crop.extend_from_slice(&slide.data[s..s + w as usize]);
            }
            drop(slide);

            let svg_place = SvgPlacing {
                warp: Some(&warp),
                mirror,
                projector: None,
            };
            let d = VectorDrawing {
                placement: &p,
                vector: &vector,
                silhouette: &[],
            };
            let paths = slide_paths(&[d], &svg_place, 1);
            // Courbes du SVG en pixels du rectangle.
            let to_px = |q: Point| {
                Point::new(
                    q.x / px_mm - f64::from(rect.x0),
                    q.y / px_mm - f64::from(rect.y0),
                )
            };
            let contours = paths[0]
                .paths
                .iter()
                .map(|path| Contour {
                    start: to_px(path.start),
                    segments: path
                        .segments
                        .iter()
                        .map(|s| match *s {
                            Segment::Line { to } => Segment::Line { to: to_px(to) },
                            Segment::Cubic { c1, c2, to } => Segment::Cubic {
                                c1: to_px(c1),
                                c2: to_px(c2),
                                to: to_px(to),
                            },
                        })
                        .collect(),
                    area: 0.0,
                })
                .collect::<Vec<_>>();
            assert_eq!(contours.len(), vector.contours.len());
            let in_px = vectorize::Vectorization {
                width: w,
                height: h,
                options: options(),
                contours,
                stats: VectorizeStats {
                    contours: 0,
                    removed_contours: 0,
                    raw_points: 0,
                    lines: 0,
                    cubics: 0,
                    max_fit_error_px: 0.0,
                },
            };
            let png = GrayImage {
                width: w,
                height: h,
                data: crop,
            };
            let c = compare(&png, &in_px);
            let mm = c.max_deviation_px * px_mm;
            assert!(c.source_pixels > 10_000, "{}", c.source_pixels);
            assert!(mm < 0.02, "miroir {mirror} : écart {mm:.4} mm ({:?})", c);
        }
    }

    /// Le transport suit la chaîne exacte à la tolérance près, partout sur
    /// les courbes (pas seulement là où il a été vérifié).
    #[test]
    fn transported_curves_follow_the_exact_chain() {
        let warp = warp();
        let image = drawing();
        let vector = vectorize::vectorize(&GrayImage::from_raster(&image), &options()).unwrap();
        let p = placement(&warp, 200.0, 80.0);
        let place = SvgPlacing {
            warp: Some(&warp),
            mirror: true,
            projector: None,
        };
        let (w, h) = (f64::from(vector.width), f64::from(vector.height));
        let map = |q: Point| slide_point(&p, &place, q.x / w, q.y / h);
        let t = Transporter {
            map,
            tolerance: TOLERANCE_MM,
            step: 0.5,
        };
        let mut worst: f64 = 0.0;
        let mut pieces = 0;
        for contour in &vector.contours {
            let path = t.contour(contour.start, &contour.segments).unwrap();
            pieces += path.segments.len();
            // Chaque morceau d'origine, échantillonné, comparé aux courbes
            // transportées aplaties très finement.
            let flat = flatten(&path, 1e-5);
            let mut a = contour.start;
            for s in &contour.segments {
                for k in 1..8 {
                    let t = f64::from(k) / 8.0;
                    let q = match *s {
                        Segment::Line { to } => lerp(a, to, t),
                        Segment::Cubic { c1, c2, to } => cubic_at(a, c1, c2, to, t),
                    };
                    let exact = map(q).unwrap();
                    let n = flat.len();
                    let d = (0..n)
                        .map(|i| {
                            crate::vectorize::fit::distance_to_segment(
                                exact,
                                flat[i],
                                flat[(i + 1) % n],
                            )
                        })
                        .fold(f64::INFINITY, f64::min);
                    worst = worst.max(d);
                }
                a = s.end();
            }
        }
        assert!(worst < 2.0 * TOLERANCE_MM, "écart {worst} mm");
        // Les courbes restent des courbes : pas des milliers de segments.
        let source: usize = vector.contours.iter().map(|c| c.segments.len()).sum();
        assert!(pieces < source * 4, "{pieces} morceaux pour {source}");
    }

    /// Un dessin derrière un autre est découpé par la silhouette de celui
    /// de devant.
    #[test]
    fn a_front_silhouette_cuts_the_drawing_behind() {
        let warp = warp();
        let image = drawing();
        let vector = vectorize::vectorize(&GrayImage::from_raster(&image), &options()).unwrap();
        let back = placement(&warp, 100.0, 100.0);
        let mut front = back.clone();
        front.id = "devant".into();
        // Silhouette de devant : un carré au milieu du dessin, que l'anneau
        // traverse (ses coins restent dehors).
        let square: SilhouettePolygon =
            vec![[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]];
        let place = SvgPlacing {
            warp: Some(&warp),
            mirror: false,
            projector: None,
        };
        let empty: [SilhouettePolygon; 0] = [];
        let silhouettes = [square];
        let drawings = [
            VectorDrawing {
                placement: &back,
                vector: &vector,
                silhouette: &empty,
            },
            VectorDrawing {
                placement: &front,
                vector: &vector,
                silhouette: &silhouettes,
            },
        ];
        let out = slide_paths(&drawings, &place, 1);
        assert!(out[0].clipped && !out[1].clipped);
        // Aucun point du dessin de derrière à l'intérieur du carré, ramené
        // dans le dessin de devant (marge au-dessus de la tolérance), et il
        // en reste dehors. Les bords du carré sont courbes dans la
        // diapositive : on teste donc dans le dessin.
        let strictly_inside = |q: Point| {
            let f = warp.apply(q).unwrap();
            let (u, v) = front.drawing_point(PointMm::new(f.x, f.y)).unwrap();
            let m = 0.002;
            (0.25 + m..0.75 - m).contains(&u) && (0.25 + m..0.75 - m).contains(&v)
        };
        let points: Vec<Point> = out[0].paths.iter().flat_map(|p| flatten(p, 1e-3)).collect();
        assert!(points.len() > 100);
        assert!(points.iter().all(|&q| !strictly_inside(q)));
        // Le dessin de devant, lui, a des courbes dans le carré (l'anneau y
        // passe).
        let front_points: Vec<Point> = out[1].paths.iter().flat_map(|p| flatten(p, 1e-3)).collect();
        assert!(front_points.iter().any(|&q| strictly_inside(q)));

        let svg = write_svg(&out, true, "Essai & A");
        assert!(svg.contains("width=\"300mm\""));
        // Le cadre de la diapositive : 280 mm, décalé de la marge.
        assert!(svg.contains(
            "<rect id=\"cadre-diapositive\" x=\"10\" y=\"10\" width=\"280\" height=\"280\""
        ));
        let bare = write_svg(&out, false, "Essai");
        assert!(bare.contains("x=\"0\" y=\"0\" width=\"280\" height=\"280\""));
        assert!(svg.contains("fill-rule=\"evenodd\""));
        assert!(svg.contains("Essai &amp; A"));
    }
}

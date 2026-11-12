//! Plans DXF : lecture par le crate `dxf`, rendu au trait par `tiny-skia`.
//!
//! Unité du fichier : l'unité de dessin du DXF. Le DXF a son origine où
//! l'architecte l'a voulue et son axe y vers le haut ; le repère du fichier
//! retenu ici part du coin haut-gauche du rectangle qui englobe tout le
//! dessin, y vers le bas (décision D-3.1-d). Ce coin, en coordonnées du DXF,
//! est gardé dans le projet (`facade.origin`).
//!
//! Ce qui est tracé : lignes, polylignes (arrondis compris), cercles, arcs,
//! ellipses, splines, solides (contour), blocs insérés (imbriqués, en
//! réseau), et les blocs dessinés des cotes. Ce qui ne l'est pas encore :
//! textes, hachures, images, objets 3D. Tout est tracé en noir, d'un trait
//! d'un pixel d'écran quel que soit le zoom : un plan reste net à tout zoom.

use std::collections::HashMap;
use std::f64::consts::{PI, TAU};
use std::path::Path;

use dxf::entities::{Entity, EntityType};
use dxf::enums::Units;
use dxf::Drawing;
use resvg::tiny_skia::{Color, LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};

use super::{FacadeError, VectorPlan};

/// Profondeur maximale de blocs imbriqués (un bloc qui s'insère lui-même
/// tournerait sans fin).
const MAX_BLOCK_DEPTH: usize = 16;

/// Nombre maximal de traits gardés ; au-delà, le plan est refusé plutôt que
/// de saturer la mémoire.
const MAX_STROKES: usize = 5_000_000;

/// Une ligne brisée, en coordonnées du repère du fichier (y vers le bas).
#[derive(Debug, Clone)]
struct Stroke2 {
    points: Vec<[f64; 2]>,
    closed: bool,
    /// Rectangle englobant : x min, y min, x max, y max.
    bbox: [f64; 4],
}

pub struct DxfPlan {
    strokes: Vec<Stroke2>,
    width: f64,
    height: f64,
    /// Coin haut-gauche du repère du fichier, en coordonnées du DXF.
    origin: [f64; 2],
    /// Millimètres par unité de dessin, si le fichier le déclare (`$INSUNITS`).
    mm_per_unit: Option<f64>,
}

/// Transformation affine 2D : x' = a x + c y + e ; y' = b x + d y + f.
#[derive(Debug, Clone, Copy)]
struct Affine2([f64; 6]);

impl Affine2 {
    const IDENTITY: Self = Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    fn apply(&self, [x, y]: [f64; 2]) -> [f64; 2] {
        let [a, b, c, d, e, f] = self.0;
        [a * x + c * y + e, b * x + d * y + f]
    }

    /// `self` après `other` : on applique d'abord `other`.
    fn then(&self, other: &Self) -> Self {
        let [a1, b1, c1, d1, e1, f1] = other.0;
        let [a2, b2, c2, d2, e2, f2] = self.0;
        Self([
            a2 * a1 + c2 * b1,
            b2 * a1 + d2 * b1,
            a2 * c1 + c2 * d1,
            b2 * c1 + d2 * d1,
            a2 * e1 + c2 * f1 + e2,
            b2 * e1 + d2 * f1 + f2,
        ])
    }

    fn translate(x: f64, y: f64) -> Self {
        Self([1.0, 0.0, 0.0, 1.0, x, y])
    }

    fn rotate(radians: f64) -> Self {
        let (s, c) = radians.sin_cos();
        Self([c, s, -s, c, 0.0, 0.0])
    }

    fn scale(x: f64, y: f64) -> Self {
        Self([x, 0.0, 0.0, y, 0.0, 0.0])
    }

    /// Facteur d'agrandissement moyen (pour régler la finesse des courbes).
    fn mean_scale(&self) -> f64 {
        let [a, b, c, d, _, _] = self.0;
        (a * d - b * c).abs().sqrt().max(1e-12)
    }
}

/// Repère propre à l'objet (OCS) : avec une normale vers le bas (dessin
/// en miroir), l'axe x est retourné. Les autres orientations 3D ne servent
/// pas sur un plan de façade et sont traitées comme la normale vers le haut.
fn ocs(normal_z: f64) -> Affine2 {
    if normal_z < 0.0 {
        Affine2::scale(-1.0, 1.0)
    } else {
        Affine2::IDENTITY
    }
}

/// Ce qui est collecté en coordonnées du DXF, avant passage au repère du
/// fichier.
struct Collector<'a> {
    blocks: HashMap<&'a str, &'a dxf::Block>,
    hidden_layers: Vec<&'a str>,
    /// Écart maximal toléré entre une courbe et ses segments, en unités de
    /// dessin.
    tolerance: f64,
    strokes: Vec<Stroke2>,
    too_many: bool,
}

impl<'a> Collector<'a> {
    fn push(&mut self, points: Vec<[f64; 2]>, closed: bool, transform: &Affine2) {
        if points.len() < 2 || self.too_many {
            return;
        }
        if self.strokes.len() >= MAX_STROKES {
            self.too_many = true;
            return;
        }
        let points: Vec<[f64; 2]> = points.into_iter().map(|p| transform.apply(p)).collect();
        if points
            .iter()
            .any(|p| !p[0].is_finite() || !p[1].is_finite())
        {
            return;
        }
        let mut bbox = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for p in &points {
            bbox[0] = bbox[0].min(p[0]);
            bbox[1] = bbox[1].min(p[1]);
            bbox[2] = bbox[2].max(p[0]);
            bbox[3] = bbox[3].max(p[1]);
        }
        self.strokes.push(Stroke2 {
            points,
            closed,
            bbox,
        });
    }

    /// Nombre de segments pour un arc de `sweep` radians et de rayon `radius`
    /// (déjà multiplié par l'agrandissement du bloc).
    fn segments(&self, radius: f64, sweep: f64) -> usize {
        let radius = radius.abs().max(1e-12);
        // Flèche d'une corde : r (1 − cos(θ/2)) ≈ r θ² / 8.
        let step = (8.0 * self.tolerance / radius).sqrt().clamp(1e-4, PI / 8.0);
        ((sweep.abs() / step).ceil() as usize).clamp(2, 100_000)
    }

    fn arc_points(
        &self,
        center: [f64; 2],
        radius: f64,
        start: f64,
        sweep: f64,
        scale: f64,
    ) -> Vec<[f64; 2]> {
        let n = self.segments(radius * scale, sweep);
        (0..=n)
            .map(|i| {
                let angle = start + sweep * i as f64 / n as f64;
                [
                    center[0] + radius * angle.cos(),
                    center[1] + radius * angle.sin(),
                ]
            })
            .collect()
    }

    fn visible(&self, entity: &Entity) -> bool {
        let common = &entity.common;
        !common.is_in_paper_space
            && common.is_visible
            && !self
                .hidden_layers
                .iter()
                .any(|layer| *layer == common.layer)
    }

    fn entities(
        &mut self,
        entities: impl Iterator<Item = &'a Entity>,
        transform: &Affine2,
        depth: usize,
    ) {
        for entity in entities {
            if self.visible(entity) {
                self.entity(entity, transform, depth);
            }
        }
    }

    fn entity(&mut self, entity: &'a Entity, transform: &Affine2, depth: usize) {
        let scale = transform.mean_scale();
        match &entity.specific {
            EntityType::Line(line) => {
                self.push(
                    vec![[line.p1.x, line.p1.y], [line.p2.x, line.p2.y]],
                    false,
                    transform,
                );
            }
            EntityType::LwPolyline(poly) => {
                let vertices: Vec<([f64; 2], f64)> = poly
                    .vertices
                    .iter()
                    .map(|v| ([v.x, v.y], v.bulge))
                    .collect();
                let closed = poly.flags & 1 != 0;
                let points = self.bulged(&vertices, closed, scale);
                let t = transform.then(&ocs(poly.extrusion_direction.z));
                self.push(points, closed, &t);
            }
            EntityType::Polyline(poly) => {
                // Les maillages 3D (drapeaux 16 et 64) ne sont pas un dessin de façade.
                if poly.flags & (16 | 64) != 0 {
                    return;
                }
                let vertices: Vec<([f64; 2], f64)> = poly
                    .vertices()
                    .map(|v| ([v.location.x, v.location.y], v.bulge))
                    .collect();
                let closed = poly.flags & 1 != 0;
                let points = self.bulged(&vertices, closed, scale);
                let t = if poly.flags & 8 != 0 {
                    *transform // polyligne 3D : coordonnées générales
                } else {
                    transform.then(&ocs(poly.normal.z))
                };
                self.push(points, closed, &t);
            }
            EntityType::Circle(circle) => {
                let points = self.arc_points(
                    [circle.center.x, circle.center.y],
                    circle.radius,
                    0.0,
                    TAU,
                    scale,
                );
                self.push(points, true, &transform.then(&ocs(circle.normal.z)));
            }
            EntityType::Arc(arc) => {
                let start = arc.start_angle.to_radians();
                let mut sweep = arc.end_angle.to_radians() - start;
                while sweep <= 0.0 {
                    sweep += TAU;
                }
                let points = self.arc_points(
                    [arc.center.x, arc.center.y],
                    arc.radius,
                    start,
                    sweep,
                    scale,
                );
                self.push(points, false, &transform.then(&ocs(arc.normal.z)));
            }
            EntityType::Ellipse(ellipse) => {
                let major = [ellipse.major_axis.x, ellipse.major_axis.y];
                let ratio = ellipse.minor_axis_ratio;
                let sign = if ellipse.normal.z < 0.0 { -1.0 } else { 1.0 };
                let minor = [-major[1] * ratio * sign, major[0] * ratio * sign];
                let start = ellipse.start_parameter;
                let mut sweep = ellipse.end_parameter - start;
                while sweep <= 0.0 {
                    sweep += TAU;
                }
                let radius = major[0].hypot(major[1]);
                let n = self.segments(radius * scale, sweep);
                let points = (0..=n)
                    .map(|i| {
                        let t = start + sweep * i as f64 / n as f64;
                        let (s, c) = t.sin_cos();
                        [
                            ellipse.center.x + major[0] * c + minor[0] * s,
                            ellipse.center.y + major[1] * c + minor[1] * s,
                        ]
                    })
                    .collect();
                let closed = (sweep - TAU).abs() < 1e-9;
                self.push(points, closed, transform);
            }
            EntityType::Spline(spline) => {
                let points = spline_points(spline);
                let closed = spline.flags & 1 != 0;
                self.push(points, closed, transform);
            }
            EntityType::Solid(solid) => {
                // Ordre des sommets d'un SOLID : 1, 2, 4, 3.
                let points = vec![
                    [solid.first_corner.x, solid.first_corner.y],
                    [solid.second_corner.x, solid.second_corner.y],
                    [solid.fourth_corner.x, solid.fourth_corner.y],
                    [solid.third_corner.x, solid.third_corner.y],
                ];
                self.push(
                    points,
                    true,
                    &transform.then(&ocs(solid.extrusion_direction.z)),
                );
            }
            EntityType::Insert(insert) => {
                if depth >= MAX_BLOCK_DEPTH {
                    return;
                }
                let Some(block) = self.blocks.get(insert.name.as_str()).copied() else {
                    return;
                };
                let base = Affine2::translate(-block.base_point.x, -block.base_point.y);
                let scaled =
                    Affine2::scale(insert.x_scale_factor, insert.y_scale_factor).then(&base);
                let rotation = Affine2::rotate(insert.rotation.to_radians());
                let columns = insert.column_count.max(1);
                let rows = insert.row_count.max(1);
                for row in 0..rows {
                    for column in 0..columns {
                        let offset = rotation.apply([
                            f64::from(column) * insert.column_spacing,
                            f64::from(row) * insert.row_spacing,
                        ]);
                        let local = Affine2::translate(
                            insert.location.x + offset[0],
                            insert.location.y + offset[1],
                        )
                        .then(&rotation)
                        .then(&scaled);
                        let placed = transform
                            .then(&ocs(insert.extrusion_direction.z))
                            .then(&local);
                        self.entities(block.entities.iter(), &placed, depth + 1);
                    }
                }
            }
            EntityType::RotatedDimension(d) => {
                self.dimension_block(&d.dimension_base.block_name, transform, depth)
            }
            EntityType::RadialDimension(d) => {
                self.dimension_block(&d.dimension_base.block_name, transform, depth)
            }
            EntityType::DiameterDimension(d) => {
                self.dimension_block(&d.dimension_base.block_name, transform, depth)
            }
            EntityType::AngularThreePointDimension(d) => {
                self.dimension_block(&d.dimension_base.block_name, transform, depth)
            }
            EntityType::OrdinateDimension(d) => {
                self.dimension_block(&d.dimension_base.block_name, transform, depth)
            }
            _ => {}
        }
    }

    /// Une cote est dessinée par un bloc anonyme, en coordonnées générales.
    fn dimension_block(&mut self, name: &str, transform: &Affine2, depth: usize) {
        if depth >= MAX_BLOCK_DEPTH {
            return;
        }
        if let Some(block) = self.blocks.get(name).copied() {
            self.entities(block.entities.iter(), transform, depth + 1);
        }
    }

    /// Sommets d'une polyligne, avec ses arrondis (« bulge ») changés en arcs.
    fn bulged(&self, vertices: &[([f64; 2], f64)], closed: bool, scale: f64) -> Vec<[f64; 2]> {
        let mut points = Vec::with_capacity(vertices.len());
        let count = vertices.len();
        let segments = if closed {
            count
        } else {
            count.saturating_sub(1)
        };
        if let Some(first) = vertices.first() {
            points.push(first.0);
        }
        for i in 0..segments {
            let (p1, bulge) = vertices[i];
            let (p2, _) = vertices[(i + 1) % count];
            if bulge.abs() > 1e-12 {
                // Angle balayé : 4 atan(bulge) ; rayon déduit de la corde.
                let sweep = 4.0 * bulge.atan();
                let chord = (p2[0] - p1[0]).hypot(p2[1] - p1[1]);
                if chord > 0.0 {
                    let radius = chord / (2.0 * (sweep / 2.0).sin());
                    let mid = [(p1[0] + p2[0]) / 2.0, (p1[1] + p2[1]) / 2.0];
                    // Distance signée du milieu de la corde au centre.
                    let h = radius * (sweep / 2.0).cos();
                    let normal = [-(p2[1] - p1[1]) / chord, (p2[0] - p1[0]) / chord];
                    let center = [mid[0] + normal[0] * h, mid[1] + normal[1] * h];
                    let start = (p1[1] - center[1]).atan2(p1[0] - center[0]);
                    let n = self.segments(radius * scale, sweep);
                    for k in 1..n {
                        let angle = start + sweep * k as f64 / n as f64;
                        points.push([
                            center[0] + radius.abs() * angle.cos(),
                            center[1] + radius.abs() * angle.sin(),
                        ]);
                    }
                }
            }
            if !(closed && i + 1 == segments) {
                points.push(p2);
            }
        }
        points
    }
}

/// Points d'une spline : courbe B-spline (rationnelle si elle a des poids)
/// évaluée par l'algorithme de de Boor ; à défaut, ses points de passage.
fn spline_points(spline: &dxf::entities::Spline) -> Vec<[f64; 2]> {
    let degree = spline.degree_of_curve.max(1) as usize;
    let control: Vec<[f64; 2]> = spline.control_points.iter().map(|p| [p.x, p.y]).collect();
    let knots = &spline.knot_values;
    let n = control.len();
    if n <= degree || knots.len() != n + degree + 1 {
        return spline.fit_points.iter().map(|p| [p.x, p.y]).collect();
    }
    let weights: Vec<f64> = if spline.weight_values.len() == n {
        spline.weight_values.clone()
    } else {
        vec![1.0; n]
    };
    let (t0, t1) = (knots[degree], knots[n]);
    if t1.partial_cmp(&t0) != Some(std::cmp::Ordering::Greater) {
        return control;
    }
    let samples = (n * 16).clamp(16, 20_000);
    (0..=samples)
        .map(|i| {
            let t = t0 + (t1 - t0) * i as f64 / samples as f64;
            de_boor(t, degree, &control, &weights, knots)
        })
        .collect()
}

fn de_boor(
    t: f64,
    degree: usize,
    control: &[[f64; 2]],
    weights: &[f64],
    knots: &[f64],
) -> [f64; 2] {
    let n = control.len();
    // Intervalle de nœuds qui contient t.
    let mut span = degree;
    while span < n - 1 && t >= knots[span + 1] {
        span += 1;
    }
    let mut d: Vec<[f64; 3]> = (0..=degree)
        .map(|j| {
            let i = span - degree + j;
            let w = weights[i];
            [control[i][0] * w, control[i][1] * w, w]
        })
        .collect();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = span - degree + j;
            let denominator = knots[i + degree + 1 - r] - knots[i];
            let alpha = if denominator.abs() < 1e-300 {
                0.0
            } else {
                (t - knots[i]) / denominator
            };
            let previous = d[j - 1];
            for (value, before) in d[j].iter_mut().zip(previous) {
                *value = (1.0 - alpha) * before + alpha * *value;
            }
        }
    }
    let [x, y, w] = d[degree];
    if w.abs() < 1e-300 {
        [x, y]
    } else {
        [x / w, y / w]
    }
}

/// Millimètres par unité de dessin d'après `$INSUNITS`, si elle est connue.
fn units_mm(units: Units) -> Option<f64> {
    Some(match units {
        Units::Millimeters => 1.0,
        Units::Centimeters => 10.0,
        Units::Meters => 1000.0,
        Units::Inches => 25.4,
        Units::Feet => 304.8,
        _ => return None,
    })
}

fn collect(drawing: &Drawing, tolerance: f64) -> Result<Vec<Stroke2>, FacadeError> {
    let blocks = drawing
        .blocks()
        .map(|block| (block.name.as_str(), block))
        .collect();
    let hidden_layers = drawing
        .layers()
        .filter(|layer| !layer.is_layer_on)
        .map(|layer| layer.name.as_str())
        .collect();
    let mut collector = Collector {
        blocks,
        hidden_layers,
        tolerance,
        strokes: Vec::new(),
        too_many: false,
    };
    collector.entities(drawing.entities(), &Affine2::IDENTITY, 0);
    if collector.too_many {
        return Err(FacadeError::Unreadable(format!(
            "plan DXF trop chargé : plus de {MAX_STROKES} traits"
        )));
    }
    Ok(collector.strokes)
}

fn extents(strokes: &[Stroke2]) -> Option<[f64; 4]> {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for stroke in strokes {
        b[0] = b[0].min(stroke.bbox[0]);
        b[1] = b[1].min(stroke.bbox[1]);
        b[2] = b[2].max(stroke.bbox[2]);
        b[3] = b[3].max(stroke.bbox[3]);
    }
    (b[0].is_finite() && b[2] >= b[0] && b[3] >= b[1]).then_some(b)
}

impl DxfPlan {
    pub fn open(path: &Path) -> Result<Self, FacadeError> {
        let drawing = Drawing::load_file(path)
            .map_err(|error| FacadeError::Unreadable(format!("DXF illisible : {error}")))?;
        Self::from_drawing(&drawing)
    }

    fn from_drawing(drawing: &Drawing) -> Result<Self, FacadeError> {
        // Premier passage, courbes grossières, pour connaître l'étendue du
        // dessin ; second passage avec une finesse réglée sur elle : un
        // quart de pixel au niveau le plus fin des tuiles.
        let rough = collect(drawing, f64::INFINITY)?;
        let Some(b) = extents(&rough) else {
            return Err(FacadeError::Unreadable(
                "le DXF ne contient aucun trait lisible (seuls lignes, polylignes, arcs, cercles, ellipses, splines et blocs sont lus)".into(),
            ));
        };
        let side = (b[2] - b[0]).max(b[3] - b[1]).max(1e-9);
        let tolerance = side / f64::from(super::VECTOR_LEVEL0_SIDE) / 4.0;
        let strokes = collect(drawing, tolerance)?;
        let b = extents(&strokes).unwrap_or(b);
        // Un dessin réduit à un trait horizontal ou vertical garde une
        // épaisseur minimale, pour rester une image.
        let width = (b[2] - b[0]).max(side * 1e-3);
        let height = (b[3] - b[1]).max(side * 1e-3);
        let origin = [b[0], b[3]];
        // Passage au repère du fichier : origine au coin haut-gauche, y vers le bas.
        let strokes = strokes
            .into_iter()
            .map(|stroke| {
                let points: Vec<[f64; 2]> = stroke
                    .points
                    .iter()
                    .map(|p| [p[0] - origin[0], origin[1] - p[1]])
                    .collect();
                let bbox = [
                    stroke.bbox[0] - origin[0],
                    origin[1] - stroke.bbox[3],
                    stroke.bbox[2] - origin[0],
                    origin[1] - stroke.bbox[1],
                ];
                Stroke2 {
                    points,
                    closed: stroke.closed,
                    bbox,
                }
            })
            .collect();
        Ok(Self {
            strokes,
            width,
            height,
            origin,
            mm_per_unit: units_mm(drawing.header.default_drawing_units),
        })
    }

    /// Coin haut-gauche du repère du fichier, en coordonnées du DXF.
    pub fn origin(&self) -> [f64; 2] {
        self.origin
    }

    /// Millimètres par unité de dessin, si le fichier le déclare.
    pub fn mm_per_unit(&self) -> Option<f64> {
        self.mm_per_unit
    }

    pub fn stroke_count(&self) -> usize {
        self.strokes.len()
    }
}

impl VectorPlan for DxfPlan {
    fn size(&self) -> (f64, f64) {
        (self.width, self.height)
    }

    fn render(&self, scale: f64, offset_x: f64, offset_y: f64, width: u32, height: u32) -> Vec<u8> {
        let Some(mut pixmap) = Pixmap::new(width, height) else {
            return vec![255; width as usize * height as usize * 4];
        };
        pixmap.fill(Color::WHITE);
        let mut paint = Paint::default();
        paint.set_color(Color::BLACK);
        paint.anti_alias = true;
        let stroke = Stroke {
            width: 1.0,
            line_cap: LineCap::Butt,
            ..Stroke::default()
        };
        // Zone visible en unités du fichier, avec un pixel de marge.
        let margin = 2.0 / scale;
        let view = [
            offset_x / scale - margin,
            offset_y / scale - margin,
            (offset_x + f64::from(width)) / scale + margin,
            (offset_y + f64::from(height)) / scale + margin,
        ];
        let to_px = |p: &[f64; 2]| {
            (
                (p[0] * scale - offset_x) as f32,
                (p[1] * scale - offset_y) as f32,
            )
        };
        for s in &self.strokes {
            let b = s.bbox;
            if b[2] < view[0] || b[0] > view[2] || b[3] < view[1] || b[1] > view[3] {
                continue;
            }
            let mut builder = PathBuilder::new();
            let (x, y) = to_px(&s.points[0]);
            builder.move_to(x, y);
            for p in &s.points[1..] {
                let (x, y) = to_px(p);
                builder.line_to(x, y);
            }
            if s.closed {
                builder.close();
            }
            if let Some(path) = builder.finish() {
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
        pixmap.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dxf::entities::{Arc as DxfArc, Line, LwPolyline};
    use dxf::LwPolylineVertex;
    use dxf::Point;

    fn drawing_with(entities: Vec<EntityType>) -> Drawing {
        let mut drawing = Drawing::new();
        for specific in entities {
            drawing.add_entity(Entity::new(specific));
        }
        drawing
    }

    #[test]
    fn the_frame_starts_at_the_top_left_corner_with_y_down() {
        let drawing = drawing_with(vec![EntityType::Line(Line::new(
            Point::new(100.0, 50.0, 0.0),
            Point::new(300.0, 150.0, 0.0),
        ))]);
        let plan = DxfPlan::from_drawing(&drawing).unwrap();
        assert_eq!(plan.size(), (200.0, 100.0));
        assert_eq!(plan.origin(), [100.0, 150.0]);
        // Le point bas-gauche (100, 50) du DXF est en (0, 100) : en bas.
        assert_eq!(plan.strokes[0].points, vec![[0.0, 100.0], [200.0, 0.0]]);
    }

    #[test]
    fn a_half_circle_bulge_becomes_an_arc() {
        let mut poly = LwPolyline::default();
        for (x, bulge) in [(0.0, 1.0), (10.0, 0.0)] {
            poly.vertices.push(LwPolylineVertex {
                x,
                y: 0.0,
                bulge,
                ..Default::default()
            });
        }
        let plan =
            DxfPlan::from_drawing(&drawing_with(vec![EntityType::LwPolyline(poly)])).unwrap();
        let (width, height) = plan.size();
        assert!((width - 10.0).abs() < 1e-6);
        // Un demi-cercle de rayon 5 : 5 de haut.
        assert!((height - 5.0).abs() < 1e-3, "{height}");
    }

    #[test]
    fn arcs_are_fine_enough_at_the_deepest_level() {
        let mut arc = DxfArc::new(Point::new(0.0, 0.0, 0.0), 1000.0, 0.0, 90.0);
        arc.thickness = 0.0;
        let plan = DxfPlan::from_drawing(&drawing_with(vec![EntityType::Arc(arc)])).unwrap();
        // Chaque point est sur le cercle, et la corde la plus longue s'en
        // écarte de moins d'un quart de pixel du niveau le plus fin.
        let pixel = 1000.0 / f64::from(crate::facade::VECTOR_LEVEL0_SIDE);
        let points = &plan.strokes[0].points;
        let origin = plan.origin();
        for pair in points.windows(2) {
            let mid = [
                (pair[0][0] + pair[1][0]) / 2.0 + origin[0],
                origin[1] - (pair[0][1] + pair[1][1]) / 2.0,
            ];
            let sagitta = 1000.0 - mid[0].hypot(mid[1]);
            assert!(sagitta < pixel / 4.0 + 1e-9, "flèche {sagitta}");
        }
    }

    #[test]
    fn inserted_blocks_are_drawn_in_place() {
        let mut drawing = Drawing::new();
        let mut block = dxf::Block {
            name: "fenetre".into(),
            ..Default::default()
        };
        block.entities.push(Entity::new(EntityType::Line(Line::new(
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
        ))));
        drawing.add_block(block);
        let insert = dxf::entities::Insert {
            name: "fenetre".into(),
            location: Point::new(10.0, 20.0, 0.0),
            x_scale_factor: 2.0,
            rotation: 90.0,
            ..Default::default()
        };
        drawing.add_entity(Entity::new(EntityType::Insert(insert)));
        // Un repère pour fixer l'origine en (0, 0).
        drawing.add_entity(Entity::new(EntityType::Line(Line::new(
            Point::new(0.0, 0.0, 0.0),
            Point::new(0.0, 0.0, 0.0),
        ))));
        let plan = DxfPlan::from_drawing(&drawing).unwrap();
        let origin = plan.origin();
        let block_line = &plan.strokes[0].points;
        let native: Vec<[f64; 2]> = block_line
            .iter()
            .map(|p| [p[0] + origin[0], origin[1] - p[1]])
            .collect();
        // Le trait de 1 unité, agrandi 2 fois et tourné de 90°, va de (10, 20) à (10, 22).
        assert!((native[0][0] - 10.0).abs() < 1e-9 && (native[0][1] - 20.0).abs() < 1e-9);
        assert!((native[1][0] - 10.0).abs() < 1e-9 && (native[1][1] - 22.0).abs() < 1e-9);
    }

    #[test]
    fn a_quadratic_spline_follows_its_parabola() {
        // Spline de degré 2 à trois points de contrôle, nœuds 0 0 0 1 1 1 :
        // c'est une courbe de Bézier ; son milieu est en (1, 1) pour les
        // points (0, 0), (1, 2), (2, 0).
        let spline = dxf::entities::Spline {
            degree_of_curve: 2,
            knot_values: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            control_points: vec![
                Point::new(0.0, 0.0, 0.0),
                Point::new(1.0, 2.0, 0.0),
                Point::new(2.0, 0.0, 0.0),
            ],
            ..Default::default()
        };
        let points = spline_points(&spline);
        assert_eq!(points.first(), Some(&[0.0, 0.0]));
        let last = points.last().unwrap();
        assert!((last[0] - 2.0).abs() < 1e-12 && last[1].abs() < 1e-12);
        let middle = points[points.len() / 2];
        assert!(
            (middle[0] - 1.0).abs() < 1e-12 && (middle[1] - 1.0).abs() < 1e-12,
            "{middle:?}"
        );
    }
}

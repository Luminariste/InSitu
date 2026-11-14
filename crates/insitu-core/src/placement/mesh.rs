//! Déformation libre d'un dessin posé (tâche 4.4).
//!
//! Une déformation est une grille de `columns × rows` cases tendue sur un
//! **cadre** : un rectangle tourné de la façade (le dessin lui-même, ou le
//! cadre d'un groupe au moment de le déformer). Chaque nœud de la grille est
//! donné en fractions du cadre ; au repos, le nœud (i, j) est en
//! (i / columns, j / rows). Déplacer les nœuds déforme le dessin.
//!
//! - **Quatre coins** (une seule case) : la case suit les quatre coins en
//!   perspective (homographie) ; les droites restent droites, comme un
//!   dessin projeté de biais.
//! - **Grille** (plus d'une case) : surface lisse qui passe par chaque nœud
//!   (interpolation cubique de Catmull-Rom, comme `GridWarp`, D-2.3-b) ; un
//!   rang de nœuds mis en arc fait épouser une voussure.
//!
//! Le chemin d'un point du dessin vers la façade : placement (taille,
//! rotation, miroir) → fractions du cadre → grille → façade. Le sens inverse
//! (export) résout la grille par la méthode de Newton.

use crate::geometry::{
    has_fold, invert_by_newton, Differentiable, Homography, Jacobian, Point, Rect, Transform,
};
use crate::project::model::{Deformation, DeformationFrame};
use crate::project::PointMm;

/// Nombre maximal de cases d'une grille, par côté.
pub const MAX_DIVISIONS: u32 = 16;

impl DeformationFrame {
    /// Point de la façade aux fractions (`s`, `t`) du cadre.
    pub fn facade_point(&self, s: f64, t: f64) -> PointMm {
        let x = (s - 0.5) * self.width;
        let y = (t - 0.5) * self.height;
        let (sin, cos) = self.rotation.to_radians().sin_cos();
        PointMm::new(
            self.center.x + x * cos - y * sin,
            self.center.y + x * sin + y * cos,
        )
    }

    /// Fractions du cadre du point `p` de la façade.
    pub fn fractions(&self, p: PointMm) -> (f64, f64) {
        let (dx, dy) = (p.x - self.center.x, p.y - self.center.y);
        let (sin, cos) = self.rotation.to_radians().sin_cos();
        (
            (dx * cos + dy * sin) / self.width + 0.5,
            (-dx * sin + dy * cos) / self.height + 0.5,
        )
    }
}

/// La grille, en fractions du cadre : (s, t) au repos → (s, t) déformé.
pub struct Mesh<'a> {
    deformation: &'a Deformation,
    /// Quatre coins : le trapèze et son inverse, calculés une fois.
    corners: Option<(Homography, Homography)>,
}

/// Le trapèze qui envoie le carré unité sur les quatre coins `n` (haut-
/// gauche, haut-droit, bas-gauche, bas-droit), en formule directe : une
/// grille est relue à chaque point exporté, l'ajustement général
/// ([`Homography::from_four_pairs`]) y coûtait des centaines de
/// microsecondes (D-3.6-i). `None` si les coins sont alignés.
fn square_to_corners(n: &[[f64; 2]]) -> Option<Homography> {
    // Dans l'ordre du tour : (0,0), (1,0), (1,1), (0,1).
    let [x0, y0] = n[0];
    let [x1, y1] = n[1];
    let [x2, y2] = n[3];
    let [x3, y3] = n[2];
    let (dx1, dx2, dx3) = (x1 - x2, x3 - x2, x0 - x1 + x2 - x3);
    let (dy1, dy2, dy3) = (y1 - y2, y3 - y2, y0 - y1 + y2 - y3);
    let det = dx1 * dy2 - dx2 * dy1;
    let scale = (dx1.abs() + dx2.abs()) * (dy1.abs() + dy2.abs());
    if !det.is_finite() || det.abs() <= 1e-12 * scale {
        return None;
    }
    let g = (dx3 * dy2 - dx2 * dy3) / det;
    let h = (dx1 * dy3 - dx3 * dy1) / det;
    Homography::from_matrix([
        [x1 - x0 + g * x1, x3 - x0 + h * x3, x0],
        [y1 - y0 + g * y1, y3 - y0 + h * y3, y0],
        [g, h, 1.0],
    ])
    .ok()
}

impl<'a> Mesh<'a> {
    /// `None` si la grille est incohérente (voir [`check`]).
    pub fn new(deformation: &'a Deformation) -> Option<Self> {
        let (c, r) = (deformation.columns as usize, deformation.rows as usize);
        if c == 0 || r == 0 || deformation.nodes.len() != (c + 1) * (r + 1) {
            return None;
        }
        let corners = if c == 1 && r == 1 {
            let n = &deformation.nodes;
            let unit = [
                Point::new(0.0, 0.0),
                Point::new(1.0, 0.0),
                Point::new(0.0, 1.0),
                Point::new(1.0, 1.0),
            ];
            let target = [0, 1, 2, 3].map(|k| Point::new(n[k][0], n[k][1]));
            let h = match square_to_corners(n) {
                Some(h) => h,
                None => Homography::from_four_pairs(unit, target).ok()?,
            };
            let inverse = h.inverse().ok()?;
            Some((h, inverse))
        } else {
            None
        };
        Some(Self {
            deformation,
            corners,
        })
    }

    /// Nœud (i, j). Un rang au-delà du bord, le nœud fantôme prolonge la
    /// grille en parabole (en ligne droite s'il n'y a que deux nœuds) : un
    /// arc garde sa courbure jusqu'au bout, une grille au repos reste plate.
    fn node(&self, i: isize, j: isize) -> Point {
        let (c, r) = (
            self.deformation.columns as isize,
            self.deformation.rows as isize,
        );
        let extend = |n: isize, a: Point, b: Point, d: Point| {
            if n >= 2 {
                a * 3.0 - b * 3.0 + d
            } else {
                a * 2.0 - b
            }
        };
        if i < 0 {
            return extend(c, self.node(0, j), self.node(1, j), self.node(2.min(c), j));
        }
        if i > c {
            return extend(
                c,
                self.node(c, j),
                self.node(c - 1, j),
                self.node((c - 2).max(0), j),
            );
        }
        if j < 0 {
            return extend(r, self.node(i, 0), self.node(i, 1), self.node(i, 2.min(r)));
        }
        if j > r {
            return extend(
                r,
                self.node(i, r),
                self.node(i, r - 1),
                self.node(i, (r - 2).max(0)),
            );
        }
        let n = self.deformation.nodes[(j * (c + 1) + i) as usize];
        Point::new(n[0], n[1])
    }
}

/// Poids de Catmull-Rom et leurs dérivées en `a` (0 ≤ a ≤ 1).
fn weights(a: f64) -> ([f64; 4], [f64; 4]) {
    let (a2, a3) = (a * a, a * a * a);
    (
        [
            (-a3 + 2.0 * a2 - a) / 2.0,
            (3.0 * a3 - 5.0 * a2 + 2.0) / 2.0,
            (-3.0 * a3 + 4.0 * a2 + a) / 2.0,
            (a3 - a2) / 2.0,
        ],
        [
            (-3.0 * a2 + 4.0 * a - 1.0) / 2.0,
            (9.0 * a2 - 10.0 * a) / 2.0,
            (-9.0 * a2 + 8.0 * a + 1.0) / 2.0,
            (3.0 * a2 - 2.0 * a) / 2.0,
        ],
    )
}

impl Transform for Mesh<'_> {
    fn apply(&self, p: Point) -> Option<Point> {
        self.apply_with_jacobian(p).map(|(q, _)| q)
    }

    fn apply_inverse(&self, p: Point) -> Option<Point> {
        if let Some((_, inverse)) = &self.corners {
            return inverse.apply(p);
        }
        let forward = |q| self.apply_with_jacobian(q);
        if let Some(q) = invert_by_newton(forward, p, p) {
            return Some(q);
        }
        // Grande déformation : on repart du nœud d'un treillis fin dont
        // l'image est la plus proche.
        let n = 4 * self.deformation.columns.max(self.deformation.rows) as usize;
        let mut best = (f64::INFINITY, p);
        for j in 0..=n {
            for i in 0..=n {
                let q = Point::new(i as f64 / n as f64, j as f64 / n as f64);
                if let Some(v) = self.apply(q) {
                    let e = (v - p).norm();
                    if e < best.0 {
                        best = (e, q);
                    }
                }
            }
        }
        invert_by_newton(forward, p, best.1)
    }
}

impl Differentiable for Mesh<'_> {
    fn apply_with_jacobian(&self, p: Point) -> Option<(Point, Jacobian)> {
        if let Some((h, _)) = &self.corners {
            return h.apply_with_jacobian(p);
        }
        let (c, r) = (
            self.deformation.columns as f64,
            self.deformation.rows as f64,
        );
        let (x, y) = (p.x * c, p.y * r);
        let i = (x.floor() as isize).clamp(0, c as isize - 1);
        let j = (y.floor() as isize).clamp(0, r as isize - 1);
        let (wx, dx) = weights(x - i as f64);
        let (wy, dy) = weights(y - j as f64);
        let mut q = Point::new(0.0, 0.0);
        let mut ds = Point::new(0.0, 0.0);
        let mut dt = Point::new(0.0, 0.0);
        for (b, (&wyb, &dyb)) in wy.iter().zip(&dy).enumerate() {
            for (a, (&wxa, &dxa)) in wx.iter().zip(&dx).enumerate() {
                let n = self.node(i - 1 + a as isize, j - 1 + b as isize);
                q = q + n * (wxa * wyb);
                ds = ds + n * (dxa * wyb * c);
                dt = dt + n * (wxa * dyb * r);
            }
        }
        let jacobian = [[ds.x, dt.x], [ds.y, dt.y]];
        q.is_finite().then_some((q, jacobian))
    }
}

/// Grille au repos de `columns × rows` cases sur `frame`.
pub fn identity(frame: DeformationFrame, columns: u32, rows: u32) -> Deformation {
    let mut nodes = Vec::with_capacity(((columns + 1) * (rows + 1)) as usize);
    for j in 0..=rows {
        for i in 0..=columns {
            nodes.push([
                f64::from(i) / f64::from(columns),
                f64::from(j) / f64::from(rows),
            ]);
        }
    }
    Deformation {
        frame,
        columns,
        rows,
        nodes,
        extra: Default::default(),
    }
}

/// Ce qui ne va pas dans une déformation, ou `None` si elle est utilisable :
/// taille de grille, nombre de nœuds, nombres finis, cadre non vide, et
/// aucun repli (le dessin ne doit pas se retourner sur lui-même).
pub fn check(d: &Deformation) -> Option<String> {
    if d.columns == 0 || d.rows == 0 || d.columns > MAX_DIVISIONS || d.rows > MAX_DIVISIONS {
        return Some(format!(
            "la grille doit avoir de 1 à {MAX_DIVISIONS} cases par côté"
        ));
    }
    let expected = ((d.columns + 1) * (d.rows + 1)) as usize;
    if d.nodes.len() != expected {
        return Some(format!(
            "une grille de {} × {} cases a {expected} nœuds, pas {}",
            d.columns,
            d.rows,
            d.nodes.len()
        ));
    }
    let f = &d.frame;
    if !(f.width > 0.0 && f.height > 0.0 && f.center.x.is_finite() && f.center.y.is_finite())
        || !f.rotation.is_finite()
        || d.nodes.iter().flatten().any(|v| !v.is_finite())
    {
        return Some("le cadre ou un nœud n'est pas un nombre fini".into());
    }
    let Some(mesh) = Mesh::new(d) else {
        return Some("les quatre coins ne forment pas un quadrilatère".into());
    };
    let unit = Rect::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0)).expect("carré unité");
    let samples = 8 * d.columns.max(d.rows) as usize;
    if has_fold(&mesh, unit, samples) {
        return Some("la déformation replie le dessin sur lui-même".into());
    }
    None
}

/// Image par la déformation du point `p` de la façade (avant déformation).
pub fn forward(d: &Deformation, p: PointMm) -> PointMm {
    let (s, t) = d.frame.fractions(p);
    match Mesh::new(d).and_then(|m| m.apply(Point::new(s, t))) {
        Some(q) => d.frame.facade_point(q.x, q.y),
        None => p,
    }
}

/// Antécédent par la déformation du point `p` de la façade.
pub fn inverse(d: &Deformation, p: PointMm) -> Option<PointMm> {
    let (s, t) = d.frame.fractions(p);
    let q = Mesh::new(d)?.apply_inverse(Point::new(s, t))?;
    Some(d.frame.facade_point(q.x, q.y))
}

#[cfg(test)]
mod corner_tests {
    use super::*;

    #[test]
    fn la_formule_directe_des_quatre_coins_vaut_l_ajustement() {
        let unit = [
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
            Point::new(1.0, 1.0),
        ];
        let quads: [[[f64; 2]; 4]; 3] = [
            [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            [[0.1, -0.05], [0.95, 0.1], [-0.1, 0.9], [1.2, 1.1]],
            [[0.2, 0.0], [0.8, 0.0], [0.0, 1.0], [1.0, 1.0]],
        ];
        for n in quads {
            let direct = square_to_corners(&n).expect("quadrilatère");
            let target = [0, 1, 2, 3].map(|k| Point::new(n[k][0], n[k][1]));
            let fitted = Homography::from_four_pairs(unit, target).unwrap();
            for (u, v) in [
                (0.0, 0.0),
                (1.0, 0.0),
                (0.0, 1.0),
                (1.0, 1.0),
                (0.3, 0.7),
                (1.4, -0.2),
            ] {
                let a = direct.apply(Point::new(u, v)).unwrap();
                let b = fitted.apply(Point::new(u, v)).unwrap();
                assert!((a - b).norm() < 1e-9, "{n:?} en ({u}, {v}) : {a:?} ≠ {b:?}");
            }
        }
        assert!(square_to_corners(&[[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0]]).is_none());
    }
}

//! La mire : description géométrique et dessin imprimable (tâche 2.2).
//!
//! La mire est la feuille de 280 mm posée sur la vitre du rétroprojecteur
//! pour la prise de vue en chambre noire. Ce module décrit la nouvelle mire
//! retenue par l'étude 2.1 (`docs/etudes/2.1-detection-mire.md`) :
//!
//! - la grille de 10 mm, la croix centrale, le cercle rouge de 60 mm de rayon
//!   et le disque blanc central de 30 mm, **exactement aux mêmes places que
//!   sur l'ancienne mire** : une photo de l'ancienne mire reste exploitable
//!   par la même détection (décision D-2.1-c) ;
//! - des traits fins de 0,35 mm au lieu de 0,1 mm ;
//! - 16 repères codés AprilTag 36h11 (numéros 0 à 15) de 16 mm, chacun au
//!   centre d'un bloc de 2 × 2 cases vidé de ses traits intérieurs ;
//! - un identifiant écrit dans la marge.
//!
//! Tout est en millimètres, dans le repère de la diapositive : origine en
//! haut à gauche de la mire, x vers la droite, y vers le bas. La détection
//! (tâche 2.4) lit cette même description : positions des repères et des
//! croisements de la grille.

pub mod apriltag;
pub mod pdf;

use crate::geometry::Point;
use crate::units::SLIDE_SIZE_MM;

/// Numéro de version de la mire, écrit dans son identifiant. La mire
/// d'Épinal (fichier Illustrator, sans repère codé) est la version 1.
pub const TARGET_VERSION: u32 = 2;

/// Pas de la grille, en mm.
pub const GRID_PITCH_MM: f64 = 10.0;

/// Nombre de lignes de grille dans chaque direction (0 à 280 mm compris).
pub const GRID_LINES: usize = 29;

/// Épaisseur des traits fins (grille, disque central, tirets), en mm.
pub const THIN_LINE_MM: f64 = 0.35;

/// Épaisseur de la croix centrale, en mm (comme l'ancienne mire).
pub const CROSS_LINE_MM: f64 = 0.76;

/// Rayon et épaisseur du cercle rouge, en mm (comme l'ancienne mire).
pub const RED_CIRCLE_RADIUS_MM: f64 = 60.0;
pub const RED_CIRCLE_LINE_MM: f64 = 0.9;

/// Rayon du disque blanc central, en mm. La grille et la croix s'y arrêtent.
pub const DISC_RADIUS_MM: f64 = 30.0;

/// Les quatre tirets en diagonale dans le disque vont de ce rayon au bord.
pub const DISC_TICK_INNER_RADIUS_MM: f64 = 14.4;

/// Un croisement plus proche du centre que cette distance n'est pas
/// mesurable : il est dans le disque, ou trop près de son bord (étude 2.1).
pub const NODE_MIN_CENTER_DISTANCE_MM: f64 = 34.0;

/// Côté d'un bloc recevant un repère (2 × 2 cases), en mm.
pub const MARKER_BLOCK_MM: f64 = 20.0;

/// Côté d'un repère codé, pourtour noir compris, en mm (modules de 2 mm).
pub const MARKER_SIZE_MM: f64 = 16.0;

/// Abscisses et ordonnées du coin haut-gauche des blocs, en mm. Les 16 blocs
/// sont numérotés ligne par ligne, de gauche à droite et de haut en bas.
pub const MARKER_BLOCK_POSITIONS_MM: [f64; 4] = [10.0, 70.0, 190.0, 250.0];

/// Rouge du cercle, en composantes de 0 à 1 (l'encre rouge de l'étude 2.1).
pub const RED: [f64; 3] = [230.0 / 255.0, 30.0 / 255.0, 40.0 / 255.0];

/// Un repère codé posé sur la mire.
#[derive(Debug, Clone, PartialEq)]
pub struct Marker {
    /// Numéro AprilTag 36h11.
    pub id: usize,
    /// Coin haut-gauche du bloc de 20 mm qui le reçoit.
    pub block_origin: Point,
    /// Coins du repère (pourtour noir), dans l'ordre haut-gauche,
    /// haut-droit, bas-droit, bas-gauche, tel que la mire est lue à l'endroit.
    pub corners: [Point; 4],
}

impl Marker {
    pub fn center(&self) -> Point {
        Point::new(
            self.block_origin.x + MARKER_BLOCK_MM / 2.0,
            self.block_origin.y + MARKER_BLOCK_MM / 2.0,
        )
    }

    /// Le point est-il strictement à l'intérieur du bloc (pourtour exclu) ?
    pub fn block_contains(&self, p: Point) -> bool {
        let (x0, y0) = (self.block_origin.x, self.block_origin.y);
        p.x > x0 && p.x < x0 + MARKER_BLOCK_MM && p.y > y0 && p.y < y0 + MARKER_BLOCK_MM
    }
}

/// Description géométrique de la nouvelle mire.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetLayout {
    /// Côté de la mire (carré de la grille), en mm.
    pub size_mm: f64,
    pub markers: Vec<Marker>,
}

impl TargetLayout {
    /// La mire à repères codés (version 2).
    pub fn coded() -> Self {
        let margin = (MARKER_BLOCK_MM - MARKER_SIZE_MM) / 2.0;
        let mut markers = Vec::with_capacity(16);
        for &y0 in &MARKER_BLOCK_POSITIONS_MM {
            for &x0 in &MARKER_BLOCK_POSITIONS_MM {
                let (a, b) = (margin, margin + MARKER_SIZE_MM);
                markers.push(Marker {
                    id: markers.len(),
                    block_origin: Point::new(x0, y0),
                    corners: [
                        Point::new(x0 + a, y0 + a),
                        Point::new(x0 + b, y0 + a),
                        Point::new(x0 + b, y0 + b),
                        Point::new(x0 + a, y0 + b),
                    ],
                });
            }
        }
        TargetLayout {
            size_mm: SLIDE_SIZE_MM,
            markers,
        }
    }

    pub fn center(&self) -> Point {
        Point::new(self.size_mm / 2.0, self.size_mm / 2.0)
    }

    /// Identifiant imprimé dans la marge.
    pub fn identifier(&self) -> String {
        format!(
            "InSitu - mire {} mm - version {} - repères AprilTag 36h11 n° 0 à {}",
            self.size_mm,
            TARGET_VERSION,
            self.markers.len() - 1
        )
    }

    /// Les croisements de la grille où deux traits se croisent vraiment, en
    /// mm : ni dans le disque central ou à moins de 4 mm de son bord, ni sur un bloc de
    /// repère (pourtour compris : trop près du repère pour être mesuré, comme
    /// dans l'étude 2.1). Ceux de la croix centrale sont inclus.
    pub fn grid_nodes(&self) -> Vec<Point> {
        let c = self.center();
        let mut nodes = Vec::new();
        for j in 0..GRID_LINES {
            for i in 0..GRID_LINES {
                let p = Point::new(i as f64 * GRID_PITCH_MM, j as f64 * GRID_PITCH_MM);
                if (p.x - c.x).hypot(p.y - c.y) <= NODE_MIN_CENTER_DISTANCE_MM {
                    continue;
                }
                let on_block = self.markers.iter().any(|m| {
                    let (x0, y0) = (m.block_origin.x, m.block_origin.y);
                    p.x >= x0
                        && p.x <= x0 + MARKER_BLOCK_MM
                        && p.y >= y0
                        && p.y <= y0 + MARKER_BLOCK_MM
                });
                if !on_block {
                    nodes.push(p);
                }
            }
        }
        nodes
    }

    /// Le dessin complet de la mire, en primitives, dans le repère de la mire.
    pub fn drawing(&self) -> Vec<Shape> {
        let mut shapes = Vec::new();
        let size = self.size_mm;
        let c = self.center();

        // Grille fine, puis croix : chaque ligne est coupée là où elle
        // traverserait le disque central ou l'intérieur d'un bloc de repère.
        for k in 0..GRID_LINES {
            let t = k as f64 * GRID_PITCH_MM;
            let is_cross = (t - c.x).abs() < 1e-9;
            let width = if is_cross {
                CROSS_LINE_MM
            } else {
                THIN_LINE_MM
            };
            for vertical in [true, false] {
                for (a, b) in self.line_spans(t, vertical) {
                    let (p, q) = if vertical {
                        (Point::new(t, a), Point::new(t, b))
                    } else {
                        (Point::new(a, t), Point::new(b, t))
                    };
                    shapes.push(Shape::Line {
                        from: p,
                        to: q,
                        width,
                        color: Color::Black,
                    });
                }
            }
        }
        debug_assert!((size / GRID_PITCH_MM).round() as usize + 1 == GRID_LINES);

        // Cercle rouge.
        shapes.push(Shape::Circle {
            center: c,
            radius: RED_CIRCLE_RADIUS_MM,
            width: RED_CIRCLE_LINE_MM,
            color: Color::Red,
        });

        // Disque central : son cercle et ses quatre tirets en diagonale.
        shapes.push(Shape::Circle {
            center: c,
            radius: DISC_RADIUS_MM,
            width: THIN_LINE_MM,
            color: Color::Black,
        });
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let u = std::f64::consts::FRAC_1_SQRT_2;
            let at = |r: f64| Point::new(c.x + sx * u * r, c.y + sy * u * r);
            shapes.push(Shape::Line {
                from: at(DISC_TICK_INNER_RADIUS_MM),
                to: at(DISC_RADIUS_MM),
                width: THIN_LINE_MM,
                color: Color::Black,
            });
        }

        // Repères codés : les modules noirs, ligne par ligne, regroupés en
        // bandes horizontales.
        for marker in &self.markers {
            let modules = apriltag::marker_modules(marker.id);
            let module = MARKER_SIZE_MM / apriltag::MARKER_MODULES as f64;
            let origin = marker.corners[0];
            let mut rects = Vec::new();
            for (row, line) in modules.iter().enumerate() {
                let mut col = 0;
                while col < line.len() {
                    if !line[col] {
                        col += 1;
                        continue;
                    }
                    let start = col;
                    while col < line.len() && line[col] {
                        col += 1;
                    }
                    rects.push(Rect {
                        x: origin.x + start as f64 * module,
                        y: origin.y + row as f64 * module,
                        width: (col - start) as f64 * module,
                        height: module,
                    });
                }
            }
            shapes.push(Shape::Rects { rects });
        }
        shapes
    }

    /// Morceaux d'une ligne de grille à tracer, sous forme d'intervalles
    /// `(début, fin)` le long de la ligne. `t` est l'abscisse d'une ligne
    /// verticale (`vertical`) ou l'ordonnée d'une ligne horizontale.
    fn line_spans(&self, t: f64, vertical: bool) -> Vec<(f64, f64)> {
        let c = self.center();
        let mut cuts: Vec<(f64, f64)> = Vec::new();
        // Disque central.
        let d = t - if vertical { c.x } else { c.y };
        if d.abs() < DISC_RADIUS_MM {
            let h = (DISC_RADIUS_MM * DISC_RADIUS_MM - d * d).sqrt();
            let along = if vertical { c.y } else { c.x };
            cuts.push((along - h, along + h));
        }
        // Intérieur des blocs : seule la ligne qui passe au milieu du bloc
        // le traverse ; le pourtour du bloc reste tracé.
        for m in &self.markers {
            let (across, along) = if vertical {
                (m.block_origin.x, m.block_origin.y)
            } else {
                (m.block_origin.y, m.block_origin.x)
            };
            if t > across + 1e-9 && t < across + MARKER_BLOCK_MM - 1e-9 {
                cuts.push((along, along + MARKER_BLOCK_MM));
            }
        }
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut spans = Vec::new();
        let mut start = 0.0;
        for (a, b) in cuts {
            if a > start {
                spans.push((start, a));
            }
            start = start.max(b);
        }
        if start < self.size_mm {
            spans.push((start, self.size_mm));
        }
        spans
    }
}

/// Couleur d'encre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black,
    Red,
}

/// Rectangle plein, en mm (coin haut-gauche et dimensions).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Primitive de dessin de la mire, en mm.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// Trait droit, bouts carrés au ras des extrémités.
    Line {
        from: Point,
        to: Point,
        width: f64,
        color: Color,
    },
    /// Cercle tracé (non rempli) ; le trait est centré sur le rayon.
    Circle {
        center: Point,
        radius: f64,
        width: f64,
        color: Color,
    },
    /// Rectangles noirs remplis d'un seul tenant (un repère codé).
    Rects { rects: Vec<Rect> },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> TargetLayout {
        TargetLayout::coded()
    }

    #[test]
    fn sixteen_markers_numbered_row_by_row() {
        let l = layout();
        assert_eq!(l.markers.len(), 16);
        assert_eq!(l.markers[0].corners[0], Point::new(12.0, 12.0));
        assert_eq!(l.markers[3].corners[1], Point::new(268.0, 12.0));
        assert_eq!(l.markers[15].corners[2], Point::new(268.0, 268.0));
        assert_eq!(l.markers[4].block_origin, Point::new(10.0, 70.0));
        for (i, m) in l.markers.iter().enumerate() {
            assert_eq!(m.id, i);
            let side = m.corners[1].x - m.corners[0].x;
            assert_eq!(side, MARKER_SIZE_MM);
            assert_eq!(m.corners[3].y - m.corners[0].y, MARKER_SIZE_MM);
        }
    }

    /// Les repères ne touchent ni la croix, ni le cercle rouge, ni le disque,
    /// et restent dans la mire.
    #[test]
    fn markers_stay_clear_of_cross_circle_and_disc() {
        let l = layout();
        let c = l.center();
        for m in &l.markers {
            let (x0, y0) = (m.block_origin.x, m.block_origin.y);
            let x1 = x0 + MARKER_BLOCK_MM;
            let y1 = y0 + MARKER_BLOCK_MM;
            assert!(x0 >= 0.0 && y0 >= 0.0 && x1 <= l.size_mm && y1 <= l.size_mm);
            assert!(!(x0..=x1).contains(&c.x) && !(y0..=y1).contains(&c.y));
            // Distances du centre au bloc : la plus proche et la plus lointaine.
            let nx = c.x.clamp(x0, x1) - c.x;
            let ny = c.y.clamp(y0, y1) - c.y;
            let near = nx.hypot(ny);
            let fx = (x0 - c.x).abs().max((x1 - c.x).abs());
            let fy = (y0 - c.y).abs().max((y1 - c.y).abs());
            let far = fx.hypot(fy);
            let ring = RED_CIRCLE_RADIUS_MM + RED_CIRCLE_LINE_MM;
            assert!(near > ring || far < RED_CIRCLE_RADIUS_MM - RED_CIRCLE_LINE_MM);
        }
    }

    /// Autour de chaque repère, au moins 1,8 mm de blanc (un module de 2 mm,
    /// moins la moitié du trait du pourtour) : aucun trait ne traverse la
    /// zone de silence.
    #[test]
    fn quiet_zone_around_markers_is_free_of_lines() {
        let l = layout();
        for shape in l.drawing() {
            let Shape::Line {
                from, to, width, ..
            } = shape
            else {
                continue;
            };
            for m in &l.markers {
                let inner = Rect {
                    x: m.block_origin.x + width / 2.0,
                    y: m.block_origin.y + width / 2.0,
                    width: MARKER_BLOCK_MM - width,
                    height: MARKER_BLOCK_MM - width,
                };
                // Les traits sont horizontaux ou verticaux, sauf les tirets
                // du disque, loin des blocs : on teste le milieu et les bouts.
                for p in [
                    from,
                    to,
                    Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0),
                ] {
                    let inside = p.x > inner.x
                        && p.x < inner.x + inner.width
                        && p.y > inner.y
                        && p.y < inner.y + inner.height;
                    assert!(!inside, "trait {from:?} → {to:?} dans le bloc {}", m.id);
                }
                // Un trait qui traverse le bloc de part en part.
                let crosses_x =
                    from.x.min(to.x) <= inner.x && from.x.max(to.x) >= inner.x + inner.width;
                let crosses_y =
                    from.y.min(to.y) <= inner.y && from.y.max(to.y) >= inner.y + inner.height;
                let within_x = from.x > inner.x && from.x < inner.x + inner.width;
                let within_y = from.y > inner.y && from.y < inner.y + inner.height;
                assert!(!(crosses_y && within_x) && !(crosses_x && within_y));
            }
        }
        let quiet = (MARKER_BLOCK_MM - MARKER_SIZE_MM) / 2.0 - THIN_LINE_MM / 2.0;
        assert!(quiet >= 1.8);
    }

    #[test]
    fn grid_lines_stop_at_the_disc_and_cover_the_whole_square() {
        let l = layout();
        // Ligne y = 50 : aucun repère, aucun disque, d'un bord à l'autre.
        assert_eq!(l.line_spans(50.0, false), vec![(0.0, 280.0)]);
        // Croix verticale : coupée par le disque de 110 à 170.
        assert_eq!(
            l.line_spans(140.0, true),
            vec![(0.0, 110.0), (170.0, 280.0)]
        );
        // Ligne x = 20 : traverse les blocs de la première colonne.
        assert_eq!(
            l.line_spans(20.0, true),
            vec![
                (0.0, 10.0),
                (30.0, 70.0),
                (90.0, 190.0),
                (210.0, 250.0),
                (270.0, 280.0)
            ]
        );
        // Le pourtour d'un bloc reste entier.
        assert_eq!(l.line_spans(10.0, true), vec![(0.0, 280.0)]);
    }

    #[test]
    fn grid_nodes_skip_disc_and_marker_blocks() {
        let l = layout();
        let nodes = l.grid_nodes();
        // 29 × 29 = 841 croisements, moins 9 par bloc de repère, moins ceux
        // à 34 mm du centre ou moins (37) : 660, comme dans l'étude 2.1.
        let in_disc = (0..GRID_LINES)
            .flat_map(|j| (0..GRID_LINES).map(move |i| (i, j)))
            .filter(|&(i, j)| {
                let (x, y) = (i as f64 * 10.0 - 140.0, j as f64 * 10.0 - 140.0);
                x.hypot(y) <= NODE_MIN_CENTER_DISTANCE_MM
            })
            .count();
        assert_eq!(in_disc, 37);
        assert_eq!(nodes.len(), 660);
        assert_eq!(nodes.len(), 841 - 16 * 9 - in_disc);
        assert!(nodes.contains(&Point::new(0.0, 0.0)));
        assert!(nodes.contains(&Point::new(140.0, 100.0)));
        assert!(!nodes.contains(&Point::new(20.0, 20.0)));
        assert!(!nodes.contains(&Point::new(10.0, 10.0)));
        assert!(!nodes.contains(&Point::new(140.0, 120.0)));
        assert!(!nodes.contains(&Point::new(110.0, 130.0)));
    }

    #[test]
    fn identifier_names_the_version() {
        let id = layout().identifier();
        assert!(id.contains("version 2"), "{id}");
        assert!(id.contains("280 mm"), "{id}");
    }
}

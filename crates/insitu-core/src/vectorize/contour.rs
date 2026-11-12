//! Contours bruts : la ligne de niveau du seuil, suivie case par case
//! (« marching squares »).
//!
//! Les nœuds de la grille sont les centres des pixels, en (i + 0,5 ; j + 0,5).
//! Un nœud est « dedans » (dans le trait) si sa valeur atteint le seuil. Le
//! contour passe sur chaque arête qui relie un nœud dedans à un nœud dehors, à
//! l'endroit où l'interpolation linéaire des deux valeurs vaut le niveau
//! `seuil − 0,5` : les valeurs étant entières, ce niveau ne tombe jamais sur un
//! nœud, et un centre de pixel n'est jamais sur le contour. Autour de l'image,
//! une bordure noire (valeur 0) ferme tous les contours.
//!
//! Sens de parcours : le trait est toujours à droite (y vers le bas). Les
//! contours extérieurs et les trous tournent donc en sens opposés, ce qui
//! permet de les remplir avec la règle « nonzero » comme « evenodd ».

use crate::geometry::Point;

/// L'image en niveaux de gris, vue comme un champ de valeurs.
pub(crate) struct Field<'a> {
    pub width: usize,
    pub height: usize,
    pub data: &'a [u8],
    pub threshold: u8,
}

impl Field<'_> {
    fn level(&self) -> f32 {
        f32::from(self.threshold) - 0.5
    }

    #[inline]
    fn value(&self, i: isize, j: isize) -> f32 {
        if i < 0 || j < 0 || i >= self.width as isize || j >= self.height as isize {
            0.0
        } else {
            f32::from(self.data[j as usize * self.width + i as usize])
        }
    }

    #[inline]
    pub fn inside(&self, i: isize, j: isize) -> bool {
        self.value(i, j) > self.level()
    }

    /// Position du passage du contour entre deux nœuds voisins.
    #[inline]
    fn crossing(&self, v0: f32, v1: f32) -> f64 {
        f64::from((self.level() - v0) / (v1 - v0))
    }
}

/// Côtés d'une case : la case (ci, cj) a pour coin haut-gauche le nœud
/// (ci, cj).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

/// Une arête de la grille : horizontale entre (i, j) et (i + 1, j), ou
/// verticale entre (i, j) et (i, j + 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    H(isize, isize),
    V(isize, isize),
}

fn edge_of(ci: isize, cj: isize, side: Side) -> Edge {
    match side {
        Side::Top => Edge::H(ci, cj),
        Side::Bottom => Edge::H(ci, cj + 1),
        Side::Left => Edge::V(ci, cj),
        Side::Right => Edge::V(ci + 1, cj),
    }
}

/// Arêtes horizontales déjà parcourues : chaque contour fermé en franchit au
/// moins une, ce qui sert à ne le suivre qu'une fois.
struct Visited {
    stride: usize,
    bits: Vec<u64>,
}

impl Visited {
    fn new(width: usize, height: usize) -> Self {
        let stride = width + 1;
        Self {
            stride,
            bits: vec![0; (stride * height).div_ceil(64)],
        }
    }

    fn index(&self, i: isize, j: isize) -> usize {
        j as usize * self.stride + (i + 1) as usize
    }

    fn get(&self, i: isize, j: isize) -> bool {
        let k = self.index(i, j);
        self.bits[k / 64] >> (k % 64) & 1 == 1
    }

    fn set(&mut self, i: isize, j: isize) {
        let k = self.index(i, j);
        self.bits[k / 64] |= 1 << (k % 64);
    }
}

/// Tous les contours fermés de l'image, en pixels (origine au coin
/// haut-gauche de l'image). Chaque contour est une suite de points, le
/// dernier relié au premier.
pub(crate) fn trace(field: &Field) -> Vec<Vec<Point>> {
    let (w, h) = (field.width as isize, field.height as isize);
    let mut visited = Visited::new(field.width, field.height);
    let mut contours = Vec::new();
    for j in 0..h {
        let row = &field.data[j as usize * field.width..(j as usize + 1) * field.width];
        // Saut rapide des lignes entièrement noires (ou entièrement dedans,
        // sans bord) : la plupart des lignes d'un dessin le sont en partie.
        let mut previous = false;
        for i in -1..w {
            let next = if i + 1 < w {
                f32::from(row[(i + 1) as usize]) > field.level()
            } else {
                false
            };
            if previous != next && !visited.get(i, j) {
                contours.push(follow(field, &mut visited, i, j, previous));
            }
            previous = next;
        }
    }
    contours
}

/// Suit un contour à partir de l'arête horizontale (i, j)–(i + 1, j).
/// `inside_left` : le nœud de gauche est dans le trait.
fn follow(
    field: &Field,
    visited: &mut Visited,
    i: isize,
    j: isize,
    inside_left: bool,
) -> Vec<Point> {
    let start = Edge::H(i, j);
    let mut points = vec![edge_point(field, start)];
    visited.set(i, j);
    // Trait à droite du parcours (y vers le bas) : dedans à gauche, on
    // descend ; dedans à droite, on monte.
    let (mut ci, mut cj, mut entry) = if inside_left {
        (i, j, Side::Top)
    } else {
        (i, j - 1, Side::Bottom)
    };
    loop {
        let exit = exit_side(field, ci, cj, entry);
        let edge = edge_of(ci, cj, exit);
        if edge == start {
            break;
        }
        if let Edge::H(ei, ej) = edge {
            visited.set(ei, ej);
        }
        points.push(edge_point(field, edge));
        (ci, cj, entry) = match exit {
            Side::Top => (ci, cj - 1, Side::Bottom),
            Side::Bottom => (ci, cj + 1, Side::Top),
            Side::Left => (ci - 1, cj, Side::Right),
            Side::Right => (ci + 1, cj, Side::Left),
        };
    }
    points
}

fn edge_point(field: &Field, edge: Edge) -> Point {
    match edge {
        Edge::H(i, j) => {
            let f = field.crossing(field.value(i, j), field.value(i + 1, j));
            Point::new(i as f64 + 0.5 + f, j as f64 + 0.5)
        }
        Edge::V(i, j) => {
            let f = field.crossing(field.value(i, j), field.value(i, j + 1));
            Point::new(i as f64 + 0.5, j as f64 + 0.5 + f)
        }
    }
}

/// Côté par lequel le contour quitte la case (ci, cj), entré par `entry`.
fn exit_side(field: &Field, ci: isize, cj: isize, entry: Side) -> Side {
    // Coins : a haut-gauche, b haut-droite, c bas-droite, d bas-gauche.
    let a = field.inside(ci, cj);
    let b = field.inside(ci + 1, cj);
    let c = field.inside(ci + 1, cj + 1);
    let d = field.inside(ci, cj + 1);
    let crossed = [a != b, b != c, c != d, d != a];
    let sides = [Side::Top, Side::Right, Side::Bottom, Side::Left];
    let count = crossed.iter().filter(|&&x| x).count();
    debug_assert!(count == 2 || count == 4, "case incohérente");
    if count == 2 {
        return sides
            .iter()
            .zip(crossed)
            .find(|(side, x)| *x && **side != entry)
            .map(|(side, _)| *side)
            .expect("deux côtés franchis");
    }
    // Case en selle (deux coins dedans opposés) : la valeur au centre, moyenne
    // des quatre coins, décide si les deux coins dedans se touchent. Le
    // contour isole alors un coin : celui, parmi les extrémités du côté
    // d'entrée, qui est dans l'état contraire du centre.
    let center = (field.value(ci, cj)
        + field.value(ci + 1, cj)
        + field.value(ci + 1, cj + 1)
        + field.value(ci, cj + 1))
        / 4.0;
    let center_inside = center > field.level();
    // Extrémités du côté d'entrée, et les côtés de chaque coin.
    let (first, second) = match entry {
        Side::Top => ((a, Corner::A), (b, Corner::B)),
        Side::Right => ((b, Corner::B), (c, Corner::C)),
        Side::Bottom => ((c, Corner::C), (d, Corner::D)),
        Side::Left => ((d, Corner::D), (a, Corner::A)),
    };
    let isolated = if first.0 != center_inside {
        first.1
    } else {
        second.1
    };
    let (s1, s2) = isolated.sides();
    if s1 == entry {
        s2
    } else {
        s1
    }
}

#[derive(Clone, Copy)]
enum Corner {
    A,
    B,
    C,
    D,
}

impl Corner {
    fn sides(self) -> (Side, Side) {
        match self {
            Corner::A => (Side::Top, Side::Left),
            Corner::B => (Side::Top, Side::Right),
            Corner::C => (Side::Right, Side::Bottom),
            Corner::D => (Side::Bottom, Side::Left),
        }
    }
}

/// Aire signée d'un contour fermé (positive pour un contour extérieur).
pub(crate) fn signed_area(points: &[Point]) -> f64 {
    let n = points.len();
    let mut sum = 0.0;
    for k in 0..n {
        let p = points[k];
        let q = points[(k + 1) % n];
        sum += p.x * q.y - q.x * p.y;
    }
    sum / 2.0
}

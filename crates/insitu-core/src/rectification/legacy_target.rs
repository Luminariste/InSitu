//! L'ancienne mire, celle d'Épinal (page 2 de `jeu-essai/epinal/references/mire/mire-280.ai`),
//! décrite en millimètres de diapositive.
//!
//! Grille de 10 mm (29 traits par direction, de 0 à 280 mm), croix épaisse
//! au centre, cercle rouge de 60 mm de rayon, disque blanc central de 30 mm
//! de rayon, six petits cercles de 5 mm de rayon sur une diagonale (ils
//! seuls disent où est le bas). Mesures relevées par l'étude 2.1
//! (`etudes/2.1-detection/mires.py`).

use crate::geometry::Point;
use crate::units::SLIDE_SIZE_MM;

/// Pas de la grille, en mm.
pub(crate) const GRID_STEP_MM: f64 = 10.0;
/// Nombre de traits de la grille dans chaque direction.
pub(crate) const GRID_LINES: usize = 29;
/// Centre de la mire (croix, cercle rouge).
pub(crate) const CENTRE_MM: f64 = SLIDE_SIZE_MM / 2.0;
/// Rayon du cercle rouge, en mm.
pub(crate) const RED_CIRCLE_RADIUS_MM: f64 = 60.0;
/// Les croisements plus proches du centre que ce rayon tombent dans le disque
/// blanc central (30 mm) ou sur son pourtour : ils ne sont pas mesurables.
const CENTRAL_DISC_EXCLUSION_MM: f64 = 34.0;
/// Rayon des six petits cercles de la diagonale, en mm.
pub(crate) const SMALL_CIRCLE_RADIUS_MM: f64 = 5.0;

/// Centres des six petits cercles, du bas-gauche vers le centre.
pub(crate) fn small_circle_centres() -> [Point; 6] {
    std::array::from_fn(|k| Point::new(15.0 + 20.0 * k as f64, 265.0 - 20.0 * k as f64))
}

/// Un croisement de la grille : colonne `i`, ligne `j` (0 à 28).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Node {
    pub i: usize,
    pub j: usize,
}

impl Node {
    pub fn position(self) -> Point {
        Point::new(self.i as f64 * GRID_STEP_MM, self.j as f64 * GRID_STEP_MM)
    }

    pub fn index(self) -> usize {
        self.j * GRID_LINES + self.i
    }

    pub fn distance_to_centre(self) -> f64 {
        self.position().distance(Point::new(CENTRE_MM, CENTRE_MM))
    }
}

/// Tous les croisements de la grille, ligne par ligne.
pub(crate) fn all_nodes() -> impl Iterator<Item = Node> {
    (0..GRID_LINES).flat_map(|j| (0..GRID_LINES).map(move |i| Node { i, j }))
}

/// Un croisement où deux traits fins se croisent bien (hors disque central).
pub(crate) fn is_measurable(node: Node) -> bool {
    node.distance_to_centre() > CENTRAL_DISC_EXCLUSION_MM
}

/// Nombre de croisements mesurables (804).
pub(crate) fn measurable_count() -> usize {
    all_nodes().filter(|n| is_measurable(*n)).count()
}

/// Tourne un point de la mire d'un quart de tour (sens des aiguilles d'une
/// montre à l'écran, y vers le bas), `quarters` fois, autour du centre.
pub(crate) fn rotate_quarters(p: Point, quarters: u8) -> Point {
    let c = Point::new(CENTRE_MM, CENTRE_MM);
    let mut d = p - c;
    for _ in 0..quarters % 4 {
        d = Point::new(-d.y, d.x);
    }
    c + d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_legacy_target_has_804_measurable_crossings() {
        assert_eq!(measurable_count(), 804);
        assert_eq!(all_nodes().count(), 841);
    }

    #[test]
    fn four_quarter_turns_are_the_identity() {
        let p = Point::new(15.0, 265.0);
        assert_eq!(rotate_quarters(p, 4), p);
        let q = rotate_quarters(p, 1);
        assert_eq!(q, Point::new(15.0, 15.0));
        assert_eq!(rotate_quarters(rotate_quarters(p, 3), 1), p);
    }
}

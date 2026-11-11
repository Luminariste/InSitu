//! Outils partagés par les tests de géométrie : hasard reproductible et
//! déformations « réalistes » (trapèze marqué, papier qui gondole).

use super::{Homography, Point, Transform};
use crate::units::SLIDE_SIZE_MM;

/// Générateur pseudo-aléatoire minimal, pour des tests reproductibles sans
/// dépendance.
pub(crate) struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1))
    }

    /// Nombre uniforme dans [0, 1[.
    pub fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn point_in_slide(&mut self) -> Point {
        Point::new(self.unit() * SLIDE_SIZE_MM, self.unit() * SLIDE_SIZE_MM)
    }

    /// Décalage aléatoire, chaque coordonnée dans [−amplitude, +amplitude].
    pub fn offset(&mut self, amplitude: f64) -> Point {
        Point::new(
            (2.0 * self.unit() - 1.0) * amplitude,
            (2.0 * self.unit() - 1.0) * amplitude,
        )
    }
}

pub(crate) fn assert_close(got: Point, want: Point, tolerance: f64) {
    assert!(
        got.distance(want) <= tolerance,
        "{got:?} au lieu de {want:?} (écart {})",
        got.distance(want)
    );
}

/// Les quatre coins de la diapositive, dans le sens des aiguilles d'une
/// montre à partir du coin haut-gauche.
pub(crate) fn slide_corners() -> [Point; 4] {
    let s = SLIDE_SIZE_MM;
    [
        Point::new(0.0, 0.0),
        Point::new(s, 0.0),
        Point::new(s, s),
        Point::new(0.0, s),
    ]
}

/// Un trapèze marqué : les coins de la diapositive bougent de 5 à 25 mm.
pub(crate) fn trapezoid() -> Homography {
    // Calculé une seule fois : cette fonction est appelée des milliers de
    // fois par les tests.
    static TRAPEZOID: std::sync::OnceLock<Homography> = std::sync::OnceLock::new();
    *TRAPEZOID.get_or_init(build_trapezoid)
}

fn build_trapezoid() -> Homography {
    Homography::from_four_pairs(
        slide_corners(),
        [
            Point::new(12.0, 8.0),
            Point::new(262.0, -5.0),
            Point::new(301.0, 290.0),
            Point::new(-25.0, 271.0),
        ],
    )
    .unwrap()
}

/// Ondulation du papier (quelques millimètres, longueurs d'onde de 90 à
/// 200 mm) plus une distorsion d'objectif en barillet de 2 mm aux coins.
pub(crate) fn paper_wave(p: Point) -> Point {
    use std::f64::consts::TAU;
    let wave = Point::new(
        2.5 * (TAU * p.y / 130.0 + 0.4).sin() + 1.2 * (TAU * p.x / 90.0).sin(),
        3.0 * (TAU * p.x / 150.0 + 1.1).sin() * (TAU * p.y / 200.0).cos(),
    );
    let center = Point::new(SLIDE_SIZE_MM / 2.0, SLIDE_SIZE_MM / 2.0);
    let d = p - center;
    let lens = d * (-2.6e-7 * (d.x * d.x + d.y * d.y));
    wave + lens
}

/// La déformation réaliste de référence : ondulation puis trapèze.
pub(crate) fn realistic(p: Point) -> Point {
    trapezoid().apply(p + paper_wave(p)).unwrap()
}

/// Grille régulière de `count × count` points couvrant la diapositive, comme
/// les intersections d'une mire.
pub(crate) fn slide_grid(count: usize) -> Vec<Point> {
    let step = SLIDE_SIZE_MM / (count - 1) as f64;
    let mut points = Vec::with_capacity(count * count);
    for j in 0..count {
        for i in 0..count {
            points.push(Point::new(i as f64 * step, j as f64 * step));
        }
    }
    points
}

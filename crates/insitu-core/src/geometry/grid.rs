//! Déformation par grille : un trapèze, plus des déplacements donnés aux
//! nœuds d'une grille régulière et interpolés entre eux.
//!
//! C'est la forme rapide d'une déformation : quel que soit le nombre de
//! points qui ont servi à la construire, transformer un point ne lit que 16
//! nœuds. Elle sert à déformer les images (tâche 3.4) et pourra porter la
//! grille de déformation libre d'un dessin (tâche 4.4).

use super::warp::invert_by_newton;
use super::{Differentiable, GeometryError, Homography, Jacobian, Point, PointWarp, Transform};

/// Rectangle aux côtés parallèles aux axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    /// Refuse un rectangle vide, retourné ou non fini.
    pub fn new(min: Point, max: Point) -> Result<Self, GeometryError> {
        if min.is_finite() && max.is_finite() && max.x > min.x && max.y > min.y {
            Ok(Self { min, max })
        } else {
            Err(GeometryError::InvalidParameter(
                "le rectangle doit avoir une largeur et une hauteur positives",
            ))
        }
    }

    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }
}

/// Déformation échantillonnée sur une grille régulière du plan source.
///
/// Sens direct : `cible = trapèze(source) + déplacement(source)`. Le
/// déplacement est interpolé entre les nœuds par des cubiques de Catmull-Rom
/// (il passe par les valeurs des nœuds, et sa pente est continue). Hors de
/// la grille, il garde la valeur du bord le plus proche.
///
/// **Sens inverse.** Comme pour [`PointWarp`] : itérations de Newton, mais
/// chaque itération coûte un temps constant. L'aller-retour (direct puis
/// inverse) est exact à mieux qu'un millionième de millimètre tant que la déformation ne se
/// replie pas ; si elle se replie, le sens inverse renvoie `None` ou l'un
/// des antécédents (voir [`has_fold`](super::has_fold)).
#[derive(Debug, Clone, PartialEq)]
pub struct GridWarp {
    base: Homography,
    base_inverse: Homography,
    region: Rect,
    columns: usize,
    rows: usize,
    step: Point,
    /// Déplacement de chaque nœud, ligne par ligne.
    displacements: Vec<Point>,
}

impl GridWarp {
    /// Construit la déformation à partir des déplacements des nœuds :
    /// `columns × rows` valeurs, ligne par ligne, le premier nœud en
    /// `region.min`, le dernier en `region.max`.
    pub fn from_displacements(
        base: Homography,
        region: Rect,
        columns: usize,
        rows: usize,
        displacements: Vec<Point>,
    ) -> Result<Self, GeometryError> {
        if columns < 2 || rows < 2 {
            return Err(GeometryError::InvalidParameter(
                "la grille doit avoir au moins 2 nœuds dans chaque direction",
            ));
        }
        if displacements.len() != columns * rows {
            return Err(GeometryError::InvalidParameter(
                "le nombre de déplacements ne correspond pas à la taille de la grille",
            ));
        }
        if displacements.iter().any(|d| !d.is_finite()) {
            return Err(GeometryError::InvalidParameter(
                "un déplacement de la grille n'est pas un nombre fini",
            ));
        }
        Ok(Self {
            base,
            base_inverse: base.inverse()?,
            region,
            columns,
            rows,
            step: Point::new(
                region.width() / (columns - 1) as f64,
                region.height() / (rows - 1) as f64,
            ),
            displacements,
        })
    }

    /// Construit la déformation en évaluant `displacement` à chaque nœud.
    pub fn sample<F>(
        base: Homography,
        region: Rect,
        columns: usize,
        rows: usize,
        displacement: F,
    ) -> Result<Self, GeometryError>
    where
        F: Fn(Point) -> Point,
    {
        if columns < 2 || rows < 2 {
            return Err(GeometryError::InvalidParameter(
                "la grille doit avoir au moins 2 nœuds dans chaque direction",
            ));
        }
        let mut values = Vec::with_capacity(columns * rows);
        for j in 0..rows {
            for i in 0..columns {
                values.push(displacement(Point::new(
                    region.min.x + region.width() * i as f64 / (columns - 1) as f64,
                    region.min.y + region.height() * j as f64 / (rows - 1) as f64,
                )));
            }
        }
        Self::from_displacements(base, region, columns, rows, values)
    }

    /// Échantillonne une déformation par points sur `region` (plan source),
    /// avec des nœuds espacés d'au plus `max_step`.
    ///
    /// L'écart entre la grille et la déformation d'origine dépend du pas et
    /// diminue à peu près comme son carré : pour 225 points de mire sur un
    /// papier qui gondole de quelques millimètres, il vaut au plus 0,005 mm
    /// avec un pas de 2,5 mm et 0,02 mm avec un pas de 5 mm (mesuré par les
    /// tests). Les deux sens de la grille restent, eux, exactement
    /// réciproques quel que soit le pas.
    pub fn from_point_warp(
        warp: &PointWarp,
        region: Rect,
        max_step: f64,
    ) -> Result<Self, GeometryError> {
        if !(max_step.is_finite() && max_step > 0.0) {
            return Err(GeometryError::InvalidParameter(
                "le pas de la grille doit être positif",
            ));
        }
        let count = |length: f64| -> Result<usize, GeometryError> {
            let cells = (length / max_step).ceil();
            if cells > 20_000.0 {
                return Err(GeometryError::InvalidParameter(
                    "le pas de la grille est trop fin pour cette région",
                ));
            }
            Ok(cells.max(1.0) as usize + 1)
        };
        Self::sample(
            *warp.base(),
            region,
            count(region.width())?,
            count(region.height())?,
            |p| warp.correction(p),
        )
    }

    /// Le trapèze seul, sans les déplacements de la grille.
    pub fn base(&self) -> &Homography {
        &self.base
    }

    pub fn region(&self) -> Rect {
        self.region
    }

    /// Nombre de nœuds : (colonnes, lignes).
    pub fn size(&self) -> (usize, usize) {
        (self.columns, self.rows)
    }

    /// Valeur au nœud `(i, j)`. Un cran au-delà du bord, la valeur est
    /// prolongée en ligne droite, ce qui garde la précision de
    /// l'interpolation près des bords.
    fn node(&self, i: isize, j: isize) -> Point {
        let (columns, rows) = (self.columns as isize, self.rows as isize);
        if i < 0 {
            self.node(0, j) * 2.0 - self.node(1, j)
        } else if i >= columns {
            self.node(columns - 1, j) * 2.0 - self.node(columns - 2, j)
        } else if j < 0 {
            self.node(i, 0) * 2.0 - self.node(i, 1)
        } else if j >= rows {
            self.node(i, rows - 1) * 2.0 - self.node(i, rows - 2)
        } else {
            self.displacements[(j * columns + i) as usize]
        }
    }

    /// Déplacement interpolé au point `p`, et sa dérivée.
    fn displacement_with_jacobian(&self, p: Point) -> (Point, Jacobian) {
        let (ix, tx, inside_x) = locate((p.x - self.region.min.x) / self.step.x, self.columns);
        let (iy, ty, inside_y) = locate((p.y - self.region.min.y) / self.step.y, self.rows);
        let (wx, dwx) = catmull_rom(tx);
        let (wy, dwy) = catmull_rom(ty);
        let mut value = Point::new(0.0, 0.0);
        let mut ddx = Point::new(0.0, 0.0);
        let mut ddy = Point::new(0.0, 0.0);
        for b in 0..4 {
            let mut row_value = Point::new(0.0, 0.0);
            let mut row_slope = Point::new(0.0, 0.0);
            for a in 0..4 {
                let node = self.node(ix as isize - 1 + a as isize, iy as isize - 1 + b as isize);
                row_value = row_value + node * wx[a];
                row_slope = row_slope + node * dwx[a];
            }
            value = value + row_value * wy[b];
            ddx = ddx + row_slope * wy[b];
            ddy = ddy + row_value * dwy[b];
        }
        // Hors de la grille le déplacement est constant : pente nulle.
        let sx = if inside_x { 1.0 / self.step.x } else { 0.0 };
        let sy = if inside_y { 1.0 / self.step.y } else { 0.0 };
        (value, [[ddx.x * sx, ddy.x * sy], [ddx.y * sx, ddy.y * sy]])
    }

    /// Déplacement interpolé au point `p` (plan source).
    pub fn displacement(&self, p: Point) -> Point {
        self.displacement_with_jacobian(p).0
    }
}

/// Place une coordonnée (en pas de grille) dans sa cellule : indice du nœud
/// de gauche, position dans la cellule entre 0 et 1, et si l'on est bien
/// à l'intérieur de la grille.
fn locate(u: f64, count: usize) -> (usize, f64, bool) {
    let last = (count - 1) as f64;
    if u < 0.0 {
        (0, 0.0, false)
    } else if u >= last {
        (count - 2, 1.0, u == last)
    } else {
        let i = u.floor();
        (i as usize, u - i, true)
    }
}

/// Poids de Catmull-Rom des 4 nœuds voisins, et leurs dérivées.
fn catmull_rom(t: f64) -> ([f64; 4], [f64; 4]) {
    let t2 = t * t;
    let t3 = t2 * t;
    (
        [
            0.5 * (-t3 + 2.0 * t2 - t),
            0.5 * (3.0 * t3 - 5.0 * t2 + 2.0),
            0.5 * (-3.0 * t3 + 4.0 * t2 + t),
            0.5 * (t3 - t2),
        ],
        [
            0.5 * (-3.0 * t2 + 4.0 * t - 1.0),
            0.5 * (9.0 * t2 - 10.0 * t),
            0.5 * (-9.0 * t2 + 8.0 * t + 1.0),
            0.5 * (3.0 * t2 - 2.0 * t),
        ],
    )
}

impl Transform for GridWarp {
    fn apply(&self, p: Point) -> Option<Point> {
        self.apply_with_jacobian(p).map(|(q, _)| q)
    }

    fn apply_inverse(&self, p: Point) -> Option<Point> {
        // Départ : trapèze inverse, corrigé une fois du déplacement local.
        let rough = self.base_inverse.apply(p)?;
        let start = self
            .base_inverse
            .apply(p - self.displacement(rough))
            .unwrap_or(rough);
        invert_by_newton(|q| self.apply_with_jacobian(q), p, start)
    }
}

impl Differentiable for GridWarp {
    fn apply_with_jacobian(&self, p: Point) -> Option<(Point, Jacobian)> {
        let (mut q, mut j) = self.base.apply_with_jacobian(p)?;
        let (d, dj) = self.displacement_with_jacobian(p);
        q = q + d;
        for (row, drow) in j.iter_mut().zip(dj) {
            row[0] += drow[0];
            row[1] += drow[1];
        }
        q.is_finite().then_some((q, j))
    }
}

#[cfg(test)]
mod tests {
    use super::super::has_fold;
    use super::super::test_util::{
        assert_close, paper_wave, realistic, slide_grid, trapezoid, Lcg,
    };
    use super::*;
    use crate::units::SLIDE_SIZE_MM;

    fn slide() -> Rect {
        Rect::new(
            Point::new(0.0, 0.0),
            Point::new(SLIDE_SIZE_MM, SLIDE_SIZE_MM),
        )
        .unwrap()
    }

    /// La déformation de référence : 225 points de mire (grille 15 × 15, tous
    /// les 20 mm), trapèze marqué et papier gondolé de quelques millimètres.
    fn mire_warp() -> PointWarp {
        let source = slide_grid(15);
        let target: Vec<Point> = source.iter().map(|s| realistic(*s)).collect();
        PointWarp::fit(&source, &target, 0.0).unwrap()
    }

    #[test]
    fn passes_through_its_nodes_and_reproduces_linear_fields() {
        // Un champ affine est reproduit exactement, bords compris.
        let field = |p: Point| Point::new(0.5 + 0.01 * p.x, -0.02 * p.y + 0.003 * p.x);
        let grid = GridWarp::sample(Homography::IDENTITY, slide(), 8, 5, field).unwrap();
        assert_eq!(grid.size(), (8, 5));
        let mut rng = Lcg::new(6);
        for _ in 0..200 {
            let p = rng.point_in_slide();
            assert_close(grid.displacement(p), field(p), 1e-12);
        }
        for corner in [Point::new(0.0, 0.0), Point::new(280.0, 280.0)] {
            assert_close(grid.displacement(corner), field(corner), 1e-12);
        }
        // Un champ quelconque est reproduit aux nœuds.
        let grid = GridWarp::sample(Homography::IDENTITY, slide(), 8, 8, paper_wave).unwrap();
        for node in slide_grid(8) {
            assert_close(grid.displacement(node), paper_wave(node), 1e-12);
        }
    }

    #[test]
    fn jacobian_matches_finite_differences() {
        let grid = GridWarp::sample(trapezoid(), slide(), 29, 29, paper_wave).unwrap();
        let mut rng = Lcg::new(12);
        let h = 1e-5;
        for _ in 0..200 {
            let p = rng.point_in_slide();
            let (_, j) = grid.apply_with_jacobian(p).unwrap();
            let f = |q: Point| grid.apply(q).unwrap();
            let ddx = (f(p + Point::new(h, 0.0)) - f(p - Point::new(h, 0.0))) * (0.5 / h);
            let ddy = (f(p + Point::new(0.0, h)) - f(p - Point::new(0.0, h))) * (0.5 / h);
            assert_close(Point::new(j[0][0], j[1][0]), ddx, 1e-6);
            assert_close(Point::new(j[0][1], j[1][1]), ddy, 1e-6);
        }
    }

    #[test]
    fn sampling_error_is_measured() {
        // Écart entre la grille et la déformation par points qu'elle
        // remplace, sur 4 000 points au hasard, pour trois pas de grille.
        let warp = mire_warp();
        let mut rng = Lcg::new(31);
        let probes: Vec<Point> = (0..4000).map(|_| rng.point_in_slide()).collect();
        let exact: Vec<Point> = probes.iter().map(|p| warp.apply(*p).unwrap()).collect();
        let worst = |step: f64| -> f64 {
            let grid = GridWarp::from_point_warp(&warp, slide(), step).unwrap();
            probes
                .iter()
                .zip(&exact)
                .map(|(p, e)| grid.apply(*p).unwrap().distance(*e))
                .fold(0.0, f64::max)
        };
        let (coarse, medium, fine) = (worst(10.0), worst(5.0), worst(2.5));
        eprintln!(
            "écart grille / modèle exact : pas 10 mm {coarse:.2e} mm, \
             pas 5 mm {medium:.2e} mm, pas 2,5 mm {fine:.2e} mm"
        );
        assert!(fine < medium && medium < coarse);
        assert!(fine < 0.006, "pas de 2,5 mm : {fine} mm");
        assert!(medium < 0.025, "pas de 5 mm : {medium} mm");
    }

    #[test]
    fn round_trip_is_exact_on_the_whole_slide() {
        // Critère d'acceptation de la tâche 2.3 : aller-retour à moins de
        // 0,01 mm. Mesuré tous les millimètres sur le carré de 280 mm, bords
        // compris, dans les deux ordres.
        let grid = GridWarp::from_point_warp(&mire_warp(), slide(), 2.5).unwrap();
        assert_eq!(grid.size(), (113, 113));
        assert!(!has_fold(&grid, slide(), 280));
        let mut worst_forward_first = 0.0_f64;
        let mut worst_inverse_first = 0.0_f64;
        for j in 0..=280 {
            for i in 0..=280 {
                let p = Point::new(i as f64, j as f64);
                // Direct puis inverse : on doit revenir au point de départ.
                let there = grid.apply(p).unwrap();
                let back = grid.apply_inverse(there).unwrap();
                worst_forward_first = worst_forward_first.max(back.distance(p));
                // Inverse puis direct, en partant d'un point de la cible.
                let again = grid.apply(back).unwrap();
                worst_inverse_first = worst_inverse_first.max(again.distance(there));
            }
        }
        eprintln!(
            "aller-retour sur 78 961 points : direct puis inverse {worst_forward_first:.3e} mm, \
             inverse puis direct {worst_inverse_first:.3e} mm"
        );
        assert!(worst_forward_first < 1e-6, "{worst_forward_first} mm");
        assert!(worst_inverse_first < 1e-6, "{worst_inverse_first} mm");
    }

    #[test]
    fn round_trip_survives_a_harsher_warp() {
        // Trois fois plus d'ondulation que le cas réaliste (jusqu'à 10 mm),
        // et des points pris jusqu'à 20 mm hors de la diapositive.
        let harsh = |p: Point| paper_wave(p) * 3.0;
        let grid = GridWarp::sample(trapezoid(), slide(), 113, 113, harsh).unwrap();
        assert!(!has_fold(&grid, slide(), 280));
        let mut rng = Lcg::new(17);
        let mut worst = 0.0_f64;
        for _ in 0..20_000 {
            let p = rng.point_in_slide() * (320.0 / 280.0) - Point::new(20.0, 20.0);
            let back = grid.apply_inverse(grid.apply(p).unwrap()).unwrap();
            worst = worst.max(back.distance(p));
        }
        eprintln!("aller-retour, ondulation triplée : {worst:.3e} mm");
        assert!(worst < 1e-6, "{worst} mm");
    }

    #[test]
    fn a_folded_grid_is_detected_and_never_inverted_wrongly() {
        // Une bosse de 80 mm sur 60 mm de large : la pente dépasse 1, la
        // déformation se replie.
        let bump = |p: Point| {
            let d = p.distance(Point::new(140.0, 140.0)) / 30.0;
            Point::new(80.0 * (-d * d).exp(), 0.0)
        };
        let grid = GridWarp::sample(Homography::IDENTITY, slide(), 141, 141, bump).unwrap();
        assert!(has_fold(&grid, slide(), 280));
        let mut found = 0;
        for q in slide_grid(57) {
            if let Some(back) = grid.apply_inverse(q) {
                assert_close(grid.apply(back).unwrap(), q, 1e-6);
                found += 1;
            }
        }
        // Loin de la bosse, le sens inverse reste disponible.
        assert!(found > 2500, "{found} points inversés sur 3 249");
    }

    #[test]
    fn outside_the_grid_the_displacement_stays_constant() {
        let grid = GridWarp::sample(Homography::IDENTITY, slide(), 29, 29, paper_wave).unwrap();
        let edge = grid.displacement(Point::new(280.0, 100.0));
        assert_close(grid.displacement(Point::new(400.0, 100.0)), edge, 1e-12);
        let corner = grid.displacement(Point::new(0.0, 0.0));
        assert_close(grid.displacement(Point::new(-50.0, -9.0)), corner, 1e-12);
    }

    #[test]
    fn invalid_grids_are_refused() {
        let zero = |_: Point| Point::new(0.0, 0.0);
        assert!(GridWarp::sample(Homography::IDENTITY, slide(), 1, 5, zero).is_err());
        assert!(GridWarp::from_displacements(
            Homography::IDENTITY,
            slide(),
            3,
            3,
            vec![Point::new(0.0, 0.0); 8]
        )
        .is_err());
        assert!(GridWarp::from_point_warp(&mire_warp(), slide(), 0.0).is_err());
        assert!(GridWarp::from_point_warp(&mire_warp(), slide(), 1e-6).is_err());
        assert!(Rect::new(Point::new(1.0, 1.0), Point::new(1.0, 5.0)).is_err());
        let grid = GridWarp::sample(Homography::IDENTITY, slide(), 3, 3, zero).unwrap();
        assert_eq!(grid.apply(Point::new(f64::NAN, 1.0)), None);
    }
}

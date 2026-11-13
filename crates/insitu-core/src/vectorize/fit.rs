//! Lissage d'un contour brut : suite de droites et de courbes de Bézier
//! cubiques, chacune à moins de la tolérance des points du contour.
//!
//! Méthode :
//!
//! 1. Les **angles vifs** sont repérés sur une version simplifiée du contour
//!    (Douglas-Peucker à [`CORNER_SCALE_PX`]) : un sommet où la direction
//!    tourne de plus de l'angle choisi est un coin. Ailleurs, la courbe reste
//!    lisse (tangente continue).
//! 2. Entre deux coins, les points bruts sont approchés par une droite si
//!    elle passe à moins de la tolérance de tous, sinon par une cubique
//!    ajustée aux moindres carrés (méthode de Schneider, « An Algorithm for
//!    Automatically Fitting Digitized Curves », Graphics Gems, 1990). Si
//!    l'écart dépasse la tolérance, le morceau est coupé là où l'écart est le
//!    plus grand, avec une tangente commune aux deux moitiés.
//!
//! L'écart est mesuré dans les deux sens : de chaque point brut à la courbe
//! (au paramètre correspondant), et du milieu de la courbe entre deux points
//! au segment qui les relie.

use serde::Serialize;

use crate::geometry::Point;

/// Échelle de recherche des coins, en pixels : en dessous, les crénelages
/// d'une image sans anticrénelage (marches de 0,35 px) ne sont pas des coins.
pub const CORNER_SCALE_PX: f64 = 0.75;

/// Un morceau de contour, depuis la fin du précédent.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Segment {
    Line { to: Point },
    Cubic { c1: Point, c2: Point, to: Point },
}

impl Segment {
    pub fn end(&self) -> Point {
        match *self {
            Segment::Line { to } | Segment::Cubic { to, .. } => to,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FitParams {
    pub tolerance: f64,
    /// Cosinus de l'angle de virage au-delà duquel un sommet est un coin.
    pub corner_cos: f64,
}

/// Lisse un contour fermé : moyenne gaussienne des points voisins le long
/// du contour, d'écart type `sigma` (en points, soit à peu près en pixels :
/// les points du contour brut sont espacés d'environ un pixel), corrigée
/// pour ne pas rétrécir les courbes (« twicing » : 2G − G∘G, qui garde
/// les grandes formes et efface les petites ondulations). Un contour trop
/// court pour la fenêtre est lissé sur sa propre longueur.
pub(crate) fn smooth_closed(points: &[Point], sigma: f64) -> Vec<Point> {
    let n = points.len();
    if sigma <= 0.0 || n < 5 {
        return points.to_vec();
    }
    let once = gaussian_closed(points, sigma);
    let twice = gaussian_closed(&once, sigma);
    once.iter()
        .zip(&twice)
        .map(|(a, b)| Point {
            x: 2.0 * a.x - b.x,
            y: 2.0 * a.y - b.y,
        })
        .collect()
}

fn gaussian_closed(points: &[Point], sigma: f64) -> Vec<Point> {
    let n = points.len();
    let radius = ((3.0 * sigma).ceil() as usize).min((n - 1) / 2).max(1);
    let weights: Vec<f64> = (0..=radius)
        .map(|k| (-((k * k) as f64) / (2.0 * sigma * sigma)).exp())
        .collect();
    let total: f64 = weights[0] + 2.0 * weights[1..].iter().sum::<f64>();
    (0..n)
        .map(|i| {
            let mut x = weights[0] * points[i].x;
            let mut y = weights[0] * points[i].y;
            for (k, w) in weights.iter().enumerate().skip(1) {
                let before = points[(i + n - k) % n];
                let after = points[(i + k) % n];
                x += w * (before.x + after.x);
                y += w * (before.y + after.y);
            }
            Point {
                x: x / total,
                y: y / total,
            }
        })
        .collect()
}

/// Contour lissé : point de départ, morceaux (le dernier revient au départ),
/// et écart maximal mesuré pendant l'ajustement.
pub(crate) fn fit_closed(points: &[Point], params: FitParams) -> (Point, Vec<Segment>, f64) {
    let n = points.len();
    debug_assert!(n >= 3);
    let anchors = anchors(points, params);
    let mut segments = Vec::new();
    let mut max_error: f64 = 0.0;
    let count = anchors.len();
    for k in 0..count {
        let (a, sharp_a) = anchors[k];
        let (b, sharp_b) = anchors[(k + 1) % count];
        let len = if b > a { b - a } else { b + n - a };
        let run: Vec<Point> = (0..=len).map(|t| points[(a + t) % n]).collect();
        let t1 = if sharp_a {
            one_sided(&run)
        } else {
            centered_closed(points, a)
        };
        let t2 = if sharp_b {
            let reversed: Vec<Point> = run.iter().rev().copied().collect();
            one_sided(&reversed)
        } else {
            centered_closed(points, b) * -1.0
        };
        max_error = max_error.max(fit_run(&run, t1, t2, params.tolerance, &mut segments));
    }
    (points[anchors[0].0], segments, max_error)
}

/// Points d'ancrage du contour : les coins (vifs), ou à défaut deux points
/// éloignés où la courbe reste lisse.
fn anchors(points: &[Point], params: FitParams) -> Vec<(usize, bool)> {
    let n = points.len();
    let far = (1..n)
        .max_by(|&i, &j| {
            let di = points[i].distance(points[0]);
            let dj = points[j].distance(points[0]);
            di.total_cmp(&dj)
        })
        .unwrap_or(n / 2);
    let mut kept = vec![0];
    douglas_peucker(points, 0, far, CORNER_SCALE_PX, &mut kept);
    kept.push(far);
    douglas_peucker(points, far, n, CORNER_SCALE_PX, &mut kept);
    kept.sort_unstable();
    kept.dedup();
    let m = kept.len();
    let mut corners = Vec::new();
    if m >= 3 {
        for k in 0..m {
            let p = points[kept[(k + m - 1) % m]];
            let q = points[kept[k]];
            let r = points[kept[(k + 1) % m]];
            let (u, v) = (q - p, r - q);
            let (lu, lv) = (u.norm(), v.norm());
            if lu > 0.0 && lv > 0.0 {
                let cos = (u.x * v.x + u.y * v.y) / (lu * lv);
                if cos < params.corner_cos {
                    corners.push((kept[k], true));
                }
            }
        }
    }
    match corners.len() {
        0 => vec![(0, false), (far, false)],
        1 => {
            // Un seul coin : on ajoute un ancrage lisse à l'opposé, pour ne
            // pas ajuster une seule courbe qui revient sur elle-même.
            let c = corners[0].0;
            vec![corners[0], ((c + n / 2) % n, false)]
        }
        _ => corners,
    }
}

/// Simplification de Douglas-Peucker entre `first` et `last` (indices pris
/// modulo n) ; ajoute les sommets gardés, sans les extrémités.
pub(crate) fn douglas_peucker(
    points: &[Point],
    first: usize,
    last: usize,
    tol: f64,
    kept: &mut Vec<usize>,
) {
    let n = points.len();
    let mut stack = vec![(first, last)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let (pa, pb) = (points[a % n], points[b % n]);
        let mut best = (0.0, a);
        for k in a + 1..b {
            let d = distance_to_segment(points[k % n], pa, pb);
            if d > best.0 {
                best = (d, k);
            }
        }
        if best.0 > tol {
            kept.push(best.1 % n);
            stack.push((a, best.1));
            stack.push((best.1, b));
        }
    }
}

/// Distance de p au segment [a, b].
pub(crate) fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let len2 = ab.x * ab.x + ab.y * ab.y;
    if len2 == 0.0 {
        return p.distance(a);
    }
    let t = (((p.x - a.x) * ab.x + (p.y - a.y) * ab.y) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Portée des estimations de tangente, en pixels : au-delà du bruit d'un
/// contour brut, en deçà de la courbure d'un trait.
const TANGENT_SPREAD_PX: f64 = 2.0;

fn normalized(v: Point) -> Option<Point> {
    let len = v.norm();
    (len > 1e-12).then(|| v * (1.0 / len))
}

/// Tangente au départ d'une suite de points, vers l'avant.
fn one_sided(run: &[Point]) -> Point {
    let start = run[0];
    let limit = (run.len() - 1).div_ceil(2).max(1);
    let mut k = 1;
    while k < limit && run[k].distance(start) < TANGENT_SPREAD_PX {
        k += 1;
    }
    normalized(run[k] - start)
        .or_else(|| normalized(run[run.len() - 1] - start))
        .unwrap_or(Point::new(1.0, 0.0))
}

/// Tangente centrée en un point d'un contour fermé, dans le sens du parcours.
fn centered_closed(points: &[Point], i: usize) -> Point {
    let n = points.len();
    let center = points[i];
    let mut k = 1;
    while k < n / 4
        && points[(i + k) % n].distance(points[(i + n - k) % n]) < 2.0 * TANGENT_SPREAD_PX
    {
        k += 1;
    }
    normalized(points[(i + k) % n] - points[(i + n - k) % n])
        .or_else(|| normalized(points[(i + 1) % n] - center))
        .unwrap_or(Point::new(1.0, 0.0))
}

/// Tangente centrée à l'intérieur d'une suite, dans le sens du parcours.
fn centered_run(run: &[Point], i: usize) -> Point {
    let max_k = i.min(run.len() - 1 - i).max(1);
    let mut k = 1;
    while k < max_k && run[i + k].distance(run[i - k]) < 2.0 * TANGENT_SPREAD_PX {
        k += 1;
    }
    normalized(run[(i + k).min(run.len() - 1)] - run[i.saturating_sub(k)])
        .unwrap_or(Point::new(1.0, 0.0))
}

/// Ajuste une suite ouverte de points (extrémités comprises) ; `t1` est la
/// tangente de départ (vers l'avant), `t2` celle d'arrivée, tournée vers
/// l'arrière. Ajoute les morceaux dans l'ordre et renvoie l'écart maximal.
fn fit_run(run: &[Point], t1: Point, t2: Point, tol: f64, out: &mut Vec<Segment>) -> f64 {
    let mut max_error: f64 = 0.0;
    // Pile explicite plutôt que récursion : un contour peut compter des
    // dizaines de milliers de points.
    let mut stack = vec![(0usize, run.len() - 1, t1, t2)];
    while let Some((a, b, ta, tb)) = stack.pop() {
        let d = &run[a..=b];
        match fit_piece(d, ta, tb, tol) {
            Ok((segment, error)) => {
                out.push(segment);
                max_error = max_error.max(error);
            }
            Err(split) => {
                let s = a + split;
                let tc = centered_run(run, s);
                // La moitié gauche est traitée d'abord : on l'empile en dernier.
                stack.push((s, b, tc, tb));
                stack.push((a, s, ta, tc * -1.0));
            }
        }
    }
    max_error
}

/// Une droite courte posée sur un arc passe à l'intérieur de l'arc : elle
/// amincit ou épaissit le trait. Une droite de moins de [`LONG_LINE_PX`]
/// n'est donc acceptée qu'à [`SHORT_LINE_SHARE`] de la tolérance. Plus
/// longue, l'arc qu'elle remplacerait est presque droit (rayon de plus de
/// 100 px à 0,3 px de tolérance) : toute la tolérance est permise, ce qui
/// garde d'un seul tenant un bord droit dont les coins sont arrondis par
/// l'anticrénelage.
const SHORT_LINE_SHARE: f64 = 0.25;
const LONG_LINE_PX: f64 = 16.0;

/// Un seul morceau pour `d`, ou l'indice où couper.
fn fit_piece(d: &[Point], t1: Point, t2: Point, tol: f64) -> Result<(Segment, f64), usize> {
    let m = d.len() - 1;
    let (first, last) = (d[0], d[m]);
    let line_error = d
        .iter()
        .map(|&p| distance_to_segment(p, first, last))
        .fold(0.0, f64::max);
    let line_limit = if first.distance(last) >= LONG_LINE_PX {
        tol
    } else {
        SHORT_LINE_SHARE * tol
    };
    if m == 1 || line_error <= line_limit {
        return Ok((Segment::Line { to: last }, line_error));
    }
    let mut u = chord_parameters(d);
    let mut bezier = generate(d, &u, t1, t2);
    let (mut error, mut split) = max_error(d, &u, &bezier);
    if error <= tol {
        return Ok((bezier.segment(), error));
    }
    if error <= 4.0 * tol {
        for _ in 0..4 {
            reparameterize(d, &mut u, &bezier);
            bezier = generate(d, &u, t1, t2);
            (error, split) = max_error(d, &u, &bezier);
            if error <= tol {
                return Ok((bezier.segment(), error));
            }
        }
    }
    Err(split.clamp(1, m - 1))
}

#[derive(Debug, Clone, Copy)]
struct Bezier([Point; 4]);

impl Bezier {
    fn at(&self, t: f64) -> Point {
        let [p0, p1, p2, p3] = self.0;
        let s = 1.0 - t;
        p0 * (s * s * s) + p1 * (3.0 * s * s * t) + p2 * (3.0 * s * t * t) + p3 * (t * t * t)
    }

    fn derivative(&self, t: f64) -> Point {
        let [p0, p1, p2, p3] = self.0;
        let s = 1.0 - t;
        (p1 - p0) * (3.0 * s * s) + (p2 - p1) * (6.0 * s * t) + (p3 - p2) * (3.0 * t * t)
    }

    fn second(&self, t: f64) -> Point {
        let [p0, p1, p2, p3] = self.0;
        (p2 - p1 * 2.0 + p0) * (6.0 * (1.0 - t)) + (p3 - p2 * 2.0 + p1) * (6.0 * t)
    }

    fn segment(&self) -> Segment {
        Segment::Cubic {
            c1: self.0[1],
            c2: self.0[2],
            to: self.0[3],
        }
    }
}

fn chord_parameters(d: &[Point]) -> Vec<f64> {
    let mut u = Vec::with_capacity(d.len());
    let mut total = 0.0;
    u.push(0.0);
    for w in d.windows(2) {
        total += w[0].distance(w[1]);
        u.push(total);
    }
    if total > 0.0 {
        for v in &mut u {
            *v /= total;
        }
    }
    u
}

fn dot(a: Point, b: Point) -> f64 {
    a.x * b.x + a.y * b.y
}

/// Cubique aux moindres carrés, tangentes imposées aux extrémités.
fn generate(d: &[Point], u: &[f64], t1: Point, t2: Point) -> Bezier {
    let (first, last) = (d[0], d[d.len() - 1]);
    let (mut c00, mut c01, mut c11, mut x0, mut x1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (&p, &t) in d.iter().zip(u) {
        let s = 1.0 - t;
        let (b0, b1, b2, b3) = (s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t);
        let a1 = t1 * b1;
        let a2 = t2 * b2;
        c00 += dot(a1, a1);
        c01 += dot(a1, a2);
        c11 += dot(a2, a2);
        let rest = p - (first * (b0 + b1) + last * (b2 + b3));
        x0 += dot(a1, rest);
        x1 += dot(a2, rest);
    }
    let det = c00 * c11 - c01 * c01;
    let chord = first.distance(last);
    let (mut alpha1, mut alpha2) = if det.abs() > 1e-12 {
        ((x0 * c11 - c01 * x1) / det, (c00 * x1 - c01 * x0) / det)
    } else {
        (0.0, 0.0)
    };
    // Repli de Wu et Barsky : poignées au tiers de la corde. Il sert aussi
    // quand une poignée dépasse la longueur du chemin : la courbe ferait alors
    // une boucle entre deux points, que la mesure aux points ne verrait pas.
    let eps = 1e-6 * chord.max(1e-6);
    let arc: f64 = d.windows(2).map(|w| w[0].distance(w[1])).sum();
    let plausible = |alpha: f64| alpha.is_finite() && alpha > eps && alpha <= arc;
    if !(plausible(alpha1) && plausible(alpha2)) {
        alpha1 = chord / 3.0;
        alpha2 = chord / 3.0;
    }
    Bezier([first, first + t1 * alpha1, last + t2 * alpha2, last])
}

/// Écart maximal entre les points et la courbe, et où couper.
fn max_error(d: &[Point], u: &[f64], bezier: &Bezier) -> (f64, usize) {
    let m = d.len() - 1;
    let mut best = (0.0, m / 2);
    for i in 1..m {
        let e = bezier.at(u[i]).distance(d[i]);
        if e > best.0 {
            best = (e, i);
        }
    }
    // Entre deux points, la courbe ne doit pas s'écarter du segment qui les
    // relie : sinon un renflement passerait inaperçu.
    for i in 0..m {
        for f in [0.25, 0.5, 0.75] {
            let q = bezier.at(u[i] + f * (u[i + 1] - u[i]));
            let e = distance_to_segment(q, d[i], d[i + 1]);
            if e > best.0 {
                best = (e, if i == 0 { 1 } else { i });
            }
        }
    }
    best
}

/// Une itération de Newton : chaque paramètre vers le point de la courbe le
/// plus proche du point brut.
fn reparameterize(d: &[Point], u: &mut [f64], bezier: &Bezier) {
    for (p, t) in d.iter().zip(u.iter_mut()) {
        let q = bezier.at(*t) - *p;
        let q1 = bezier.derivative(*t);
        let q2 = bezier.second(*t);
        let num = dot(q, q1);
        let den = dot(q1, q1) + dot(q, q2);
        if den.abs() > 1e-12 {
            *t = (*t - num / den).clamp(0.0, 1.0);
        }
    }
}

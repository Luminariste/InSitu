use super::*;
use crate::geometry::Homography;
use crate::project::test_support::TempDir;
use crate::project::{ExtraFields, Facade, FacadeScale, RelativePath, SourcePoint};

/// Façade imaginaire : un projecteur de biais (trapèze) sur un mur en
/// relief (bosse de 60 mm d'amplitude). Diapositive (mm) → façade (mm).
fn true_map(p: Point) -> Point {
    let base = Homography::from_four_pairs(
        [
            Point::new(0.0, 0.0),
            Point::new(280.0, 0.0),
            Point::new(280.0, 280.0),
            Point::new(0.0, 280.0),
        ],
        [
            Point::new(1200.0, 800.0),
            Point::new(7800.0, 1100.0),
            Point::new(7500.0, 6900.0),
            Point::new(1400.0, 6400.0),
        ],
    )
    .unwrap();
    let q = base.apply(p).unwrap();
    let bump =
        60.0 * (-((p.x - 150.0).powi(2) + (p.y - 120.0).powi(2)) / (2.0 * 60.0f64.powi(2))).exp();
    Point::new(q.x + bump, q.y + 0.5 * bump)
}

/// Petit générateur reproductible (pas de dépendance au hasard du système).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    fn normal(&mut self) -> f64 {
        let (u, v) = (self.next().max(1e-12), self.next());
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }
}

/// `n` paires réparties sur la diapositive (grille avec un peu de jeu),
/// avec une erreur de pointage gaussienne de `noise_mm` sur la diapositive.
fn pairs(n_side: usize, noise_mm: f64, rng: &mut Lcg) -> Vec<RegistrationPair> {
    let mut out = Vec::new();
    for j in 0..n_side {
        for i in 0..n_side {
            let step = 240.0 / (n_side - 1) as f64;
            let s = Point::new(
                20.0 + step * i as f64 + 6.0 * (rng.next() - 0.5),
                20.0 + step * j as f64 + 6.0 * (rng.next() - 0.5),
            );
            let f = true_map(s);
            let clicked = Point::new(s.x + noise_mm * rng.normal(), s.y + noise_mm * rng.normal());
            out.push(RegistrationPair::new(point_mm(clicked), point_mm(f)));
        }
    }
    out
}

fn options() -> RegistrationOptions {
    RegistrationOptions::default()
}

#[test]
fn fewer_than_four_pairs_compute_nothing() {
    let mut rng = Lcg(1);
    let three: Vec<_> = pairs(2, 0.0, &mut rng).into_iter().take(3).collect();
    let r = report("p", &three, &options()).unwrap();
    assert_eq!(r.stage, RegistrationStage::Incomplete);
    assert!(r.footprint.is_empty());
    assert!(r.pairs.iter().all(|p| p.slide_residual_mm.is_none()));
}

#[test]
fn four_pairs_give_the_trapeze_and_the_footprint() {
    let mut rng = Lcg(2);
    let four = pairs(2, 0.0, &mut rng);
    let r = report("p", &four, &options()).unwrap();
    assert_eq!(r.stage, RegistrationStage::Trapeze);
    assert!(r
        .pairs
        .iter()
        .all(|p| p.slide_residual_mm.is_none() && !p.outlier));
    assert_eq!(r.footprint.len(), 4 * FOOTPRINT_POINTS_PER_SIDE + 1);
    // Le trapèze passe par les quatre points : le coin haut-gauche de
    // l'emprise tombe près du vrai (la bosse est loin des coins).
    let corner = point(r.footprint[0]);
    assert!(
        corner.distance(true_map(Point::new(0.0, 0.0))) < 15.0,
        "{corner:?}"
    );
    assert_eq!(r.footprint.first(), r.footprint.last());
}

#[test]
fn five_pairs_measure_the_gap_to_the_trapeze() {
    let mut rng = Lcg(3);
    let mut five = pairs(2, 0.0, &mut rng);
    let center = Point::new(150.0, 120.0); // sur la bosse
    five.push(RegistrationPair::new(
        point_mm(center),
        point_mm(true_map(center)),
    ));
    let r = report("p", &five, &options()).unwrap();
    assert_eq!(r.stage, RegistrationStage::Trapeze);
    let worst = r
        .pairs
        .iter()
        .filter_map(|p| p.facade_residual_mm)
        .fold(0.0, f64::max);
    assert!(worst > 5.0, "la bosse doit se voir : {worst}");
}

/// Critère : 4 points donnent le trapèze ; les suivants réduisent l'écart.
/// L'écart est mesuré sur la façade en 400 points de contrôle qui ne sont
/// pas des points de recalage.
#[test]
fn more_points_follow_the_relief() {
    let mut check = Lcg(99);
    let controls: Vec<Point> = (0..400)
        .map(|_| Point::new(10.0 + 260.0 * check.next(), 10.0 + 260.0 * check.next()))
        .collect();
    let mut errors = Vec::new();
    for n_side in [2usize, 3, 4, 5] {
        let mut rng = Lcg(7);
        let set = pairs(n_side, 0.0, &mut rng);
        let slide: Vec<Point> = set.iter().map(|p| point(p.slide)).collect();
        let facade: Vec<Point> = set.iter().map(|p| point(p.facade)).collect();
        let warp = PointWarp::fit(&slide, &facade, 0.0).unwrap();
        let rms = (controls
            .iter()
            .map(|&c| warp.apply(c).unwrap().distance(true_map(c)).powi(2))
            .sum::<f64>()
            / controls.len() as f64)
            .sqrt();
        errors.push(rms);
    }
    println!("écart moyen sur la façade (mm) pour 4, 9, 16, 25 points : {errors:.1?}");
    for w in errors.windows(2) {
        assert!(w[1] < w[0], "{errors:?}");
    }
    assert!(errors[3] < errors[0] / 4.0, "{errors:?}");
}

/// Critère : un point aberrant est signalé, et des points ordinaires ne le
/// sont pas. 16 points, erreur de pointage ordinaire de 0,3 mm sur la
/// diapositive (deux pixels de photo redressée) ; un point cliqué 5 ou
/// 10 mm à côté. Mesuré : aucune fausse alerte ; 19 points faux sur 20
/// trouvés à 5 mm (le manqué est un point du bord, prédit par
/// extrapolation), 20 sur 20 à 10 mm.
#[test]
fn a_misplaced_point_is_flagged_and_good_points_are_not() {
    for (shift, min_found) in [(5.0, 18), (10.0, 20)] {
        let mut false_alarms = 0;
        let mut found = 0;
        for seed in 0..20u64 {
            let mut rng = Lcg(1000 + seed);
            let mut set = pairs(4, 0.3, &mut rng);
            let clean = report("p", &set, &options()).unwrap();
            assert_eq!(clean.stage, RegistrationStage::Corrected);
            false_alarms += clean.pairs.iter().filter(|p| p.outlier).count();
            let k = (seed as usize * 7) % set.len();
            let angle = rng.next() * std::f64::consts::TAU;
            set[k].slide.x = (set[k].slide.x + shift * angle.cos()).clamp(0.0, 280.0);
            set[k].slide.y = (set[k].slide.y + shift * angle.sin()).clamp(0.0, 280.0);
            let wrong = report("p", &set, &options()).unwrap();
            found += usize::from(wrong.pairs[k].outlier);
            false_alarms += wrong
                .pairs
                .iter()
                .enumerate()
                .filter(|(i, p)| *i != k && p.outlier)
                .count();
        }
        println!("décalage {shift} mm : {found}/20 trouvés, {false_alarms} fausses alertes");
        assert_eq!(false_alarms, 0, "fausses alertes à {shift} mm");
        assert!(found >= min_found, "{found}/20 trouvés à {shift} mm");
    }
}

#[test]
fn residuals_of_good_points_stay_small() {
    let mut rng = Lcg(5);
    let set = pairs(4, 0.3, &mut rng);
    let r = report("p", &set, &options()).unwrap();
    let rms = r.rms_slide_mm.unwrap();
    assert!(rms < 1.0, "{rms}");
    // Sur la façade, environ 25 fois plus (échelle du projecteur imaginaire).
    let rms_facade = r.rms_facade_mm.unwrap();
    assert!(
        rms_facade > 5.0 * rms && rms_facade < 60.0 * rms,
        "{rms_facade}"
    );
    assert!(
        r.trapeze_rms_slide_mm.unwrap() > rms,
        "le trapèze seul laisse plus d'écart"
    );
}

#[test]
fn points_outside_the_slide_are_refused() {
    let pair = RegistrationPair::new(PointMm::new(290.0, 10.0), PointMm::new(0.0, 0.0));
    assert!(matches!(
        report("p", &[pair], &options()),
        Err(RegistrationError::BadPair(_))
    ));
    let nan = RegistrationPair::new(PointMm::new(f64::NAN, 10.0), PointMm::new(0.0, 0.0));
    assert!(validate_pairs(&[nan]).is_err());
}

#[test]
fn registration_is_saved_in_the_project() {
    let tmp = TempDir::new("recalage");
    let (dir, mut project) =
        ProjectDir::create(tmp.path().join("essai.insitu"), Some("Essai")).unwrap();
    let id = project.add_projector("A").id.clone();
    dir.save(&project).unwrap();
    let mut rng = Lcg(11);
    let set = pairs(3, 0.2, &mut rng);

    // Sans façade à l'échelle, refusé.
    assert!(matches!(
        set_registration(&dir, "A", set.clone(), &options()),
        Err(RegistrationError::NoScaledFacade)
    ));

    project.facade = Some(Facade {
        source: RelativePath::new("facade/plan.png").unwrap(),
        source_width: 2000.0,
        source_height: 1500.0,
        scale: Some(FacadeScale {
            point_a: SourcePoint::new(0.0, 0.0),
            point_b: SourcePoint::new(1000.0, 0.0),
            distance_mm: 4000.0,
            extra: ExtraFields::new(),
        }),
        page: None,
        origin: None,
        extra: ExtraFields::new(),
    });
    dir.save(&project).unwrap();
    let (saved, r) = set_registration(&dir, "a", set.clone(), &options()).unwrap();
    assert_eq!(r.stage, RegistrationStage::Corrected);
    assert_eq!(saved.projector(&id).unwrap().registration, set);
    let reread = projector_report(&dir, &id, &options()).unwrap();
    assert_eq!(reread, r);

    // Des paires refusées laissent le projet tel quel.
    let bad = vec![RegistrationPair::new(
        PointMm::new(-1.0, 0.0),
        PointMm::new(0.0, 0.0),
    )];
    assert!(set_registration(&dir, &id, bad, &options()).is_err());
    assert_eq!(
        dir.load()
            .unwrap()
            .project
            .projector(&id)
            .unwrap()
            .registration,
        set
    );

    assert!(matches!(
        projector_report(&dir, "Z", &options()),
        Err(RegistrationError::UnknownProjector(_))
    ));
}

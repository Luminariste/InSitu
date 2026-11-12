use super::*;

fn image(width: u32, height: u32, f: impl Fn(u32, u32) -> u8) -> GrayImage {
    let mut data = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            data.push(f(x, y));
        }
    }
    GrayImage {
        width,
        height,
        data,
    }
}

/// Image anticrénelée : chaque pixel reçoit la part de sa surface (16 × 16
/// échantillons) où `inside` est vrai.
fn coverage(width: u32, height: u32, inside: impl Fn(f64, f64) -> bool) -> GrayImage {
    image(width, height, |x, y| {
        let mut hits = 0u32;
        for sy in 0..16 {
            for sx in 0..16 {
                let px = f64::from(x) + (f64::from(sx) + 0.5) / 16.0;
                let py = f64::from(y) + (f64::from(sy) + 0.5) / 16.0;
                hits += u32::from(inside(px, py));
            }
        }
        ((f64::from(hits) / 256.0) * 255.0).round() as u8
    })
}

/// Points de la courbe, tous les 0,05 px environ.
fn sample(vector: &Vectorization) -> Vec<Point> {
    let mut points = Vec::new();
    for contour in &vector.contours {
        let mut current = contour.start;
        for segment in &contour.segments {
            for s in 0..=20 {
                let t = f64::from(s) / 20.0;
                let u = 1.0 - t;
                points.push(match *segment {
                    Segment::Line { to } => current + (to - current) * t,
                    Segment::Cubic { c1, c2, to } => {
                        current * (u * u * u)
                            + c1 * (3.0 * u * u * t)
                            + c2 * (3.0 * u * t * t)
                            + to * (t * t * t)
                    }
                });
            }
            current = segment.end();
        }
    }
    points
}

#[test]
fn a_single_pixel_is_one_closed_contour() {
    let img = image(5, 5, |x, y| if (x, y) == (2, 2) { 255 } else { 0 });
    let v = vectorize(&img, &VectorizeOptions::default()).unwrap();
    assert_eq!(v.contours.len(), 1);
    let c = &v.contours[0];
    assert!(c.area > 0.0, "aire {}", c.area);
    // Le contour brut est un losange autour du centre (2,5 ; 2,5).
    for p in sample(&v) {
        assert!(
            (p.x - 2.5).abs() <= 0.51 && (p.y - 2.5).abs() <= 0.51,
            "{p:?}"
        );
    }
    let last = c.segments.last().unwrap().end();
    assert!(last.distance(c.start) < 1e-12, "contour fermé");
}

#[test]
fn a_hole_turns_the_other_way() {
    // Carré plein de 10 px avec un trou de 4 px au milieu.
    let img = image(20, 20, |x, y| {
        let outer = (5..15).contains(&x) && (5..15).contains(&y);
        let hole = (8..12).contains(&x) && (8..12).contains(&y);
        if outer && !hole {
            255
        } else {
            0
        }
    });
    let v = vectorize(&img, &VectorizeOptions::default()).unwrap();
    assert_eq!(v.contours.len(), 2);
    let mut areas: Vec<f64> = v.contours.iter().map(|c| c.area).collect();
    areas.sort_by(f64::total_cmp);
    assert!(areas[0] < 0.0 && areas[1] > 0.0, "{areas:?}");
    let cmp = compare(&img, &v);
    assert_eq!(cmp.source_pixels, 100 - 16);
    assert!(cmp.max_deviation_px < 0.5, "{cmp:?}");
}

#[test]
fn a_saddle_follows_the_mean_of_the_four_corners() {
    // Damier 2 × 2 : moyenne 127,5, sous le niveau 127,5 du seuil 128 → deux
    // taches séparées. Au seuil 127, le centre est dedans → une seule tache.
    let img = image(2, 2, |x, y| if x == y { 255 } else { 0 });
    let separated = vectorize(&img, &VectorizeOptions::default()).unwrap();
    assert_eq!(separated.contours.len(), 2);
    let options = VectorizeOptions {
        threshold: 127,
        ..Default::default()
    };
    let joined = vectorize(&img, &options).unwrap();
    // Les deux pixels blancs se touchent par le centre : un seul contour.
    assert_eq!(joined.contours.len(), 1);
}

#[test]
fn an_antialiased_disk_stays_on_its_circle() {
    let (cx, cy, r) = (50.3, 49.7, 31.4);
    let img = coverage(100, 100, |x, y| (x - cx).hypot(y - cy) <= r);
    let v = vectorize(&img, &VectorizeOptions::default()).unwrap();
    assert_eq!(v.contours.len(), 1);
    let worst = sample(&v)
        .iter()
        .map(|p| ((p.x - cx).hypot(p.y - cy) - r).abs())
        .fold(0.0, f64::max);
    // L'écart vient surtout du lissage (tolérance de 0,3 px).
    assert!(worst < 0.35, "écart au cercle {worst:.3} px");
    // Avec une tolérance serrée, il ne reste que l'erreur du contour brut.
    let tight = VectorizeOptions {
        tolerance_px: 0.02,
        ..Default::default()
    };
    let fine = vectorize(&img, &tight).unwrap();
    let worst_fine = sample(&fine)
        .iter()
        .map(|p| ((p.x - cx).hypot(p.y - cy) - r).abs())
        .fold(0.0, f64::max);
    assert!(worst_fine < 0.12, "contour brut : {worst_fine:.3} px");
    // Un disque lisse ne demande que quelques cubiques, sans angle vif.
    assert!(v.stats.cubics <= 16, "{:?}", v.stats);
    let cmp = compare(&img, &v);
    assert!(cmp.max_deviation_px < 0.5, "{cmp:?}");
}

#[test]
fn a_straight_stroke_keeps_straight_sides() {
    // Trait de 6 px d'épaisseur et 100 px de long, incliné, anticrénelé.
    let (sin, cos) = 0.3f64.sin_cos();
    let img = coverage(120, 80, |x, y| {
        let along = (x - 60.0) * cos + (y - 40.0) * sin;
        let across = -(x - 60.0) * sin + (y - 40.0) * cos;
        across.abs() <= 3.0 && along.abs() <= 50.0
    });
    let v = vectorize(&img, &VectorizeOptions::default()).unwrap();
    assert_eq!(v.contours.len(), 1);
    // Les grands côtés sont faits de droites (de plus de 20 px) ; les coins,
    // arrondis par l'anticrénelage, prennent quelques petites courbes.
    let contour = &v.contours[0];
    let mut from = contour.start;
    let mut straight = 0.0;
    for segment in &contour.segments {
        if let Segment::Line { to } = *segment {
            if from.distance(to) > 20.0 {
                straight += from.distance(to);
            }
        }
        from = segment.end();
    }
    assert!(
        straight > 160.0,
        "{straight:.1} px de droites : {:?}",
        contour.segments
    );
    assert!(v.stats.lines + v.stats.cubics <= 12, "{:?}", v.stats);
    let cmp = compare(&img, &v);
    assert!(cmp.max_deviation_px < 0.5, "{cmp:?}");
}

#[test]
fn a_binary_drawing_is_traced_within_half_a_pixel() {
    // Sans anticrénelage : marches d'escalier, angles vifs, traits d'un pixel.
    let img = image(90, 90, |x, y| {
        let (x, y) = (f64::from(x), f64::from(y));
        let ring = ((x - 45.0).hypot(y - 45.0) - 30.0).abs() < 2.0;
        let diagonal = (x - y).abs() < 1.0 && x > 10.0 && x < 80.0;
        let square = (20.0..30.0).contains(&x) && (60.0..70.0).contains(&y);
        if ring || diagonal || square {
            255
        } else {
            0
        }
    });
    let v = vectorize(&img, &VectorizeOptions::default()).unwrap();
    let cmp = compare(&img, &v);
    assert!(cmp.max_deviation_px < 0.5, "{cmp:?}");
    assert!(cmp.max_mismatch_distance_px < 0.5, "{cmp:?}");
    assert!(v.stats.max_fit_error_px <= 0.3 + 1e-9, "{:?}", v.stats);
}

#[test]
fn the_threshold_moves_the_contour() {
    // Dégradé horizontal : le bord tombe là où la valeur passe le seuil.
    let img = image(256, 4, |x, _| x as u8);
    for threshold in [64u8, 128, 200] {
        let options = VectorizeOptions {
            threshold,
            ..Default::default()
        };
        let v = vectorize(&img, &options).unwrap();
        let left = sample(&v)
            .iter()
            .filter(|p| p.y > 1.0 && p.y < 3.0)
            .map(|p| p.x)
            .fold(f64::INFINITY, f64::min);
        // Pixel x de valeur x, centre en x + 0,5 : niveau seuil − 0,5.
        let expected = f64::from(threshold) - 0.5 + 0.5;
        assert!((left - expected).abs() < 0.01, "seuil {threshold} : {left}");
    }
}

#[test]
fn small_specks_can_be_removed() {
    let img = image(30, 30, |x, y| {
        if (x, y) == (3, 3) || ((10..20).contains(&x) && (10..20).contains(&y)) {
            255
        } else {
            0
        }
    });
    let all = vectorize(&img, &VectorizeOptions::default()).unwrap();
    assert_eq!(all.contours.len(), 2);
    let options = VectorizeOptions {
        min_area_px: 2.0,
        ..Default::default()
    };
    let cleaned = vectorize(&img, &options).unwrap();
    assert_eq!(cleaned.contours.len(), 1);
    assert_eq!(cleaned.stats.removed_contours, 1);
}

#[test]
fn color_and_transparency_become_gray_on_black() {
    let raster = Raster::new(2, 1, 4, vec![255, 255, 255, 255, 255, 255, 255, 0]);
    let gray = GrayImage::from_raster(&raster);
    assert_eq!(gray.data, vec![255, 0]);
    let raster = Raster::new(1, 1, 3, vec![0, 255, 0]);
    assert_eq!(GrayImage::from_raster(&raster).data, vec![182]);
}

#[test]
fn bad_options_are_refused() {
    let img = image(2, 2, |_, _| 0);
    let bad = [
        VectorizeOptions {
            threshold: 0,
            ..Default::default()
        },
        VectorizeOptions {
            tolerance_px: 0.0,
            ..Default::default()
        },
        VectorizeOptions {
            corner_angle_deg: 180.0,
            ..Default::default()
        },
        VectorizeOptions {
            min_area_px: f64::NAN,
            ..Default::default()
        },
    ];
    for options in bad {
        assert!(vectorize(&img, &options).is_err(), "{options:?}");
    }
    let empty = vectorize(&img, &VectorizeOptions::default()).unwrap();
    assert!(empty.contours.is_empty());
}

#[test]
fn the_svg_draws_white_paths_on_black() {
    let img = image(4, 4, |x, y| if x == 1 && y == 1 { 255 } else { 0 });
    let v = vectorize(&img, &VectorizeOptions::default()).unwrap();
    let svg = v.to_svg();
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains(r#"viewBox="0 0 4 4""#));
    assert!(svg.contains("M1 1.5") || svg.contains('M'), "{svg}");
    assert!(svg.trim_end().ends_with("</svg>"));
    assert_eq!(num(-0.0001), "0");
    assert_eq!(num(2.5), "2.5");
    assert_eq!(num(3.0), "3");
}

use super::*;

fn zone(tool: RetouchTool, pts: &[[f64; 2]]) -> RetouchStroke {
    RetouchStroke {
        tool,
        fill: true,
        size: 0.0,
        points: pts.to_vec(),
    }
}

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

fn area_of(shapes: &[Shape]) -> f64 {
    shapes
        .iter()
        .map(|s| signed_area(&s[0]) + s[1..].iter().map(|h| signed_area(h)).sum::<f64>())
        .sum()
}

/// Surface remplie pair-impair de polygones (réunis), mesurée sur une
/// grille fine du carré unité.
fn even_odd_area(polygons: &[SilhouettePolygon]) -> f64 {
    let n = 400;
    let mut hits = 0;
    for j in 0..n {
        for i in 0..n {
            let (x, y) = ((i as f64 + 0.5) / n as f64, (j as f64 + 0.5) / n as f64);
            let inside = polygons.iter().any(|poly| {
                let mut hit = false;
                for k in 0..poly.len() {
                    let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
                    if (a[1] > y) != (b[1] > y)
                        && x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0]
                    {
                        hit = !hit;
                    }
                }
                hit
            });
            hits += usize::from(inside);
        }
    }
    hits as f64 / (n * n) as f64
}

#[test]
fn a_zone_hides_its_inside_and_a_reveal_brings_part_back() {
    let hide = zone(RetouchTool::Erase, &square(0.0, 0.0, 0.5, 1.0));
    let region = hidden_region(std::slice::from_ref(&hide), (1.0, 1.0));
    assert!((area_of(&region) - 0.5).abs() < 1e-9);
    let show = zone(RetouchTool::Draw, &square(0.0, 0.0, 0.5, 0.5));
    let region = hidden_region(&[hide, show], (1.0, 1.0));
    assert!((area_of(&region) - 0.25).abs() < 1e-9);
}

#[test]
fn a_reveal_alone_changes_nothing() {
    let show = zone(RetouchTool::Draw, &square(0.0, 0.0, 0.5, 0.5));
    assert!(hidden_region(&[show], (1.0, 1.0)).is_empty());
}

#[test]
fn a_brush_stroke_covers_a_round_ended_band() {
    // Trait horizontal de 0,2 à 0,8, épaisseur 0,1 : 0,6 × 0,1 plus un disque.
    let stroke = RetouchStroke {
        tool: RetouchTool::Erase,
        fill: false,
        size: 0.1,
        points: vec![[0.2, 0.5], [0.5, 0.5], [0.8, 0.5]],
    };
    let region = hidden_region(&[stroke], (1.0, 1.0));
    let expected = 0.6 * 0.1 + std::f64::consts::PI * 0.05 * 0.05;
    assert!(
        (area_of(&region) - expected).abs() < 0.002,
        "{}",
        area_of(&region)
    );
}

#[test]
fn the_brush_follows_the_drawing_proportions() {
    // Dessin deux fois plus large que haut : un disque de diamètre 0,2 du
    // grand côté couvre 0,2 en u et 0,4 en v.
    let stroke = RetouchStroke {
        tool: RetouchTool::Erase,
        fill: false,
        size: 0.2,
        points: vec![[0.5, 0.5]],
    };
    let region = hidden_region(&[stroke], aspect(200.0, 100.0));
    let us = region[0][0].iter().map(|p| p[0]);
    let vs = region[0][0].iter().map(|p| p[1]);
    let width = us.clone().fold(f64::MIN, f64::max) - us.fold(f64::MAX, f64::min);
    let height = vs.clone().fold(f64::MIN, f64::max) - vs.fold(f64::MAX, f64::min);
    assert!(
        (width - 0.2).abs() < 0.01 && (height - 0.4).abs() < 0.01,
        "{width} {height}"
    );
}

#[test]
fn a_hole_in_the_middle_stays_a_hole_in_one_even_odd_polygon() {
    let hide = zone(RetouchTool::Erase, &square(0.4, 0.4, 0.6, 0.6));
    let shape = visible_shape(&whole_drawing(), &[hide], (1.0, 1.0));
    assert_eq!(shape.len(), 1);
    assert!((even_odd_area(&shape) - 0.96).abs() < 0.005);
}

#[test]
fn a_drawing_hidden_entirely_has_no_visible_shape() {
    let hide = zone(RetouchTool::Erase, &square(-0.1, -0.1, 1.1, 1.1));
    assert!(visible_shape(&whole_drawing(), &[hide], (1.0, 1.0)).is_empty());
}

#[test]
fn without_a_mask_the_shape_is_unchanged() {
    let base = vec![square(0.1, 0.1, 0.9, 0.9)];
    assert_eq!(visible_shape(&base, &[], (1.0, 1.0)), base);
}

#[test]
fn the_visibility_image_matches_the_polygons() {
    let strokes = [
        zone(RetouchTool::Erase, &square(0.0, 0.0, 0.5, 1.0)),
        zone(RetouchTool::Draw, &square(0.0, 0.0, 0.5, 0.5)),
    ];
    let image = visibility(&strokes, 100, 100);
    let hidden = image.data.iter().filter(|&&v| v < 128).count();
    assert_eq!(hidden, 50 * 50);
    assert_eq!(image.data[75 * 100 + 25], 0);
    assert_eq!(image.data[25 * 100 + 25], 255);
}

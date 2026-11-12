//! La nouvelle mire (version 2) est détectée sans aide, et dans le bon sens.
//!
//! Elle n'a plus les six petits cercles qui donnaient le sens de l'ancienne
//! (D-2.2-c) : le sens se lit sur ses repères codés (D-essai-a). Sans cela,
//! la détection seule échouait toujours, et la pose des coins à la main
//! donnait une diapositive tournée d'un quart de tour sans prévenir.
//!
//! La photo est simulée : le PDF à imprimer (`docs/mire/mire-insitu-v2.pdf`)
//! rendu par le moteur, tourné de 0 à 3 quarts de tour et posé sur un fond
//! sombre, comme une feuille photographiée d'aplomb.

use std::path::Path;

use image::{Rgb, RgbImage};
use insitu_core::facade::display::render_whole;
use insitu_core::geometry::Point;
use insitu_core::rectification::{self, DetectOptions, TargetDetection};

/// Page A3 de la mire : 297 × 420 mm, carré de 280 mm au centre.
const PAGE_W_MM: f64 = 297.0;
const PAGE_H_MM: f64 = 420.0;
const OFFSET: (u32, u32) = (300, 200);

struct Photo {
    image: RgbImage,
    /// px par mm de page.
    scale: f64,
    /// Quarts de tour (sens des aiguilles d'une montre) de la page.
    turns: u8,
    /// Taille de la page rendue avant rotation, en px.
    page_px: (f64, f64),
}

impl Photo {
    fn new(turns: u8) -> Photo {
        let pdf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/mire/mire-insitu-v2.pdf");
        let (w, h, rgba) = render_whole(&pdf, None, turns, 2400).expect("mire rendue");
        let mut image = RgbImage::from_pixel(w + 2 * OFFSET.0, h + 2 * OFFSET.1, Rgb([25, 25, 25]));
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                let a = f64::from(rgba[i + 3]) / 255.0;
                // Fond blanc sous la page (le rendu peut être transparent).
                let c = |k: usize| (f64::from(rgba[i + k]) + 255.0 * (1.0 - a)).min(255.0) as u8;
                image.put_pixel(x + OFFSET.0, y + OFFSET.1, Rgb([c(0), c(1), c(2)]));
            }
        }
        let long = f64::from(w.max(h));
        let scale = long / PAGE_H_MM;
        Photo {
            image,
            scale,
            turns,
            page_px: (PAGE_W_MM * scale, PAGE_H_MM * scale),
        }
    }

    /// Où tombe, dans la photo, un point de la diapositive (mm).
    fn photo_of(&self, slide: Point) -> Point {
        let mut p = Point::new(
            (slide.x + (PAGE_W_MM - 280.0) / 2.0) * self.scale,
            (slide.y + (PAGE_H_MM - 280.0) / 2.0) * self.scale,
        );
        let (mut w, mut h) = self.page_px;
        for _ in 0..self.turns {
            // Quart de tour dans le sens des aiguilles d'une montre.
            p = Point::new(h - p.y, p.x);
            std::mem::swap(&mut w, &mut h);
        }
        Point::new(p.x + f64::from(OFFSET.0), p.y + f64::from(OFFSET.1))
    }

    /// Écart médian (px) entre les croisements trouvés et leur vraie place.
    fn median_error(&self, d: &TargetDetection) -> f64 {
        let mut errors: Vec<f64> = d
            .nodes
            .iter()
            .map(|n| n.photo.distance(self.photo_of(n.slide)))
            .collect();
        assert!(errors.len() > 300, "{} croisements seulement", errors.len());
        errors.sort_by(f64::total_cmp);
        errors[errors.len() / 2]
    }
}

#[test]
fn new_target_is_found_alone_in_the_right_direction() {
    for turns in 0..4 {
        let photo = Photo::new(turns);
        let d = rectification::detect(&photo.image, &DetectOptions::default())
            .unwrap_or_else(|e| panic!("{turns} quart(s) de tour : {e}"));
        let err = photo.median_error(&d);
        // Un mauvais sens donnerait des centaines de pixels d'écart.
        assert!(
            err < 3.0,
            "{turns} quart(s) de tour : écart médian {err:.1} px"
        );
    }
}

#[test]
fn new_target_from_corners_is_in_the_right_direction() {
    for turns in 0..4 {
        let photo = Photo::new(turns);
        // Coins donnés dans le désordre, à 1 mm près.
        let s = 280.0;
        let corners = [
            Point::new(s - 1.0, s),
            Point::new(1.0, 0.0),
            Point::new(0.0, s - 1.0),
            Point::new(s, 1.0),
        ]
        .map(|p| photo.photo_of(p));
        let options = DetectOptions {
            corners: Some(corners),
            ..DetectOptions::default()
        };
        let d = rectification::detect(&photo.image, &options)
            .unwrap_or_else(|e| panic!("{turns} quart(s) de tour : {e}"));
        let err = photo.median_error(&d);
        assert!(
            err < 3.0,
            "{turns} quart(s) de tour : écart médian {err:.1} px"
        );
    }
}

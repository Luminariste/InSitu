//! Commande `insitu vectorize` : vectorise un dessin et mesure l'écart avec
//! l'image, sans écran.

use std::path::Path;
use std::time::Instant;

use insitu_core::tiles::read_raster;
use insitu_core::vectorize::{compare, vectorize, GrayImage, VectorizeOptions};
use serde_json::json;

/// Ce qui peut mal tourner.
pub enum Problem {
    Usage(String),
    Failed(String),
}

pub const USAGE: &str = r#"Usage : insitu vectorize <image> [--svg <sortie.svg>] [options]

Vectorise un dessin (trait clair sur fond sombre) : contours suivis à pleine
résolution, puis lissés en droites et courbes de Bézier. Une image en couleur
est ramenée à sa luminance ; la transparence compte comme du noir.

  --svg F          Écrit le résultat en SVG (pixels de l'image, blanc sur noir).
  --threshold N    Seuil de 1 à 255 : un pixel au moins aussi clair est dans le
                   trait (128 par défaut).
  --tolerance P    Écart maximal toléré entre les courbes et le contour de
                   l'image, en pixels (0,3 par défaut). Plus grand : plus lisse,
                   moins de points.
  --corner A       Angle en degrés au-delà duquel un virage est un angle vif
                   (70 par défaut).
  --min-area A     Ignore les taches et trous de moins de A pixels carrés
                   (0 par défaut : tout est gardé).
  --threads N      Nombre de fils de calcul (tous les cœurs par défaut).
  --check          Mesure ensuite l'écart avec l'image (critère : moins d'un
                   demi-pixel), indépendamment de l'ajustement.

Affiche en JSON les chiffres de la vectorisation, la durée et, avec --check,
la mesure de l'écart."#;

pub fn run(args: &[&str]) -> Result<String, Problem> {
    if matches!(args, [] | ["--help" | "-h"]) {
        return Ok(USAGE.to_string());
    }
    let usage = |message: &str| Problem::Usage(format!("{message} Voir : insitu vectorize --help"));
    let mut options = VectorizeOptions::default();
    let mut svg = None;
    let mut check = false;
    let mut paths = Vec::new();
    let mut rest = args.iter().copied();
    while let Some(arg) = rest.next() {
        let mut number = |what: &str| -> Result<f64, Problem> {
            rest.next()
                .and_then(|v| v.replace(',', ".").parse::<f64>().ok())
                .filter(|v| v.is_finite())
                .ok_or_else(|| usage(&format!("{arg} attend {what}.")))
        };
        match arg {
            "--svg" => {
                svg = Some(
                    rest.next()
                        .ok_or_else(|| usage("--svg attend un fichier."))?,
                );
            }
            "--threshold" => {
                let v = number("un nombre de 1 à 255")?;
                if !(1.0..=255.0).contains(&v) || v.fract() != 0.0 {
                    return Err(usage("--threshold attend un nombre entier de 1 à 255."));
                }
                options.threshold = v as u8;
            }
            "--tolerance" => options.tolerance_px = number("un nombre de pixels")?,
            "--corner" => options.corner_angle_deg = number("un angle en degrés")?,
            "--min-area" => options.min_area_px = number("une aire en pixels carrés")?,
            "--threads" => options.threads = number("un nombre")?.max(0.0) as usize,
            "--check" => check = true,
            other if other.starts_with("--") => {
                return Err(usage(&format!("Option inconnue : {other}")));
            }
            path => paths.push(path),
        }
    }
    let [image] = paths[..] else {
        return Err(usage("Il faut une image, et une seule."));
    };

    let raster = read_raster(Path::new(image)).map_err(|e| Problem::Failed(e.to_string()))?;
    let gray = GrayImage::from_raster(&raster);
    drop(raster);
    let start = Instant::now();
    let vector = vectorize(&gray, &options)
        .map_err(|e| Problem::Failed(format!("Vectorisation impossible : {e}.")))?;
    let seconds = start.elapsed().as_secs_f64();
    if let Some(path) = svg {
        std::fs::write(path, vector.to_svg())
            .map_err(|e| Problem::Failed(format!("Impossible d'écrire « {path} » : {e}")))?;
    }
    let comparison = if check {
        let start = Instant::now();
        let comparison = compare(&gray, &vector);
        Some(json!({
            "comparison": comparison,
            "seconds": round2(start.elapsed().as_secs_f64()),
            "withinHalfPixel": comparison.max_deviation_px < 0.5
                && comparison.max_mismatch_distance_px < 0.5,
        }))
    } else {
        None
    };
    let summary = json!({
        "width": vector.width,
        "height": vector.height,
        "options": vector.options,
        "stats": vector.stats,
        "seconds": round2(seconds),
        "check": comparison,
    });
    serde_json::to_string_pretty(&summary)
        .map_err(|e| Problem::Failed(format!("Impossible d'écrire le résultat en JSON : {e}")))
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

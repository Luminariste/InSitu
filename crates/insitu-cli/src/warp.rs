//! Commande `insitu warp` : déforme une image vers une diapositive, sans
//! écran, pour vérifier le moteur de déformation sur de vrais fichiers.

use std::path::Path;
use std::time::Instant;

use insitu_core::geometry::{GridWarp, Point, PointWarp, Rect, Transform};
use insitu_core::resample::{warp_raster, WarpOptions};
use insitu_core::tiles::{read_raster, Raster};
use insitu_core::units::{mm_to_px, DEFAULT_EXPORT_DPI, SLIDE_SIZE_MM};
use serde::Deserialize;
use serde_json::json;

/// Ce qui peut mal tourner.
pub enum Problem {
    Usage(String),
    Failed(String),
}

pub const USAGE: &str = r#"Usage : insitu warp <image> <sortie.png> <paires.json> [--px N | --dpi D] [--threads N]

Calcule la diapositive carrée de 280 mm qui montre <image> déformée, et
l'enregistre en PNG (mêmes canaux que l'image : gris, RVB ou RVB et
transparence). Hors de l'image, la diapositive est noire.

Fichier de paires (même forme que pour insitu geometry fit) :

  {
    "pairs": [
      { "source": [0, 0],     "target": [812.0, 140.5] },
      { "source": [280, 0],   "target": [9120.3, 410.0] },
      { "source": [280, 280], "target": [8840.7, 9020.1] },
      { "source": [0, 280],   "target": [95.2, 8700.4] }
    ],
    "smoothing": 0
  }

  source     Un point de la diapositive, en mm (origine en haut à gauche).
  target     Le même point dans <image>, en pixels.
  smoothing  Facultatif : voir insitu geometry --help.

Avec 4 paires, la déformation est un trapèze ; au-delà, une correction lisse
passe par tous les points.

  --px N      Côté de la diapositive en pixels.
  --dpi D     Ou sa résolution (1 400 dpi par défaut, soit 15 433 px).
  --threads N Nombre de fils de calcul (tous les cœurs par défaut).

Affiche en JSON la taille produite et la durée du calcul (lecture et
écriture des fichiers non comprises)."#;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WarpRequest {
    pairs: Vec<PointPair>,
    smoothing: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PointPair {
    source: Point,
    target: Point,
}

pub fn run(args: &[&str]) -> Result<String, Problem> {
    if matches!(args, [] | ["--help" | "-h"]) {
        return Ok(USAGE.to_string());
    }
    let usage = |message: &str| Problem::Usage(format!("{message} Voir : insitu warp --help"));
    let mut size = mm_to_px(SLIDE_SIZE_MM, DEFAULT_EXPORT_DPI);
    let mut options = WarpOptions::default();
    let mut paths = Vec::new();
    let mut rest = args.iter().copied();
    while let Some(arg) = rest.next() {
        match arg {
            "--px" => {
                size = rest
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|n: &u32| *n > 0)
                    .ok_or_else(|| usage("--px attend un nombre de pixels"))?;
            }
            "--dpi" => {
                let dpi: f64 = rest
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|d: &f64| d.is_finite() && *d > 0.0 && *d <= 10_000.0)
                    .ok_or_else(|| usage("--dpi attend une résolution de 1 à 10 000"))?;
                size = mm_to_px(SLIDE_SIZE_MM, dpi).max(1);
            }
            "--threads" => {
                options.threads = rest
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| usage("--threads attend un nombre"))?;
            }
            other if other.starts_with("--") => {
                return Err(usage(&format!("Option inconnue : {other}")));
            }
            path => paths.push(path),
        }
    }
    let [image, output, pairs] = paths[..] else {
        return Err(usage(
            "Il faut une image, un fichier de sortie et un fichier de paires.",
        ));
    };

    let warp = read_pairs(pairs).map_err(Problem::Failed)?;
    let source = read_raster(Path::new(image)).map_err(|e| Problem::Failed(e.to_string()))?;
    let px_per_mm = f64::from(size) / SLIDE_SIZE_MM;
    let map = |p: Point| warp.apply(p * (1.0 / px_per_mm));
    let start = Instant::now();
    let slide = warp_raster(&source, size, size, map, &options)
        .map_err(|e| Problem::Failed(format!("Déformation impossible : {e}.")))?;
    let seconds = start.elapsed().as_secs_f64();
    save_png(&slide, Path::new(output)).map_err(Problem::Failed)?;
    let summary = json!({
        "width": slide.width,
        "height": slide.height,
        "channels": slide.channels,
        "seconds": (seconds * 100.0).round() / 100.0,
    });
    serde_json::to_string_pretty(&summary)
        .map_err(|e| Problem::Failed(format!("Impossible d'écrire le résultat en JSON : {e}")))
}

/// Lit les paires et prépare la déformation diapositive (mm) → image (px),
/// échantillonnée sur une grille de 2,5 mm (D-2.3-b).
fn read_pairs(path: &str) -> Result<GridWarp, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Impossible de lire « {path} » : {e}"))?;
    let request: WarpRequest = serde_json::from_str(&text).map_err(|e| {
        format!("« {path} » n'a pas le format attendu ({e}). Voir : insitu warp --help")
    })?;
    let source: Vec<Point> = request.pairs.iter().map(|p| p.source).collect();
    let target: Vec<Point> = request.pairs.iter().map(|p| p.target).collect();
    let fitted = PointWarp::fit(&source, &target, request.smoothing.unwrap_or(0.0))
        .map_err(|e| format!("Ajustement impossible : {e}."))?;
    let slide = Rect::new(
        Point::new(0.0, 0.0),
        Point::new(SLIDE_SIZE_MM, SLIDE_SIZE_MM),
    )
    .expect("carré de la diapositive");
    GridWarp::from_point_warp(&fitted, slide, 2.5).map_err(|e| format!("Grille impossible : {e}."))
}

fn save_png(raster: &Raster, path: &Path) -> Result<(), String> {
    let color = match raster.channels {
        1 => image::ExtendedColorType::L8,
        3 => image::ExtendedColorType::Rgb8,
        _ => image::ExtendedColorType::Rgba8,
    };
    image::save_buffer_with_format(
        path,
        &raster.data,
        raster.width,
        raster.height,
        color,
        image::ImageFormat::Png,
    )
    .map_err(|e| format!("Impossible d'écrire « {} » : {e}", path.display()))
}

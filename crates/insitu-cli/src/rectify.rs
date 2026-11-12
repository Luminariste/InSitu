//! Commande `insitu rectify …` : détection de la mire et redressement d'une
//! photo brute, sans écran.

use std::path::Path;
use std::time::Instant;

use insitu_core::geometry::Point;
use insitu_core::rectification::{self, DetectOptions, DEFAULT_SMOOTHING};
use insitu_core::units::RECTIFIED_PHOTO_DPI;
use serde_json::json;

/// Ce qui peut mal tourner.
pub enum Problem {
    Usage(String),
    Failed(String),
}

pub const USAGE: &str = r#"Usage : insitu rectify <photo> [options]
       insitu rectify check <image redressée>

Cherche la mire dans une photo brute (JPEG, PNG, TIFF ou HEIC), mesure les
croisements de sa grille et affiche le compte rendu en JSON.

Options :
  --output <image>       Écrit la photo redressée (PNG, TIFF ou JPEG, d'après
                         l'extension) : le carré de 280 mm de la diapositive.
  --dpi <n>              Résolution de la photo redressée (500 par défaut,
                         soit 5 512 px, comme les photos redressées d'Épinal).
  --confidence <image>   Écrit la carte de confiance (PNG, une case de 2 mm
                         par pixel) : blanc = sûr, gris = erreur estimée
                         (noir à 1 mm et plus), noir = zone non couverte.
  --points <fichier>     Écrit les croisements mesurés et la carte de
                         confiance en JSON.
  --corners x,y,x,y,x,y,x,y
                         Position approchée, en pixels de la photo, des quatre
                         coins de la mire, dans n'importe quel ordre. Remplace
                         la recherche du cercle rouge et de la croix ; le sens
                         est lu sur les petits cercles, la grille affine.
  --smoothing <s>        Lissage de la correction (0 par défaut).

« check » mesure les croisements de la grille dans une image déjà redressée
(la nôtre, ou une photo redressée à la main) et dit à quelle distance de leur
place théorique ils tombent, en mm.

Les pixels de la photo sont comptés comme dans le fichier (sans tenir compte
de la rotation EXIF) ; le centre du premier pixel est en (0,5 ; 0,5).
Code de sortie : 0 si la mire est trouvée, 1 sinon (la cause est affichée).
"#;

pub fn run(args: &[&str]) -> Result<String, Problem> {
    match args {
        [] | ["--help" | "-h"] => return Ok(USAGE.to_string()),
        ["check", path] => return check(path),
        ["check", ..] => {
            return Err(Problem::Usage(
                "Usage : insitu rectify check <image redressée>".into(),
            ))
        }
        _ => {}
    }
    let usage = |m: &str| Problem::Usage(m.to_string());
    let mut photo = None;
    let mut output = None;
    let mut confidence = None;
    let mut points = None;
    let mut dpi = RECTIFIED_PHOTO_DPI;
    let mut options = DetectOptions {
        smoothing: DEFAULT_SMOOTHING,
        ..DetectOptions::default()
    };
    let mut rest = args.iter().copied();
    while let Some(arg) = rest.next() {
        match arg {
            "--output" => {
                output = Some(
                    rest.next()
                        .ok_or_else(|| usage("--output attend un fichier"))?,
                )
            }
            "--confidence" => {
                confidence = Some(
                    rest.next()
                        .ok_or_else(|| usage("--confidence attend un fichier"))?,
                )
            }
            "--points" => {
                points = Some(
                    rest.next()
                        .ok_or_else(|| usage("--points attend un fichier"))?,
                )
            }
            "--dpi" => {
                dpi = rest
                    .next()
                    .and_then(|v| v.parse::<f64>().ok())
                    .filter(|v| *v >= 10.0 && *v <= 3000.0)
                    .ok_or_else(|| usage("--dpi attend un nombre entre 10 et 3000"))?
            }
            "--smoothing" => {
                options.smoothing = rest
                    .next()
                    .and_then(|v| v.parse::<f64>().ok())
                    .filter(|v| *v >= 0.0)
                    .ok_or_else(|| usage("--smoothing attend un nombre positif ou nul"))?
            }
            "--corners" => {
                let values: Vec<f64> = rest
                    .next()
                    .unwrap_or("")
                    .split(',')
                    .filter_map(|v| v.trim().parse().ok())
                    .collect();
                let [a, b, c, d, e, f, g, h] = values[..] else {
                    return Err(usage(
                        "--corners attend huit nombres séparés par des virgules",
                    ));
                };
                options.corners = Some([
                    Point::new(a, b),
                    Point::new(c, d),
                    Point::new(e, f),
                    Point::new(g, h),
                ]);
            }
            other if other.starts_with("--") => {
                return Err(Problem::Usage(format!("Option inconnue : {other}")))
            }
            path if photo.is_none() => photo = Some(path),
            _ => return Err(usage("Une seule photo à la fois.")),
        }
    }
    let photo_path = photo.ok_or_else(|| usage("Il manque la photo."))?;
    let failed = |e: rectification::DetectionError| Problem::Failed(e.to_string());

    let start = Instant::now();
    let image = rectification::read_photo(Path::new(photo_path)).map_err(failed)?;
    let detection = rectification::detect(&image, &options).map_err(failed)?;
    let detect_s = start.elapsed().as_secs_f64();
    let map = detection.confidence_map();
    let covered: Vec<f64> = map.error_mm.iter().flatten().copied().collect();
    if let Some(path) = output {
        let rectified = rectification::rectify(&image, &detection, dpi);
        rectified
            .save(path)
            .map_err(|e| Problem::Failed(format!("{path} : écriture impossible : {e}")))?;
    }
    if let Some(path) = confidence {
        map.to_image()
            .save(path)
            .map_err(|e| Problem::Failed(format!("{path} : écriture impossible : {e}")))?;
    }
    if let Some(path) = points {
        let text = serde_json::to_string_pretty(&json!({
            "nodes": detection.nodes,
            "confidence": map,
        }))
        .map_err(|e| Problem::Failed(e.to_string()))?;
        std::fs::write(path, text)
            .map_err(|e| Problem::Failed(format!("{path} : écriture impossible : {e}")))?;
    }
    let summary = json!({
        "photo": photo_path,
        "method": detection.method,
        "photoSize": [detection.photo_width, detection.photo_height],
        "nodes": detection.nodes.len(),
        "measurableNodes": detection.measurable_nodes,
        "rawNodes": detection.raw_nodes,
        "outliersRemoved": detection.outliers_removed,
        "quarterTurns": detection.quarter_turns,
        "orientationMargin": detection.orientation_margin,
        "gridUnread": detection.grid_unread,
        "redCircleSectors": detection.red_circle_sectors,
        "pxPerMm": detection.px_per_mm,
        "homographyResidualMm": detection.homography_residual_mm,
        "leaveOneOutMm": detection.leave_one_out_mm,
        "coverage": detection.coverage,
        "estimatedErrorMm": rectification::ErrorSummary::of(&covered),
        "smoothing": detection.smoothing,
        "corners": [
            detection.slide_to_photo(Point::new(0.0, 0.0)),
            detection.slide_to_photo(Point::new(280.0, 0.0)),
            detection.slide_to_photo(Point::new(280.0, 280.0)),
            detection.slide_to_photo(Point::new(0.0, 280.0)),
        ],
        "output": output.map(|p| json!({ "path": p, "dpi": dpi })),
        "detectionSeconds": detect_s,
        "totalSeconds": start.elapsed().as_secs_f64(),
    });
    serde_json::to_string_pretty(&summary).map_err(|e| Problem::Failed(e.to_string()))
}

/// `insitu rectify check <image>` : où tombent les croisements d'une image
/// déjà redressée.
fn check(path: &str) -> Result<String, Problem> {
    let failed = |e: rectification::DetectionError| Problem::Failed(e.to_string());
    let image = rectification::read_photo(Path::new(path)).map_err(failed)?;
    let check = rectification::check_rectified(&image).map_err(failed)?;
    serde_json::to_string_pretty(&json!({
        "image": path,
        "nodes": check.nodes.len(),
        "measurableNodes": check.measurable_nodes,
        "deviationMm": check.deviation_mm,
    }))
    .map_err(|e| Problem::Failed(e.to_string()))
}

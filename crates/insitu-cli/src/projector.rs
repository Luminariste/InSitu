//! Commandes `insitu projector …` : le redressement dans un projet, comme le
//! fait l'écran de redressement de l'app (tâche 2.5).

use std::path::Path;

use insitu_core::geometry::Point;
use insitu_core::project::ProjectDir;
use insitu_core::rectification::projector::{
    self, add_projector, export_rectified, import_raw_photo, projector_view, rectify_projector,
    RectificationView,
};
use insitu_core::units::RECTIFIED_PHOTO_DPI;
use serde_json::json;

/// Ce qui peut mal tourner.
pub enum Problem {
    Usage(String),
    Failed(String),
}

fn failed(error: impl std::fmt::Display) -> Problem {
    Problem::Failed(error.to_string())
}

fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string_pretty(value).expect("sérialisable")
}

/// Résumé lisible d'un redressement ; avec `full`, tout ce que reçoit
/// l'écran de l'app (chaque croisement, contour de la mire).
fn summary(view: &RectificationView, full: bool) -> String {
    if full {
        return json(view);
    }
    json(&json!({
        "projectorId": view.projector_id,
        "method": view.method,
        "photoSize": [view.photo_width, view.photo_height],
        "nodes": view.nodes.len(),
        "measurableNodes": view.measurable_nodes,
        "outliersRemoved": view.outliers_removed,
        "corners": view.corners,
        "leaveOneOutMm": view.leave_one_out_mm,
        "estimatedErrorMm": view.estimated_error_mm,
        "coverage": view.coverage,
        "pxPerMm": view.px_per_mm,
        "seconds": view.seconds,
    }))
}

fn parse_corners(text: &str) -> Result<[Point; 4], Problem> {
    let values: Vec<f64> = text
        .split(',')
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    let [a, b, c, d, e, f, g, h] = values[..] else {
        return Err(Problem::Usage(
            "--corners attend huit nombres séparés par des virgules".into(),
        ));
    };
    Ok([
        Point::new(a, b),
        Point::new(c, d),
        Point::new(e, f),
        Point::new(g, h),
    ])
}

/// Exécute `insitu projector <args>` et rend le texte à afficher.
pub fn run(args: &[&str]) -> Result<String, Problem> {
    let open = |folder: &str| ProjectDir::open(folder).map_err(failed);
    match args {
        [] | ["--help" | "-h"] => Ok(USAGE.to_string()),
        ["add", folder, label] => {
            let (_, id) = add_projector(&open(folder)?, label).map_err(failed)?;
            Ok(json(&json!({ "id": id, "label": label.trim() })))
        }
        ["photo", folder, key, photo] => {
            let project =
                import_raw_photo(&open(folder)?, key, Path::new(photo)).map_err(failed)?;
            let p = projector::find_projector(&project, key).expect("projecteur trouvé");
            Ok(json(p))
        }
        ["rectify", folder, key, rest @ ..] => {
            let corners = match rest {
                [] => None,
                ["--corners", text] => Some(parse_corners(text)?),
                _ => return Err(Problem::Usage(USAGE.to_string())),
            };
            let (_, view) = rectify_projector(&open(folder)?, key, corners).map_err(failed)?;
            Ok(summary(&view, false))
        }
        ["show", folder, key, rest @ ..] => {
            let with_nodes = match rest {
                [] => false,
                ["--nodes"] => true,
                _ => return Err(Problem::Usage(USAGE.to_string())),
            };
            match projector_view(&open(folder)?, key).map_err(failed)? {
                Some(view) => Ok(summary(&view, with_nodes)),
                None => Ok("Ce projecteur n'est pas encore redressé.".into()),
            }
        }
        ["export", folder, key, out, rest @ ..] => {
            let dpi = match rest {
                [] => RECTIFIED_PHOTO_DPI,
                ["--dpi", value] => value
                    .replace(',', ".")
                    .parse()
                    .map_err(|_| Problem::Usage("--dpi attend un nombre".into()))?,
                _ => return Err(Problem::Usage(USAGE.to_string())),
            };
            let exported =
                export_rectified(&open(folder)?, key, Path::new(out), dpi).map_err(failed)?;
            Ok(json(&exported))
        }
        [other, ..] => Err(Problem::Usage(format!(
            "Commande de projecteur inconnue : {other}\n\n{USAGE}"
        ))),
    }
}

pub const USAGE: &str = r#"Usage : insitu projector <commande>

Le projecteur est désigné par son identifiant ou par son nom (« A »).

  add <projet.insitu> <nom>
      Ajoute un projecteur au projet.

  photo <projet.insitu> <projecteur> <photo>
      Copie la photo de mire (JPEG, PNG, TIFF ou HEIC) dans photos-brutes/
      et en fait la photo du projecteur. Son redressement précédent est oublié.

  rectify <projet.insitu> <projecteur> [--corners x,y,x,y,x,y,x,y]
      Détecte la mire et enregistre la photo redressée (PNG à 500 dpi, dans
      photos-redressees/). Avec --corners : position approchée, en pixels
      de la photo, des quatre coins de la mire, dans n'importe quel ordre, à
      la place de la recherche automatique ; le sens de la mire est lu sur
      les petits cercles, la grille affine ensuite.

  show <projet.insitu> <projecteur> [--nodes]
      Relit le redressement enregistré (écarts, couverture, coins dans la
      photo) ; --nodes écrit tout ce que reçoit l'écran de l'app, dont
      chaque croisement et le contour de la mire.

  export <projet.insitu> <projecteur> <fichier.tif|.png> [--dpi N]
      Exporte la photo redressée en TIFF (LZW) ou PNG, résolution inscrite
      dans le fichier (500 dpi par défaut, de 50 à 1 400).
"#;

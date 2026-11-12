//! Commandes `insitu target …` : la mire, sans écran.

use insitu_core::target::pdf::{self, PageFormat};
use insitu_core::target::{TargetLayout, TARGET_VERSION};
use serde_json::json;

/// Ce qui peut mal tourner dans une commande de mire.
pub enum Problem {
    /// Commande mal écrite.
    Usage(String),
    /// Le fichier n'a pas pu être écrit.
    Failed(String),
}

pub const USAGE: &str = "Usage :
  insitu target pdf <fichier.pdf> [--page a3|carre]
      Écrit la mire imprimable (version 2, à repères codés).
      --page a3     page A3 avec la consigne d'impression et deux
                    pointillés pour recouper la feuille en carré (défaut)
      --page carre  page carrée de 297 mm, sans consigne
  insitu target info
      Décrit la mire en JSON : repères (coins en mm), croisements de la grille.";

/// Exécute `insitu target <args>` et rend le texte à afficher.
pub fn run(args: &[&str]) -> Result<String, Problem> {
    match args {
        [] | ["--help" | "-h"] => Ok(USAGE.to_string()),
        ["info"] => Ok(info()),
        ["pdf", rest @ ..] => write_pdf(rest),
        [other, ..] => Err(Problem::Usage(format!(
            "Commande de mire inconnue : {other}"
        ))),
    }
}

fn write_pdf(args: &[&str]) -> Result<String, Problem> {
    let (path, format) = match args {
        [path] => (*path, PageFormat::A3),
        [path, "--page", page] | ["--page", page, path] => {
            let format = match *page {
                "a3" => PageFormat::A3,
                "carre" | "carré" => PageFormat::Square,
                other => {
                    return Err(Problem::Usage(format!(
                        "--page attend a3 ou carre, pas « {other} »"
                    )))
                }
            };
            (*path, format)
        }
        _ => return Err(Problem::Usage(USAGE.to_string())),
    };
    let layout = TargetLayout::coded();
    let bytes = pdf::render(&layout, format);
    std::fs::write(path, &bytes)
        .map_err(|e| Problem::Failed(format!("Impossible d'écrire {path} : {e}")))?;
    let (w, h) = format.size_mm();
    Ok(format!(
        "Mire version {TARGET_VERSION} écrite dans {path} (page de {w} × {h} mm, {} octets).\n\
         Imprimer à 100 % : le carré de la grille doit mesurer 280 mm.",
        bytes.len()
    ))
}

fn info() -> String {
    let layout = TargetLayout::coded();
    let nodes = layout.grid_nodes();
    let value = json!({
        "version": TARGET_VERSION,
        "identifier": layout.identifier(),
        "sizeMm": layout.size_mm,
        "gridPitchMm": insitu_core::target::GRID_PITCH_MM,
        "markerFamily": "tag36h11",
        "markerSizeMm": insitu_core::target::MARKER_SIZE_MM,
        "markers": layout.markers.iter().map(|m| json!({
            "id": m.id,
            "corners": m.corners,
        })).collect::<Vec<_>>(),
        "gridNodeCount": nodes.len(),
        "gridNodes": nodes,
    });
    serde_json::to_string_pretty(&value).expect("JSON de la mire")
}

//! `insitu` : le moteur en ligne de commande.
//!
//! Chaque capacité du moteur doit rester appelable d'ici, pour que les agents
//! puissent la vérifier sans interface. Sous-commandes disponibles :
//!
//! - `insitu info` : carte d'identité du moteur, en JSON ;
//! - `insitu geometry fit <fichier.json>` : ajuste une déformation sur des
//!   paires de points et affiche les écarts et les points aberrants, en JSON.

use std::process::ExitCode;

use insitu_core::geometry::{fit_with_report, FitOptions, Point};
use serde::Deserialize;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["info"] => {
            let info = insitu_core::engine_info();
            println!(
                "{}",
                serde_json::to_string_pretty(&info).expect("EngineInfo est sérialisable")
            );
            ExitCode::SUCCESS
        }
        ["geometry", "fit", "--help" | "-h"] | ["geometry"] | ["geometry", "--help" | "-h"] => {
            println!("{GEOMETRY_USAGE}");
            ExitCode::SUCCESS
        }
        ["geometry", "fit", path] => match geometry_fit(path) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("Erreur : {message}");
                ExitCode::FAILURE
            }
        },
        ["geometry", ..] => {
            eprintln!("Commande incomplète ou inconnue.\n\n{GEOMETRY_USAGE}");
            ExitCode::from(2)
        }
        ["--help" | "-h"] | [] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        [other, ..] => {
            eprintln!("Commande inconnue : {other}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

const USAGE: &str = "Usage : insitu <commande>

Commandes :
  info                          Affiche la carte d'identité du moteur (JSON)
  geometry fit <fichier.json>   Ajuste une déformation sur des paires de points
                                (détails : insitu geometry --help)";

const GEOMETRY_USAGE: &str = r#"Usage : insitu geometry fit <fichier.json>

Ajuste une déformation (trapèze + correction locale lisse) sur des paires de
points, et affiche en JSON les écarts résiduels et les points aberrants.

Fichier d'entrée :

  {
    "pairs": [
      { "source": [12.5, 40.0], "target": [1031.2, 2210.7] },
      { "source": [250.0, 38.0], "target": [5120.9, 2188.4] }
    ],
    "smoothing": 0,
    "outlierFactor": 6,
    "outlierFloor": 0
  }

  pairs          Au moins 4 paires. "source" et "target" sont des points
                 [x, y], chacun dans son repère et son unité (par exemple la
                 diapositive en mm et la façade en mm, ou la photo en pixels
                 et la diapositive en mm). x vers la droite, y vers le bas.
  smoothing      Facultatif, 0 par défaut. À 0 la déformation passe exactement
                 par les points ; plus la valeur est grande, plus elle est
                 raide (essayer 0.01 à 1).
  outlierFactor  Facultatif, 6 par défaut. Un point est déclaré aberrant si
                 son score dépasse ce nombre de fois le score médian des
                 autres points.
  outlierFloor   Facultatif. Score, dans l'unité de la cible, en dessous
                 duquel un point n'est jamais déclaré aberrant : la précision
                 attendue d'un bon point.

Sortie (les indices de points commencent à 0, dans l'ordre du fichier ; les
écarts sont dans l'unité de la cible, ceux en "...InSource" dans l'unité de la
source) :

  pointCount                   Nombre de paires.
  homography                   Matrice 3 × 3 du trapèze.
  homographyResiduals          Ce que le trapèze seul laisse comme écart :
                               perPoint (un par point), rms (moyenne
                               quadratique), max, worstIndex.
  homographyResidualsInSource  Les mêmes, ramenés côté source.
  warpResiduals                Écarts de la déformation complète (nuls quand
                               smoothing vaut 0 : elle passe par les points).
  leaveOneOut                  Validation croisée : écart de chaque point à la
                               position que prédisent les autres points sans
                               lui. C'est l'écart utile pour juger un point.
  leaveOneOutInSource          Les mêmes, ramenés côté source.
  outlierScores                Score de chaque point : son écart de validation
                               croisée, corrigé de sa position (un point au
                               bord est naturellement moins bien prédit).
  outlierThreshold             Seuil de score appliqué.
  outliers                     Indices des points aberrants, le plus net
                               d'abord.

Avec moins de 6 paires, il n'y a pas assez de points pour une validation
croisée : leaveOneOut, outlierScores et outlierThreshold valent null.

Code de sortie : 0 si l'ajustement a réussi (même avec des points aberrants),
1 si le fichier est illisible ou les points inutilisables, 2 si la commande
est mal écrite."#;

/// Contenu du fichier lu par `insitu geometry fit`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FitRequest {
    pairs: Vec<PointPair>,
    smoothing: Option<f64>,
    outlier_factor: Option<f64>,
    outlier_floor: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PointPair {
    source: Point,
    target: Point,
}

fn geometry_fit(path: &str) -> Result<String, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("impossible de lire « {path} » : {e}"))?;
    let request: FitRequest = serde_json::from_str(&text).map_err(|e| {
        format!("« {path} » n'a pas le format attendu ({e}). Voir : insitu geometry --help")
    })?;
    let defaults = FitOptions::default();
    let options = FitOptions {
        smoothing: request.smoothing.unwrap_or(defaults.smoothing),
        outlier_factor: request.outlier_factor.unwrap_or(defaults.outlier_factor),
        outlier_floor: request.outlier_floor.unwrap_or(defaults.outlier_floor),
    };
    let source: Vec<Point> = request.pairs.iter().map(|p| p.source).collect();
    let target: Vec<Point> = request.pairs.iter().map(|p| p.target).collect();
    let (_, report) = fit_with_report(&source, &target, &options).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

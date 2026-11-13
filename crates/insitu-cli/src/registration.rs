//! Commandes `insitu registration …` : le recalage d'un projecteur sur la
//! façade, comme le fait l'écran Recalage de l'app (tâche 3.2).

use insitu_core::project::{PointMm, ProjectDir, RegistrationPair};
use insitu_core::registration::auto::projector_adjustment;
use insitu_core::registration::{
    projector_report, set_registration, RegistrationOptions, RegistrationReport,
};
use serde::Deserialize;

/// Ce qui peut mal tourner.
pub enum Problem {
    Usage(String),
    Failed(String),
}

pub const USAGE: &str = r#"Usage :
  insitu registration show <projet.insitu> <projecteur> [--factor F] [--floor MM]
  insitu registration set <projet.insitu> <projecteur> <paires.json> [--factor F] [--floor MM]
  insitu registration preview <paires.json>
  insitu registration adjust <projet.insitu> <projecteur> [<paires.json>]

Recalage d'un projecteur sur la façade : des points communs entre sa photo
redressée (mm de diapositive, 0 à 280) et la façade (mm réels, façade mise
à l'échelle). <projecteur> : son identifiant ou son nom (« A »).

  show   Affiche en JSON le compte rendu du recalage enregistré.
  set    Remplace les paires du projecteur par celles du fichier, enregistre
         le projet et affiche le compte rendu. Fichier :

           { "pairs": [ { "slide": [21.4, 18.9], "facade": [612.0, 480.0] }, … ] }

  preview  Pose provisoire, sans projet ni enregistrement : emprise et
           maillage de la diapositive sur la façade pour ces paires (ce que
           l'écran Recalage redessine pendant qu'on glisse un point).

  adjust   Ajustement automatique : part de la pose de la photo sur la
           façade (les paires enregistrées, ou celles du fichier) et la cale
           sur les lignes du plan. Affiche en JSON les quatre paires aux
           coins de la diapositive, l'emprise, l'accord des contours avant
           et après (« startScore », « score », de 0 à 1) et le plus grand
           déplacement d'un coin (« movedMm »). Rien n'est enregistré : pour
           garder le résultat, passer ses paires à « set ».

  --factor F   Un point est aberrant si son score dépasse F fois le score
               médian des autres (6 par défaut).
  --floor MM   Écart sur la diapositive, en mm, sous lequel un point n'est
               jamais aberrant (1 par défaut).

Compte rendu : "stage" vaut "incomplete" (moins de 4 paires), "trapeze"
(4 ou 5 paires) ou "corrected" (6 et plus : relief corrigé, points
aberrants recherchés). Pour chaque paire, l'écart sur la diapositive et sur
la façade (mm) et "outlier". "footprint" : contour de la diapositive posé
sur la façade (emprise du projecteur)."#;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairsFile {
    pairs: Vec<PairEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairEntry {
    slide: [f64; 2],
    facade: [f64; 2],
}

pub fn run(args: &[&str]) -> Result<String, Problem> {
    if matches!(args, [] | ["--help" | "-h"]) {
        return Ok(USAGE.to_string());
    }
    let usage =
        |message: &str| Problem::Usage(format!("{message} Voir : insitu registration --help"));
    let mut options = RegistrationOptions::default();
    let mut words = Vec::new();
    let mut rest = args.iter().copied();
    while let Some(arg) = rest.next() {
        match arg {
            "--factor" => {
                options.outlier_factor = rest
                    .next()
                    .and_then(|v| v.replace(',', ".").parse().ok())
                    .ok_or_else(|| usage("--factor attend un nombre."))?;
            }
            "--floor" => {
                options.outlier_floor_slide_mm = rest
                    .next()
                    .and_then(|v| v.replace(',', ".").parse().ok())
                    .ok_or_else(|| usage("--floor attend un nombre de mm."))?;
            }
            other if other.starts_with("--") => {
                return Err(usage(&format!("Option inconnue : {other}")));
            }
            word => words.push(word),
        }
    }
    if let ["preview", file] = words[..] {
        let preview = insitu_core::overview::registration_preview(&read_pairs(file)?);
        return Ok(serde_json::to_string_pretty(&preview).expect("sérialisable"));
    }
    if let ["adjust", folder, projector, ref file @ ..] = words[..] {
        let pairs = match file {
            [] => None,
            [file] => Some(read_pairs(file)?),
            _ => return Err(usage("adjust prend au plus un fichier de paires.")),
        };
        let dir = ProjectDir::open(folder).map_err(failed)?;
        let report = projector_adjustment(&dir, projector, pairs.as_deref()).map_err(failed)?;
        return Ok(serde_json::to_string_pretty(&report).expect("sérialisable"));
    }
    let report = match words[..] {
        ["show", folder, projector] => {
            let dir = ProjectDir::open(folder).map_err(failed)?;
            projector_report(&dir, projector, &options).map_err(failed)?
        }
        ["set", folder, projector, file] => {
            let pairs = read_pairs(file)?;
            let dir = ProjectDir::open(folder).map_err(failed)?;
            set_registration(&dir, projector, pairs, &options)
                .map_err(failed)?
                .1
        }
        _ => return Err(usage("Commande de recalage incomplète ou inconnue.")),
    };
    Ok(json(&report))
}

fn failed(error: impl std::fmt::Display) -> Problem {
    Problem::Failed(error.to_string())
}

fn json(report: &RegistrationReport) -> String {
    serde_json::to_string_pretty(report).expect("sérialisable")
}

fn read_pairs(path: &str) -> Result<Vec<RegistrationPair>, Problem> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Problem::Failed(format!("Impossible de lire « {path} » : {e}")))?;
    let file: PairsFile = serde_json::from_str(&text).map_err(|e| {
        Problem::Failed(format!(
            "« {path} » n'a pas le format attendu ({e}). Voir : insitu registration --help"
        ))
    })?;
    Ok(file
        .pairs
        .into_iter()
        .map(|p| {
            RegistrationPair::new(
                PointMm::new(p.slide[0], p.slide[1]),
                PointMm::new(p.facade[0], p.facade[1]),
            )
        })
        .collect())
}

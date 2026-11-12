//! Commandes `insitu assembly …` : assemblage de photos de façade (tâche
//! 6.4), comme l'écran Assemblage de l'app.

use std::path::Path;

use insitu_core::assembly::auto::analyze;
use insitu_core::assembly::{
    add_photo, remove_photo, render, report, set_pairs, RenderOptions, DEFAULT_MAX_SIDE,
};
use insitu_core::project::{AssemblyPair, Project, ProjectDir, SourcePoint};
use serde::Deserialize;

pub const USAGE: &str = r#"Usage :
  insitu assembly add <projet.insitu> <photo> [--anchor PHOTO]
  insitu assembly remove <projet.insitu> <photo>
  insitu assembly pairs <projet.insitu> <photo> <appui> <paires.json>
  insitu assembly auto <projet.insitu>
  insitu assembly show <projet.insitu>
  insitu assembly render <projet.insitu> [--max-side PX] [--threads N]

Assemblage de plusieurs photos de façade en une seule image, qui devient la
façade du projet. La première photo ajoutée est la référence ; chaque autre
photo est recalée sur une photo déjà placée (son appui) par au moins quatre
points communs. <photo>, <appui> : identifiant ou nom (sans extension).

  add     Copie la photo dans le projet (facade/assemblage/). Appui par
          défaut : la dernière photo ajoutée.
  remove  Retire une photo de l'assemblage (refusé si une autre s'appuie
          sur elle).
  pairs   Remplace l'appui et les points communs d'une photo. Fichier, en
          pixels de chaque photo :

            { "pairs": [ { "here": [812, 344], "there": [95, 360] }, … ] }

          « here » : dans la photo ; « there » : dans son appui.
  auto    Raccord automatique : trouve les détails communs entre les
          photos, choisit l'appui de chacune et remplace ses points
          communs. Une photo sans détail commun garde les siens.
  show    Compte rendu en JSON : pour chaque photo, placée ou non, écart de
          chaque point (pixels de l'appui, dès 5 points), points aberrants
          (dès 6), coins dans le repère de la référence.
  render  Fabrique facade/assemblage.jpg et en fait la façade du projet.
          --max-side : plus grand côté de l'image (12 000 px par défaut)."#;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairsFile {
    pairs: Vec<PairEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairEntry {
    here: [f64; 2],
    there: [f64; 2],
}

/// Identifiant d'une photo, donnée par son identifiant ou son nom.
fn photo_id(project: &Project, wanted: &str) -> Result<String, (bool, String)> {
    project
        .facade_assembly
        .as_ref()
        .and_then(|a| {
            a.photos
                .iter()
                .find(|p| p.id == wanted)
                .or_else(|| a.photos.iter().find(|p| p.name == wanted))
        })
        .map(|p| p.id.clone())
        .ok_or_else(|| {
            (
                false,
                format!("Aucune photo « {wanted} » dans l'assemblage."),
            )
        })
}

pub fn run(args: &[&str]) -> Result<String, (bool, String)> {
    let usage = |m: &str| (true, format!("{m} Voir : insitu assembly --help"));
    let failed = |e: &dyn std::fmt::Display| (false, e.to_string());
    let open = |folder: &str| ProjectDir::open(folder).map_err(|e| failed(&e));
    let show =
        |project: &Project| serde_json::to_string_pretty(&report(project)).expect("sérialisable");
    match args {
        [] | ["--help" | "-h"] => Ok(USAGE.to_string()),
        ["add", folder, photo, rest @ ..] => {
            let dir = open(folder)?;
            let anchor = match rest {
                [] => None,
                ["--anchor", anchor] => {
                    let project = dir.load().map_err(|e| failed(&e))?.project;
                    Some(photo_id(&project, anchor)?)
                }
                _ => return Err(usage("Option inconnue ou incomplète.")),
            };
            let (project, _) =
                add_photo(&dir, Path::new(photo), anchor.as_deref()).map_err(|e| failed(&e))?;
            Ok(show(&project))
        }
        ["remove", folder, photo] => {
            let dir = open(folder)?;
            let project = dir.load().map_err(|e| failed(&e))?.project;
            let id = photo_id(&project, photo)?;
            let project = remove_photo(&dir, &id).map_err(|e| failed(&e))?;
            Ok(show(&project))
        }
        ["pairs", folder, photo, anchor, file] => {
            let dir = open(folder)?;
            let project = dir.load().map_err(|e| failed(&e))?.project;
            let (id, anchor) = (photo_id(&project, photo)?, photo_id(&project, anchor)?);
            let text = std::fs::read_to_string(file)
                .map_err(|e| (false, format!("Impossible de lire « {file} » : {e}")))?;
            let parsed: PairsFile = serde_json::from_str(&text)
                .map_err(|e| (true, format!("Fichier de paires illisible : {e}")))?;
            let pairs = parsed
                .pairs
                .iter()
                .map(|p| AssemblyPair {
                    here: SourcePoint::new(p.here[0], p.here[1]),
                    there: SourcePoint::new(p.there[0], p.there[1]),
                })
                .collect();
            let (_, diagnosis) = set_pairs(&dir, &id, &anchor, pairs).map_err(|e| failed(&e))?;
            Ok(serde_json::to_string_pretty(&diagnosis).expect("sérialisable"))
        }
        ["auto", folder] => {
            let (_, done) = analyze(&open(folder)?).map_err(|e| failed(&e))?;
            Ok(serde_json::to_string_pretty(&done).expect("sérialisable"))
        }
        ["show", folder] => {
            let project = open(folder)?.load().map_err(|e| failed(&e))?.project;
            Ok(show(&project))
        }
        ["render", folder, rest @ ..] => {
            let mut options = RenderOptions {
                max_side: DEFAULT_MAX_SIDE,
                threads: 0,
            };
            let mut rest = rest.iter();
            while let Some(arg) = rest.next() {
                let value = rest.next().and_then(|v| v.parse::<u32>().ok());
                match (*arg, value) {
                    ("--max-side", Some(v)) if v >= 1 => options.max_side = v,
                    ("--threads", Some(v)) => options.threads = v as usize,
                    _ => return Err(usage("Option inconnue ou incomplète.")),
                }
            }
            let (_, done) = render(&open(folder)?, &options).map_err(|e| failed(&e))?;
            Ok(serde_json::to_string_pretty(&done).expect("sérialisable"))
        }
        _ => Err(usage("Commande d'assemblage incomplète ou inconnue.")),
    }
}

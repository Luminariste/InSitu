//! Commande `insitu overview` : la vue façade d'un projet (tâche 3.3).

use std::path::PathBuf;

use insitu_core::overview::project_overview;
use insitu_core::project::ProjectDir;

pub const USAGE: &str = r#"Usage :
  insitu overview <projet.insitu> [--cache DOSSIER]

Vue façade : pour chaque projecteur recalé, son emprise sur la façade (mm),
sa surface (m²) et le maillage qui pose sa photo redressée sur la façade ;
pour chaque couple de projecteurs qui se chevauchent, la zone de
recouvrement (contours et surface). Résultat en JSON.

  --cache DOSSIER  Y fabrique l'aperçu (1 024 px) de chaque photo redressée."#;

pub fn run(args: &[&str]) -> Result<String, (bool, String)> {
    let usage = |m: &str| (true, format!("{m} Voir : insitu overview --help"));
    match args {
        [] | ["--help" | "-h"] => Ok(USAGE.to_string()),
        [folder, rest @ ..] => {
            let cache: Option<PathBuf> = match rest {
                [] => None,
                ["--cache", dir] => Some(dir.into()),
                _ => return Err(usage("Option inconnue ou incomplète.")),
            };
            let dir = ProjectDir::open(folder).map_err(|e| (false, e.to_string()))?;
            let view =
                project_overview(&dir, cache.as_deref()).map_err(|e| (false, e.to_string()))?;
            Ok(serde_json::to_string_pretty(&view).expect("sérialisable"))
        }
    }
}

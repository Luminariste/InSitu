//! Commande `insitu simulate` : simulation et alertes d'un projet (tâche 6.1).

use insitu_core::project::ProjectDir;
use insitu_core::simulation::simulate;

pub const USAGE: &str = r#"Usage :
  insitu simulate <projet.insitu> --cache DOSSIER [--json]

Simulation de tous les projecteurs sur la façade, et ses alertes :
  - dessin coupé : aucune diapositive ne le contient en entier, ou le choix
    à la main le met dans une diapositive dont il dépasse ;
  - hors emprise : le dessin ne tombe dans aucune emprise ;
  - projeté deux fois : dans un recouvrement, un trait allumé dans les deux
    diapositives (fresque importée en une image, ou dessins qui se touchent).

Chaque alerte donne le dessin concerné et le rectangle de la façade (mm)
où regarder. Sans --json : une alerte par ligne.

  --cache DOSSIER  Y range (ou y relit) l'aperçu de chaque diapositive.
  --json           Le résultat complet en JSON (aperçus et alertes)."#;

pub fn run(args: &[&str]) -> Result<String, (bool, String)> {
    let usage = |m: &str| (true, format!("{m} Voir : insitu simulate --help"));
    match args {
        [] | ["--help" | "-h"] => Ok(USAGE.to_string()),
        [folder, rest @ ..] => {
            let (cache, json) = match rest {
                ["--cache", dir] => (*dir, false),
                ["--cache", dir, "--json"] | ["--json", "--cache", dir] => (*dir, true),
                _ => return Err(usage("Il faut --cache DOSSIER.")),
            };
            let dir = ProjectDir::open(folder).map_err(|e| (false, e.to_string()))?;
            let result = simulate(&dir, cache.as_ref()).map_err(|e| (false, e.to_string()))?;
            if json {
                return Ok(serde_json::to_string_pretty(&result).expect("sérialisable"));
            }
            if result.alerts.is_empty() {
                return Ok("Aucune alerte.".to_string());
            }
            Ok(result
                .alerts
                .iter()
                .map(|a| {
                    let r = &a.bounds;
                    format!(
                        "{} [{:.0}, {:.0}, {:.0} × {:.0} mm]{}",
                        a.message,
                        r.x,
                        r.y,
                        r.width,
                        r.height,
                        a.placement
                            .as_deref()
                            .map(|p| format!(" (placement {p})"))
                            .unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
    }
}

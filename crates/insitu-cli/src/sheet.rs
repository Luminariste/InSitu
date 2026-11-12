//! Commande `insitu sheet` : fiche de calepinage en PDF (tâche 6.2).

use insitu_core::project::ProjectDir;
use insitu_core::sheet::write_sheet;

pub const USAGE: &str = r#"Usage :
  insitu sheet <projet.insitu> <fiche.pdf> --cache DOSSIER

Fiche de calepinage, en PDF A3 couché, à imprimer pour le site :
  - page 1 : le plan de la façade avec l'emprise de chaque projecteur, les
    recouvrements et les dessins posés ; la liste des diapositives, les
    réglages d'export du projet et les alertes de la simulation ;
  - pages suivantes : chaque diapositive (vignette, fichier d'export,
    recalage, photo, dessins), six par page.

  --cache DOSSIER  Y range (ou y relit) l'aperçu de chaque diapositive."#;

pub fn run(args: &[&str]) -> Result<String, (bool, String)> {
    let usage = |m: &str| (true, format!("{m} Voir : insitu sheet --help"));
    match args {
        [] | ["--help" | "-h"] => Ok(USAGE.to_string()),
        [folder, out, "--cache", cache] => {
            let dir = ProjectDir::open(folder).map_err(|e| (false, e.to_string()))?;
            let report = write_sheet(&dir, cache.as_ref(), out.as_ref(), None)
                .map_err(|e| (false, e.to_string()))?;
            Ok(format!(
                "Fiche écrite dans {out} : {} pages, {} diapositives, {} alerte(s).",
                report.pages, report.slides, report.alerts
            ))
        }
        _ => Err(usage(
            "Il faut le projet, le fichier PDF et --cache DOSSIER.",
        )),
    }
}

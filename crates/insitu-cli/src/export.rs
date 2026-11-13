//! Commandes `insitu fresco` et `insitu export` : import de la fresque en une
//! image et export des diapositives pour la gravure (tâche 3.5).

use std::path::Path;

use insitu_core::export::{export_control_plans, export_slides, import_fresco_page};
use insitu_core::project::{ProjectDir, RectMm};
use serde_json::json;

pub const FRESCO_USAGE: &str = r#"Usage :
  insitu fresco <projet.insitu> <fichier> [--frame X Y LARGEUR HAUTEUR] [--page N]

Copie la fresque dans le projet (dossier fresque/) et la pose sur la façade.
Le fichier est une image (PNG, TIFF, JPEG, BMP), un PDF, un fichier
Illustrator .ai enregistré « compatible PDF », ou un SVG. Un fichier
vectoriel est gardé tel quel : l'export le relit à la résolution de chaque
diapositive ; un aperçu en image est rangé à côté pour l'affichage.

Sans --frame, la fresque couvre toute la largeur de la façade à partir du
coin haut-gauche, en gardant ses proportions : la façade doit alors être
mise à l'échelle. --frame donne le rectangle de façade occupé, en mm.
--page choisit la page d'un PDF ou le plan de travail d'un .ai (1 par
défaut)."#;

pub const EXPORT_USAGE: &str = r#"Usage :
  insitu export <projet.insitu> <dossier> [options]

Écrit un PNG par projecteur recalé (<projet>-<diapositive>.png) : la
fresque vue par ce projecteur. Les options partent des réglages d'export du
projet, et ceux utilisés y sont enregistrés.

  --dpi N           Résolution (1 400 par défaut : 15 433 px pour 280 mm)
  --mirror          Image retournée gauche-droite      (--no-mirror)
  --negative        Noir et blanc inversés              (--no-negative)
  --margin          Plaque de 300 mm, nom et repères dans la marge de 10 mm
  --no-margin       Image de 280 mm seule (par défaut)
  --one-bit         Noir et blanc pur (PNG 1 bit)       (--gray : niveaux de gris)
  --threshold N     Seuil du noir et blanc, de 1 à 255 (128 par défaut)
  --svg             Un SVG en mm à côté de chaque PNG : les courbes des
                    dessins posés (--no-svg)
  --sans-fresque    Les dessins posés seuls, sans la fresque importée en
                    une image (--avec-fresque, par défaut)
  --only NOM        Ce projecteur seulement (répétable)
  --plan-de-controle
                    À la place des diapositives : le plan de la façade vu
                    par chaque projecteur, pour graver une plaque qui
                    vérifie le recalage (<projet>-<diapositive> - PLAN DE
                    CONTROLE.png). Les réglages du projet ne changent pas.

Compte rendu en JSON ; l'avancement s'écrit sur la sortie d'erreur."#;

pub fn fresco(args: &[&str]) -> Result<String, (bool, String)> {
    let usage = |m: &str| (true, format!("{m} Voir : insitu fresco --help"));
    match args {
        [] | ["--help" | "-h"] => Ok(FRESCO_USAGE.to_string()),
        [folder, image, rest @ ..] => {
            let n = |s: &str| {
                s.replace(',', ".")
                    .parse::<f64>()
                    .map_err(|_| usage("Nombre illisible."))
            };
            let mut frame = None;
            let mut page = None;
            let mut rest = rest;
            loop {
                match rest {
                    [] => break,
                    ["--frame", x, y, w, h, tail @ ..] => {
                        frame = Some(RectMm {
                            x: n(x)?,
                            y: n(y)?,
                            width: n(w)?,
                            height: n(h)?,
                        });
                        rest = tail;
                    }
                    ["--page", p, tail @ ..] => {
                        page = Some(
                            p.parse::<u32>()
                                .ok()
                                .filter(|&p| p > 0)
                                .ok_or_else(|| usage("Numéro de page illisible."))?,
                        );
                        rest = tail;
                    }
                    _ => return Err(usage("Option inconnue ou incomplète.")),
                }
            }
            let dir = ProjectDir::open(folder).map_err(|e| (false, e.to_string()))?;
            let project = import_fresco_page(&dir, Path::new(image), frame, page)
                .map_err(|e| (false, e.to_string()))?;
            Ok(serde_json::to_string_pretty(&project.fresco).expect("sérialisable"))
        }
        _ => Err(usage("Il faut le projet et le fichier de la fresque.")),
    }
}

pub fn export(args: &[&str]) -> Result<String, (bool, String)> {
    let usage = |m: &str| (true, format!("{m} Voir : insitu export --help"));
    match args {
        [] | ["--help" | "-h"] => Ok(EXPORT_USAGE.to_string()),
        [folder, destination, rest @ ..] => {
            let dir = ProjectDir::open(folder).map_err(|e| (false, e.to_string()))?;
            let mut settings = dir
                .load()
                .map_err(|e| (false, e.to_string()))?
                .project
                .export;
            let mut only = Vec::new();
            let mut control_plan = false;
            let mut rest = rest.iter();
            while let Some(&option) = rest.next() {
                match option {
                    "--mirror" => settings.mirror = true,
                    "--no-mirror" => settings.mirror = false,
                    "--negative" => settings.negative = true,
                    "--no-negative" => settings.negative = false,
                    "--margin" => settings.margin = true,
                    "--no-margin" => settings.margin = false,
                    "--one-bit" => settings.one_bit = true,
                    "--gray" => settings.one_bit = false,
                    "--svg" => settings.svg = true,
                    "--no-svg" => settings.svg = false,
                    "--sans-fresque" => settings.fresco = false,
                    "--avec-fresque" => settings.fresco = true,
                    "--plan-de-controle" => control_plan = true,
                    "--dpi" | "--threshold" | "--only" => {
                        let value = rest.next().ok_or_else(|| usage("Valeur manquante."))?;
                        match option {
                            "--dpi" => {
                                settings.dpi = value
                                    .replace(',', ".")
                                    .parse()
                                    .map_err(|_| usage("Résolution illisible."))?
                            }
                            "--threshold" => {
                                settings.threshold = value
                                    .parse()
                                    .map_err(|_| usage("Seuil illisible (1 à 255)."))?
                            }
                            _ => only.push(value.to_string()),
                        }
                    }
                    _ => return Err(usage(&format!("Option inconnue : {option}."))),
                }
            }
            let only = (!only.is_empty()).then_some(only);
            let run = if control_plan {
                export_control_plans
            } else {
                export_slides
            };
            let report = run(
                &dir,
                Path::new(destination),
                &settings,
                only.as_deref(),
                0,
                &|p| {
                    if let Some(current) = &p.current {
                        eprintln!("{}/{} : {current}…", p.done + 1, p.total);
                    }
                },
            )
            .map_err(|e| (false, e.to_string()))?;
            Ok(serde_json::to_string_pretty(&json!(report)).expect("sérialisable"))
        }
        _ => Err(usage("Il faut le projet et le dossier d'export.")),
    }
}

pub const SLIDES_USAGE: &str = r#"Usage :
  insitu slides <projet.insitu> --cache DOSSIER [--side N] [--sans-fresque] [--plan]

Vue diapositive : pour chaque projecteur, sa photo redressée et l'aperçu de
sa diapositive calculée (PNG de N px, 2 048 par défaut, blanc à opacité
variable), fabriqué dans DOSSIER s'il n'y est pas déjà. Résultat en JSON.

  --sans-fresque  l'aperçu sans la fresque, avec les seuls dessins posés
  --plan          aussi le plan de la façade vu par chaque projecteur"#;

pub fn slides(args: &[&str]) -> Result<String, (bool, String)> {
    use insitu_core::export::preview::{slide_views_with, SlideContent, SLIDE_PREVIEW_SIDE};
    let usage = |m: &str| (true, format!("{m} Voir : insitu slides --help"));
    match args {
        [] | ["--help" | "-h"] => Ok(SLIDES_USAGE.to_string()),
        [folder, "--cache", cache, rest @ ..] => {
            let mut side = SLIDE_PREVIEW_SIDE;
            let mut content = SlideContent::default();
            let mut rest = rest;
            loop {
                rest = match rest {
                    [] => break,
                    ["--side", n, more @ ..] => {
                        side = n.parse().map_err(|_| usage("Taille illisible."))?;
                        more
                    }
                    ["--sans-fresque", more @ ..] => {
                        content.fresco = false;
                        more
                    }
                    ["--plan", more @ ..] => {
                        content.plan = true;
                        more
                    }
                    _ => return Err(usage("Option inconnue ou incomplète.")),
                };
            }
            let dir = ProjectDir::open(folder).map_err(|e| (false, e.to_string()))?;
            let views = slide_views_with(&dir, Path::new(cache), side, content)
                .map_err(|e| (false, e.to_string()))?;
            Ok(serde_json::to_string_pretty(&views).expect("sérialisable"))
        }
        _ => Err(usage("Il faut le projet et --cache DOSSIER.")),
    }
}

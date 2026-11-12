//! Commandes `insitu place …` : dessins posés sur la façade, comme l'écran
//! Façade de l'app (tâches 4.2 à 4.6).

use insitu_core::placement::{self, new_placement};
use insitu_core::project::{Layer, Placement, PointMm, ProjectDir};

/// Ce qui peut mal tourner.
pub enum Problem {
    Usage(String),
    Failed(String),
}

pub const USAGE: &str = r#"Usage :
  insitu place list <projet.insitu>
  insitu place add <projet.insitu> <dessin> --at X,Y --width MM [--rotation DEG] [--mirror] [--group NOM]
  insitu place remove <projet.insitu> <id>…
  insitu place set <projet.insitu> <placements.json> [--history DOSSIER]
  insitu place deform <projet.insitu> <id>… (--corners | --grid CxR | --none)
  insitu place slides <projet.insitu>
  insitu place assign <projet.insitu> <id>… (--to PROJECTEUR | --auto)
  insitu place adjust <projet.insitu> <id> (--corners | --grid CxR | --none)

Dessins posés sur la façade. Longueurs en mm réels de façade, origine en haut
à gauche, y vers le bas ; rotation en degrés, sens des aiguilles d'une montre.

  list    Affiche les placements en JSON, du plus au fond au plus en avant.
  add     Pose un dessin de la bibliothèque (nom ou identifiant) au premier
          plan, centré en X,Y, large de --width mm ; la hauteur suit les
          proportions du fichier. Affiche l'identifiant du placement.
  remove  Retire des dessins posés (la bibliothèque n'est pas touchée).
  set     Remplace tous les placements par ceux du fichier JSON (même forme
          que la sortie de list), comme l'app à la fin de chaque geste.
          Rien n'est enregistré si un placement est incorrect. Le fichier
          peut aussi être {"layers": [...], "placements": [...]} : calques
          et dessins posés remplacés ensemble.
  deform  Tend une grille au repos sur les dessins donnés, ensemble (tâche
          4.4) : --corners, quatre coins en perspective ; --grid 3x2, une
          grille lisse de 3 × 2 cases ; --none retire la déformation. Les
          nœuds se déplacent ensuite par set (champ deformation.nodes).
  slides  Affiche en JSON la diapositive de chaque dessin posé (tâche 4.5) :
          projecteur, choix à la main ou non, dessin entier ou à cheval, et
          les diapositives qui le contiennent en entier.
  assign  Choisit à la main la diapositive des dessins donnés (--to, libellé
          ou identifiant du projecteur) ; --auto les rend à la proposition
          du moteur.
  adjust  Retouche le dessin dans la diapositive qui le porte, sans toucher
          à la façade (tâche 4.6) : --corners ou --grid CxR tend une grille
          au repos en mm de diapositive ; --none retire la retouche. Les
          nœuds se déplacent ensuite par set (champ adjustment).

  --history DOSSIER  Note l'enregistrement dans l'historique d'annulation
                     rangé là (voir insitu project undo)."#;

pub fn run(args: &[&str]) -> Result<String, Problem> {
    if matches!(args, [] | ["--help" | "-h"]) {
        return Ok(USAGE.to_string());
    }
    let usage = |message: &str| Problem::Usage(format!("{message} Voir : insitu place --help"));
    let number = |value: Option<&&str>, what: &str| {
        value
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .ok_or_else(|| usage(&format!("{what} attend un nombre.")))
    };
    let mut at: Option<PointMm> = None;
    let mut width: Option<f64> = None;
    let mut rotation = 0.0;
    let mut mirrored = false;
    let mut group: Option<String> = None;
    let mut history: Option<String> = None;
    let mut grid: Option<(u32, u32)> = None;
    let mut target: Option<Option<String>> = None;
    let mut words = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--at" => {
                let point = args.get(i + 1).and_then(|v| {
                    let (x, y) = v.split_once(',')?;
                    Some(PointMm::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
                });
                at = Some(point.ok_or_else(|| usage("--at attend X,Y en mm."))?);
                i += 1;
            }
            "--width" => {
                width = Some(number(args.get(i + 1), "--width")?);
                i += 1;
            }
            "--rotation" => {
                rotation = number(args.get(i + 1), "--rotation")?;
                i += 1;
            }
            "--group" => {
                group = Some(
                    args.get(i + 1)
                        .ok_or_else(|| usage("--group attend un nom."))?
                        .to_string(),
                );
                i += 1;
            }
            "--history" => {
                history = Some(
                    args.get(i + 1)
                        .ok_or_else(|| usage("--history attend un dossier."))?
                        .to_string(),
                );
                i += 1;
            }
            "--mirror" => mirrored = true,
            "--auto" => target = Some(None),
            "--to" => {
                target = Some(Some(
                    args.get(i + 1)
                        .ok_or_else(|| usage("--to attend un projecteur."))?
                        .to_string(),
                ));
                i += 1;
            }
            "--corners" => grid = Some((1, 1)),
            "--none" => grid = Some((0, 0)),
            "--grid" => {
                let size = args.get(i + 1).and_then(|v| {
                    let (c, r) = v.split_once(['x', '×'])?;
                    Some((c.trim().parse().ok()?, r.trim().parse().ok()?))
                });
                grid = Some(size.ok_or_else(|| usage("--grid attend CxR, par exemple 3x2."))?);
                i += 1;
            }
            other if other.starts_with("--") => {
                return Err(usage(&format!("Option inconnue : {other}")))
            }
            word => words.push(word),
        }
        i += 1;
    }
    match words[..] {
        ["list", folder] => {
            let dir = ProjectDir::open(folder).map_err(failed)?;
            let project = dir.load().map_err(failed)?.project;
            Ok(serde_json::to_string_pretty(&project.placements).expect("sérialisable"))
        }
        ["add", folder, drawing] => {
            let dir = ProjectDir::open(folder).map_err(failed)?;
            let project = dir.load().map_err(failed)?.project;
            let center = at.ok_or_else(|| usage("--at X,Y est nécessaire."))?;
            let width = width.ok_or_else(|| usage("--width est nécessaire."))?;
            let mut placement = new_placement(&project, drawing, center, width).map_err(failed)?;
            placement.rotation = rotation;
            placement.mirrored = mirrored;
            placement.group = group;
            let (_, id) = placement::add(&dir, placement).map_err(failed)?;
            Ok(id)
        }
        ["remove", folder, ref ids @ ..] if !ids.is_empty() => {
            let dir = ProjectDir::open(folder).map_err(failed)?;
            let ids: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
            let project = placement::remove(&dir, &ids).map_err(failed)?;
            Ok(format!(
                "{} dessin(s) posé(s) restent.",
                project.placements.len()
            ))
        }
        ["set", folder, file] => {
            let mut dir = ProjectDir::open(folder).map_err(failed)?;
            if let Some(root) = history {
                dir = dir.with_history(root);
            }
            let text = std::fs::read_to_string(file)
                .map_err(|e| Problem::Failed(format!("Impossible de lire « {file} » : {e}")))?;
            #[derive(serde::Deserialize)]
            #[serde(untagged)]
            enum Input {
                Placements(Vec<Placement>),
                Composition {
                    layers: Vec<Layer>,
                    placements: Vec<Placement>,
                },
            }
            let input: Input = serde_json::from_str(&text)
                .map_err(|e| Problem::Failed(format!("« {file} » est incorrect : {e}")))?;
            let project = match input {
                Input::Placements(placements) => placement::set_placements(&dir, placements),
                Input::Composition { layers, placements } => {
                    placement::set_composition(&dir, layers, placements)
                }
            }
            .map_err(failed)?;
            Ok(format!(
                "{} dessin(s) posé(s), {} calque(s).",
                project.placements.len(),
                project.layers.len()
            ))
        }
        ["deform", folder, ref ids @ ..] if !ids.is_empty() => {
            let (columns, rows) =
                grid.ok_or_else(|| usage("Choisir --corners, --grid CxR ou --none."))?;
            let dir = ProjectDir::open(folder).map_err(failed)?;
            let ids: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
            placement::deform(&dir, &ids, columns, rows).map_err(failed)?;
            Ok(if columns == 0 {
                format!("{} dessin(s) redressé(s).", ids.len())
            } else {
                format!("Grille de {columns} × {rows} sur {} dessin(s).", ids.len())
            })
        }
        ["adjust", folder, id] => {
            let (columns, rows) =
                grid.ok_or_else(|| usage("Choisir --corners, --grid CxR ou --none."))?;
            let dir = ProjectDir::open(folder).map_err(failed)?;
            let project = placement::adjust::adjust(&dir, id, columns, rows).map_err(failed)?;
            Ok(
                match project
                    .placements
                    .iter()
                    .find(|p| p.id == id)
                    .and_then(|p| p.adjustment.as_ref())
                {
                    Some(a) => format!(
                        "Retouche de {columns} × {rows} dans la diapositive {}.",
                        project
                            .projector(&a.projector)
                            .map(|p| p.label.as_str())
                            .unwrap_or("?")
                    ),
                    None => "Retouche retirée.".to_string(),
                },
            )
        }
        ["slides", folder] => {
            let dir = ProjectDir::open(folder).map_err(failed)?;
            let project = dir.load().map_err(failed)?.project;
            let all = placement::assign::assignments(&project);
            Ok(serde_json::to_string_pretty(&all).expect("sérialisable"))
        }
        ["assign", folder, ref ids @ ..] if !ids.is_empty() => {
            let target = target.ok_or_else(|| usage("Choisir --to PROJECTEUR ou --auto."))?;
            let dir = ProjectDir::open(folder).map_err(failed)?;
            let ids: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
            placement::assign(&dir, &ids, target.as_deref()).map_err(failed)?;
            Ok(match target {
                Some(p) => format!("{} dessin(s) dans la diapositive {p}.", ids.len()),
                None => format!("{} dessin(s) rendu(s) à la proposition.", ids.len()),
            })
        }
        _ => Err(usage("Commande de placement incomplète ou inconnue.")),
    }
}

fn failed(error: impl std::fmt::Display) -> Problem {
    Problem::Failed(error.to_string())
}

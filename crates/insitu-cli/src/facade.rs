//! Commandes `insitu facade …` : import du plan de façade, sans écran.

use std::path::Path;
use std::time::Instant;

use insitu_core::facade::{self, display, PlanChoice};
use insitu_core::project::{PointMm, Project, ProjectDir, SourcePoint};

/// Ce qui peut mal tourner dans une commande de façade.
pub enum Problem {
    Usage(String),
    Failed(String),
}

fn failed(error: impl std::fmt::Display) -> Problem {
    Problem::Failed(error.to_string())
}

/// Sépare `--page N` des autres arguments.
fn take_page<'a>(args: &[&'a str]) -> Result<(Option<u32>, Vec<&'a str>), Problem> {
    let mut page = None;
    let mut rest = Vec::new();
    let mut iter = args.iter().copied();
    while let Some(arg) = iter.next() {
        if arg == "--page" {
            page =
                Some(iter.next().and_then(|v| v.parse().ok()).ok_or_else(|| {
                    Problem::Usage("--page attend un numéro de page (1, 2…)".into())
                })?);
        } else {
            rest.push(arg);
        }
    }
    Ok((page, rest))
}

fn number(text: &str) -> Result<f64, Problem> {
    text.replace(',', ".")
        .parse()
        .map_err(|_| Problem::Usage(format!("« {text} » n'est pas un nombre")))
}

fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string_pretty(value).expect("sérialisable")
}

/// Sépare les `--layer NOM` (répétables) des autres arguments.
fn take_layers<'a>(args: &[&'a str]) -> Result<(Vec<String>, Vec<&'a str>), Problem> {
    let mut layers = Vec::new();
    let mut rest = Vec::new();
    let mut iter = args.iter().copied();
    while let Some(arg) = iter.next() {
        if arg == "--layer" {
            layers.push(
                iter.next()
                    .ok_or_else(|| Problem::Usage("--layer attend un nom de calque".into()))?
                    .to_string(),
            );
        } else {
            rest.push(arg);
        }
    }
    Ok((layers, rest))
}

/// Exécute `insitu facade <args>` et rend le texte à afficher.
pub fn run(args: &[&str]) -> Result<String, Problem> {
    let (page, args) = take_page(args)?;
    let (layers, args) = take_layers(&args)?;
    if let ["plans", rest @ ..] = &args[..] {
        return plans(rest, page, layers);
    }
    let layers = (!layers.is_empty()).then_some(layers);
    match args[..] {
        [] | ["--help" | "-h"] => Ok(USAGE.to_string()),
        ["inspect", plan] => facade::inspect(Path::new(plan), page)
            .map(|info| json(&info))
            .map_err(failed),
        ["import", project, plan] => {
            let dir = ProjectDir::open(project).map_err(failed)?;
            let project = facade::import_facade(&dir, Path::new(plan), page).map_err(failed)?;
            Ok(json(&project.facade))
        }
        ["remove", project] => {
            let dir = ProjectDir::open(project).map_err(failed)?;
            facade::remove_facade(&dir).map_err(failed)?;
            Ok("Plan retiré du projet ; le fichier reste dans facade/.".to_string())
        }
        ["rotate", project, turns] => {
            let dir = ProjectDir::open(project).map_err(failed)?;
            let turns: i32 = turns.parse().map_err(|_| {
                Problem::Usage("rotate attend un nombre de quarts de tour (1, -1, 2…)".into())
            })?;
            let project = facade::rotate_facade(&dir, turns).map_err(failed)?;
            Ok(json(&project.facade))
        }
        ["scale", project, xa, ya, xb, yb, distance] => {
            let dir = ProjectDir::open(project).map_err(failed)?;
            let a = SourcePoint::new(number(xa)?, number(ya)?);
            let b = SourcePoint::new(number(xb)?, number(yb)?);
            let project = facade::set_scale(&dir, a, b, number(distance)?).map_err(failed)?;
            let facade = project.facade.expect("façade présente");
            let (width, height) = facade.size_mm().expect("échelle posée");
            Ok(format!(
                "{}\nFaçade : {:.3} m × {:.3} m",
                json(&facade),
                width / 1000.0,
                height / 1000.0
            ))
        }
        ["tiles", plan, cache] => {
            let start = Instant::now();
            let shown = display::open_for_display_layers(
                Path::new(plan),
                page,
                layers.as_deref(),
                0,
                Path::new(cache),
            )
            .map_err(failed)?;
            eprintln!(
                "Affichage préparé en {:.1} s (niveaux fins rendus à la demande)",
                start.elapsed().as_secs_f64()
            );
            Ok(json(&shown))
        }
        ["render", plan, out, ref rest @ ..] => {
            let side = match rest {
                [] => 4096,
                ["--width", value] => value
                    .parse()
                    .map_err(|_| Problem::Usage("--width attend un nombre de pixels".into()))?,
                _ => return Err(Problem::Usage(USAGE.to_string())),
            };
            let start = Instant::now();
            let (width, height, rgba) =
                display::render_whole_layers(Path::new(plan), page, layers.as_deref(), 0, side)
                    .map_err(failed)?;
            image::save_buffer(out, &rgba, width, height, image::ExtendedColorType::Rgba8)
                .map_err(failed)?;
            Ok(format!(
                "{out} : {width} × {height} px, rendu en {:.1} s",
                start.elapsed().as_secs_f64()
            ))
        }
        [other, ..] => Err(Problem::Usage(format!(
            "Commande de façade inconnue : {other}\n\n{USAGE}"
        ))),
    }
}

/// Un groupe désigné par son identifiant ou son nom.
fn group_id(project: &Project, wanted: &str) -> Result<String, Problem> {
    project
        .plan_groups
        .iter()
        .find(|g| g.id == wanted || g.name.eq_ignore_ascii_case(wanted))
        .map(|g| g.id.clone())
        .ok_or_else(|| Problem::Failed(format!("Aucun plan ajouté ne s'appelle « {wanted} ».")))
}

/// Une vue d'un groupe désignée par son identifiant ou son nom.
fn plan_id(project: &Project, group: &str, wanted: &str) -> Result<String, Problem> {
    project
        .plan_group(group)
        .and_then(|g| {
            g.plans
                .iter()
                .find(|p| p.id == wanted || p.name.eq_ignore_ascii_case(wanted))
        })
        .map(|p| p.id.clone())
        .ok_or_else(|| Problem::Failed(format!("Aucune vue ne s'appelle « {wanted} ».")))
}

fn summary(project: &Project) -> String {
    let mut out = String::new();
    if let Some(facade) = &project.facade {
        out.push_str(&format!("Plan de référence : {}\n", facade.source));
    }
    for group in &project.plan_groups {
        let (w, h) = group.size_mm();
        out.push_str(&format!(
            "{}{} [{}] : {} ; coin ({:.0} ; {:.0}) mm, {:.4} mm par unité, {:.2} × {:.2} m, {} quart(s) de tour\n",
            if group.hidden { "(caché) " } else { "" },
            group.name,
            group.id,
            group.source,
            group.origin.x,
            group.origin.y,
            group.mm_per_unit,
            w / 1000.0,
            h / 1000.0,
            group.rotation,
        ));
        for plan in &group.plans {
            out.push_str(&format!(
                "  {}{} [{}]{}{}\n",
                if plan.hidden { "(cachée) " } else { "" },
                plan.name,
                plan.id,
                plan.page.map_or(String::new(), |p| format!(", page {p}")),
                plan.layers
                    .as_ref()
                    .map_or(String::new(), |l| format!(", calques : {}", l.join(", "))),
            ));
        }
    }
    if out.is_empty() {
        out.push_str("Aucun plan.\n");
    }
    out
}

/// `insitu facade plans …` : les plans ajoutés à l'espace de travail.
fn plans(args: &[&str], page: Option<u32>, layers: Vec<String>) -> Result<String, Problem> {
    let open = |path: &str| -> Result<(ProjectDir, Project), Problem> {
        let dir = ProjectDir::open(path).map_err(failed)?;
        let project = dir.load().map_err(failed)?.project;
        Ok((dir, project))
    };
    let turns = |text: &str| -> Result<i32, Problem> {
        text.parse()
            .map_err(|_| Problem::Usage("attendu : un nombre de quarts de tour (1, -1, 2…)".into()))
    };
    let project = match args[..] {
        ["list", path] => return Ok(summary(&open(path)?.1)),
        ["add", path, file] => {
            let dir = ProjectDir::open(path).map_err(failed)?;
            // Une vue par calque nommé ; sans calque, une vue entière.
            let choices: Vec<PlanChoice> = if layers.is_empty() {
                vec![PlanChoice {
                    page,
                    ..PlanChoice::default()
                }]
            } else {
                layers
                    .iter()
                    .map(|name| PlanChoice {
                        page,
                        layers: Some(vec![name.clone()]),
                        name: None,
                    })
                    .collect()
            };
            facade::add_plan_group(&dir, Path::new(file), &choices).map_err(failed)?
        }
        ["place", path, group, x, y, k] => {
            let (dir, project) = open(path)?;
            let id = group_id(&project, group)?;
            facade::place_plan_group(&dir, &id, PointMm::new(number(x)?, number(y)?), number(k)?)
                .map_err(failed)?
        }
        ["align", path, group, x1, y1, x2, y2, tx1, ty1, tx2, ty2] => {
            let (dir, project) = open(path)?;
            let id = group_id(&project, group)?;
            let p = |x: &str, y: &str| -> Result<PointMm, Problem> {
                Ok(PointMm::new(number(x)?, number(y)?))
            };
            let aligned = facade::align_plan_group(
                &dir,
                &id,
                [p(x1, y1)?, p(x2, y2)?],
                [p(tx1, ty1)?, p(tx2, ty2)?],
            )
            .map_err(failed)?;
            return Ok(format!(
                "{}Échelle multipliée par {:.4} ; écart d'angle restant : {:.2}°",
                summary(&aligned.project),
                aligned.factor,
                aligned.turn_degrees
            ));
        }
        ["rotate", path, group, n] => {
            let (dir, project) = open(path)?;
            let id = group_id(&project, group)?;
            facade::rotate_plan_group(&dir, &id, turns(n)?).map_err(failed)?
        }
        [action @ ("show" | "hide"), path, group, ref view @ ..] if view.len() <= 1 => {
            let (dir, project) = open(path)?;
            let id = group_id(&project, group)?;
            let view = view
                .first()
                .map(|v| plan_id(&project, &id, v))
                .transpose()?;
            facade::set_plan_hidden(&dir, &id, view.as_deref(), action == "hide").map_err(failed)?
        }
        ["rename", path, group, name] => {
            let (dir, project) = open(path)?;
            let id = group_id(&project, group)?;
            facade::rename_plan(&dir, &id, None, name).map_err(failed)?
        }
        ["rename", path, group, view, name] => {
            let (dir, project) = open(path)?;
            let id = group_id(&project, group)?;
            let view = plan_id(&project, &id, view)?;
            facade::rename_plan(&dir, &id, Some(&view), name).map_err(failed)?
        }
        ["remove", path, group, ref view @ ..] if view.len() <= 1 => {
            let (dir, project) = open(path)?;
            let id = group_id(&project, group)?;
            let view = view
                .first()
                .map(|v| plan_id(&project, &id, v))
                .transpose()?;
            facade::remove_plan(&dir, &id, view.as_deref()).map_err(failed)?
        }
        ["move", path, group, place] => {
            let (dir, project) = open(path)?;
            let id = group_id(&project, group)?;
            let place: usize = place
                .parse()
                .map_err(|_| Problem::Usage("move attend une place (0 : au fond)".into()))?;
            facade::move_plan_group(&dir, &id, place).map_err(failed)?
        }
        _ => return Err(Problem::Usage(USAGE.to_string())),
    };
    Ok(summary(&project))
}

pub const USAGE: &str = r#"Usage : insitu facade <commande>

  inspect <plan> [--page N]
      Nature du plan (pdf, svg, dxf, image), dimensions dans son unité
      (point PDF, pixel CSS, unité de dessin DXF, pixel), pages d'un PDF.

  import <projet.insitu> <plan> [--page N]
      Copie le plan dans le dossier facade/ du projet et en fait la façade.
      La mise à l'échelle précédente est oubliée.

  remove <projet.insitu>
      Retire le plan (et sa mise à l'échelle) du projet. Le fichier reste
      dans facade/ ; dessins posés et points de recalage sont gardés.

  rotate <projet.insitu> <quarts de tour>
      Tourne le plan de la façade : 1 = un quart de tour dans le sens des
      aiguilles d'une montre, -1 = sens inverse, 2 = demi-tour. L'échelle,
      les dessins posés et les points de recalage tournent avec lui.

  scale <projet.insitu> <xA> <yA> <xB> <yB> <cote en mm>
      Met la façade à l'échelle : deux points du plan, dans son unité
      (origine en haut à gauche, y vers le bas), et leur distance réelle.

  tiles <plan> <dossier> [--page N] [--layer NOM]…
      Prépare l'affichage dans un dossier de cache, comme le fait l'app.

  render <plan vectoriel> <sortie.png> [--page N] [--width PX] [--layer NOM]…
      Rend le plan entier en une image (4 096 px de côté par défaut) ;
      avec --layer, seulement ces calques d'un PDF ou d'un .ai.

Plans ajoutés par-dessus le plan de référence (groupe ou vue : identifiant
ou nom ; positions en mm de façade, origine en haut à gauche) :

  plans list <projet.insitu>
  plans add <projet.insitu> <fichier> [--page N] [--layer NOM]…
      Un groupe tiré du fichier ; une vue par calque nommé, sinon une vue
      entière. Même fichier que le plan de référence : posé pile dessus.
  plans place <projet.insitu> <groupe> <x> <y> <mm par unité du fichier>
  plans align <projet.insitu> <groupe> <x1> <y1> <x2> <y2> <X1> <Y1> <X2> <Y2>
      Les points (x, y) du plan, là où ils sont, viennent en (X, Y).
  plans rotate <projet.insitu> <groupe> <quarts de tour>
  plans show|hide <projet.insitu> <groupe> [vue]
  plans rename <projet.insitu> <groupe> [vue] <nom>
  plans remove <projet.insitu> <groupe> [vue]
  plans move <projet.insitu> <groupe> <place, 0 = au fond>
"#;

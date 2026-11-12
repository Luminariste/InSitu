//! Commandes `insitu facade …` : import du plan de façade, sans écran.

use std::path::Path;
use std::time::Instant;

use insitu_core::facade::{self, display};
use insitu_core::project::{ProjectDir, SourcePoint};

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

/// Exécute `insitu facade <args>` et rend le texte à afficher.
pub fn run(args: &[&str]) -> Result<String, Problem> {
    let (page, args) = take_page(args)?;
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
            let shown = facade::open_for_display(Path::new(plan), page, Path::new(cache))
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
                display::render_whole(Path::new(plan), page, side).map_err(failed)?;
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

pub const USAGE: &str = r#"Usage : insitu facade <commande>

  inspect <plan> [--page N]
      Nature du plan (pdf, svg, dxf, image), dimensions dans son unité
      (point PDF, pixel CSS, unité de dessin DXF, pixel), pages d'un PDF.

  import <projet.insitu> <plan> [--page N]
      Copie le plan dans le dossier facade/ du projet et en fait la façade.
      La mise à l'échelle précédente est oubliée.

  scale <projet.insitu> <xA> <yA> <xB> <yB> <cote en mm>
      Met la façade à l'échelle : deux points du plan, dans son unité
      (origine en haut à gauche, y vers le bas), et leur distance réelle.

  tiles <plan> <dossier> [--page N]
      Prépare l'affichage dans un dossier de cache, comme le fait l'app.

  render <plan vectoriel> <sortie.png> [--page N] [--width PX]
      Rend le plan entier en une image (4 096 px de côté par défaut).
"#;

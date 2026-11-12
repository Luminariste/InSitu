//! `insitu` : le moteur en ligne de commande.
//!
//! Chaque capacité du moteur doit rester appelable d'ici, pour que les agents
//! puissent la vérifier sans interface. Sous-commandes disponibles :
//!
//! - `insitu info` : carte d'identité du moteur, en JSON ;
//! - `insitu geometry fit <fichier.json>` : ajuste une déformation sur des
//!   paires de points et affiche les écarts et les points aberrants, en JSON
//!   (format du fichier : `insitu geometry --help`) ;
//! - `insitu project new <dossier.insitu> [--name <nom>]` : crée un projet ;
//! - `insitu project info <dossier.insitu>` : résumé d'un projet, en JSON ;
//! - `insitu project validate <dossier.insitu>` : contrôle le JSON et la
//!   présence des fichiers référencés ;
//! - `insitu projector add|photo|rectify|show|export …` : redressement de la
//!   photo d'un projecteur dans un projet, comme l'écran de redressement
//!   (détail : `insitu projector --help`) ;
//! - `insitu rectify <photo> [--output <image>] …` : détecte la mire dans
//!   une photo brute et la redresse (options : `insitu rectify --help`) ;
//! - `insitu target pdf <fichier.pdf> [--page a3|carre]` : écrit la mire
//!   imprimable ; `insitu target info` la décrit en JSON (repères, grille) ;
//! - `insitu facade inspect|import|scale|tiles|render …` : import du plan de
//!   façade et mise à l'échelle (détail : `insitu facade --help`) ;
//! - `insitu tiles <image> <dossier>` : pyramide de tuiles d'une image, pour
//!   la visionneuse (descripteur en JSON, durée sur la sortie d'erreur) ;
//! - `insitu warp <image> <sortie.png> <paires.json>` : déforme une image
//!   vers une diapositive de 280 mm (format : `insitu warp --help`).
//!
//! Code de sortie : 0 si tout va bien, 1 en cas d'échec ou de projet
//! invalide, 2 si la commande est mal écrite.

mod facade;
mod geometry;
mod projector;
mod rectify;
mod target;
mod warp;

use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use insitu_core::project::{self, ProjectDir, ProjectError};
use insitu_core::tiles::{self, TileFormat, TileOptions};
use serde_json::json;

/// Ce qui peut mal tourner dans une commande.
enum Failure {
    /// Commande mal écrite : on rappelle l'usage.
    Usage(String),
    /// La commande a échoué ; le message a déjà été préparé.
    Failed(String),
}

impl From<ProjectError> for Failure {
    fn from(error: ProjectError) -> Self {
        Failure::Failed(error.to_string())
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match run(&args) {
        Ok(code) => code,
        Err(Failure::Usage(message)) => {
            eprintln!("{message}\n\n{}", usage());
            ExitCode::from(2)
        }
        Err(Failure::Failed(message)) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn run(args: &[&str]) -> Result<ExitCode, Failure> {
    match args {
        ["info"] => {
            print_json(&insitu_core::engine_info())?;
            Ok(ExitCode::SUCCESS)
        }
        ["geometry", rest @ ..] => match geometry::run(rest) {
            Ok(text) => {
                println!("{text}");
                Ok(ExitCode::SUCCESS)
            }
            Err(geometry::Problem::Usage(message)) => Err(Failure::Usage(message)),
            Err(geometry::Problem::Failed(message)) => Err(Failure::Failed(message)),
        },
        ["facade", rest @ ..] => match facade::run(rest) {
            Ok(text) => {
                println!("{text}");
                Ok(ExitCode::SUCCESS)
            }
            Err(facade::Problem::Usage(message)) => Err(Failure::Usage(message)),
            Err(facade::Problem::Failed(message)) => Err(Failure::Failed(message)),
        },
        ["project", rest @ ..] => run_project(rest),
        ["projector", rest @ ..] => match projector::run(rest) {
            Ok(text) => {
                println!("{text}");
                Ok(ExitCode::SUCCESS)
            }
            Err(projector::Problem::Usage(message)) => Err(Failure::Usage(message)),
            Err(projector::Problem::Failed(message)) => Err(Failure::Failed(message)),
        },
        ["rectify", rest @ ..] => match rectify::run(rest) {
            Ok(text) => {
                println!("{text}");
                Ok(ExitCode::SUCCESS)
            }
            Err(rectify::Problem::Usage(message)) => Err(Failure::Usage(message)),
            Err(rectify::Problem::Failed(message)) => Err(Failure::Failed(message)),
        },
        ["target", rest @ ..] => match target::run(rest) {
            Ok(text) => {
                println!("{text}");
                Ok(ExitCode::SUCCESS)
            }
            Err(target::Problem::Usage(message)) => Err(Failure::Usage(message)),
            Err(target::Problem::Failed(message)) => Err(Failure::Failed(message)),
        },
        ["tiles", rest @ ..] => run_tiles(rest),
        ["warp", rest @ ..] => match warp::run(rest) {
            Ok(text) => {
                println!("{text}");
                Ok(ExitCode::SUCCESS)
            }
            Err(warp::Problem::Usage(message)) => Err(Failure::Usage(message)),
            Err(warp::Problem::Failed(message)) => Err(Failure::Failed(message)),
        },
        [] | ["--help"] | ["-h"] => {
            println!("{}", usage());
            Ok(ExitCode::SUCCESS)
        }
        [other, ..] => Err(Failure::Usage(format!("Commande inconnue : {other}"))),
    }
}

/// `insitu tiles <image> <dossier> [--tile-size N] [--format png|jpeg] [--quality Q]`
fn run_tiles(args: &[&str]) -> Result<ExitCode, Failure> {
    let usage = |message: &str| Failure::Usage(message.to_string());
    let mut options = TileOptions::default();
    let mut paths = Vec::new();
    let mut rest = args.iter().copied();
    while let Some(arg) = rest.next() {
        match arg {
            "--tile-size" => {
                options.tile_size = rest
                    .next()
                    .and_then(|value| value.parse().ok())
                    .ok_or_else(|| usage("--tile-size attend un nombre de pixels"))?;
            }
            "--format" => {
                options.format = Some(match rest.next() {
                    Some("png") => TileFormat::Png,
                    Some("jpeg") | Some("jpg") => TileFormat::Jpeg,
                    _ => return Err(usage("--format attend png ou jpeg")),
                });
            }
            "--quality" => {
                options.jpeg_quality = rest
                    .next()
                    .and_then(|value| value.parse().ok())
                    .ok_or_else(|| usage("--quality attend un nombre de 1 à 100"))?;
            }
            other if other.starts_with("--") => {
                return Err(Failure::Usage(format!("Option inconnue : {other}")));
            }
            path => paths.push(path),
        }
    }
    let [image, out_dir] = paths[..] else {
        return Err(usage(
            "Usage : insitu tiles <image> <dossier> [--tile-size N] [--format png|jpeg] [--quality Q]",
        ));
    };

    let start = Instant::now();
    let pyramid = tiles::build_pyramid(Path::new(image), Path::new(out_dir), &options)
        .map_err(|error| Failure::Failed(error.to_string()))?;
    print_json(&pyramid)?;
    eprintln!(
        "{} tuiles sur {} niveaux, en {:.1} s",
        pyramid.tile_count(),
        pyramid.levels.len(),
        start.elapsed().as_secs_f64()
    );
    Ok(ExitCode::SUCCESS)
}

fn run_project(args: &[&str]) -> Result<ExitCode, Failure> {
    match args {
        ["new", folder] => project_new(folder, None),
        ["new", folder, "--name", name] | ["new", "--name", name, folder] => {
            project_new(folder, Some(name))
        }
        ["info", folder] => project_info(folder),
        ["validate", folder] => project_validate(folder),
        ["new", ..] => Err(Failure::Usage(
            "Usage : insitu project new <dossier.insitu> [--name <nom>]".to_string(),
        )),
        ["info", ..] | ["validate", ..] => Err(Failure::Usage(format!(
            "Usage : insitu project {} <dossier.insitu>",
            args[0]
        ))),
        [other, ..] => Err(Failure::Usage(format!(
            "Commande de projet inconnue : {other}"
        ))),
        [] => Err(Failure::Usage(
            "Il manque la commande de projet.".to_string(),
        )),
    }
}

fn project_new(folder: &str, name: Option<&str>) -> Result<ExitCode, Failure> {
    let (dir, project) = ProjectDir::create(folder, name)?;
    println!(
        "Projet « {} » créé dans {}",
        project.name,
        dir.root().display()
    );
    Ok(ExitCode::SUCCESS)
}

fn project_info(folder: &str) -> Result<ExitCode, Failure> {
    let dir = ProjectDir::open(folder)?;
    let loaded = dir.load()?;
    let lock = project::read_lock(&dir)?;
    let project = &loaded.project;
    let summary = json!({
        "folder": dir.root().display().to_string(),
        "formatVersion": project.format_version,
        "migratedFrom": loaded.migrated_from,
        "id": project.id,
        "name": project.name,
        "createdAt": project.created_at,
        "facade": project.facade.as_ref().map(|facade| json!({
            "source": facade.source,
            "scaled": facade.scale.is_some(),
            "sizeMm": facade.size_mm().map(|(width, height)| json!({
                "width": width,
                "height": height,
            })),
        })),
        "projectors": project.projectors.iter().map(|projector| json!({
            "id": projector.id,
            "label": projector.label,
            "rawPhoto": projector.raw_photo,
            "rectifiedPhoto": projector.rectified_photo,
            "rectified": projector.rectification.is_some(),
            "registrationPoints": projector.registration.len(),
        })).collect::<Vec<_>>(),
        "fresco": project.fresco.as_ref().map(|fresco| &fresco.source),
        "export": {
            "dpi": project.export.dpi,
            "mirror": project.export.mirror,
            "negative": project.export.negative,
        },
        "unknownFields": loaded.unknown_fields,
        "lock": lock,
    });
    print_json(&summary)?;
    Ok(ExitCode::SUCCESS)
}

fn project_validate(folder: &str) -> Result<ExitCode, Failure> {
    let dir = ProjectDir::open(folder)?;
    let report = project::validate(&dir);
    for issue in &report.issues {
        println!("{issue}");
    }
    let errors = report.errors().count();
    let warnings = report.warnings().count();
    if report.is_valid() {
        println!("Projet valide ({warnings} avertissement(s)).");
        Ok(ExitCode::SUCCESS)
    } else {
        println!("Projet invalide : {errors} erreur(s), {warnings} avertissement(s).");
        Ok(ExitCode::from(1))
    }
}

fn print_json(value: &impl serde::Serialize) -> Result<(), Failure> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| Failure::Failed(format!("Impossible d'écrire le résultat en JSON : {e}")))?;
    println!("{text}");
    Ok(())
}

fn usage() -> &'static str {
    "Usage : insitu <commande>\n\n\
     Commandes :\n  \
     info                                          Affiche la carte d'identité du moteur (JSON)\n  \
     geometry fit <fichier.json>                   Ajuste une déformation sur des paires de points\n  \
     geometry --help                               Décrit le fichier attendu et le résultat\n  \
     facade --help                                 Import du plan de façade et mise à l'échelle\n  \
     project new <dossier.insitu> [--name <nom>]   Crée un projet vide\n  \
     project info <dossier.insitu>                 Affiche le résumé d'un projet (JSON)\n  \
     project validate <dossier.insitu>             Contrôle le projet et ses fichiers\n  \
     projector --help                              Photo de mire d'un projecteur : import, redressement, export\n  \
     rectify <photo> [--output <image>]            Détecte la mire et redresse la photo (rectify --help)\n  \
     target pdf <fichier.pdf> [--page a3|carre]    Écrit la mire imprimable (PDF, 280 mm)\n  \
     target info                                   Décrit la mire en JSON (repères, grille)\n  \
     tiles <image> <dossier>                       Fabrique la pyramide de tuiles d'une image\n      \
     --tile-size N                               côté des tuiles en pixels (512 par défaut)\n      \
     --format png|jpeg                           format imposé (sinon choisi d'après l'image)\n      \
     --quality Q                                 qualité JPEG de 1 à 100 (88 par défaut)\n  \
     warp <image> <sortie.png> <paires.json>       Déforme une image vers une diapositive de 280 mm\n  \
     warp --help                                   Décrit le fichier de paires et les options"
}

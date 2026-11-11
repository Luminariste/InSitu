//! `insitu` : le moteur en ligne de commande.
//!
//! Chaque capacité du moteur doit rester appelable d'ici, pour que les agents
//! puissent la vérifier sans interface. Sous-commandes disponibles :
//!
//! - `insitu info` : carte d'identité du moteur, en JSON ;
//! - `insitu project new <dossier.insitu> [--name <nom>]` : crée un projet ;
//! - `insitu project info <dossier.insitu>` : résumé d'un projet, en JSON ;
//! - `insitu project validate <dossier.insitu>` : contrôle le JSON et la
//!   présence des fichiers référencés.
//!
//! Code de sortie : 0 si tout va bien, 1 en cas d'échec ou de projet
//! invalide, 2 si la commande est mal écrite.

use std::process::ExitCode;

use insitu_core::project::{self, ProjectDir, ProjectError};
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
        ["project", rest @ ..] => run_project(rest),
        [] | ["--help"] | ["-h"] => {
            println!("{}", usage());
            Ok(ExitCode::SUCCESS)
        }
        [other, ..] => Err(Failure::Usage(format!("Commande inconnue : {other}"))),
    }
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
     project new <dossier.insitu> [--name <nom>]   Crée un projet vide\n  \
     project info <dossier.insitu>                 Affiche le résumé d'un projet (JSON)\n  \
     project validate <dossier.insitu>             Contrôle le projet et ses fichiers"
}

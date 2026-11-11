//! `insitu` : le moteur en ligne de commande.
//!
//! Chaque capacité du moteur doit rester appelable d'ici, pour que les agents
//! puissent la vérifier sans interface. Sous-commandes disponibles :
//!
//! - `insitu info` : carte d'identité du moteur, en JSON ;
//! - `insitu tiles <image> <dossier>` : pyramide de tuiles d'une image.

use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use insitu_core::tiles::{self, TileFormat, TileOptions};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("info") => {
            let info = insitu_core::engine_info();
            println!(
                "{}",
                serde_json::to_string_pretty(&info).expect("EngineInfo est sérialisable")
            );
            ExitCode::SUCCESS
        }
        Some("tiles") => match run_tiles(&args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                ExitCode::FAILURE
            }
        },
        Some("--help") | Some("-h") | None => {
            println!("{}", usage());
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("Commande inconnue : {other}\n\n{}", usage());
            ExitCode::from(2)
        }
    }
}

/// `insitu tiles <image> <dossier> [--tile-size N] [--format png|jpeg] [--quality Q]`
///
/// Le descripteur de la pyramide sort en JSON sur la sortie standard ; le
/// temps de fabrication sort sur la sortie d'erreur.
fn run_tiles(args: &[String]) -> Result<(), String> {
    let mut options = TileOptions::default();
    let mut paths = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = |name: &str| {
            rest.next()
                .ok_or_else(|| format!("Il manque une valeur après {name}"))
        };
        match arg.as_str() {
            "--tile-size" => {
                options.tile_size = value("--tile-size")?
                    .parse()
                    .map_err(|_| "--tile-size attend un nombre de pixels".to_string())?;
            }
            "--format" => {
                options.format = Some(match value("--format")?.as_str() {
                    "png" => TileFormat::Png,
                    "jpeg" | "jpg" => TileFormat::Jpeg,
                    other => return Err(format!("Format inconnu : {other} (png ou jpeg)")),
                });
            }
            "--quality" => {
                options.jpeg_quality = value("--quality")?
                    .parse()
                    .map_err(|_| "--quality attend un nombre de 1 à 100".to_string())?;
            }
            other if other.starts_with("--") => {
                return Err(format!("Option inconnue : {other}\n\n{}", usage()));
            }
            path => paths.push(path),
        }
    }
    let [image, out_dir] = paths[..] else {
        return Err(format!(
            "`tiles` attend une image et un dossier de sortie.\n\n{}",
            usage()
        ));
    };

    let start = Instant::now();
    let pyramid = tiles::build_pyramid(Path::new(image), Path::new(out_dir), &options)
        .map_err(|error| format!("Échec : {error}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&pyramid).expect("Pyramid est sérialisable")
    );
    eprintln!(
        "{} tuiles sur {} niveaux, en {:.1} s",
        pyramid.tile_count(),
        pyramid.levels.len(),
        start.elapsed().as_secs_f64()
    );
    Ok(())
}

fn usage() -> &'static str {
    "Usage : insitu <commande>\n\n\
     Commandes :\n  \
     info                      Affiche la carte d'identité du moteur (JSON)\n  \
     tiles <image> <dossier>   Fabrique la pyramide de tuiles d'une image\n      \
     --tile-size N           côté des tuiles en pixels (512 par défaut)\n      \
     --format png|jpeg       format imposé (choisi d'après le contenu par défaut)\n      \
     --quality Q             qualité JPEG de 1 à 100 (88 par défaut)"
}

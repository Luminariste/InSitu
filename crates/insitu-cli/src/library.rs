//! Commandes `insitu library …` : la bibliothèque de dessins d'un projet,
//! comme l'écran Bibliothèque de l'app (tâche 4.1).

use std::path::{Path, PathBuf};
use std::time::Instant;

use insitu_core::library::{
    self, decode, encode_png, import_folder, rendition, ImportOptions, THUMBNAIL_SIDE,
};
use insitu_core::project::ProjectDir;

/// Ce qui peut mal tourner.
pub enum Problem {
    Usage(String),
    Failed(String),
}

pub const USAGE: &str = r#"Usage :
  insitu library import <projet.insitu> <dossier> [--cache DOSSIER] [--threads N]
  insitu library list <projet.insitu> [--cache DOSSIER]
  insitu library thumbnail <dessin> <sortie.png> [--size PX]

Bibliothèque de dessins (traits blancs sur fond noir ; PNG, JPEG, TIFF, BMP,
PSD aplati, SVG).

  import     Copie le dossier et ses sous-dossiers dans le projet, sous
             bibliotheque/<nom du dossier>/, inscrit chaque dessin et
             enregistre le projet. Affiche le compte rendu en JSON (dessins
             ajoutés, déjà présents, fichiers illisibles) ; la durée est
             écrite sur la sortie d'erreur.
  list       Affiche la bibliothèque en JSON.
  thumbnail  Écrit la vignette d'un fichier de dessin : réduit de moitié en
             moitié jusqu'à tenir dans --size (320 par défaut), le noir
             rendu transparent.

  --cache DOSSIER  Où ranger les vignettes (comme le cache de l'app). Sans
                   cette option, import et list n'en font pas.
  --threads N      Nombre de fils (par défaut : un par cœur, 8 au plus)."#;

pub fn run(args: &[&str]) -> Result<String, Problem> {
    if matches!(args, [] | ["--help" | "-h"]) {
        return Ok(USAGE.to_string());
    }
    let usage = |message: &str| Problem::Usage(format!("{message} Voir : insitu library --help"));
    let mut cache: Option<PathBuf> = None;
    let mut threads = 0usize;
    let mut size = THUMBNAIL_SIDE;
    let mut words = Vec::new();
    let mut rest = args.iter().copied();
    while let Some(arg) = rest.next() {
        match arg {
            "--cache" => {
                cache = Some(
                    rest.next()
                        .ok_or_else(|| usage("--cache attend un dossier."))?
                        .into(),
                )
            }
            "--threads" => {
                threads = rest
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| usage("--threads attend un nombre."))?;
            }
            "--size" => {
                size = rest
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|&v: &u32| v >= 1)
                    .ok_or_else(|| usage("--size attend un nombre de pixels."))?;
            }
            other if other.starts_with("--") => {
                return Err(usage(&format!("Option inconnue : {other}")))
            }
            word => words.push(word),
        }
    }
    match words[..] {
        ["import", folder, source] => {
            let dir = ProjectDir::open(folder).map_err(failed)?;
            let options = ImportOptions {
                cache_root: cache,
                threads,
            };
            let start = Instant::now();
            let (_, report) =
                import_folder(&dir, Path::new(source), &options, &|_| {}).map_err(failed)?;
            eprintln!("Import en {:.1} s.", start.elapsed().as_secs_f64());
            Ok(serde_json::to_string_pretty(&report).expect("sérialisable"))
        }
        ["list", folder] => {
            let dir = ProjectDir::open(folder).map_err(failed)?;
            let entries = library::list(&dir, cache.as_deref(), threads).map_err(failed)?;
            Ok(serde_json::to_string_pretty(&entries).expect("sérialisable"))
        }
        ["thumbnail", file, output] => {
            let decoded = decode(Path::new(file), size).map_err(failed)?;
            let small = rendition(decoded.raster, size);
            let bytes = encode_png(&small).map_err(Problem::Failed)?;
            std::fs::write(output, bytes)
                .map_err(|e| Problem::Failed(format!("Impossible d'écrire « {output} » : {e}")))?;
            Ok(format!("{output} : {} × {} px", small.width, small.height))
        }
        _ => Err(usage("Commande de bibliothèque incomplète ou inconnue.")),
    }
}

fn failed(error: impl std::fmt::Display) -> Problem {
    Problem::Failed(error.to_string())
}

//! Commandes `insitu library …` : la bibliothèque de dessins d'un projet,
//! comme l'écran Bibliothèque de l'app (tâche 4.1).

use std::path::{Path, PathBuf};
use std::time::Instant;

use insitu_core::library::silhouette::{load_silhouettes, rasterize, set_silhouette};
use insitu_core::library::vectors::{
    drawing_vectors, set_vector_settings, vectorize_drawings, VectorSummary,
};
use insitu_core::library::{
    self, decode, encode_png, import_folder, rendition, ImportOptions, THUMBNAIL_SIDE,
};
use insitu_core::project::{ProjectDir, SilhouettePolygon, VectorSettings};

/// Ce qui peut mal tourner.
pub enum Problem {
    Usage(String),
    Failed(String),
}

pub const USAGE: &str = r#"Usage :
  insitu library import <projet.insitu> <dossier> [--cache DOSSIER] [--threads N]
  insitu library list <projet.insitu> [--cache DOSSIER]
  insitu library remove <projet.insitu> <dessin>
  insitu library thumbnail <dessin> <sortie.png> [--size PX]
  insitu library vectors <projet.insitu> [<dessin>] --cache DOSSIER
         [--threshold N] [--tolerance PX] [--corner DEG] [--min-area PX2]
         [--default]
  insitu library silhouette <projet.insitu> <dessin> --cache DOSSIER
         [--from CONTOURS.json | --none | --default] [--mask SORTIE.png]

Bibliothèque de dessins (traits blancs sur fond noir ; PNG, JPEG, TIFF, BMP,
PSD aplati, SVG).

  import     Copie le dossier et ses sous-dossiers dans le projet, sous
             bibliotheque/<nom du dossier>/, inscrit chaque dessin et
             enregistre le projet ; avec --cache, vectorise ensuite les
             nouveaux dessins. Affiche le compte rendu en JSON (dessins
             ajoutés, déjà présents, fichiers illisibles) ; la durée est
             écrite sur la sortie d'erreur.
  list       Affiche la bibliothèque en JSON.
  remove     Retire un dessin (identifiant ou nom) de la bibliothèque, et ses
             placements sur la façade. Le fichier reste dans bibliotheque/.
  thumbnail  Écrit la vignette d'un fichier de dessin : réduit de moitié en
             moitié jusqu'à tenir dans --size (320 par défaut), le noir
             rendu transparent.
  vectors    Vecteurs des dessins (tâche 5.2), calculés s'ils ne sont pas
             dans le cache. Sans <dessin> (identifiant ou nom) : un tableau
             de toute la bibliothèque (contours, morceaux, écart maximal
             entre les courbes et l'image, durée). Avec <dessin> : le compte
             rendu en JSON. Les options de réglage changent les réglages du
             dessin et enregistrent le projet ; --default le ramène aux
             réglages par défaut (seuil 128, tolérance 0,3 px, angle 70°,
             aire minimale 0).
  silhouette Silhouette d'un dessin (tâche 4.7) en JSON : polygones en
             fractions du dessin, et « manual » s'il a été retouché. --from
             la remplace par les contours d'un fichier JSON (une liste de
             polygones, chacun une liste de points [u, v]) ; --none retire
             la silhouette (le dessin ne cache plus rien) ; --default revient
             à la détection automatique. Ces trois options enregistrent le
             projet. --mask écrit la silhouette en image (blanc dedans), au
             côté --size (1 024 par défaut).

  --cache DOSSIER  Où ranger vignettes et vecteurs (comme le cache de
                   l'app). Sans cette option, import et list n'en font pas.
  --threads N      Nombre de fils (par défaut : un par cœur, 8 au plus)."#;

pub fn run(args: &[&str]) -> Result<String, Problem> {
    if matches!(args, [] | ["--help" | "-h"]) {
        return Ok(USAGE.to_string());
    }
    let usage = |message: &str| Problem::Usage(format!("{message} Voir : insitu library --help"));
    let mut cache: Option<PathBuf> = None;
    let mut threads = 0usize;
    let mut size = THUMBNAIL_SIDE;
    let mut settings: Option<VectorSettings> = None;
    let mut reset = false;
    let mut size_given = false;
    let mut from: Option<String> = None;
    let mut none = false;
    let mut mask: Option<String> = None;
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
                size_given = true;
            }
            "--from" | "--mask" => {
                let value = rest
                    .next()
                    .ok_or_else(|| usage(&format!("{arg} attend un fichier.")))?
                    .to_string();
                if arg == "--from" {
                    from = Some(value);
                } else {
                    mask = Some(value);
                }
            }
            "--none" => none = true,
            "--threshold" | "--tolerance" | "--corner" | "--min-area" => {
                let value: f64 = rest
                    .next()
                    .and_then(|v| v.replace(',', ".").parse().ok())
                    .ok_or_else(|| usage(&format!("{arg} attend un nombre.")))?;
                let s = settings.get_or_insert_with(VectorSettings::default);
                match arg {
                    "--threshold" => {
                        if !(1.0..=255.0).contains(&value) || value.fract() != 0.0 {
                            return Err(usage("--threshold attend un entier de 1 à 255."));
                        }
                        s.threshold = value as u8;
                    }
                    "--tolerance" => s.tolerance_px = value,
                    "--corner" => s.corner_angle_deg = value,
                    _ => s.min_area_px = value,
                }
            }
            "--default" => reset = true,
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
            // Puis les vecteurs des nouveaux dessins (tâche 5.2), comme l'app.
            if let (Some(cache), false) = (&options.cache_root, report.added.is_empty()) {
                let vectors = vectorize_drawings(&dir, &report.added, cache, threads, &|_| {})
                    .map_err(failed)?;
                eprintln!(
                    "{} dessins vectorisés en {:.1} s.",
                    vectors.done, vectors.seconds
                );
                for failure in &vectors.failures {
                    eprintln!("Non vectorisé : {} : {}", failure.file, failure.reason);
                }
            }
            Ok(serde_json::to_string_pretty(&report).expect("sérialisable"))
        }
        ["list", folder] => {
            let dir = ProjectDir::open(folder).map_err(failed)?;
            let entries = library::list(&dir, cache.as_deref(), threads).map_err(failed)?;
            Ok(serde_json::to_string_pretty(&entries).expect("sérialisable"))
        }
        ["remove", folder, drawing] => {
            let dir = ProjectDir::open(folder).map_err(failed)?;
            let project = dir.load().map_err(failed)?.project;
            let found = library::find(&project, drawing).map_err(failed)?;
            let (id, name) = (found.id.clone(), found.name.clone());
            let (_, placed) = library::remove_drawing(&dir, &id).map_err(failed)?;
            Ok(format!(
                "« {name} » retiré de la bibliothèque ({placed} placement(s) retiré(s)) ; le fichier reste dans le projet."
            ))
        }
        ["thumbnail", file, output] => {
            let decoded = decode(Path::new(file), size).map_err(failed)?;
            let small = rendition(decoded.raster, size);
            let bytes = encode_png(&small).map_err(Problem::Failed)?;
            std::fs::write(output, bytes)
                .map_err(|e| Problem::Failed(format!("Impossible d'écrire « {output} » : {e}")))?;
            Ok(format!("{output} : {} × {} px", small.width, small.height))
        }
        ["vectors", folder, ref only @ ..] if only.len() <= 1 => {
            let cache = cache.ok_or_else(|| usage("vectors demande --cache DOSSIER."))?;
            let dir = ProjectDir::open(folder).map_err(failed)?;
            if reset && settings.is_some() {
                return Err(usage("--default ne se combine pas avec un réglage."));
            }
            match only {
                [drawing] => {
                    let project = dir.load().map_err(failed)?.project;
                    let id = library::find(&project, drawing).map_err(failed)?.id.clone();
                    let summary = if reset || settings.is_some() {
                        let wanted = if reset { None } else { settings };
                        set_vector_settings(&dir, &id, wanted, &cache)
                            .map_err(failed)?
                            .1
                    } else {
                        drawing_vectors(&dir, &id, &cache).map_err(failed)?
                    };
                    Ok(serde_json::to_string_pretty(&summary).expect("sérialisable"))
                }
                _ => {
                    if reset || settings.is_some() {
                        return Err(usage("Les réglages se changent dessin par dessin."));
                    }
                    let project = dir.load().map_err(failed)?.project;
                    let mut table = String::from(
                        "dessin\tréglages\tcontours\tmorceaux\técart max (px)\tdurée (s)\n",
                    );
                    for drawing in &project.library {
                        let line = match drawing_vectors(&dir, &drawing.id, &cache) {
                            Ok(summary) => summary_line(&summary),
                            Err(error) => format!("erreur : {error}"),
                        };
                        table.push_str(&format!("{}\t{line}\n", drawing.name));
                    }
                    Ok(table.trim_end().to_string())
                }
            }
        }
        ["silhouette", folder, drawing] => {
            let cache = cache.ok_or_else(|| usage("silhouette demande --cache DOSSIER."))?;
            if [from.is_some(), none, reset].iter().filter(|&&x| x).count() > 1 {
                return Err(usage("--from, --none et --default s'excluent."));
            }
            let dir = ProjectDir::open(folder).map_err(failed)?;
            let project = dir.load().map_err(failed)?.project;
            let id = library::find(&project, drawing).map_err(failed)?.id.clone();
            let current = if from.is_some() || none || reset {
                let wanted: Option<Vec<SilhouettePolygon>> = match &from {
                    Some(file) => {
                        let text = std::fs::read_to_string(file).map_err(|e| {
                            Problem::Failed(format!("Impossible de lire « {file} » : {e}"))
                        })?;
                        Some(serde_json::from_str(&text).map_err(|e| {
                            Problem::Failed(format!("Contours illisibles dans « {file} » : {e}"))
                        })?)
                    }
                    None if none => Some(Vec::new()),
                    None => None,
                };
                set_silhouette(&dir, &id, wanted, &cache).map_err(failed)?.1
            } else {
                load_silhouettes(&dir, std::slice::from_ref(&id), &cache)
                    .map_err(failed)?
                    .pop()
                    .expect("un dessin demandé")
            };
            if let Some(output) = &mask {
                let side = if size_given { size } else { 1024 };
                let d = project.drawing(&id).expect("trouvé plus haut");
                let scale = f64::from(side) / d.width.max(d.height);
                let (w, h) = (
                    ((d.width * scale).round() as u32).max(1),
                    ((d.height * scale).round() as u32).max(1),
                );
                let image = rasterize(&current.polygons, w, h);
                let bytes = encode_png(&image).map_err(Problem::Failed)?;
                std::fs::write(output, bytes).map_err(|e| {
                    Problem::Failed(format!("Impossible d'écrire « {output} » : {e}"))
                })?;
            }
            Ok(serde_json::to_string_pretty(&current).expect("sérialisable"))
        }
        _ => Err(usage("Commande de bibliothèque incomplète ou inconnue.")),
    }
}

fn summary_line(summary: &VectorSummary) -> String {
    format!(
        "{}\t{}\t{}\t{:.3}\t{:.2}",
        if summary.default_settings {
            "défaut"
        } else {
            "propres"
        },
        summary.stats.contours,
        summary.stats.lines + summary.stats.cubics,
        summary.comparison.max_deviation_px,
        summary.seconds
    )
}

fn failed(error: impl std::fmt::Display) -> Problem {
    Problem::Failed(error.to_string())
}

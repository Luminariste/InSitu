//! `insitu illustrator` : lecture d'un fichier Illustrator et import de sa
//! composition en dessins posés (tâche 6.3).

use std::path::Path;

use insitu_core::illustrator::{import_composition, pose, preview_composition, read, resolve_link};
use insitu_core::library::ImportOptions;
use insitu_core::project::{ProjectDir, RectMm};

pub const USAGE: &str = r#"Usage :
  insitu illustrator inspect <fichier.ai>
  insitu illustrator preview <projet.insitu> <fichier.ai> [--frame X Y LARGEUR HAUTEUR]
  insitu illustrator import <projet.insitu> <fichier.ai> [--frame X Y LARGEUR HAUTEUR]
                            [--cache <dossier>] [--images 0,3,4…] [--layer NOM]…

inspect  Décrit en JSON le plan de travail et les images liées du .ai : calque,
         lien, fichier retrouvé sur ce Mac (ou null), position sur une façade
         de 1 000 mm de large (pour contrôle).
preview  Ce que l'import ferait dans ce projet, sans rien écrire (JSON) : calques
         d'Illustrator, chaque fichier lié (déjà dans la bibliothèque, à ajouter
         ou introuvable) et chaque image à sa place sur la façade, avec son rang.
import   Ajoute chaque fichier lié à la bibliothèque (bibliotheque/<nom du .ai>/)
         et pose chaque image placée sur la façade, dans un calque du nom de
         celui d'Illustrator. Le plan de travail couvre toute la largeur de la
         façade (qui doit être à l'échelle), ou le rectangle --frame en mm.
         --cache : dossier des vignettes, comme l'app.
         --images : n'importer que ces images (rangs de « preview ») ;
         --layer : que les images de ce calque (répétable). Un lien déjà
         dans la bibliothèque (même fichier, ou même nom s'il manque ici)
         reprend ce dessin au lieu d'en ajouter un.

Le .ai doit avoir été enregistré avec « Créer un fichier compatible PDF ».
Les fichiers liés sont cherchés à côté du .ai, comme dans Illustrator."#;

pub fn run(args: &[&str]) -> Result<String, (bool, String)> {
    let usage = |m: &str| (true, format!("{m} Voir : insitu illustrator --help"));
    match args {
        [] | ["--help" | "-h"] => Ok(USAGE.to_string()),
        ["inspect", ai] => {
            let path = Path::new(ai);
            let doc = read(path).map_err(|e| (false, e.to_string()))?;
            let (aw, ah) = doc.artboard_size();
            let frame = RectMm {
                x: 0.0,
                y: 0.0,
                width: 1_000.0,
                height: 1_000.0 * ah / aw,
            };
            let images: Vec<_> = doc
                .placed
                .iter()
                .map(|p| {
                    let pose = pose(p, doc.artboard, &frame);
                    serde_json::json!({
                        "layer": p.layer,
                        "visible": p.visible,
                        "link": p.absolute,
                        "relativeDir": p.relative_dir,
                        "found": resolve_link(path, p).map(|f| f.display().to_string()),
                        "sizePx": [p.size_px().0, p.size_px().1],
                        "center": [pose.center.x, pose.center.y],
                        "width": pose.width,
                        "height": pose.height,
                        "rotation": pose.rotation,
                        "mirrored": pose.mirrored,
                    })
                })
                .collect();
            Ok(serde_json::to_string_pretty(&serde_json::json!({
                "artboard": { "widthPt": aw, "heightPt": ah },
                "embedded": doc.embedded,
                "placed": images,
            }))
            .expect("sérialisable"))
        }
        ["import", folder, ai, rest @ ..] => {
            let n = |s: &str| {
                s.replace(',', ".")
                    .parse::<f64>()
                    .map_err(|_| usage("Nombre illisible."))
            };
            let mut frame = None;
            let mut options = ImportOptions::default();
            let mut images: Option<Vec<usize>> = None;
            let mut layers: Vec<String> = Vec::new();
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
                    ["--cache", cache, tail @ ..] => {
                        options.cache_root = Some(cache.into());
                        rest = tail;
                    }
                    ["--images", list, tail @ ..] => {
                        let parsed: Result<Vec<usize>, _> = list
                            .split(',')
                            .filter(|t| !t.trim().is_empty())
                            .map(|t| t.trim().parse::<usize>())
                            .collect();
                        images = Some(parsed.map_err(|_| usage("Liste d'images illisible."))?);
                        rest = tail;
                    }
                    ["--layer", name, tail @ ..] => {
                        layers.push(name.to_string());
                        rest = tail;
                    }
                    _ => return Err(usage("Option inconnue ou incomplète.")),
                }
            }
            let dir = ProjectDir::open(folder).map_err(|e| (false, e.to_string()))?;
            if !layers.is_empty() {
                let preview = preview_composition(&dir, Path::new(ai), frame)
                    .map_err(|e| (false, e.to_string()))?;
                let in_layers: Vec<usize> = preview
                    .images
                    .iter()
                    .enumerate()
                    .filter(|(_, im)| layers.contains(&preview.layers[im.layer].name))
                    .map(|(i, _)| i)
                    .collect();
                images = Some(match images {
                    Some(list) => list.into_iter().filter(|i| in_layers.contains(i)).collect(),
                    None => in_layers,
                });
            }
            let (_, report) = import_composition(
                &dir,
                Path::new(ai),
                frame,
                images.as_deref(),
                &options,
                &|_| {},
            )
            .map_err(|e| (false, e.to_string()))?;
            Ok(serde_json::to_string_pretty(&report).expect("sérialisable"))
        }
        ["preview", folder, ai, rest @ ..] => {
            let frame = match rest {
                [] => None,
                ["--frame", x, y, w, h] => {
                    let n = |s: &str| {
                        s.replace(',', ".")
                            .parse::<f64>()
                            .map_err(|_| usage("Nombre illisible."))
                    };
                    Some(RectMm {
                        x: n(x)?,
                        y: n(y)?,
                        width: n(w)?,
                        height: n(h)?,
                    })
                }
                _ => return Err(usage("Option inconnue ou incomplète.")),
            };
            let dir = ProjectDir::open(folder).map_err(|e| (false, e.to_string()))?;
            let preview = preview_composition(&dir, Path::new(ai), frame)
                .map_err(|e| (false, e.to_string()))?;
            Ok(serde_json::to_string_pretty(&preview).expect("sérialisable"))
        }
        _ => Err(usage("Commande inconnue.")),
    }
}

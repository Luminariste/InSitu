//! Vecteurs en bibliothèque (tâche 5.2) : chaque dessin est vectorisé par le
//! moteur de la tâche 5.1, avec des réglages propres au dessin s'il en a.
//!
//! Comme les vignettes (D-4.1-d), le résultat est rangé dans le cache de
//! l'app, sous `vecteurs/`, et pas dans le projet : seuls les réglages d'un
//! dessin sont écrits dans `projet.json` (champ `vector`). La clé du cache
//! tient compte du fichier (chemin, taille, date) et des réglages : changer
//! un réglage ou le fichier refait la vectorisation.
//!
//! Après un import, [`vectorize_drawings`] vectorise les nouveaux dessins,
//! plusieurs à la fois, pendant qu'on parcourt déjà la bibliothèque : l'import
//! lui-même garde sa vitesse (critère de 4.1).
//!
//! Pour chaque dessin, trois fichiers :
//! - `<clé>.json` : le compte rendu ([`VectorSummary`]) ;
//! - `<clé>-trait.svg` : les courbes remplies de blanc, sur fond transparent ;
//! - `<clé>-contour.svg` : les courbes au trait rouge, à poser sur l'image.
//!
//! Les deux SVG sont en pixels du dessin vectorisé, comme l'image de
//! comparaison ([`COMPARE_SIDE`]) : l'interface les superpose dans le même
//! cadre.

use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::time::{Instant, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::project::model::new_id;
use crate::project::{Project, ProjectDir, VectorSettings};
use crate::tiles;
use crate::vectorize::{self, Comparison, GrayImage, VectorizeOptions, VectorizeStats};

use super::read::decode;
use super::{
    ensure_rendition, parallel_each, thread_count, unreadable, ImportFailure, ImportProgress,
    LibraryError,
};

/// Sous-dossier du cache où sont rangés les vecteurs.
pub const VECTOR_DIR: &str = "vecteurs";

/// Un SVG de la bibliothèque est rendu à ce côté (pixels) avant d'être
/// vectorisé, comme une image de 4 096 px.
pub const SVG_VECTOR_SIDE: u32 = 4096;

/// Plus grand côté de l'image de comparaison (le dessin tel quel, fond noir
/// rendu transparent). Un dessin de 4 096 px y est à pleine résolution.
pub const COMPARE_SIDE: u32 = 4096;

/// Version du calcul : à augmenter si la vectorisation change, pour que les
/// vecteurs en cache soient refaits.
const VECTOR_VERSION: u32 = 1;

/// Compte rendu de la vectorisation d'un dessin, tel que l'interface le
/// montre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorSummary {
    /// Réglages utilisés.
    pub settings: VectorSettings,
    /// Vrai si ce sont les réglages par défaut (rien d'écrit dans le projet).
    pub default_settings: bool,
    /// Taille de l'image vectorisée, en pixels : celle du fichier, ou le
    /// rendu à 4 096 px d'un SVG. C'est le cadre des deux SVG.
    pub width: u32,
    pub height: u32,
    pub stats: VectorizeStats,
    /// Écart mesuré entre les courbes et l'image (critère de 5.1 : moins
    /// d'un demi-pixel).
    pub comparison: Comparison,
    /// Durée de la vectorisation et de la mesure, en secondes.
    pub seconds: f64,
    /// Courbes remplies de blanc, fond transparent (fichier du cache).
    pub filled_svg: String,
    /// Courbes au trait rouge, fond transparent (fichier du cache).
    pub outline_svg: String,
    /// Le dessin tel quel, fond noir transparent, 4 096 px au plus (fichier
    /// du cache). Rempli par [`drawing_vectors`] ; vide dans le fichier
    /// `<clé>.json`.
    #[serde(default)]
    pub image: String,
}

/// Réglages d'un dessin, ceux par défaut s'il n'en a pas.
pub fn settings_of(project: &Project, id: &str) -> Result<VectorSettings, LibraryError> {
    project
        .drawing(id)
        .map(|d| d.vector.unwrap_or_default())
        .ok_or_else(|| LibraryError::UnknownDrawing(id.to_string()))
}

/// Options du moteur de vectorisation pour ces réglages.
pub fn options(settings: &VectorSettings, threads: usize) -> VectorizeOptions {
    VectorizeOptions {
        threshold: settings.threshold,
        tolerance_px: settings.tolerance_px,
        corner_angle_deg: settings.corner_angle_deg,
        min_area_px: settings.min_area_px,
        threads,
    }
}

/// Vérifie des réglages sans rien calculer.
pub fn check_settings(settings: &VectorSettings) -> Result<(), LibraryError> {
    vectorize::validate(&options(settings, 1)).map_err(|e| LibraryError::Settings(e.to_string()))
}

/// Préfixe des fichiers du cache (`vecteurs/<clé>`) pour ce fichier et ces
/// réglages.
pub fn vector_request_prefix(
    path: &Path,
    settings: &VectorSettings,
) -> Result<String, LibraryError> {
    let canonical = fs::canonicalize(path).map_err(|e| unreadable(path, e))?;
    let metadata = fs::metadata(&canonical).map_err(|e| unreadable(path, e))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    let identity = format!(
        "{}|{}|{modified}|{}|{}|{}|{}|v{VECTOR_VERSION}",
        canonical.display(),
        metadata.len(),
        settings.threshold,
        settings.tolerance_px,
        settings.corner_angle_deg,
        settings.min_area_px,
    );
    Ok(format!(
        "{VECTOR_DIR}/{:016x}",
        tiles::fnv1a_64(identity.as_bytes())
    ))
}

/// Lit le compte rendu en cache, s'il existe et se lit.
fn cached(cache_root: &Path, prefix: &str) -> Option<VectorSummary> {
    let text = fs::read_to_string(cache_root.join(format!("{prefix}.json"))).ok()?;
    let summary: VectorSummary = serde_json::from_str(&text).ok()?;
    let complete = cache_root.join(&summary.filled_svg).is_file()
        && cache_root.join(&summary.outline_svg).is_file();
    complete.then_some(summary)
}

/// Vectorise le dessin `path` avec `settings` et range le résultat dans le
/// cache, sauf s'il y est déjà. `threads` : fils de la vectorisation (0 :
/// tous).
pub fn ensure_vectors(
    path: &Path,
    settings: &VectorSettings,
    cache_root: &Path,
    threads: usize,
) -> Result<VectorSummary, LibraryError> {
    let prefix = vector_request_prefix(path, settings)?;
    if let Some(summary) = cached(cache_root, &prefix) {
        return Ok(summary);
    }
    let start = Instant::now();
    // Un SVG est rendu à 4 096 px ; une image est lue telle quelle.
    let raster = decode(path, SVG_VECTOR_SIDE)?.raster;
    let gray = GrayImage::from_raster(&raster);
    drop(raster);
    let vector = vectorize::vectorize(&gray, &options(settings, threads))
        .map_err(|e| LibraryError::Settings(e.to_string()))?;
    let comparison = vectorize::compare(&gray, &vector);
    let summary = VectorSummary {
        settings: *settings,
        default_settings: *settings == VectorSettings::default(),
        width: vector.width,
        height: vector.height,
        stats: vector.stats.clone(),
        comparison,
        seconds: start.elapsed().as_secs_f64(),
        filled_svg: format!("{prefix}-trait.svg"),
        outline_svg: format!("{prefix}-contour.svg"),
        image: String::new(),
    };
    let (w, h) = (vector.width, vector.height);
    let data = vector.path_data();
    let filled = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n\
         <path fill=\"white\" fill-rule=\"nonzero\" d=\"{data}\"/>\n</svg>\n"
    );
    let outline = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n\
         <path fill=\"none\" stroke=\"#ff2d2d\" stroke-width=\"1\" vector-effect=\"non-scaling-stroke\" \
         stroke-linejoin=\"round\" d=\"{data}\"/>\n</svg>\n"
    );
    write_atomic(cache_root, &summary.filled_svg, filled.as_bytes())?;
    write_atomic(cache_root, &summary.outline_svg, outline.as_bytes())?;
    let json = serde_json::to_vec_pretty(&summary).expect("sérialisable");
    // Le compte rendu en dernier : sa présence dit que tout est prêt.
    write_atomic(cache_root, &format!("{prefix}.json"), &json)?;
    Ok(summary)
}

/// Écrit un fichier du cache sans jamais laisser voir un fichier à moitié
/// écrit.
pub(super) fn write_atomic(
    cache_root: &Path,
    request: &str,
    bytes: &[u8],
) -> Result<(), LibraryError> {
    let target = cache_root.join(request);
    let failed = |reason: String| LibraryError::Cache {
        path: target.clone(),
        reason,
    };
    let folder = target.parent().expect("<dossier>/<fichier>");
    fs::create_dir_all(folder).map_err(|e| failed(e.to_string()))?;
    let temp = folder.join(format!(".~{}.{}", new_id(), std::process::id()));
    let written = fs::File::create(&temp)
        .and_then(|mut file| file.write_all(bytes))
        .and_then(|()| fs::rename(&temp, &target));
    if let Err(error) = written {
        let _ = fs::remove_file(&temp);
        return Err(failed(error.to_string()));
    }
    Ok(())
}

/// Les vecteurs d'un dessin du projet, avec ses réglages, calculés s'ils ne
/// sont pas en cache (autre Mac, cache vidé, réglage changé), et l'image de
/// comparaison.
pub fn drawing_vectors(
    dir: &ProjectDir,
    id: &str,
    cache_root: &Path,
) -> Result<VectorSummary, LibraryError> {
    let project = dir.load()?.project;
    vectors_in(dir, &project, id, cache_root)
}

fn vectors_in(
    dir: &ProjectDir,
    project: &Project,
    id: &str,
    cache_root: &Path,
) -> Result<VectorSummary, LibraryError> {
    let settings = settings_of(project, id)?;
    let drawing = project.drawing(id).expect("vérifié par settings_of");
    let path = super::retouch::retouched_file(dir, drawing)?;
    let mut summary = ensure_vectors(&path, &settings, cache_root, 0)?;
    summary.image = ensure_rendition(&path, cache_root, COMPARE_SIDE)?;
    Ok(summary)
}

/// Change les réglages de vectorisation d'un dessin (`None` : revenir aux
/// réglages par défaut), enregistre le projet et renvoie les nouveaux
/// vecteurs. Des réglages refusés ne modifient pas le projet.
pub fn set_vector_settings(
    dir: &ProjectDir,
    id: &str,
    settings: Option<VectorSettings>,
    cache_root: &Path,
) -> Result<(Project, VectorSummary), LibraryError> {
    if let Some(settings) = &settings {
        check_settings(settings)?;
    }
    let mut project = dir.load()?.project;
    let drawing = project
        .library
        .iter_mut()
        .find(|d| d.id == id)
        .ok_or_else(|| LibraryError::UnknownDrawing(id.to_string()))?;
    // Les réglages par défaut ne s'écrivent pas : le dessin suivra le
    // défaut s'il change un jour.
    let wanted = settings.filter(|s| *s != VectorSettings::default());
    if drawing.vector != wanted {
        drawing.vector = wanted;
        dir.save(&project)?;
    }
    let summary = vectors_in(dir, &project, id, cache_root)?;
    Ok((project, summary))
}

/// Compte rendu de [`vectorize_drawings`].
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorizeReport {
    /// Dessins dont les vecteurs sont prêts (calculés ou déjà en cache).
    pub done: usize,
    /// Dessins qui n'ont pas pu être vectorisés (`file` : leur nom).
    pub failures: Vec<ImportFailure>,
    pub seconds: f64,
}

/// Vectorise, avec leurs réglages, les dessins `ids` du projet (tous si la
/// liste est vide) qui ne sont pas déjà en cache. Plusieurs dessins à la
/// fois (un fil chacun, 8 au plus, comme l'import) ; `progress` est appelé
/// après chacun.
pub fn vectorize_drawings(
    dir: &ProjectDir,
    ids: &[String],
    cache_root: &Path,
    threads: usize,
    progress: &(dyn Fn(ImportProgress) + Sync),
) -> Result<VectorizeReport, LibraryError> {
    let start = Instant::now();
    let project = dir.load()?.project;
    let chosen: Vec<_> = project
        .library
        .iter()
        .filter(|d| ids.is_empty() || ids.contains(&d.id))
        .collect();
    let total = chosen.len();
    progress(ImportProgress { done: 0, total });
    let results = parallel_each(
        &chosen,
        thread_count(threads),
        |drawing| {
            let path = super::retouch::retouched_file(dir, drawing)?;
            ensure_vectors(&path, &drawing.vector.unwrap_or_default(), cache_root, 1)
        },
        |done| progress(ImportProgress { done, total }),
    );
    let mut report = VectorizeReport {
        done: 0,
        failures: Vec::new(),
        seconds: 0.0,
    };
    for (drawing, result) in chosen.iter().zip(results) {
        match result {
            Ok(_) => report.done += 1,
            Err(error) => report.failures.push(ImportFailure {
                file: drawing.name.clone(),
                reason: error.to_string(),
            }),
        }
    }
    report.seconds = start.elapsed().as_secs_f64();
    Ok(report)
}

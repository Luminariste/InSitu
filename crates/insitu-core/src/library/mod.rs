//! Bibliothèque de dessins (tâche 4.1).
//!
//! Un dossier de dessins (traits blancs sur fond noir) est copié dans le
//! dossier `bibliotheque/` du projet, en gardant ses sous-dossiers. Chaque
//! dessin est inscrit dans `projet.json` avec un chemin relatif (règle
//! D-1.4-b : le projet se suffit à lui-même). Le fichier copié n'est jamais
//! modifié.
//!
//! Pour l'affichage, le moteur fabrique de chaque dessin une **vignette**
//! (320 px au plus) et, à la demande, un **aperçu** (1 024 px au plus), où le
//! noir est devenu transparent ([`black_to_alpha`]). Elles sont rangées dans
//! le cache de l'app, sous `vignettes/`, et se refont si le fichier change.
//! Un import fabrique les vignettes au passage : la bibliothèque se parcourt
//! ensuite sans attente.
//!
//! - [`read`] : lecture de tous les formats de dessin ;
//! - [`import_folder`] : import d'un dossier ;
//! - [`list`] : la bibliothèque, vignettes comprises ;
//! - [`ensure_rendition`] : une vignette ou un aperçu dans le cache ;
//! - [`vectors`] : les vecteurs de chaque dessin (tâche 5.2) ;
//! - [`silhouette`] : la silhouette de chaque dessin (tâche 4.7) ;
//! - [`retouch`] : gomme et crayon sur un dessin (tâche 4.8).

pub mod organize;
pub mod psd;
pub mod read;
pub mod retouch;
pub mod shared;
pub mod silhouette;
pub mod vectors;

#[cfg(test)]
mod tests;

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder};
use serde::Serialize;

use crate::project::model::new_id;
use crate::project::{
    AssetKind, Drawing, ExtraFields, Project, ProjectDir, ProjectError, RelativePath,
    VectorSettings,
};
use crate::tiles::{self, Raster};

pub use read::{decode, DecodedDrawing, DrawingFormat, DRAWING_EXTENSIONS};

/// Plus grand côté d'une vignette, en pixels. Une image est réduite de
/// moitié en moitié tant qu'elle dépasse : la vignette mesure entre la
/// moitié et la totalité de ce côté (256 px pour un dessin de 4 096 px).
pub const THUMBNAIL_SIDE: u32 = 320;

/// Plus grand côté d'un aperçu, en pixels (même règle).
pub const PREVIEW_SIDE: u32 = 1024;

/// Sous-dossier du cache où sont rangées vignettes et aperçus.
pub const RENDITION_DIR: &str = "vignettes";

/// Version des vignettes : à augmenter si leur calcul change, pour qu'elles
/// soient refaites.
const RENDITION_VERSION: u32 = 1;

/// Au plus autant de dessins lus en même temps : un dessin de 4 096 px en
/// couleur occupe 64 Mo une fois lu.
const MAX_THREADS: usize = 8;

#[derive(Debug)]
pub enum LibraryError {
    /// Le dossier à importer n'existe pas ou n'est pas un dossier.
    NotAFolder(PathBuf),
    /// Le dossier à importer est dans le projet lui-même.
    InsideProject(PathBuf),
    /// Aucun dessin dans le dossier.
    NoDrawings(PathBuf),
    /// Un dessin illisible.
    Unreadable {
        path: PathBuf,
        reason: String,
    },
    /// Aucun dessin de la bibliothèque ne porte cet identifiant.
    UnknownDrawing(String),
    /// Réglages de vectorisation refusés.
    Settings(String),
    /// Silhouette retouchée refusée.
    Silhouette(String),
    /// Retouches à la gomme ou au crayon refusées.
    Retouch(String),
    /// Nom ou dossier refusé en rangeant la bibliothèque.
    Organize(String),
    /// Écriture d'une vignette impossible.
    Cache {
        path: PathBuf,
        reason: String,
    },
    Project(ProjectError),
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAFolder(path) => write!(f, "« {} » n'est pas un dossier.", path.display()),
            Self::InsideProject(path) => write!(
                f,
                "« {} » est dans le dossier du projet : ses dessins y sont déjà.",
                path.display()
            ),
            Self::NoDrawings(path) => write!(
                f,
                "Aucun dessin dans « {} » (formats lus : PNG, JPEG, TIFF, BMP, PSD, SVG).",
                path.display()
            ),
            Self::Unreadable { path, reason } => {
                write!(f, "Dessin illisible « {} » : {reason}", path.display())
            }
            Self::UnknownDrawing(id) => write!(f, "Aucun dessin « {id} » dans la bibliothèque."),
            Self::Settings(reason) => write!(f, "Réglages de vectorisation refusés : {reason}."),
            Self::Silhouette(reason) => write!(f, "Silhouette refusée : {reason}."),
            Self::Retouch(reason) => write!(f, "Retouches refusées : {reason}."),
            Self::Organize(reason) => {
                let mut chars = reason.chars();
                let first = chars.next().map(|c| c.to_uppercase().collect::<String>());
                write!(f, "{}{}.", first.unwrap_or_default(), chars.as_str())
            }
            Self::Cache { path, reason } => {
                write!(
                    f,
                    "Impossible d'écrire la vignette « {} » : {reason}",
                    path.display()
                )
            }
            Self::Project(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for LibraryError {}

impl From<ProjectError> for LibraryError {
    fn from(error: ProjectError) -> Self {
        Self::Project(error)
    }
}

// ---------------------------------------------------------------------------
// Fond noir rendu transparent
// ---------------------------------------------------------------------------

/// Pose l'image sur du noir : une image avec transparence perd son canal
/// alpha (chaque couleur multipliée par son opacité) ; les autres restent
/// telles quelles.
pub fn flatten_on_black(raster: Raster) -> Raster {
    if raster.channels != 4 {
        return raster;
    }
    let data = raster
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, a]| {
            let k = u32::from(a);
            [r, g, b].map(|c| ((u32::from(c) * k + 127) / 255) as u8)
        })
        .collect();
    Raster::new(raster.width, raster.height, 3, data)
}

/// Rend le noir transparent : renvoie des pixels RVBA (8 bits).
///
/// Chaque pixel devient la couleur qui, posée sur du noir, redonne
/// exactement le pixel d'origine : l'opacité est la plus forte des trois
/// composantes, la couleur est divisée par cette opacité. Un trait blanc
/// reste blanc et opaque, le noir devient invisible, un gris devient du
/// blanc à demi transparent. Le dessin, affiché sur fond noir, est donc
/// inchangé ; sur un autre fond (façade), seul le trait se voit.
pub fn black_to_alpha(raster: &Raster) -> Vec<u8> {
    match raster.channels {
        1 => raster
            .data
            .iter()
            .flat_map(|&v| [255, 255, 255, v])
            .collect(),
        3 => raster
            .data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[r, g, b]| {
                let a = r.max(g).max(b);
                if a == 0 {
                    return [0, 0, 0, 0];
                }
                let k = u32::from(a);
                let un = |c: u8| ((u32::from(c) * 255 + k / 2) / k) as u8;
                [un(r), un(g), un(b), a]
            })
            .collect(),
        _ => black_to_alpha(&flatten_on_black(raster.clone())),
    }
}

/// Réduit un dessin de moitié en moitié (moyenne en lumière, comme les
/// pyramides) jusqu'à ce que son plus grand côté tienne dans `max_side`,
/// puis rend le noir transparent. Renvoie une image RVBA.
pub fn rendition(raster: Raster, max_side: u32) -> Raster {
    let mut current = flatten_on_black(raster);
    while current.width.max(current.height) > max_side.max(1) {
        current = tiles::downsample(&current);
    }
    let rgba = black_to_alpha(&current);
    Raster::new(current.width, current.height, 4, rgba)
}

/// Encode une image en PNG (compression rapide).
pub fn encode_png(raster: &Raster) -> Result<Vec<u8>, String> {
    let color = match raster.channels {
        1 => ExtendedColorType::L8,
        3 => ExtendedColorType::Rgb8,
        _ => ExtendedColorType::Rgba8,
    };
    let mut bytes = Vec::new();
    PngEncoder::new_with_quality(&mut bytes, CompressionType::Fast, FilterType::Adaptive)
        .write_image(&raster.data, raster.width, raster.height, color)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// Vignettes et aperçus dans le cache
// ---------------------------------------------------------------------------

/// Adresse, dans le cache, de la vignette (ou de l'aperçu) de `path` :
/// `vignettes/<clé>.png`. La clé dépend du chemin, de la taille et de la
/// date du fichier : un dessin modifié reçoit une nouvelle vignette.
pub fn rendition_request_path(path: &Path, max_side: u32) -> Result<String, LibraryError> {
    let canonical = fs::canonicalize(path).map_err(|e| unreadable(path, e))?;
    let metadata = fs::metadata(&canonical).map_err(|e| unreadable(path, e))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    let identity = format!(
        "{}|{}|{modified}|{max_side}|r{RENDITION_VERSION}",
        canonical.display(),
        metadata.len()
    );
    Ok(format!(
        "{RENDITION_DIR}/{:016x}.png",
        tiles::fnv1a_64(identity.as_bytes())
    ))
}

fn unreadable(path: &Path, error: io::Error) -> LibraryError {
    LibraryError::Unreadable {
        path: path.to_path_buf(),
        reason: error.to_string(),
    }
}

/// Renvoie l'adresse dans le cache de la vignette (ou de l'aperçu) de
/// `path`, en la fabriquant si elle n'existe pas encore.
pub fn ensure_rendition(
    path: &Path,
    cache_root: &Path,
    max_side: u32,
) -> Result<String, LibraryError> {
    let request = rendition_request_path(path, max_side)?;
    if cache_root.join(&request).is_file() {
        return Ok(request);
    }
    let decoded = decode(path, max_side)?;
    store_rendition(decoded.raster, cache_root, &request, max_side)?;
    Ok(request)
}

/// Fabrique la vignette d'une image déjà lue et l'écrit dans le cache, sans
/// jamais laisser voir un fichier à moitié écrit.
fn store_rendition(
    raster: Raster,
    cache_root: &Path,
    request: &str,
    max_side: u32,
) -> Result<(), LibraryError> {
    let target = cache_root.join(request);
    let failed = |reason: String| LibraryError::Cache {
        path: target.clone(),
        reason,
    };
    let small = rendition(raster, max_side);
    let bytes = encode_png(&small).map_err(failed)?;
    let folder = target.parent().expect("vignettes/<clé>.png");
    fs::create_dir_all(folder).map_err(|e| failed(e.to_string()))?;
    let temp = folder.join(format!(".~{}.{}", new_id(), std::process::id()));
    let written = fs::File::create(&temp)
        .and_then(|mut file| file.write_all(&bytes))
        .and_then(|()| fs::rename(&temp, &target));
    if let Err(error) = written {
        let _ = fs::remove_file(&temp);
        return Err(failed(error.to_string()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Travail en parallèle
// ---------------------------------------------------------------------------

fn thread_count(requested: usize) -> usize {
    if requested > 0 {
        return requested;
    }
    std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(MAX_THREADS)
}

/// Applique `f` à chaque élément sur `threads` fils, un élément à la fois
/// par fil ; `done` est appelé après chacun avec le nombre d'éléments
/// terminés. Les résultats gardent l'ordre des éléments.
fn parallel_each<T: Sync, R: Send>(
    items: &[T],
    threads: usize,
    f: impl Fn(&T) -> R + Sync,
    done: impl Fn(usize) + Sync,
) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let finished = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<R>>> = Mutex::new((0..items.len()).map(|_| None).collect());
    let work = || loop {
        let index = next.fetch_add(1, Ordering::Relaxed);
        if index >= items.len() {
            return;
        }
        let result = f(&items[index]);
        results.lock().expect("verrou")[index] = Some(result);
        done(finished.fetch_add(1, Ordering::Relaxed) + 1);
    };
    std::thread::scope(|scope| {
        for _ in 1..threads.min(items.len()) {
            scope.spawn(work);
        }
        work();
    });
    results
        .into_inner()
        .expect("verrou")
        .into_iter()
        .map(|r| r.expect("chaque élément est traité"))
        .collect()
}

// ---------------------------------------------------------------------------
// Import d'un dossier
// ---------------------------------------------------------------------------

/// Réglages d'un import.
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// Cache où ranger les vignettes ; sans cache, elles ne sont pas faites.
    pub cache_root: Option<PathBuf>,
    /// Nombre de fils ; 0 : un par cœur, 8 au plus.
    pub threads: usize,
}

/// Avancement d'un import, pour une barre de progression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ImportProgress {
    pub done: usize,
    pub total: usize,
}

/// Un fichier qui n'a pas pu être importé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportFailure {
    /// Chemin dans le dossier importé.
    pub file: String,
    pub reason: String,
}

/// Compte rendu d'un import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// Dossier de la bibliothèque qui a reçu les dessins, par exemple
    /// `bibliotheque/Oiseaux`.
    pub folder: String,
    /// Identifiants des dessins ajoutés, dans l'ordre des fichiers.
    pub added: Vec<String>,
    /// Dessins déjà présents (même fichier, même contenu), laissés tels quels.
    pub already_present: usize,
    pub failures: Vec<ImportFailure>,
}

/// Un fichier trouvé dans le dossier à importer.
struct Found {
    path: PathBuf,
    /// Chemin dans le dossier importé, avec des `/`.
    inner: String,
}

/// Les dessins d'un dossier et de ses sous-dossiers, par ordre de chemin.
/// Les fichiers et dossiers cachés (`.DS_Store`, `._…`) sont ignorés ; les
/// liens symboliques vers des dossiers ne sont pas suivis.
fn find_drawings(root: &Path) -> io::Result<Vec<Found>> {
    let mut found = Vec::new();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((folder, prefix)) = stack.pop() {
        for entry in fs::read_dir(&folder)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let inner = if prefix.is_empty() {
                clean_segment(&name)
            } else {
                format!("{prefix}/{}", clean_segment(&name))
            };
            let kind = entry.file_type()?;
            if kind.is_dir() {
                stack.push((entry.path(), inner));
            } else if DrawingFormat::of(&entry.path()).is_some() && entry.path().is_file() {
                found.push(Found {
                    path: entry.path(),
                    inner,
                });
            }
        }
    }
    found.sort_by(|a, b| a.inner.cmp(&b.inner));
    Ok(found)
}

/// Rend un nom de fichier ou de dossier acceptable dans un chemin du
/// projet : « : » (permis par le Finder) et les caractères invisibles sont
/// remplacés par « - » ; un nom réduit à des points devient « _ ».
fn clean_segment(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c == ':' || c == '\\' || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    if cleaned.is_empty() || cleaned.chars().all(|c| c == '.') {
        "_".to_string()
    } else {
        cleaned
    }
}

/// Nom affiché d'un dessin : son nom de fichier sans l'extension.
fn display_name(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Vrai si les deux fichiers ont exactement le même contenu.
fn same_bytes(a: &Path, b: &Path) -> bool {
    match (fs::metadata(a), fs::metadata(b)) {
        (Ok(ma), Ok(mb)) if ma.len() == mb.len() => {
            matches!((fs::read(a), fs::read(b)), (Ok(x), Ok(y)) if x == y)
        }
        _ => false,
    }
}

/// Ce que le travail sur un fichier a donné.
enum Outcome {
    Added {
        source: RelativePath,
        name: String,
        width: f64,
        height: f64,
    },
    /// Déjà dans la bibliothèque à l'identique : son identifiant.
    AlreadyPresent(String),
    Failed(String),
}

/// Importe le dossier `source` (et ses sous-dossiers) dans la bibliothèque
/// du projet, sous `bibliotheque/<nom du dossier>/`, et enregistre le
/// projet. Chaque dessin est lu (un fichier illisible est signalé, pas
/// copié), copié, et reçoit sa vignette si un cache est donné. Un dessin
/// déjà importé à l'identique n'est pas ajouté une seconde fois ; un fichier
/// modifié depuis est importé à côté, avec un suffixe (`merle-2.png`).
///
/// `progress` est appelé après chaque fichier, depuis les fils de travail.
pub fn import_folder(
    dir: &ProjectDir,
    source: &Path,
    options: &ImportOptions,
    progress: &(dyn Fn(ImportProgress) + Sync),
) -> Result<(Project, ImportReport), LibraryError> {
    if !source.is_dir() {
        return Err(LibraryError::NotAFolder(source.to_path_buf()));
    }
    let canonical = fs::canonicalize(source).map_err(|e| unreadable(source, e))?;
    let root = fs::canonicalize(dir.root()).map_err(|e| unreadable(dir.root(), e))?;
    if canonical.starts_with(&root) {
        return Err(LibraryError::InsideProject(source.to_path_buf()));
    }
    let found = find_drawings(&canonical).map_err(|e| unreadable(source, e))?;
    if found.is_empty() {
        return Err(LibraryError::NoDrawings(source.to_path_buf()));
    }
    let folder_name = canonical
        .file_name()
        .map(|n| clean_segment(&n.to_string_lossy()))
        .unwrap_or_else(|| "Dessins".to_string());
    let base = format!("{}/{folder_name}", AssetKind::Drawing.directory());
    // Vérifié tout de suite : un nom de dossier refusé arrête l'import avant
    // toute copie.
    RelativePath::new(base.as_str())?;

    let (project, report, _) = import_found(dir, &base, &found, options, progress)?;
    Ok((project, report))
}

/// Importe des fichiers choisis un à un (tâche 6.3 : les liens d'un fichier
/// Illustrator) sous `base` (`bibliotheque/<dossier>`) : chaque fichier va
/// dans `base/<inner>`, `inner` étant son chemin relatif avec des `/`
/// (`Algues/algue-01.png`). Rend, pour chaque fichier dans l'ordre,
/// l'identifiant du dessin (ajouté ou déjà présent à l'identique), ou la
/// raison de l'échec.
pub fn import_files(
    dir: &ProjectDir,
    base: &str,
    files: &[(PathBuf, String)],
    options: &ImportOptions,
    progress: &(dyn Fn(ImportProgress) + Sync),
) -> Result<(Project, Vec<Result<String, String>>), LibraryError> {
    RelativePath::new(base)?;
    let found: Vec<Found> = files
        .iter()
        .map(|(path, inner)| Found {
            path: path.clone(),
            inner: inner
                .split('/')
                .map(clean_segment)
                .collect::<Vec<_>>()
                .join("/"),
        })
        .collect();
    let (project, _, ids) = import_found(dir, base, &found, options, progress)?;
    Ok((project, ids))
}

/// Importe des dessins choisis un à un ou déposés depuis le Finder
/// (demande de Benjamin, D-4.1-f) : chaque fichier va dans
/// `bibliotheque/<inner>`, `inner` étant son nom, ou son chemin dans le
/// dossier déposé (`Oiseaux/merle.png`). Mêmes règles que l'import d'un
/// dossier : un dessin déjà là à l'identique n'est pas ajouté deux fois,
/// un fichier illisible est signalé sans arrêter les autres. Rend aussi
/// les dessins de tous les fichiers lus, ajoutés ou déjà là (pour les poser
/// sur la façade quand on les y a déposés).
pub fn import_loose(
    dir: &ProjectDir,
    files: &[(PathBuf, String)],
    options: &ImportOptions,
    progress: &(dyn Fn(ImportProgress) + Sync),
) -> Result<(Project, ImportReport, Vec<String>), LibraryError> {
    let root = fs::canonicalize(dir.root()).map_err(|e| unreadable(dir.root(), e))?;
    let mut found = Vec::new();
    for (path, inner) in files {
        if let Ok(canonical) = fs::canonicalize(path) {
            if canonical.starts_with(&root) {
                return Err(LibraryError::InsideProject(path.clone()));
            }
        }
        let Some(inner) = clean_inner(inner) else {
            continue;
        };
        if DrawingFormat::of(Path::new(&inner)).is_some() {
            found.push(Found {
                path: path.clone(),
                inner,
            });
        }
    }
    if found.is_empty() {
        let shown = files
            .first()
            .map_or_else(|| PathBuf::from("?"), |(path, _)| path.clone());
        return Err(LibraryError::NoDrawings(shown));
    }
    found.sort_by(|a, b| a.inner.cmp(&b.inner));
    let (project, report, ids) = import_found(
        dir,
        AssetKind::Drawing.directory(),
        &found,
        options,
        progress,
    )?;
    let drawings = ids.into_iter().filter_map(Result::ok).collect();
    Ok((project, report, drawings))
}

/// Chemin d'un fichier déposé, rendu sûr : morceaux vides, `.` et `..`
/// retirés, fichiers cachés (`.DS_Store`, `._…`) écartés, caractères
/// refusés remplacés (comme [`clean_segment`]). `None` s'il ne reste rien.
fn clean_inner(inner: &str) -> Option<String> {
    let parts: Vec<&str> = inner
        .split(['/', '\\'])
        .filter(|part| !part.is_empty() && *part != "." && *part != "..")
        .collect();
    if parts.iter().any(|part| part.starts_with('.')) {
        return None;
    }
    let cleaned: Vec<String> = parts.into_iter().map(clean_segment).collect();
    (!cleaned.is_empty()).then(|| cleaned.join("/"))
}

/// Nom sûr pour un dossier de dépôt choisi par l'interface : lettres,
/// chiffres et tirets seulement (jamais `..` ni `/`).
pub fn clean_staging_id(id: &str) -> String {
    let cleaned: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(64)
        .collect();
    if cleaned.is_empty() {
        "depot".to_string()
    } else {
        cleaned
    }
}

/// Range dans `staging` un fichier déposé depuis le Finder, reçu en octets
/// par la fenêtre (la vue web ne donne pas son chemin, D-4.1-f), sous son
/// chemin `inner` rendu sûr. Rend le fichier écrit, à passer ensuite à
/// [`import_loose`] avec le même `inner`. `None` : fichier caché ou nom vide,
/// ignoré.
pub fn stage_dropped_file(
    staging: &Path,
    inner: &str,
    bytes: &[u8],
) -> Result<Option<(PathBuf, String)>, LibraryError> {
    let Some(inner) = clean_inner(inner) else {
        return Ok(None);
    };
    let path = staging.join(&inner);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| unreadable(parent, e))?;
    }
    fs::write(&path, bytes).map_err(|e| unreadable(&path, e))?;
    Ok(Some((path, inner)))
}

/// Le travail commun aux imports : lecture, copie et vignette de chaque
/// fichier en parallèle, puis enregistrement. Rend aussi, pour chaque
/// fichier, l'identifiant de son dessin ou la raison de l'échec.
#[allow(clippy::type_complexity)]
fn import_found(
    dir: &ProjectDir,
    base: &str,
    found: &[Found],
    options: &ImportOptions,
    progress: &(dyn Fn(ImportProgress) + Sync),
) -> Result<(Project, ImportReport, Vec<Result<String, String>>), LibraryError> {
    let project = dir.load()?.project;
    let total = found.len();
    progress(ImportProgress { done: 0, total });
    let outcomes = parallel_each(
        found,
        thread_count(options.threads),
        |file| import_one(dir, &project, base, file, options),
        |done| progress(ImportProgress { done, total }),
    );

    // Relu juste avant d'écrire : le moins possible de temps entre lecture et
    // enregistrement.
    let mut project = dir.load()?.project;
    let mut report = ImportReport {
        folder: base.to_string(),
        added: Vec::new(),
        already_present: 0,
        failures: Vec::new(),
    };
    let mut ids = Vec::with_capacity(found.len());
    for (file, outcome) in found.iter().zip(outcomes) {
        match outcome {
            Outcome::Added {
                source,
                name,
                width,
                height,
            } => {
                if let Some(existing) = project.library.iter().find(|d| d.source == source) {
                    report.already_present += 1;
                    ids.push(Ok(existing.id.clone()));
                    continue;
                }
                let id = new_id();
                report.added.push(id.clone());
                ids.push(Ok(id.clone()));
                project.library.push(Drawing {
                    id,
                    name,
                    source,
                    width,
                    height,
                    vector: None,
                    silhouette: None,
                    retouch: Vec::new(),
                    folder: None,
                    extra: ExtraFields::new(),
                });
            }
            Outcome::AlreadyPresent(id) => {
                report.already_present += 1;
                ids.push(Ok(id));
            }
            Outcome::Failed(reason) => {
                ids.push(Err(reason.clone()));
                report.failures.push(ImportFailure {
                    file: file.inner.clone(),
                    reason,
                });
            }
        }
    }
    if !report.added.is_empty() {
        dir.save(&project)?;
    }
    Ok((project, report, ids))
}

fn import_one(
    dir: &ProjectDir,
    project: &Project,
    base: &str,
    file: &Found,
    options: &ImportOptions,
) -> Outcome {
    let (subfolder, _) = file.inner.rsplit_once('/').unwrap_or(("", ""));
    let destination_dir = if subfolder.is_empty() {
        base.to_string()
    } else {
        format!("{base}/{subfolder}")
    };
    let file_name = file.inner.rsplit('/').next().unwrap_or_default();
    // Déjà importé à l'identique : rien à refaire, sauf la vignette si elle
    // manque (cache vidé).
    let already = project.library.iter().find(|d| {
        d.source.as_str() == format!("{destination_dir}/{file_name}")
            && same_bytes(&file.path, &dir.resolve(&d.source))
    });
    if let Some(drawing) = already {
        if let Some(cache) = &options.cache_root {
            let _ = ensure_rendition(&dir.resolve(&drawing.source), cache, THUMBNAIL_SIDE);
        }
        return Outcome::AlreadyPresent(drawing.id.clone());
    }
    let decoded = match decode(&file.path, THUMBNAIL_SIDE) {
        Ok(decoded) => decoded,
        Err(LibraryError::Unreadable { reason, .. }) => return Outcome::Failed(reason),
        Err(other) => return Outcome::Failed(other.to_string()),
    };
    let (width, height) = (decoded.width, decoded.height);
    let source = match dir.import_file_into(&file.path, &destination_dir) {
        Ok(source) => source,
        Err(error) => return Outcome::Failed(error.to_string()),
    };
    if let Some(cache) = &options.cache_root {
        let copied = dir.resolve(&source);
        // Un SVG se rend directement à la taille de la vignette ; une image
        // déjà lue sert telle quelle.
        let made = rendition_request_path(&copied, THUMBNAIL_SIDE)
            .and_then(|request| store_rendition(decoded.raster, cache, &request, THUMBNAIL_SIDE));
        if let Err(error) = made {
            return Outcome::Failed(error.to_string());
        }
    }
    Outcome::Added {
        source,
        name: display_name(&file.path),
        width,
        height,
    }
}

// ---------------------------------------------------------------------------
// Parcours de la bibliothèque
// ---------------------------------------------------------------------------

/// Un dessin tel que l'interface le montre.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub id: String,
    pub name: String,
    /// Dossier dans la bibliothèque (`Oiseaux/Grands`), ou vide.
    pub folder: String,
    /// Fichier, relatif au dossier projet.
    pub source: String,
    pub width: f64,
    pub height: f64,
    /// Vignette, à lire dans le cache (`vignettes/<clé>.png`).
    pub thumbnail: Option<String>,
    /// Ce qui empêche de montrer le dessin (fichier absent, illisible).
    pub problem: Option<String>,
    /// Réglages de vectorisation propres au dessin ; absent : par défaut.
    pub vector: Option<VectorSettings>,
}

/// La bibliothèque du projet, dans l'ordre d'import, avec la vignette de
/// chaque dessin. Les vignettes manquantes (autre Mac, cache vidé) sont
/// fabriquées en parallèle avant de répondre ; les autres ne coûtent qu'une
/// lecture de date de fichier.
pub fn list(
    dir: &ProjectDir,
    cache_root: Option<&Path>,
    threads: usize,
) -> Result<Vec<LibraryEntry>, LibraryError> {
    let project = dir.load()?.project;
    Ok(entries(dir, &project, cache_root, threads))
}

/// Comme [`list`], pour un projet déjà lu.
pub fn entries(
    dir: &ProjectDir,
    project: &Project,
    cache_root: Option<&Path>,
    threads: usize,
) -> Vec<LibraryEntry> {
    parallel_each(
        &project.library,
        thread_count(threads),
        |drawing| {
            let (thumbnail, problem) = if !dir.resolve(&drawing.source).is_file() {
                (None, Some("fichier absent du dossier projet".to_string()))
            } else {
                // Le dessin retouché à la gomme ou au crayon (tâche 4.8).
                let path = retouch::readable_file(dir, drawing);
                match cache_root.map(|cache| ensure_rendition(&path, cache, THUMBNAIL_SIDE)) {
                    None => (None, None),
                    Some(Ok(request)) => (Some(request), None),
                    Some(Err(error)) => (None, Some(error.to_string())),
                }
            };
            LibraryEntry {
                id: drawing.id.clone(),
                name: drawing.name.clone(),
                folder: drawing.folder().to_string(),
                source: drawing.source.to_string(),
                width: drawing.width,
                height: drawing.height,
                thumbnail,
                problem,
                vector: drawing.vector,
            }
        },
        |_| {},
    )
}

/// L'aperçu (1 024 px au plus, fond transparent) d'un dessin, fabriqué dans
/// le cache s'il n'y est pas.
pub fn preview(dir: &ProjectDir, id: &str, cache_root: &Path) -> Result<String, LibraryError> {
    let project = dir.load()?.project;
    let drawing = project
        .drawing(id)
        .ok_or_else(|| LibraryError::UnknownDrawing(id.to_string()))?;
    ensure_rendition(
        &retouch::readable_file(dir, drawing),
        cache_root,
        PREVIEW_SIDE,
    )
}

/// L'aperçu du dessin tel qu'importé, sans ses retouches (tâche 4.8) :
/// l'éditeur de retouches y peint les siennes.
pub fn original_preview(
    dir: &ProjectDir,
    id: &str,
    cache_root: &Path,
) -> Result<String, LibraryError> {
    let project = dir.load()?.project;
    let drawing = project
        .drawing(id)
        .ok_or_else(|| LibraryError::UnknownDrawing(id.to_string()))?;
    ensure_rendition(&dir.resolve(&drawing.source), cache_root, PREVIEW_SIDE)
}

/// Retire le dessin `id` de la bibliothèque, ainsi que ses placements sur
/// la façade (un placement cite toujours un dessin de la bibliothèque), et
/// enregistre le projet. Le fichier reste dans `bibliotheque/` : rien n'est
/// effacé du disque, et un nouvel import du même fichier le reprend tel quel
/// (D15-h). Rend le projet et le nombre de placements retirés.
pub fn remove_drawing(dir: &ProjectDir, id: &str) -> Result<(Project, usize), LibraryError> {
    let mut project = dir.load()?.project;
    let before = project.library.len();
    project.library.retain(|d| d.id != id);
    if project.library.len() == before {
        return Err(LibraryError::UnknownDrawing(id.to_string()));
    }
    let placed = project.placements.len();
    project.placements.retain(|p| p.drawing != id);
    let removed = placed - project.placements.len();
    dir.save(&project)?;
    Ok((project, removed))
}

/// Trouve un dessin par son identifiant, ou à défaut par son nom.
pub fn find<'a>(project: &'a Project, id_or_name: &str) -> Result<&'a Drawing, LibraryError> {
    project
        .drawing(id_or_name)
        .or_else(|| project.library.iter().find(|d| d.name == id_or_name))
        .ok_or_else(|| LibraryError::UnknownDrawing(id_or_name.to_string()))
}

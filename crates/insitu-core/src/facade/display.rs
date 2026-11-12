//! Affichage d'un plan de façade : pyramide de tuiles pour la visionneuse.
//!
//! Une image passe par la pyramide ordinaire ([`crate::tiles`]). Un plan
//! vectoriel a une pyramide « virtuelle » de même format (D-1.5-d) :
//!
//! - son niveau 0 mesure [`VECTOR_LEVEL0_SIDE`] pixels sur son plus grand
//!   côté, et chaque niveau est rendu directement depuis les vecteurs, à sa
//!   propre échelle (aucune réduction d'image : le trait reste fin et net à
//!   tous les niveaux) ;
//! - les niveaux grossiers sont rendus à l'ouverture, les fins à la
//!   demande, quand la visionneuse demande une tuile absente
//!   ([`read_or_render`]) ; chaque tuile rendue est gardée dans le cache.
//!
//! Le dossier de cache d'un plan vectoriel contient, en plus de
//! `pyramid.json` et des tuiles, `plan.json` (voir [`VectorRecord`]) : de
//! quoi rendre les tuiles manquantes.

use std::collections::VecDeque;
use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::UNIX_EPOCH;

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder};
use serde::{Deserialize, Serialize};

use super::{
    open_vector, FacadeError, SourceKind, VectorPlan, VECTOR_EAGER_PIXELS, VECTOR_LEVEL0_SIDE,
};
use crate::tiles::{
    self, level_layout, read_descriptor, tile_file_name, tile_rect, LevelInfo, Pyramid, TileError,
    TileFormat, TileOptions, DESCRIPTOR_FILE, PYRAMID_VERSION,
};

/// Version du rendu des plans vectoriels : à augmenter quand il change, pour
/// que les caches soient refaits.
pub const VECTOR_RENDER_VERSION: u32 = 1;

/// Fichier qui décrit, dans un dossier de cache, le plan à rendre.
pub const VECTOR_RECORD_FILE: &str = "plan.json";

/// Ce qu'il faut pour rendre une tuile manquante.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorRecord {
    pub render_version: u32,
    /// Chemin absolu du fichier de plan.
    pub source: PathBuf,
    pub kind: SourceKind,
    #[serde(default)]
    pub page: Option<u32>,
    /// Pixels du niveau 0 par unité du fichier.
    pub scale: f64,
}

/// Ce que l'interface reçoit pour afficher un plan.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacadeDisplay {
    /// Sous-dossier de la pyramide dans le cache.
    pub key: String,
    pub pyramid: Pyramid,
    pub kind: SourceKind,
    /// Dimensions du plan dans l'unité du fichier.
    pub source_width: f64,
    pub source_height: f64,
    /// Unités du fichier par pixel du niveau 0 (1 pour une image). Pour
    /// placer le plan en mm : `mmPerPixel = sourceUnitsPerPixel × échelle`.
    pub source_units_per_pixel: f64,
}

/// Prépare l'affichage du plan `path` dans le cache `cache_root`. Rapide si
/// c'est déjà fait : un plan inchangé n'est jamais rendu deux fois.
pub fn open_for_display(
    path: &Path,
    page: Option<u32>,
    cache_root: &Path,
) -> Result<FacadeDisplay, FacadeError> {
    let kind = SourceKind::of(path)?;
    if kind == SourceKind::Image {
        let cached = tiles::ensure_pyramid(path, cache_root, &TileOptions::default())?;
        let (width, height) = (cached.pyramid.width, cached.pyramid.height);
        return Ok(FacadeDisplay {
            key: cached.key,
            pyramid: cached.pyramid,
            kind,
            source_width: f64::from(width),
            source_height: f64::from(height),
            source_units_per_pixel: 1.0,
        });
    }

    let page = (kind == SourceKind::Pdf).then(|| page.unwrap_or(1));
    let key = vector_key(path, page)?;
    let dir = cache_root.join(&key);
    if let (Ok(pyramid), Ok(record)) = (read_descriptor(&dir), read_record(&dir)) {
        if pyramid.version == PYRAMID_VERSION && record.render_version == VECTOR_RENDER_VERSION {
            return Ok(display_of(key, pyramid, &record));
        }
    }

    let plan: Arc<dyn VectorPlan> = Arc::from(open_vector(path, kind, page)?);
    let (width, height) = plan.size();
    let scale = f64::from(VECTOR_LEVEL0_SIDE) / width.max(height);
    let width0 = ((width * scale).ceil() as u32).max(1);
    let height0 = ((height * scale).ceil() as u32).max(1);
    let tile_size = tiles::DEFAULT_TILE_SIZE;
    let levels = level_layout(width0, height0, tile_size);
    let record = VectorRecord {
        render_version: VECTOR_RENDER_VERSION,
        source: fs::canonicalize(path).map_err(|e| FacadeError::Io(path.to_path_buf(), e))?,
        kind,
        page,
        scale,
    };

    // Le plus petit niveau dit si le plan est en couleur ; sinon les tuiles
    // sont en gris (trois fois plus légères).
    let coarsest = *levels.last().expect("au moins un niveau");
    let overview = render_level_tile(plan.as_ref(), scale, &coarsest, tile_size, 0, 0);
    let channels = if is_gray(&overview) { 1 } else { 3 };
    let pyramid = Pyramid {
        version: PYRAMID_VERSION,
        width: width0,
        height: height0,
        tile_size,
        format: TileFormat::Png,
        channels,
        levels,
    };

    // Fabrication dans un dossier provisoire, renommé une fois prêt (comme
    // `tiles::ensure_pyramid`).
    let partial = cache_root.join(format!("{key}.partiel-{}", std::process::id()));
    let _ = fs::remove_dir_all(&partial);
    let built = (|| -> Result<(), FacadeError> {
        fs::create_dir_all(&partial).map_err(|e| FacadeError::Io(partial.clone(), e))?;
        write_json(&partial.join(VECTOR_RECORD_FILE), &record)?;
        for level in pyramid.levels.iter().filter(|l| eager(l)) {
            render_level(plan.as_ref(), &pyramid, &record, level, &partial)?;
        }
        write_json(&partial.join(DESCRIPTOR_FILE), &pyramid)
    })();
    if let Err(error) = built {
        let _ = fs::remove_dir_all(&partial);
        return Err(error);
    }
    let _ = fs::remove_dir_all(&dir);
    fs::rename(&partial, &dir).map_err(|e| FacadeError::Io(dir.clone(), e))?;
    remember(&key, plan);
    Ok(display_of(key, pyramid, &record))
}

fn display_of(key: String, pyramid: Pyramid, record: &VectorRecord) -> FacadeDisplay {
    FacadeDisplay {
        key,
        kind: record.kind,
        // Dimensions exactes du plan, et non celles arrondies au pixel.
        source_width: f64::from(pyramid.width) / record.scale,
        source_height: f64::from(pyramid.height) / record.scale,
        source_units_per_pixel: 1.0 / record.scale,
        pyramid,
    }
}

fn eager(level: &LevelInfo) -> bool {
    u64::from(level.width) * u64::from(level.height) <= VECTOR_EAGER_PIXELS
}

/// Lit le fichier du cache demandé par l'interface ; pour un plan
/// vectoriel, rend la tuile si elle n'existe pas encore. `None` si le chemin
/// est refusé, ou la tuile impossible à fournir.
pub fn read_or_render(cache_root: &Path, request_path: &str) -> Option<(&'static str, Vec<u8>)> {
    if let Some(found) = tiles::read_cached_file(cache_root, request_path) {
        return Some(found);
    }
    let file = tiles::resolve_tile_file(cache_root, request_path)?;
    let relative = file.strip_prefix(cache_root).ok()?;
    let parts: Vec<&str> = relative.iter().filter_map(|s| s.to_str()).collect();
    let [key, level, name] = parts[..] else {
        return None;
    };
    let (column, row) = name.strip_suffix(".png")?.split_once('_')?;
    let (level, column, row): (u32, u32, u32) =
        (level.parse().ok()?, column.parse().ok()?, row.parse().ok()?);
    let dir = cache_root.join(key);
    let record = read_record(&dir).ok()?;
    let pyramid = read_descriptor(&dir).ok()?;
    let info = pyramid.levels.get(level as usize)?;
    if column >= info.columns || row >= info.rows {
        return None;
    }
    let plan = plan_for(key, &record)?;
    let bytes = {
        let _permit = RenderPermit::acquire();
        let rgba = render_level_tile(
            plan.as_ref(),
            record.scale,
            info,
            pyramid.tile_size,
            column,
            row,
        );
        let (_, _, width, height) = tile_rect(info, pyramid.tile_size, column, row);
        encode_png(
            &to_channels(&rgba, pyramid.channels),
            width,
            height,
            pyramid.channels,
        )
        .ok()?
    };
    // Écriture atomique : deux demandes de la même tuile ne se gênent pas.
    let temp = file.with_extension(format!("png.~{}", std::process::id()));
    if let Some(level_dir) = file.parent() {
        let _ = fs::create_dir_all(level_dir);
    }
    if fs::write(&temp, &bytes).is_ok() && fs::rename(&temp, &file).is_err() {
        let _ = fs::remove_file(&temp);
    }
    Some(("image/png", bytes))
}

/// Rend toutes les tuiles d'un niveau, sur tous les cœurs.
fn render_level(
    plan: &dyn VectorPlan,
    pyramid: &Pyramid,
    record: &VectorRecord,
    level: &LevelInfo,
    out_dir: &Path,
) -> Result<(), FacadeError> {
    let level_dir = out_dir.join(level.level.to_string());
    fs::create_dir_all(&level_dir).map_err(|e| FacadeError::Io(level_dir.clone(), e))?;
    let total = level.tile_count() as usize;
    let next = AtomicUsize::new(0);
    let failure: Mutex<Option<FacadeError>> = Mutex::new(None);
    let work = || loop {
        let index = next.fetch_add(1, Ordering::Relaxed);
        if index >= total || failure.lock().unwrap().is_some() {
            return;
        }
        let column = index as u32 % level.columns;
        let row = index as u32 / level.columns;
        let rgba = render_level_tile(plan, record.scale, level, pyramid.tile_size, column, row);
        let (_, _, width, height) = tile_rect(level, pyramid.tile_size, column, row);
        let path = out_dir.join(tile_file_name(level.level, column, row, TileFormat::Png));
        let result = encode_png(
            &to_channels(&rgba, pyramid.channels),
            width,
            height,
            pyramid.channels,
        )
        .map_err(|message| FacadeError::Tiles(TileError::Image(path.clone(), message)))
        .and_then(|bytes| fs::write(&path, bytes).map_err(|e| FacadeError::Io(path.clone(), e)));
        if let Err(error) = result {
            failure.lock().unwrap().get_or_insert(error);
            return;
        }
    };
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    std::thread::scope(|scope| {
        for _ in 1..threads.min(total) {
            scope.spawn(work);
        }
        work();
    });
    match failure.into_inner().unwrap() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Rend une tuile d'un niveau, en RVBA. Un pixel du niveau `n` vaut
/// exactement 2ⁿ pixels du niveau 0, comme dans la visionneuse.
pub fn render_level_tile(
    plan: &dyn VectorPlan,
    scale0: f64,
    level: &LevelInfo,
    tile_size: u32,
    column: u32,
    row: u32,
) -> Vec<u8> {
    let (x, y, width, height) = tile_rect(level, tile_size, column, row);
    let scale = scale0 / f64::from(1u32 << level.level.min(31));
    plan.render(scale, f64::from(x), f64::from(y), width, height)
}

fn is_gray(rgba: &[u8]) -> bool {
    rgba.chunks_exact(4).all(|p| p[0] == p[1] && p[1] == p[2])
}

fn to_channels(rgba: &[u8], channels: u8) -> Vec<u8> {
    match channels {
        1 => rgba
            .chunks_exact(4)
            .map(|p| {
                ((u32::from(p[0]) * 54 + u32::from(p[1]) * 183 + u32::from(p[2]) * 19 + 128) >> 8)
                    as u8
            })
            .collect(),
        _ => rgba
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect(),
    }
}

fn encode_png(pixels: &[u8], width: u32, height: u32, channels: u8) -> Result<Vec<u8>, String> {
    let color = if channels == 1 {
        ExtendedColorType::L8
    } else {
        ExtendedColorType::Rgb8
    };
    let mut bytes = Vec::new();
    PngEncoder::new_with_quality(
        BufWriter::new(&mut bytes),
        CompressionType::Fast,
        FilterType::Adaptive,
    )
    .write_image(pixels, width, height, color)
    .map_err(|e| e.to_string())?;
    Ok(bytes)
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), FacadeError> {
    let text = serde_json::to_string_pretty(value).expect("sérialisable");
    fs::write(path, text).map_err(|e| FacadeError::Io(path.to_path_buf(), e))
}

fn read_record(dir: &Path) -> Result<VectorRecord, String> {
    let text = fs::read_to_string(dir.join(VECTOR_RECORD_FILE)).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn vector_key(path: &Path, page: Option<u32>) -> Result<String, FacadeError> {
    let canonical = fs::canonicalize(path).map_err(|e| FacadeError::Io(path.to_path_buf(), e))?;
    let metadata = fs::metadata(&canonical).map_err(|e| FacadeError::Io(path.to_path_buf(), e))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos());
    let identity = format!(
        "plan|{}|{}|{modified}|{page:?}|{VECTOR_LEVEL0_SIDE}|r{VECTOR_RENDER_VERSION}|v{PYRAMID_VERSION}",
        canonical.display(),
        metadata.len(),
    );
    // FNV-1a sur 64 bits, comme les clés de `tiles` ; le préfixe « plan- »
    // évite toute rencontre avec elles.
    let hash = identity.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    Ok(format!("plan-{hash:016x}"))
}

// ---------------------------------------------------------------------------
// Plans ouverts en mémoire, et limite de rendus simultanés
// ---------------------------------------------------------------------------

/// Nombre de plans gardés ouverts pour le rendu à la demande.
const OPEN_PLANS: usize = 4;

type OpenPlans = Mutex<VecDeque<(String, Arc<dyn VectorPlan>)>>;

fn open_plans() -> &'static OpenPlans {
    static PLANS: OnceLock<OpenPlans> = OnceLock::new();
    PLANS.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn remember(key: &str, plan: Arc<dyn VectorPlan>) {
    let mut plans = open_plans().lock().unwrap();
    plans.retain(|(k, _)| k != key);
    plans.push_front((key.to_string(), plan));
    plans.truncate(OPEN_PLANS);
}

fn plan_for(key: &str, record: &VectorRecord) -> Option<Arc<dyn VectorPlan>> {
    if let Some((_, plan)) = open_plans().lock().unwrap().iter().find(|(k, _)| k == key) {
        return Some(plan.clone());
    }
    // Ouvert hors du verrou : la lecture d'un gros DXF prend du temps.
    let plan: Arc<dyn VectorPlan> =
        Arc::from(open_vector(&record.source, record.kind, record.page).ok()?);
    remember(key, plan.clone());
    Some(plan)
}

/// Limite le nombre de tuiles rendues en même temps au nombre de cœurs :
/// la visionneuse peut en demander des dizaines d'un coup.
struct RenderPermit;

fn permits() -> &'static (Mutex<usize>, Condvar) {
    static PERMITS: OnceLock<(Mutex<usize>, Condvar)> = OnceLock::new();
    PERMITS.get_or_init(|| {
        let cores = std::thread::available_parallelism().map_or(1, usize::from);
        (Mutex::new(cores), Condvar::new())
    })
}

impl RenderPermit {
    fn acquire() -> Self {
        let (count, condvar) = permits();
        let mut available = count.lock().unwrap();
        while *available == 0 {
            available = condvar.wait(available).unwrap();
        }
        *available -= 1;
        RenderPermit
    }
}

impl Drop for RenderPermit {
    fn drop(&mut self) {
        let (count, condvar) = permits();
        *count.lock().unwrap() += 1;
        condvar.notify_one();
    }
}

/// Rend un plan vectoriel entier en une image dont le plus grand côté fait
/// `max_side` pixels (au plus 16 384), en RVBA. Sert à contrôler un rendu
/// sans visionneuse (`insitu facade render`).
pub fn render_whole(
    path: &Path,
    page: Option<u32>,
    max_side: u32,
) -> Result<(u32, u32, Vec<u8>), FacadeError> {
    let kind = SourceKind::of(path)?;
    let plan = open_vector(path, kind, page)?;
    let (width, height) = plan.size();
    let side = max_side.clamp(16, 16_384);
    let scale = f64::from(side) / width.max(height);
    let w = ((width * scale).ceil() as u32).max(1);
    let h = ((height * scale).ceil() as u32).max(1);
    Ok((w, h, plan.render(scale, 0.0, 0.0, w, h)))
}

/// Prépare l'affichage de la façade d'un projet (voir [`open_for_display`]).
pub fn open_project_facade(
    dir: &crate::project::ProjectDir,
    cache_root: &Path,
) -> Result<FacadeDisplay, FacadeError> {
    let project = dir.load()?.project;
    let facade = project.facade.ok_or(FacadeError::NoFacade)?;
    open_for_display(&dir.resolve(&facade.source), facade.page, cache_root)
}

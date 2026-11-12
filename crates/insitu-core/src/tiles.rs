//! Pyramide de tuiles : une grande image découpée pour l'affichage.
//!
//! L'interface n'affiche jamais une image de 15 433 px d'un bloc. Le moteur
//! la découpe une fois en tuiles carrées, à plusieurs niveaux de résolution :
//!
//! - le niveau 0 est l'image d'origine ;
//! - chaque niveau suivant est deux fois plus petit (arrondi au pixel
//!   supérieur) ;
//! - le dernier niveau tient dans une seule tuile.
//!
//! Sur disque, un dossier de pyramide contient :
//!
//! ```text
//! pyramid.json          le descripteur (voir `Pyramid`)
//! 0/0_0.png  0/1_0.png  niveau/colonne_ligne.extension
//! 1/0_0.png  …
//! ```
//!
//! Les tuiles du bord droit et du bord bas sont plus petites que les autres :
//! elles ne sont pas complétées par du vide.
//!
//! Ce module est le seul endroit du modèle où l'on parle en pixels : une
//! pyramide est une entrée d'affichage. Sa position et son échelle en
//! millimètres sont données par celui qui l'affiche.

use std::fmt;
use std::fs;
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageReader};
use serde::{Deserialize, Serialize};

/// Côté d'une tuile, en pixels, sauf réglage contraire.
pub const DEFAULT_TILE_SIZE: u32 = 512;

/// Nom du descripteur dans un dossier de pyramide.
pub const DESCRIPTOR_FILE: &str = "pyramid.json";

/// Version du format de pyramide. À augmenter dès que la disposition des
/// fichiers ou le calcul des niveaux change : les caches sont alors refaits.
pub const PYRAMID_VERSION: u32 = 1;

/// Format d'enregistrement des tuiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TileFormat {
    /// Sans perte. Pour les dessins au trait : aucune bavure autour des traits.
    Png,
    /// Avec perte, bien plus léger. Pour les photos.
    Jpeg,
}

impl TileFormat {
    pub fn extension(self) -> &'static str {
        match self {
            TileFormat::Png => "png",
            TileFormat::Jpeg => "jpg",
        }
    }
}

/// Réglages de fabrication d'une pyramide.
#[derive(Debug, Clone, PartialEq)]
pub struct TileOptions {
    /// Côté d'une tuile, en pixels.
    pub tile_size: u32,
    /// Format imposé ; `None` laisse le moteur choisir d'après le contenu
    /// (voir `detect_format`).
    pub format: Option<TileFormat>,
    /// Qualité JPEG, de 1 à 100.
    pub jpeg_quality: u8,
    /// Nombre de fils d'exécution ; `None` utilise tous les cœurs.
    pub threads: Option<usize>,
}

impl Default for TileOptions {
    fn default() -> Self {
        Self {
            tile_size: DEFAULT_TILE_SIZE,
            format: None,
            jpeg_quality: 88,
            threads: None,
        }
    }
}

/// Un niveau de la pyramide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelInfo {
    /// 0 pour la pleine résolution ; le niveau `n` est réduit de 2ⁿ.
    pub level: u32,
    /// Taille de l'image à ce niveau, en pixels.
    pub width: u32,
    pub height: u32,
    /// Nombre de tuiles en largeur et en hauteur.
    pub columns: u32,
    pub rows: u32,
}

impl LevelInfo {
    pub fn tile_count(&self) -> u64 {
        u64::from(self.columns) * u64::from(self.rows)
    }
}

/// Descripteur d'une pyramide, enregistré dans `pyramid.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pyramid {
    pub version: u32,
    /// Taille de l'image d'origine, en pixels.
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,
    pub format: TileFormat,
    /// Nombre de canaux des tuiles : 1 (gris), 3 (couleur) ou 4 (couleur et
    /// transparence).
    pub channels: u8,
    /// Du niveau 0 (pleine résolution) au plus réduit.
    pub levels: Vec<LevelInfo>,
}

impl Pyramid {
    pub fn tile_count(&self) -> u64 {
        self.levels.iter().map(LevelInfo::tile_count).sum()
    }
}

/// Une pyramide rangée dans un dossier de cache (voir `ensure_pyramid`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedPyramid {
    /// Nom du sous-dossier de la pyramide dans le cache.
    pub key: String,
    pub pyramid: Pyramid,
}

#[derive(Debug)]
pub enum TileError {
    /// Lecture ou écriture d'un fichier.
    Io(PathBuf, io::Error),
    /// Image illisible ou tuile impossible à encoder.
    Image(PathBuf, String),
    /// Réglage ou dossier de pyramide incorrect.
    Invalid(String),
}

impl fmt::Display for TileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TileError::Io(path, error) => write!(f, "{} : {error}", path.display()),
            TileError::Image(path, message) => write!(f, "{} : {message}", path.display()),
            TileError::Invalid(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for TileError {}

fn io_error(path: &Path) -> impl FnOnce(io::Error) -> TileError + '_ {
    move |error| TileError::Io(path.to_path_buf(), error)
}

// ---------------------------------------------------------------------------
// Géométrie de la pyramide
// ---------------------------------------------------------------------------

/// Niveaux d'une pyramide pour une image de `width` × `height` pixels.
///
/// Chaque niveau mesure la moitié du précédent, arrondie au pixel supérieur ;
/// on s'arrête dès qu'un niveau tient dans une seule tuile.
pub fn level_layout(width: u32, height: u32, tile_size: u32) -> Vec<LevelInfo> {
    assert!(width > 0 && height > 0, "image vide");
    assert!(tile_size > 0, "taille de tuile nulle");
    let mut levels = Vec::new();
    let (mut w, mut h) = (width, height);
    loop {
        let level = levels.len() as u32;
        levels.push(LevelInfo {
            level,
            width: w,
            height: h,
            columns: w.div_ceil(tile_size),
            rows: h.div_ceil(tile_size),
        });
        if w <= tile_size && h <= tile_size {
            return levels;
        }
        w = w.div_ceil(2);
        h = h.div_ceil(2);
    }
}

/// Rectangle `(x, y, largeur, hauteur)` couvert par une tuile dans l'image de
/// son niveau, en pixels. Les tuiles de bord sont plus petites.
pub fn tile_rect(level: &LevelInfo, tile_size: u32, column: u32, row: u32) -> (u32, u32, u32, u32) {
    assert!(
        column < level.columns && row < level.rows,
        "tuile hors du niveau"
    );
    let x = column * tile_size;
    let y = row * tile_size;
    (
        x,
        y,
        tile_size.min(level.width - x),
        tile_size.min(level.height - y),
    )
}

/// Chemin d'une tuile dans le dossier de sa pyramide : `niveau/colonne_ligne.ext`.
pub fn tile_file_name(level: u32, column: u32, row: u32, format: TileFormat) -> String {
    format!("{level}/{column}_{row}.{}", format.extension())
}

// ---------------------------------------------------------------------------
// Image en mémoire et réduction
// ---------------------------------------------------------------------------

/// Image en mémoire : 8 bits par canal, lignes de haut en bas, canaux
/// entrelacés. 1 canal = gris, 3 = RVB, 4 = RVB et transparence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub data: Vec<u8>,
}

impl Raster {
    pub fn new(width: u32, height: u32, channels: u8, data: Vec<u8>) -> Self {
        assert!(matches!(channels, 1 | 3 | 4), "1, 3 ou 4 canaux");
        assert_eq!(
            data.len(),
            width as usize * height as usize * channels as usize,
            "taille des données incohérente"
        );
        Self {
            width,
            height,
            channels,
            data,
        }
    }

    /// Copie d'un rectangle de l'image.
    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Vec<u8> {
        assert!(x + width <= self.width && y + height <= self.height);
        let c = self.channels as usize;
        let stride = self.width as usize * c;
        let mut out = Vec::with_capacity(width as usize * height as usize * c);
        for row in y..y + height {
            let start = row as usize * stride + x as usize * c;
            out.extend_from_slice(&self.data[start..start + width as usize * c]);
        }
        out
    }
}

/// Tables de passage entre les valeurs enregistrées (courbe sRGB, 0–255) et
/// la quantité de lumière (linéaire, 0–65 535).
struct Gamma {
    to_linear: [u16; 256],
    to_srgb: Vec<u8>,
}

fn gamma() -> &'static Gamma {
    static TABLES: OnceLock<Gamma> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut to_linear = [0u16; 256];
        for (value, slot) in to_linear.iter_mut().enumerate() {
            let s = value as f64 / 255.0;
            let linear = if s <= 0.040_45 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            };
            *slot = (linear * 65_535.0).round() as u16;
        }
        let to_srgb = (0..=65_535u32)
            .map(|value| {
                let linear = f64::from(value) / 65_535.0;
                let s = if linear <= 0.003_130_8 {
                    linear * 12.92
                } else {
                    1.055 * linear.powf(1.0 / 2.4) - 0.055
                };
                (s * 255.0).round() as u8
            })
            .collect();
        Gamma { to_linear, to_srgb }
    })
}

/// Réduit une image de moitié (taille arrondie au pixel supérieur).
///
/// Chaque pixel du résultat est la moyenne du carré de 2 × 2 pixels qu'il
/// remplace, calculée **en quantité de lumière** et non sur les valeurs
/// enregistrées. Raison : un trait blanc d'un pixel sur fond noir, réduit
/// quatre fois, couvre 1/16 de son pixel. Une moyenne naïve le rendrait à
/// 6 % (invisible) ; la moyenne en lumière le rend à 28 %, ce que l'œil
/// verrait en s'éloignant. Les traits fins restent lisibles dans les vues
/// d'ensemble, sans être épaissis. Un aplat reste exactement identique.
///
/// Sur un bord impair, la moyenne ne porte que sur les pixels existants. Le
/// canal de transparence, s'il existe, est moyenné tel quel.
pub fn downsample(source: &Raster) -> Raster {
    let tables = gamma();
    let c = source.channels as usize;
    let color_channels = if c == 4 { 3 } else { c };
    let (sw, sh) = (source.width as usize, source.height as usize);
    let (dw, dh) = (sw.div_ceil(2), sh.div_ceil(2));
    let stride = sw * c;
    let mut data = vec![0u8; dw * dh * c];

    for (dy, out_row) in data.chunks_exact_mut(dw * c).enumerate() {
        let top = &source.data[2 * dy * stride..(2 * dy + 1) * stride];
        let bottom = if 2 * dy + 1 < sh {
            Some(&source.data[(2 * dy + 1) * stride..(2 * dy + 2) * stride])
        } else {
            None
        };
        for (dx, out) in out_row.chunks_exact_mut(c).enumerate() {
            let x0 = 2 * dx * c;
            let has_right = 2 * dx + 1 < sw;
            let count = (1 + u32::from(has_right)) * (1 + u32::from(bottom.is_some()));
            for (channel, slot) in out.iter_mut().enumerate() {
                let is_color = channel < color_channels;
                let read = |row: &[u8], offset: usize| -> u32 {
                    let value = row[offset + channel];
                    if is_color {
                        u32::from(tables.to_linear[value as usize])
                    } else {
                        u32::from(value)
                    }
                };
                let mut sum = read(top, x0);
                if has_right {
                    sum += read(top, x0 + c);
                }
                if let Some(bottom) = bottom {
                    sum += read(bottom, x0);
                    if has_right {
                        sum += read(bottom, x0 + c);
                    }
                }
                let mean = (sum + count / 2) / count;
                *slot = if is_color {
                    tables.to_srgb[mean as usize]
                } else {
                    mean as u8
                };
            }
        }
    }
    Raster::new(dw as u32, dh as u32, source.channels, data)
}

/// Choisit le format des tuiles d'après le contenu.
///
/// Un dessin au trait est presque entièrement noir ou blanc : il part en PNG
/// (sans perte, le JPEG ferait baver les traits). Une image avec transparence
/// aussi. Le reste est traité comme une photo, en JPEG.
pub fn detect_format(raster: &Raster) -> TileFormat {
    if raster.channels == 4 {
        return TileFormat::Png;
    }
    let c = raster.channels as usize;
    let pixel_count = raster.data.len() / c;
    // Un pixel sur 97 suffit, et le pas n'est pas aligné sur la largeur.
    let step = if pixel_count > 200_000 { 97 } else { 1 };
    let (mut sampled, mut extreme) = (0u64, 0u64);
    for pixel in raster.data.chunks_exact(c).step_by(step) {
        sampled += 1;
        if pixel.iter().all(|&v| v <= 16) || pixel.iter().all(|&v| v >= 239) {
            extreme += 1;
        }
    }
    if extreme * 100 >= sampled * 92 {
        TileFormat::Png
    } else {
        TileFormat::Jpeg
    }
}

/// Lit une image (PNG, JPEG, TIFF ou HEIC) et la ramène à 8 bits par canal,
/// sans ajouter de canal : une image en gris reste sur un seul canal.
///
/// L'orientation EXIF n'est pas appliquée : les pixels gardent la disposition
/// du fichier (pour le HEIC, voir [`crate::photo`]).
pub fn read_raster(path: &Path) -> Result<Raster, TileError> {
    let image_error = |message: String| TileError::Image(path.to_path_buf(), message);
    let image = if crate::photo::is_heif_file(path) {
        let data = fs::read(path).map_err(io_error(path))?;
        crate::photo::decode_heif(&data).map_err(|e| image_error(e.0))?
    } else {
        let mut reader = ImageReader::open(path)
            .map_err(io_error(path))?
            .with_guessed_format()
            .map_err(io_error(path))?;
        // Une diapositive de 15 433 px en couleur dépasse la limite de
        // mémoire par défaut du décodeur (512 Mo) : c'est un cas normal ici.
        reader.no_limits();
        reader.decode().map_err(|e| image_error(e.to_string()))?
    };
    let (width, height) = (image.width(), image.height());
    if width == 0 || height == 0 {
        return Err(image_error("image vide".into()));
    }
    let (channels, data) = match image {
        DynamicImage::ImageLuma8(_) | DynamicImage::ImageLuma16(_) => {
            (1, image.into_luma8().into_raw())
        }
        other if other.color().has_alpha() => (4, other.into_rgba8().into_raw()),
        other => (3, other.into_rgb8().into_raw()),
    };
    Ok(Raster::new(width, height, channels, data))
}

// ---------------------------------------------------------------------------
// Fabrication
// ---------------------------------------------------------------------------

/// Fabrique la pyramide de `image_path` dans `out_dir` et renvoie son
/// descripteur. Le dossier est créé ; les fichiers qui s'y trouvent déjà sous
/// le même nom sont remplacés.
///
/// Mémoire : l'image décodée, plus le niveau suivant (un quart) pendant la
/// réduction. Une diapositive de 15 433 px en gris demande environ 300 Mo.
pub fn build_pyramid(
    image_path: &Path,
    out_dir: &Path,
    options: &TileOptions,
) -> Result<Pyramid, TileError> {
    let raster = read_raster(image_path)?;
    build_pyramid_from_raster(raster, out_dir, options)
}

/// Comme `build_pyramid`, à partir d'une image déjà en mémoire.
pub fn build_pyramid_from_raster(
    raster: Raster,
    out_dir: &Path,
    options: &TileOptions,
) -> Result<Pyramid, TileError> {
    if options.tile_size < 16 {
        return Err(TileError::Invalid(
            "la taille de tuile doit être d'au moins 16 px".into(),
        ));
    }
    if !(1..=100).contains(&options.jpeg_quality) {
        return Err(TileError::Invalid("la qualité JPEG va de 1 à 100".into()));
    }
    let format = options.format.unwrap_or_else(|| detect_format(&raster));
    // Le JPEG n'a pas de transparence : on la retire plutôt que d'échouer.
    let raster = if format == TileFormat::Jpeg && raster.channels == 4 {
        drop_alpha(raster)
    } else {
        raster
    };
    let pyramid = Pyramid {
        version: PYRAMID_VERSION,
        width: raster.width,
        height: raster.height,
        tile_size: options.tile_size,
        format,
        channels: raster.channels,
        levels: level_layout(raster.width, raster.height, options.tile_size),
    };
    let threads = options
        .threads
        .or_else(|| std::thread::available_parallelism().ok().map(usize::from))
        .unwrap_or(1)
        .max(1);

    fs::create_dir_all(out_dir).map_err(io_error(out_dir))?;
    let mut current = raster;
    for level in &pyramid.levels {
        debug_assert_eq!((current.width, current.height), (level.width, level.height));
        let level_dir = out_dir.join(level.level.to_string());
        fs::create_dir_all(&level_dir).map_err(io_error(&level_dir))?;
        write_level_tiles(&current, level, &pyramid, out_dir, options, threads)?;
        if level.level + 1 < pyramid.levels.len() as u32 {
            current = downsample(&current);
        }
    }

    // Le descripteur est écrit en dernier : sa présence dit que la pyramide
    // est complète.
    let descriptor_path = out_dir.join(DESCRIPTOR_FILE);
    let json = serde_json::to_string_pretty(&pyramid).expect("Pyramid est sérialisable");
    fs::write(&descriptor_path, json).map_err(io_error(&descriptor_path))?;
    Ok(pyramid)
}

fn drop_alpha(raster: Raster) -> Raster {
    let data = raster
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, _]| [r, g, b])
        .collect();
    Raster::new(raster.width, raster.height, 3, data)
}

/// Écrit toutes les tuiles d'un niveau, réparties sur `threads` fils.
fn write_level_tiles(
    raster: &Raster,
    level: &LevelInfo,
    pyramid: &Pyramid,
    out_dir: &Path,
    options: &TileOptions,
    threads: usize,
) -> Result<(), TileError> {
    let total = level.tile_count() as usize;
    let next = AtomicUsize::new(0);
    let failure: Mutex<Option<TileError>> = Mutex::new(None);
    let work = || loop {
        let index = next.fetch_add(1, Ordering::Relaxed);
        if index >= total || failure.lock().unwrap().is_some() {
            return;
        }
        let column = index as u32 % level.columns;
        let row = index as u32 / level.columns;
        let (x, y, width, height) = tile_rect(level, pyramid.tile_size, column, row);
        let pixels = raster.crop(x, y, width, height);
        let path = out_dir.join(tile_file_name(level.level, column, row, pyramid.format));
        if let Err(error) = write_tile(&path, &pixels, width, height, pyramid, options) {
            failure.lock().unwrap().get_or_insert(error);
            return;
        }
    };
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

fn write_tile(
    path: &Path,
    pixels: &[u8],
    width: u32,
    height: u32,
    pyramid: &Pyramid,
    options: &TileOptions,
) -> Result<(), TileError> {
    let color = match pyramid.channels {
        1 => ExtendedColorType::L8,
        3 => ExtendedColorType::Rgb8,
        _ => ExtendedColorType::Rgba8,
    };
    let file = BufWriter::new(fs::File::create(path).map_err(io_error(path))?);
    let result = match pyramid.format {
        // Compression rapide : la fabrication doit rester courte, et l'écart
        // de poids est faible sur des tuiles de cette taille.
        TileFormat::Png => {
            PngEncoder::new_with_quality(file, CompressionType::Fast, FilterType::Adaptive)
                .write_image(pixels, width, height, color)
        }
        TileFormat::Jpeg => JpegEncoder::new_with_quality(file, options.jpeg_quality)
            .write_image(pixels, width, height, color),
    };
    result.map_err(|e| TileError::Image(path.to_path_buf(), e.to_string()))
}

/// Lit le descripteur d'un dossier de pyramide.
pub fn read_descriptor(dir: &Path) -> Result<Pyramid, TileError> {
    let path = dir.join(DESCRIPTOR_FILE);
    let text = fs::read_to_string(&path).map_err(io_error(&path))?;
    serde_json::from_str(&text).map_err(|e| TileError::Invalid(format!("{} : {e}", path.display())))
}

// ---------------------------------------------------------------------------
// Cache de pyramides, pour l'application
// ---------------------------------------------------------------------------

/// Renvoie la pyramide de `image_path` rangée dans `cache_root`, en la
/// fabriquant si elle n'y est pas encore.
///
/// Le sous-dossier porte une clé calculée à partir du chemin de l'image, de
/// sa taille, de sa date de modification et des réglages : une image modifiée
/// est donc retuilée, une image inchangée ne l'est jamais deux fois.
pub fn ensure_pyramid(
    image_path: &Path,
    cache_root: &Path,
    options: &TileOptions,
) -> Result<CachedPyramid, TileError> {
    let key = cache_key(image_path, options)?;
    let dir = cache_root.join(&key);
    if let Ok(pyramid) = read_descriptor(&dir) {
        if pyramid.version == PYRAMID_VERSION {
            return Ok(CachedPyramid { key, pyramid });
        }
    }
    // Fabrication dans un dossier provisoire, renommé une fois complet : une
    // fabrication interrompue ne laisse jamais une pyramide à moitié faite
    // sous son nom définitif.
    let partial = cache_root.join(format!("{key}.partiel-{}", std::process::id()));
    let _ = fs::remove_dir_all(&partial);
    let pyramid = match build_pyramid(image_path, &partial, options) {
        Ok(pyramid) => pyramid,
        Err(error) => {
            let _ = fs::remove_dir_all(&partial);
            return Err(error);
        }
    };
    let _ = fs::remove_dir_all(&dir);
    fs::rename(&partial, &dir).map_err(io_error(&dir))?;
    Ok(CachedPyramid { key, pyramid })
}

fn cache_key(image_path: &Path, options: &TileOptions) -> Result<String, TileError> {
    let canonical = fs::canonicalize(image_path).map_err(io_error(image_path))?;
    let metadata = fs::metadata(&canonical).map_err(io_error(image_path))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos());
    let identity = format!(
        "{}|{}|{modified}|{}|{:?}|{}|v{PYRAMID_VERSION}",
        canonical.display(),
        metadata.len(),
        options.tile_size,
        options.format,
        options.jpeg_quality,
    );
    Ok(format!("{:016x}", fnv1a_64(identity.as_bytes())))
}

/// Empreinte FNV-1a sur 64 bits : courte, et stable d'une version de Rust à
/// l'autre (contrairement à celle de la bibliothèque standard).
pub(crate) fn fnv1a_64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Traduit le chemin demandé par l'interface (`clé/niveau/colonne_ligne.png`,
/// `clé/pyramid.json`) en fichier du cache.
///
/// Renvoie `None` pour tout chemin qui pourrait sortir du cache : seuls des
/// segments faits de lettres, chiffres, points, tirets et soulignés sont
/// admis, et jamais `.` ni `..`.
pub fn resolve_tile_file(cache_root: &Path, request_path: &str) -> Option<PathBuf> {
    let mut path = cache_root.to_path_buf();
    let mut segments = 0;
    for segment in request_path.trim_start_matches('/').split('/') {
        let allowed = !segment.is_empty()
            && segment != "."
            && segment != ".."
            && segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'));
        if !allowed {
            return None;
        }
        path.push(segment);
        segments += 1;
    }
    (segments >= 2).then_some(path)
}

/// Lit le fichier du cache demandé par l'interface et renvoie son type MIME
/// et son contenu ; `None` si le chemin est refusé ou le fichier absent.
pub fn read_cached_file(cache_root: &Path, request_path: &str) -> Option<(&'static str, Vec<u8>)> {
    let path = resolve_tile_file(cache_root, request_path)?;
    let bytes = fs::read(&path).ok()?;
    Some((content_type(&path), bytes))
}

/// Type MIME d'un fichier de pyramide, d'après son extension.
pub fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg") => "image/jpeg",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dossier provisoire propre à un test, supprimé en fin de test.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("insitu-tiles-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Image de synthèse reproductible : dégradé et traits fins.
    fn synthetic(width: u32, height: u32, channels: u8) -> Raster {
        let mut data = Vec::with_capacity((width * height) as usize * channels as usize);
        for y in 0..height {
            for x in 0..width {
                for channel in 0..u32::from(channels) {
                    let line = x % 37 == 5 || y % 53 == 11;
                    let value = if line {
                        255
                    } else {
                        (x * 3 + y * 7 + channel * 40) % 251
                    };
                    data.push(value as u8);
                }
            }
        }
        Raster::new(width, height, channels, data)
    }

    /// Relit toutes les tuiles d'un niveau et les recolle en une image.
    fn reassemble(dir: &Path, pyramid: &Pyramid, level: &LevelInfo) -> Raster {
        let c = pyramid.channels as usize;
        let mut data = vec![0u8; level.width as usize * level.height as usize * c];
        for row in 0..level.rows {
            for column in 0..level.columns {
                let (x, y, width, height) = tile_rect(level, pyramid.tile_size, column, row);
                let path = dir.join(tile_file_name(level.level, column, row, pyramid.format));
                let tile = read_raster(&path).unwrap();
                assert_eq!((tile.width, tile.height), (width, height), "{path:?}");
                assert_eq!(tile.channels, pyramid.channels, "{path:?}");
                for line in 0..height as usize {
                    let from = line * width as usize * c;
                    let to = ((y as usize + line) * level.width as usize + x as usize) * c;
                    data[to..to + width as usize * c]
                        .copy_from_slice(&tile.data[from..from + width as usize * c]);
                }
            }
        }
        Raster::new(level.width, level.height, pyramid.channels, data)
    }

    #[test]
    fn slide_of_15433_px_has_six_levels_and_1302_tiles() {
        let levels = level_layout(15_433, 15_433, 512);
        let sizes: Vec<u32> = levels.iter().map(|l| l.width).collect();
        assert_eq!(sizes, [15_433, 7_717, 3_859, 1_930, 965, 483]);
        let columns: Vec<u32> = levels.iter().map(|l| l.columns).collect();
        assert_eq!(columns, [31, 16, 8, 4, 2, 1]);
        let total: u64 = levels.iter().map(LevelInfo::tile_count).sum();
        assert_eq!(total, 961 + 256 + 64 + 16 + 4 + 1);
    }

    #[test]
    fn layout_handles_exact_multiples_tiny_and_oblong_images() {
        // Multiple exact de la tuile : pas de colonne vide en trop.
        let exact = level_layout(1024, 512, 512);
        assert_eq!((exact[0].columns, exact[0].rows), (2, 1));
        assert_eq!(exact.len(), 2);
        assert_eq!((exact[1].width, exact[1].height), (512, 256));

        // Image plus petite qu'une tuile : un seul niveau, une seule tuile.
        let tiny = level_layout(3, 2, 512);
        assert_eq!(tiny.len(), 1);
        assert_eq!(tiny[0].tile_count(), 1);

        // Image très allongée : la hauteur ne descend jamais sous 1 px.
        let oblong = level_layout(5000, 3, 512);
        assert_eq!(oblong.last().unwrap().width, 313);
        assert!(oblong.iter().all(|l| l.height >= 1 && l.rows == 1));

        // Un pixel de plus qu'une tuile : une colonne d'un pixel de large.
        let over = level_layout(513, 513, 512);
        assert_eq!(tile_rect(&over[0], 512, 1, 1), (512, 512, 1, 1));
        assert_eq!(tile_rect(&over[0], 512, 0, 0), (0, 0, 512, 512));
    }

    #[test]
    fn tiles_of_a_level_cover_it_exactly_once() {
        for (width, height) in [(15_433, 15_433), (5_512, 5_512), (5_145, 3_508), (513, 1)] {
            for level in level_layout(width, height, 512) {
                let mut area = 0u64;
                for row in 0..level.rows {
                    for column in 0..level.columns {
                        let (x, y, w, h) = tile_rect(&level, 512, column, row);
                        assert!(w >= 1 && h >= 1);
                        assert!(x + w <= level.width && y + h <= level.height);
                        area += u64::from(w) * u64::from(h);
                    }
                }
                assert_eq!(area, u64::from(level.width) * u64::from(level.height));
            }
        }
    }

    #[test]
    fn downsample_keeps_flat_areas_and_averages_light() {
        // Un aplat reste identique, quelle que soit sa valeur.
        for value in [0u8, 1, 17, 128, 254, 255] {
            let flat = Raster::new(5, 3, 1, vec![value; 15]);
            let half = downsample(&flat);
            assert_eq!((half.width, half.height), (3, 2));
            assert!(half.data.iter().all(|&v| v == value), "valeur {value}");
        }
        // Un pixel blanc sur quatre : 25 % de lumière, soit 137 en sRGB (une
        // moyenne naïve donnerait 64).
        let dot = Raster::new(2, 2, 1, vec![255, 0, 0, 0]);
        assert_eq!(downsample(&dot).data, [137]);
        // Bord impair : la moyenne ne porte que sur les pixels existants.
        let edge = Raster::new(3, 1, 1, vec![0, 0, 200]);
        assert_eq!(downsample(&edge).data, [0, 200]);
    }

    #[test]
    fn a_one_pixel_line_stays_visible_four_levels_down() {
        // Trait blanc vertical d'un pixel sur fond noir, réduit 16 fois.
        let mut data = vec![0u8; 64 * 64];
        for y in 0..64 {
            data[y * 64 + 20] = 255;
        }
        let mut raster = Raster::new(64, 64, 1, data);
        for _ in 0..4 {
            raster = downsample(&raster);
        }
        assert_eq!((raster.width, raster.height), (4, 4));
        let brightest = *raster.data.iter().max().unwrap();
        // 1/16 de lumière : 71 sur 255. Une moyenne naïve donnerait 16.
        assert!((68..=74).contains(&brightest), "valeur {brightest}");
    }

    #[test]
    fn downsample_averages_transparency_without_gamma() {
        let pixels = [255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let half = downsample(&Raster::new(2, 2, 4, pixels.to_vec()));
        assert_eq!(half.data, [137, 137, 137, 64]);
    }

    #[test]
    fn reassembled_levels_equal_the_downsampled_image() {
        // Tailles choisies pour avoir des bords impairs à plusieurs niveaux.
        for channels in [1u8, 3, 4] {
            let scratch = Scratch::new(&format!("recomposition-{channels}"));
            let source = synthetic(301, 173, channels);
            let options = TileOptions {
                tile_size: 64,
                format: Some(TileFormat::Png),
                ..TileOptions::default()
            };
            let pyramid = build_pyramid_from_raster(source.clone(), &scratch.0, &options).unwrap();
            assert_eq!(pyramid.levels.len(), 4);
            assert_eq!(read_descriptor(&scratch.0).unwrap(), pyramid);

            let mut expected = source;
            for level in &pyramid.levels {
                assert_eq!(reassemble(&scratch.0, &pyramid, level), expected);
                let files = fs::read_dir(scratch.0.join(level.level.to_string()))
                    .unwrap()
                    .count() as u64;
                assert_eq!(files, level.tile_count());
                expected = downsample(&expected);
            }
        }
    }

    #[test]
    fn jpeg_tiles_have_the_right_sizes_and_stay_close() {
        let scratch = Scratch::new("jpeg");
        // Dégradé doux : le JPEG doit le rendre à quelques niveaux près.
        let data = (0..200u32 * 150)
            .flat_map(|i| {
                let (x, y) = (i % 200, i / 200);
                [(x / 2 + 40) as u8, (y + 30) as u8, 120]
            })
            .collect();
        let source = Raster::new(200, 150, 3, data);
        let options = TileOptions {
            tile_size: 128,
            format: Some(TileFormat::Jpeg),
            ..TileOptions::default()
        };
        let pyramid = build_pyramid_from_raster(source.clone(), &scratch.0, &options).unwrap();
        assert_eq!(pyramid.format, TileFormat::Jpeg);
        assert!(scratch.0.join("0/1_1.jpg").is_file());
        let level0 = reassemble(&scratch.0, &pyramid, &pyramid.levels[0]);
        let worst = level0
            .data
            .iter()
            .zip(&source.data)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(worst <= 12, "écart JPEG de {worst} niveaux");
    }

    #[test]
    fn line_art_goes_to_png_and_photos_to_jpeg() {
        let mut drawing = vec![0u8; 400 * 400];
        for i in (0..drawing.len()).step_by(41) {
            drawing[i] = 255;
        }
        assert_eq!(
            detect_format(&Raster::new(400, 400, 1, drawing)),
            TileFormat::Png
        );
        assert_eq!(detect_format(&synthetic(400, 400, 3)), TileFormat::Jpeg);
        assert_eq!(detect_format(&synthetic(40, 40, 4)), TileFormat::Png);
    }

    #[test]
    fn cache_builds_once_and_rebuilds_when_the_image_changes() {
        let scratch = Scratch::new("cache");
        let image_path = scratch.0.join("source.png");
        let cache = scratch.0.join("cache");
        let save = |raster: &Raster| {
            image::save_buffer(
                &image_path,
                &raster.data,
                raster.width,
                raster.height,
                ExtendedColorType::L8,
            )
            .unwrap();
        };
        let options = TileOptions {
            tile_size: 64,
            format: Some(TileFormat::Png),
            ..TileOptions::default()
        };

        save(&synthetic(150, 90, 1));
        let first = ensure_pyramid(&image_path, &cache, &options).unwrap();
        assert_eq!(first.pyramid.width, 150);
        let marker = cache.join(&first.key).join("temoin");
        fs::write(&marker, "").unwrap();

        // Deuxième appel : même clé, et le dossier n'a pas été refait.
        let second = ensure_pyramid(&image_path, &cache, &options).unwrap();
        assert_eq!(second, first);
        assert!(marker.exists());

        // Image modifiée : nouvelle clé, nouvelle pyramide.
        save(&synthetic(151, 90, 1));
        let third = ensure_pyramid(&image_path, &cache, &options).unwrap();
        assert_ne!(third.key, first.key);
        assert_eq!(third.pyramid.width, 151);

        // L'interface lit le descripteur et les tuiles par leur chemin.
        let request = format!("/{}/{DESCRIPTOR_FILE}", third.key);
        let (mime, bytes) = read_cached_file(&cache, &request).unwrap();
        assert_eq!(mime, "application/json");
        assert_eq!(
            serde_json::from_slice::<Pyramid>(&bytes).unwrap(),
            third.pyramid
        );
        let (mime, bytes) = read_cached_file(&cache, &format!("/{}/0/2_1.png", third.key)).unwrap();
        assert_eq!(mime, "image/png");
        assert!(bytes.starts_with(b"\x89PNG"));
        assert!(read_cached_file(&cache, &format!("/{}/0/9_9.png", third.key)).is_none());
        assert!(read_cached_file(&cache, "/../source.png").is_none());

        // Aucun dossier provisoire ne reste.
        let leftovers = fs::read_dir(&cache)
            .unwrap()
            .filter(|e| {
                let name = e.as_ref().unwrap().file_name();
                name.to_string_lossy().contains("partiel")
            })
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn unreadable_image_is_reported_and_leaves_no_cache_entry() {
        let scratch = Scratch::new("illisible");
        let image_path = scratch.0.join("faux.png");
        fs::write(&image_path, b"pas une image").unwrap();
        let cache = scratch.0.join("cache");
        fs::create_dir_all(&cache).unwrap();
        let error = ensure_pyramid(&image_path, &cache, &TileOptions::default()).unwrap_err();
        assert!(matches!(error, TileError::Image(..)), "{error}");
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
    }

    #[test]
    fn tile_requests_cannot_leave_the_cache() {
        let root = Path::new("/cache");
        assert_eq!(
            resolve_tile_file(root, "/0123abcd/2/3_1.png"),
            Some(PathBuf::from("/cache/0123abcd/2/3_1.png"))
        );
        assert_eq!(
            resolve_tile_file(root, "0123abcd/pyramid.json"),
            Some(PathBuf::from("/cache/0123abcd/pyramid.json"))
        );
        for request in [
            "",
            "/",
            "pyramid.json",
            "/../secret",
            "/abc/../../etc/passwd",
            "/abc/./x.png",
            "/abc//x.png",
            "/abc/%2e%2e/x.png",
            "/abc/x y.png",
            "/abc\\..\\x.png",
        ] {
            assert_eq!(resolve_tile_file(root, request), None, "{request:?}");
        }
        assert_eq!(content_type(Path::new("a/0_0.png")), "image/png");
        assert_eq!(content_type(Path::new("a/0_0.jpg")), "image/jpeg");
        assert_eq!(
            content_type(Path::new("a/pyramid.json")),
            "application/json"
        );
    }
}

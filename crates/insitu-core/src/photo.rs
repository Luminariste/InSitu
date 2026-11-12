//! Lecture des photos : JPEG, PNG, TIFF, et HEIC (format des iPhone).
//!
//! Le HEIC est décodé par `zune-heic`, en Rust pur (décision D-2.5-a) ; les
//! autres formats par la crate `image`. Le format est reconnu au contenu du
//! fichier, pas à son extension.
//!
//! Disposition des pixels :
//! - JPEG, PNG, TIFF : celle du fichier, sans appliquer l'orientation EXIF
//!   (décision D9) : une photo d'iPhone en JPEG peut donc apparaître couchée ;
//! - HEIC : la rotation et le miroir inscrits dans le conteneur (`irot`,
//!   `imir`) et le recadrage (`clap`) sont appliqués, comme le veut la norme
//!   HEIF : ils font partie de l'image (décision D-2.5-a).

use std::fmt;
use std::path::Path;

use image::{DynamicImage, GrayImage, ImageBuffer, ImageReader, RgbImage, RgbaImage};
use zune_heic::zune_core::bytestream::ZCursor;
use zune_heic::zune_core::colorspace::ColorSpace;
use zune_heic::zune_core::options::DecoderOptions;
use zune_heic::HeifDecoder;

/// Extensions de photo proposées dans les fenêtres de choix de fichier.
pub const PHOTO_EXTENSIONS: [&str; 7] = ["jpg", "jpeg", "png", "tif", "tiff", "heic", "heif"];

/// Plus grand côté accepté, en pixels (une photo de 48 Mpx fait 8 064 px).
const MAX_SIDE: usize = 20_000;

/// Échec de lecture d'une photo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoError(pub String);

impl fmt::Display for PhotoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PhotoError {}

/// Marques HEIF reconnues dans la boîte `ftyp`.
const HEIF_BRANDS: [&[u8; 4]; 10] = [
    b"heic", b"heix", b"heim", b"heis", b"hevc", b"hevx", b"mif1", b"msf1", b"mif2", b"miaf",
];

/// Vrai si ces octets sont le début d'un fichier HEIF (HEIC compris) :
/// boîte `ftyp` en tête, avec une marque HEIF comme marque principale ou
/// parmi les marques compatibles (certains appareils mettent une autre
/// marque principale).
pub fn is_heif(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[4..8] != b"ftyp" {
        return false;
    }
    let size = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let end = size.clamp(12, bytes.len());
    let is_heif_brand = |b: &[u8]| HEIF_BRANDS.iter().any(|brand| b == &brand[..]);
    // Marque principale (8..12), version (12..16), marques compatibles.
    is_heif_brand(&bytes[8..12])
        || (16..end.saturating_sub(3))
            .step_by(4)
            .any(|i| is_heif_brand(&bytes[i..i + 4]))
}

/// Vrai si le fichier est un HEIF, d'après ses premiers octets.
pub fn is_heif_file(path: &Path) -> bool {
    use std::io::Read;
    let mut head = Vec::with_capacity(64);
    std::fs::File::open(path)
        .and_then(|f| f.take(64).read_to_end(&mut head))
        .is_ok_and(|_| is_heif(&head))
}

/// Lit un fichier HEIF. Si notre décodeur n'y arrive pas (variante non
/// prise en charge, par exemple 10 bits), sur Mac on passe par le
/// convertisseur du système, `sips` (D-2.5-k).
pub fn read_heif(path: &Path) -> Result<DynamicImage, PhotoError> {
    let data = std::fs::read(path).map_err(|e| PhotoError(e.to_string()))?;
    match decode_heif(&data) {
        Ok(image) => Ok(image),
        Err(own) => convert_with_system(path).map_err(|system| {
            PhotoError(match system {
                Some(system) => format!("{own} ; conversion par le système : {system}"),
                None => own.0,
            })
        }),
    }
}

/// Conversion par `sips` (macOS) en PNG provisoire. `Err(None)` : pas de
/// convertisseur sur ce système.
#[cfg(target_os = "macos")]
fn convert_with_system(path: &Path) -> Result<DynamicImage, Option<String>> {
    let out = std::env::temp_dir().join(format!(
        "insitu-heic-{}-{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let status = std::process::Command::new("/usr/bin/sips")
        .args(["-s", "format", "png"])
        .arg(path)
        .arg("--out")
        .arg(&out)
        .output()
        .map_err(|e| Some(e.to_string()))?;
    let result = if status.status.success() {
        let mut reader = ImageReader::open(&out)
            .map_err(|e| Some(e.to_string()))?
            .with_guessed_format()
            .map_err(|e| Some(e.to_string()))?;
        reader.no_limits();
        reader.decode().map_err(|e| Some(e.to_string()))
    } else {
        Err(Some(
            String::from_utf8_lossy(&status.stderr).trim().to_string(),
        ))
    };
    let _ = std::fs::remove_file(&out);
    result
}

#[cfg(not(target_os = "macos"))]
fn convert_with_system(_path: &Path) -> Result<DynamicImage, Option<String>> {
    Err(None)
}

/// Lit une photo et la renvoie telle quelle (gris, RVB ou RVBA, 8 ou 16
/// bits selon le fichier).
pub fn read(path: &Path) -> Result<DynamicImage, PhotoError> {
    let unreadable =
        |e: &dyn fmt::Display| PhotoError(format!("{} illisible : {e}", path.display()));
    if is_heif_file(path) {
        return read_heif(path).map_err(|e| unreadable(&e));
    }
    let mut reader = ImageReader::open(path)
        .map_err(|e| unreadable(&e))?
        .with_guessed_format()
        .map_err(|e| unreadable(&e))?;
    // Une diapositive de 15 433 px dépasse la limite de mémoire par défaut
    // du décodeur (512 Mo) : c'est un cas normal ici.
    reader.no_limits();
    reader.decode().map_err(|e| unreadable(&e))
}

/// Lit une photo en RVB 8 bits.
pub fn read_rgb(path: &Path) -> Result<RgbImage, PhotoError> {
    read(path).map(DynamicImage::into_rgb8)
}

/// Dimensions d'une photo en pixels, sans la décoder entièrement.
pub fn dimensions(path: &Path) -> Result<(u32, u32), PhotoError> {
    let unreadable =
        |e: &dyn fmt::Display| PhotoError(format!("{} illisible : {e}", path.display()));
    if is_heif_file(path) {
        let data = std::fs::read(path).map_err(|e| unreadable(&e))?;
        let mut decoder = HeifDecoder::new_with_options(ZCursor::new(data.as_slice()), options());
        if decoder.decode_headers().is_ok() {
            if let (Some(w), Some(h)) = (decoder.width(), decoder.height()) {
                return Ok((w as u32, h as u32));
            }
        }
        // En-têtes que notre décodeur ne lit pas : lecture complète (avec
        // le secours du système sur Mac).
        return read_heif(path)
            .map(|image| (image.width(), image.height()))
            .map_err(|e| unreadable(&e));
    }
    image::image_dimensions(path).map_err(|e| unreadable(&e))
}

fn options() -> DecoderOptions {
    DecoderOptions::default()
        // Le décodeur logiciel partout, pour que le Mac et les tests donnent
        // exactement les mêmes pixels (le décodeur matériel d'Apple convertit
        // les couleurs autrement).
        .hvec_set_use_videotoolbox(false)
        .set_hevc_max_mdat_size(512 << 20)
        .set_max_width(MAX_SIDE)
        .set_max_height(MAX_SIDE)
}

/// Décode un fichier HEIF déjà en mémoire.
pub fn decode_heif(data: &[u8]) -> Result<DynamicImage, PhotoError> {
    let failed = |e: &dyn fmt::Display| PhotoError(format!("HEIC : {e}"));
    let mut decoder = HeifDecoder::new_with_options(ZCursor::new(data), options());
    let pixels = decoder.decode().map_err(|e| failed(&e))?;
    let (Some(width), Some(height), Some(colorspace)) =
        (decoder.width(), decoder.height(), decoder.colorspace())
    else {
        return Err(failed(&"dimensions absentes"));
    };
    let (w, h) = (width as u32, height as u32);
    let short = || failed(&"données incomplètes");
    Ok(match colorspace {
        ColorSpace::RGB => {
            DynamicImage::ImageRgb8(ImageBuffer::from_raw(w, h, pixels).ok_or_else(short)?)
        }
        ColorSpace::RGBA => {
            DynamicImage::ImageRgba8(RgbaImage::from_raw(w, h, pixels).ok_or_else(short)?)
        }
        ColorSpace::Luma => {
            DynamicImage::ImageLuma8(GrayImage::from_raw(w, h, pixels).ok_or_else(short)?)
        }
        ColorSpace::LumaA => {
            DynamicImage::ImageLumaA8(ImageBuffer::from_raw(w, h, pixels).ok_or_else(short)?)
        }
        other => {
            return Err(failed(&format!(
                "espace de couleur {other:?} non pris en charge"
            )))
        }
    })
}

/// Format d'écriture d'une image, choisi d'après l'extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputFormat {
    Png,
    Tiff,
}

impl OutputFormat {
    /// `png`, `tif` ou `tiff`, sans tenir compte des majuscules.
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
        match extension.as_str() {
            "png" => Some(Self::Png),
            "tif" | "tiff" => Some(Self::Tiff),
            _ => None,
        }
    }
}

/// Encode une image RVB avec sa résolution inscrite (`dpi`), pour qu'un
/// autre logiciel l'ouvre à la bonne taille. PNG : compression rapide.
/// TIFF : LZW avec prédiction horizontale (sans perte).
pub fn encode_rgb(image: &RgbImage, format: OutputFormat, dpi: f64) -> Result<Vec<u8>, PhotoError> {
    let failed = |e: &dyn fmt::Display| PhotoError(format!("écriture de l'image impossible : {e}"));
    let mut bytes = Vec::new();
    match format {
        OutputFormat::Png => {
            let mut encoder = png::Encoder::new(&mut bytes, image.width(), image.height());
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_compression(png::Compression::Fast);
            let per_metre = (dpi / 0.0254).round() as u32;
            encoder.set_pixel_dims(Some(png::PixelDimensions {
                xppu: per_metre,
                yppu: per_metre,
                unit: png::Unit::Meter,
            }));
            let mut writer = encoder.write_header().map_err(|e| failed(&e))?;
            writer
                .write_image_data(image.as_raw())
                .map_err(|e| failed(&e))?;
            writer.finish().map_err(|e| failed(&e))?;
        }
        OutputFormat::Tiff => {
            use tiff::encoder::{colortype, Compression, Predictor, Rational, TiffEncoder};
            use tiff::tags::ResolutionUnit;
            let mut cursor = std::io::Cursor::new(&mut bytes);
            let mut encoder = TiffEncoder::new(&mut cursor)
                .map_err(|e| failed(&e))?
                .with_compression(Compression::Lzw)
                .with_predictor(Predictor::Horizontal);
            let mut tiff_image = encoder
                .new_image::<colortype::RGB8>(image.width(), image.height())
                .map_err(|e| failed(&e))?;
            // Résolution en points par pouce, au centième près.
            tiff_image.resolution(
                ResolutionUnit::Inch,
                Rational {
                    n: (dpi * 100.0).round() as u32,
                    d: 100,
                },
            );
            tiff_image
                .write_data(image.as_raw())
                .map_err(|e| failed(&e))?;
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_images_read_back_identical_with_their_resolution() {
        let image = RgbImage::from_fn(37, 23, |x, y| image::Rgb([x as u8 * 6, y as u8 * 9, 200]));
        for format in [OutputFormat::Png, OutputFormat::Tiff] {
            let bytes = encode_rgb(&image, format, 500.0).unwrap();
            let back = image::load_from_memory(&bytes).unwrap().into_rgb8();
            assert_eq!(back, image, "{format:?}");
        }
        // Résolution inscrite : PNG en points par mètre, TIFF en points par pouce.
        let png = encode_rgb(&image, OutputFormat::Png, 500.0).unwrap();
        let reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let dims = reader.info().pixel_dims.unwrap();
        assert_eq!(dims.xppu, 19_685); // 500 / 0,0254
        let tiff = encode_rgb(&image, OutputFormat::Tiff, 500.0).unwrap();
        let mut decoder = tiff::decoder::Decoder::new(std::io::Cursor::new(tiff)).unwrap();
        let x = decoder.get_tag(tiff::tags::Tag::XResolution).unwrap();
        assert!(
            matches!(x, tiff::decoder::ifd::Value::Rational(50_000, 100)),
            "{x:?}"
        );
    }

    #[test]
    fn output_format_comes_from_the_extension() {
        assert_eq!(
            OutputFormat::from_path(Path::new("a/b.TIF")),
            Some(OutputFormat::Tiff)
        );
        assert_eq!(
            OutputFormat::from_path(Path::new("b.tiff")),
            Some(OutputFormat::Tiff)
        );
        assert_eq!(
            OutputFormat::from_path(Path::new("b.png")),
            Some(OutputFormat::Png)
        );
        assert_eq!(OutputFormat::from_path(Path::new("b.jpg")), None);
        assert_eq!(OutputFormat::from_path(Path::new("b")), None);
    }

    #[test]
    fn heif_is_recognised_by_its_header() {
        let mut head = vec![0, 0, 0, 24];
        head.extend_from_slice(b"ftypheic");
        assert!(is_heif(&head));
        head[8..12].copy_from_slice(b"mif1");
        assert!(is_heif(&head));
        head[8..12].copy_from_slice(b"isom"); // une vidéo MP4
        assert!(!is_heif(&head));
        assert!(!is_heif(b"\xFF\xD8\xFF\xE0JFIF"));
    }

    #[test]
    fn heif_is_recognised_by_a_compatible_brand() {
        // Marque principale inconnue, « heic » parmi les marques compatibles.
        let mut head = vec![0, 0, 0, 28];
        head.extend_from_slice(b"ftypXYZ1\0\0\0\0isomheic");
        assert!(is_heif(&head));
        // Une vidéo MP4 : marques compatibles sans HEIF.
        let mut video = vec![0, 0, 0, 24];
        video.extend_from_slice(b"ftypisom\0\0\x02\0isomiso2");
        assert!(!is_heif(&video));
        // Une marque HEIF au-delà de la boîte ftyp ne compte pas.
        let mut short = vec![0, 0, 0, 16];
        short.extend_from_slice(b"ftypisom\0\0\0\0heic");
        assert!(!is_heif(&short));
    }

    #[test]
    fn garbage_heif_is_refused_with_a_message() {
        let mut data = vec![0, 0, 0, 24];
        data.extend_from_slice(b"ftypheic");
        data.extend_from_slice(&[0; 40]);
        let error = decode_heif(&data).unwrap_err();
        assert!(error.0.starts_with("HEIC"), "{error}");
    }
}

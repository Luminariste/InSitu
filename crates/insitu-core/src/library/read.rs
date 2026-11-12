//! Lecture d'un dessin, quel que soit son format (tâche 4.1) : PNG, JPEG,
//! TIFF, BMP, PSD aplati et SVG.
//!
//! Le résultat est une [`Raster`] en 8 bits par canal : gris, couleur, ou
//! couleur et transparence, comme le fichier. Un SVG est rendu à la taille
//! demandée, depuis ses vecteurs.

use std::fs;
use std::path::Path;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

use crate::tiles::{self, Raster};

use super::psd::decode_psd;
use super::LibraryError;

/// Extensions reconnues comme dessins, en minuscules.
pub const DRAWING_EXTENSIONS: [&str; 8] =
    ["png", "jpg", "jpeg", "tif", "tiff", "bmp", "psd", "svg"];

/// Nature d'un fichier de dessin, d'après son extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawingFormat {
    /// PNG, JPEG, TIFF ou BMP, lus par la bibliothèque `image`.
    Image,
    /// Photoshop : seule l'image aplatie enregistrée dans le fichier est lue.
    Psd,
    /// Vectoriel : rendu à la taille voulue.
    Svg,
}

impl DrawingFormat {
    /// `None` si l'extension n'est pas celle d'un dessin.
    pub fn of(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        match extension.as_str() {
            "psd" => Some(Self::Psd),
            "svg" => Some(Self::Svg),
            other if DRAWING_EXTENSIONS.contains(&other) => Some(Self::Image),
            _ => None,
        }
    }
}

/// Un dessin lu : ses pixels et sa taille dans l'unité du fichier.
pub struct DecodedDrawing {
    pub raster: Raster,
    /// Taille du fichier : pixels, ou pixels CSS pour un SVG.
    pub width: f64,
    pub height: f64,
}

/// Lit un dessin. Une image est lue en entier ; un SVG est rendu de façon
/// que son plus grand côté mesure `svg_side` pixels.
pub fn decode(path: &Path, svg_side: u32) -> Result<DecodedDrawing, LibraryError> {
    let unreadable = |reason: String| LibraryError::Unreadable {
        path: path.to_path_buf(),
        reason,
    };
    match DrawingFormat::of(path) {
        None => Err(unreadable(
            "ce n'est pas un format de dessin (PNG, JPEG, TIFF, BMP, PSD ou SVG)".into(),
        )),
        Some(DrawingFormat::Image) => {
            let raster = tiles::read_raster(path).map_err(|e| unreadable(e.to_string()))?;
            let (width, height) = (f64::from(raster.width), f64::from(raster.height));
            Ok(DecodedDrawing {
                raster,
                width,
                height,
            })
        }
        Some(DrawingFormat::Psd) => {
            let data = fs::read(path).map_err(|e| unreadable(e.to_string()))?;
            let raster = decode_psd(&data).map_err(unreadable)?;
            let (width, height) = (f64::from(raster.width), f64::from(raster.height));
            Ok(DecodedDrawing {
                raster,
                width,
                height,
            })
        }
        Some(DrawingFormat::Svg) => {
            let data = fs::read(path).map_err(|e| unreadable(e.to_string()))?;
            render_svg(&data, path.parent(), svg_side).map_err(unreadable)
        }
    }
}

/// Rend un SVG sur fond noir, son plus grand côté à `side` pixels.
pub fn render_svg(
    data: &[u8],
    resources_dir: Option<&Path>,
    side: u32,
) -> Result<DecodedDrawing, String> {
    let options = Options {
        resources_dir: resources_dir.map(Path::to_path_buf),
        fontdb: crate::facade::svg::system_fonts(),
        ..Options::default()
    };
    let tree = Tree::from_data(data, &options).map_err(|e| format!("SVG illisible : {e}"))?;
    let size = tree.size();
    let (width, height) = (f64::from(size.width()), f64::from(size.height()));
    let scale = f64::from(side.max(1)) / width.max(height);
    let pw = ((width * scale).round() as u32).max(1);
    let ph = ((height * scale).round() as u32).max(1);
    let mut pixmap = Pixmap::new(pw, ph).ok_or("SVG trop grand pour être rendu")?;
    resvg::render(
        &tree,
        Transform::from_scale(scale as f32, scale as f32),
        &mut pixmap.as_mut(),
    );
    // Pixels prémultipliés : c'est exactement l'image posée sur du noir.
    let rgba = pixmap.take();
    let rgb = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, _]| [r, g, b])
        .collect();
    Ok(DecodedDrawing {
        raster: Raster::new(pw, ph, 3, rgb),
        width,
        height,
    })
}

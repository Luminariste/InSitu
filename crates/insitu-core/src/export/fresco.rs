//! Fresque vectorielle : PDF, fichier Illustrator (`.ai`) ou SVG.
//!
//! Un `.ai` enregistré par Illustrator avec « Créer un fichier compatible
//! PDF » (réglage par défaut) est un PDF : il est lu comme tel, par `hayro`.
//! Un SVG est lu par `resvg`. Les deux sont en Rust pur.
//!
//! Le fichier d'origine est gardé dans le projet. À l'import, il est rendu
//! une fois en niveaux de gris ([`DISPLAY_SIDE_PX`] au plus grand côté) pour
//! l'affichage et les aperçus. À l'export, il est relu pour chaque
//! diapositive, sur la seule partie de façade qu'elle couvre, à la
//! résolution de la diapositive : les traits restent nets quelle que soit la
//! taille de la façade (décision D-3.5-i).
//!
//! Ce que rien ne couvre est noir : pas de lumière (décision D-3.5-j).

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use hayro::hayro_interpret::util::TransformExt;
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::kurbo::Affine;
use hayro::vello_cpu::color::palette::css::BLACK;
use hayro::vello_cpu::{Pixmap, RasterizerSettings, RenderContext, Resources, TargetInit};
use hayro::{render_into, RenderCache, RenderSettings};
use resvg::tiny_skia;
use resvg::usvg::{Options, Tree};

use super::ExportError;
use crate::geometry::{Point, PointWarp, Transform};
use crate::project::model::RectMm;
use crate::tiles::Raster;

/// Plus grand côté, en pixels, du rendu gardé pour l'affichage.
pub const DISPLAY_SIDE_PX: u32 = 8_192;

/// Plus grand côté, en pixels, de la partie de fresque rendue pour une
/// diapositive (576 Mo en niveaux de gris au pire). Au-delà, la résolution
/// baisse.
pub const MAX_CROP_SIDE_PX: u32 = 24_000;

/// Hauteur des bandes rendues l'une après l'autre (une bande RVBA de
/// 24 000 px de large pèse 25 Mo).
const BAND_ROWS: u32 = 256;

/// Nature d'un fichier de fresque, reconnue à son contenu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrescoFormat {
    /// Image : PNG, TIFF, JPEG, BMP.
    Raster,
    /// PDF, ou `.ai` compatible PDF.
    Pdf,
    Svg,
}

/// Reconnaît le format au contenu du fichier, puis à son extension.
pub fn format_of(path: &Path) -> Result<FrescoFormat, ExportError> {
    let unreadable = |e: std::io::Error| ExportError::Image(format!("{} : {e}", path.display()));
    let mut head = vec![0u8; 1024];
    let n = {
        use std::io::Read;
        let mut file = std::fs::File::open(path).map_err(unreadable)?;
        let mut n = 0;
        loop {
            let k = file.read(&mut head[n..]).map_err(unreadable)?;
            if k == 0 {
                break;
            }
            n += k;
            if n == head.len() {
                break;
            }
        }
        n
    };
    let head = &head[..n];
    let contains = |needle: &[u8]| head.windows(needle.len()).any(|w| w == needle);
    if contains(b"%PDF-") {
        return Ok(FrescoFormat::Pdf);
    }
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if head.starts_with(b"%!PS") {
        return Err(ExportError::Image(format!(
            "{} : fichier Illustrator ou PostScript sans contenu PDF. Dans Illustrator, \
             « Enregistrer sous… » en .ai en cochant « Créer un fichier compatible PDF », \
             ou enregistrer en PDF",
            path.display()
        )));
    }
    match extension.as_str() {
        "ai" | "pdf" => Err(ExportError::Image(format!(
            "{} : ce fichier n'est pas un PDF lisible. Pour un .ai, cocher « Créer un fichier \
             compatible PDF » à l'enregistrement dans Illustrator",
            path.display()
        ))),
        "svg" => Ok(FrescoFormat::Svg),
        _ if contains(b"<svg") => Ok(FrescoFormat::Svg),
        _ => Ok(FrescoFormat::Raster),
    }
}

enum Source {
    Pdf {
        data: Arc<Vec<u8>>,
        page_index: usize,
    },
    Svg(Box<Tree>),
}

/// Une fresque vectorielle ouverte, prête à être rendue par morceaux.
pub struct VectorFresco {
    source: Source,
    /// Taille de la page dans l'unité du fichier : points PDF (1/72 de
    /// pouce) ou pixels CSS du SVG (1/96 de pouce).
    width: f64,
    height: f64,
}

fn load_pdf(data: Arc<Vec<u8>>) -> Result<Pdf, ExportError> {
    Pdf::new(data).map_err(|error| {
        ExportError::Image(format!(
            "PDF illisible ({error:?}) ; un PDF protégé par mot de passe n'est pas pris en charge"
        ))
    })
}

impl VectorFresco {
    /// Ouvre un PDF, un `.ai` ou un SVG. `page` compte à partir de 1 et ne
    /// sert qu'au PDF.
    pub fn open(path: &Path, page: u32) -> Result<Self, ExportError> {
        let data = std::fs::read(path)
            .map_err(|e| ExportError::Image(format!("{} : {e}", path.display())))?;
        match format_of(path)? {
            FrescoFormat::Pdf => {
                let data = Arc::new(data);
                let pdf = load_pdf(data.clone())?;
                let count = pdf.pages().len();
                if count == 0 {
                    return Err(ExportError::Image("le PDF ne contient aucune page".into()));
                }
                if page == 0 || page as usize > count {
                    return Err(ExportError::BadSettings(format!(
                        "page {page} demandée, le fichier en a {count}"
                    )));
                }
                let page_index = page as usize - 1;
                let (w, h) = pdf.pages()[page_index].render_dimensions();
                drop(pdf);
                Self::checked(Source::Pdf { data, page_index }, f64::from(w), f64::from(h))
            }
            FrescoFormat::Svg => {
                let options = Options {
                    resources_dir: path.parent().map(Path::to_path_buf),
                    fontdb: crate::facade::svg::system_fonts(),
                    ..Options::default()
                };
                let tree = Tree::from_data(&data, &options)
                    .map_err(|e| ExportError::Image(format!("SVG illisible : {e}")))?;
                let size = tree.size();
                let (w, h) = (f64::from(size.width()), f64::from(size.height()));
                Self::checked(Source::Svg(Box::new(tree)), w, h)
            }
            FrescoFormat::Raster => Err(ExportError::Image(format!(
                "{} : ni PDF, ni Illustrator, ni SVG",
                path.display()
            ))),
        }
    }

    fn checked(source: Source, width: f64, height: f64) -> Result<Self, ExportError> {
        if !(width > 0.0 && height > 0.0 && width.is_finite() && height.is_finite()) {
            return Err(ExportError::Image("page vide".into()));
        }
        Ok(Self {
            source,
            width,
            height,
        })
    }

    /// Taille de la page, dans l'unité du fichier.
    pub fn size(&self) -> (f64, f64) {
        (self.width, self.height)
    }

    /// Rend la page entière en niveaux de gris, `long_side` pixels au plus
    /// grand côté.
    pub fn render_whole(&self, long_side: u32, threads: usize) -> Raster {
        let scale = f64::from(long_side) / self.width.max(self.height);
        let w = ((self.width * scale).round() as u32).max(1);
        let h = ((self.height * scale).round() as u32).max(1);
        self.render_gray(scale, scale, 0.0, 0.0, w, h, threads)
    }

    /// Rend un rectangle de `width` × `height` pixels en niveaux de gris,
    /// sur du noir. Un point (u, v) du fichier tombe au pixel
    /// (u · sx − ox, v · sy − oy) ; le pixel (i, j) couvre [i, i + 1) ×
    /// [j, j + 1). Rendu par bandes, sur `threads` fils (0 : tous les cœurs).
    #[allow(clippy::too_many_arguments)]
    pub fn render_gray(
        &self,
        sx: f64,
        sy: f64,
        ox: f64,
        oy: f64,
        width: u32,
        height: u32,
        threads: usize,
    ) -> Raster {
        let mut data = vec![0u8; width as usize * height as usize];
        let bands: Vec<(u32, &mut [u8])> = data
            .chunks_mut(width as usize * BAND_ROWS as usize)
            .enumerate()
            .map(|(k, chunk)| (k as u32 * BAND_ROWS, chunk))
            .collect();
        let count = bands.len();
        let bands = Mutex::new(bands.into_iter().map(Some).collect::<Vec<_>>());
        let next = AtomicUsize::new(0);
        let threads = if threads == 0 {
            std::thread::available_parallelism().map_or(1, |n| n.get())
        } else {
            threads
        }
        .clamp(1, count.max(1));
        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| {
                    let take = || {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        if k >= count {
                            return None;
                        }
                        bands.lock().expect("bandes")[k].take()
                    };
                    match &self.source {
                        Source::Pdf { data, page_index } => {
                            let Ok(pdf) = load_pdf(data.clone()) else {
                                return;
                            };
                            let Some(page) = pdf.pages().get(*page_index) else {
                                return;
                            };
                            let cache = RenderCache::new();
                            let base = page.initial_transform(true).to_kurbo();
                            while let Some((top, out)) = take() {
                                let rows = (out.len() / width as usize) as u32;
                                let transform = Affine::translate((-ox, -oy - f64::from(top)))
                                    * Affine::scale_non_uniform(sx, sy)
                                    * base;
                                let (w, h) = (width as u16, rows as u16);
                                let mut ctx = RenderContext::new(w, h);
                                render_into(
                                    page,
                                    &cache,
                                    &InterpreterSettings::default(),
                                    &RenderSettings::default(),
                                    &mut ctx,
                                    transform,
                                );
                                ctx.flush();
                                let mut pixmap = Pixmap::new(w, h);
                                ctx.render_with(
                                    &mut pixmap,
                                    &mut Resources::default(),
                                    RasterizerSettings {
                                        target_init: TargetInit::Clear(BLACK),
                                        ..Default::default()
                                    },
                                );
                                to_gray_into(pixmap.data_as_u8_slice(), out);
                            }
                        }
                        Source::Svg(tree) => {
                            while let Some((top, out)) = take() {
                                let rows = (out.len() / width as usize) as u32;
                                let Some(mut pixmap) = tiny_skia::Pixmap::new(width, rows) else {
                                    continue;
                                };
                                pixmap.fill(tiny_skia::Color::BLACK);
                                let transform = tiny_skia::Transform::from_row(
                                    sx as f32,
                                    0.0,
                                    0.0,
                                    sy as f32,
                                    -ox as f32,
                                    (-oy - f64::from(top)) as f32,
                                );
                                resvg::render(tree, transform, &mut pixmap.as_mut());
                                to_gray_into(pixmap.data(), out);
                            }
                        }
                    }
                });
            }
        });
        Raster::new(width, height, 1, data)
    }

    /// La partie de la fresque qu'une diapositive couvre, rendue à sa
    /// résolution, et le rectangle de façade qu'elle occupe ; `None` si la
    /// diapositive ne touche pas la fresque.
    ///
    /// `frame` : rectangle de la fresque sur la façade ; `warp` : diapositive
    /// (mm) → façade (mm) ; `px_mm` : taille d'un pixel de diapositive.
    pub fn crop_for_slide(
        &self,
        frame: &RectMm,
        warp: &PointWarp,
        px_mm: f64,
        threads: usize,
    ) -> Option<(Raster, RectMm)> {
        let (bounds, finest) = footprint(warp, px_mm)?;
        // Un pixel de fresque par pixel de diapositive, là où la diapositive
        // est la plus fine sur la façade.
        let mut r = finest;
        let margin = 4.0 * r;
        let x0 = (bounds.0 - margin).max(frame.x);
        let y0 = (bounds.1 - margin).max(frame.y);
        let x1 = (bounds.2 + margin).min(frame.x + frame.width);
        let y1 = (bounds.3 + margin).min(frame.y + frame.height);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        let longest = (x1 - x0).max(y1 - y0);
        if longest / r > f64::from(MAX_CROP_SIDE_PX) {
            r = longest / f64::from(MAX_CROP_SIDE_PX);
        }
        let nx = ((x1 - x0) / r).ceil().max(1.0) as u32;
        let ny = ((y1 - y0) / r).ceil().max(1.0) as u32;
        // Fichier → façade : u ↦ frame.x + u · frame.width / largeur.
        let (kx, ky) = (frame.width / self.width, frame.height / self.height);
        let raster = self.render_gray(
            kx / r,
            ky / r,
            (x0 - frame.x) / r,
            (y0 - frame.y) / r,
            nx,
            ny,
            threads,
        );
        let sub = RectMm {
            x: x0,
            y: y0,
            width: f64::from(nx) * r,
            height: f64::from(ny) * r,
        };
        Some((raster, sub))
    }
}

/// Rectangle de façade couvert par la diapositive (x0, y0, x1, y1), et la
/// plus petite longueur de façade, en mm, que couvre un pixel de diapositive.
fn footprint(warp: &PointWarp, px_mm: f64) -> Option<((f64, f64, f64, f64), f64)> {
    const N: usize = 32;
    let side = crate::units::SLIDE_SIZE_MM;
    let step = side / N as f64;
    let h = (px_mm * 4.0).min(step / 2.0);
    let mut bounds = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    let mut finest = f64::INFINITY;
    for j in 0..=N {
        for i in 0..=N {
            let s = Point::new(i as f64 * step, j as f64 * step);
            let Some(f) = warp.apply(s) else { continue };
            bounds.0 = bounds.0.min(f.x);
            bounds.1 = bounds.1.min(f.y);
            bounds.2 = bounds.2.max(f.x);
            bounds.3 = bounds.3.max(f.y);
            // Différences finies vers l'intérieur de la diapositive.
            let dx = if i < N { h } else { -h };
            let dy = if j < N { h } else { -h };
            for d in [Point::new(dx, 0.0), Point::new(0.0, dy)] {
                if let Some(g) = warp.apply(Point::new(s.x + d.x, s.y + d.y)) {
                    let stretch = (g.x - f.x).hypot(g.y - f.y) / h;
                    finest = finest.min(stretch * px_mm);
                }
            }
        }
    }
    (bounds.0.is_finite() && finest.is_finite() && finest > 0.0).then_some((bounds, finest))
}

/// RVBA opaque → gris (Rec. 601, comme le reste de l'export).
fn to_gray_into(rgba: &[u8], out: &mut [u8]) {
    for (v, px) in out.iter_mut().zip(rgba.chunks_exact(4)) {
        let (r, g, b) = (u32::from(px[0]), u32::from(px[1]), u32::from(px[2]));
        *v = ((299 * r + 587 * g + 114 * b + 500) / 1000) as u8;
    }
}

//! La façade : import du plan (PDF, image, SVG, DXF), affichage net à tout
//! zoom, et mise à l'échelle par deux points et une cote (tâche 3.1).
//!
//! - [`inspect`] lit un fichier de plan et donne sa nature et ses
//!   dimensions dans son unité ;
//! - [`import`] le copie dans le projet et enregistre la mise à l'échelle ;
//! - [`display`] fabrique sa pyramide de tuiles pour la visionneuse. Pour un
//!   plan vectoriel, chaque niveau est rendu directement depuis les
//!   vecteurs, et les niveaux fins ne sont rendus qu'à la demande : le trait
//!   reste net à tout zoom sans fabriquer des milliers de tuiles d'avance.
//!
//! Unité du fichier (« unité source ») selon le format :
//!
//! | Format | Unité | Origine |
//! | --- | --- | --- |
//! | Image (PNG, JPEG, TIFF, HEIC) | pixel | coin haut-gauche |
//! | PDF | point (1/72 de pouce) | coin haut-gauche de la page affichée |
//! | SVG | pixel CSS (1/96 de pouce) | coin haut-gauche |
//! | DXF | unité de dessin | coin haut-gauche du dessin (voir [`dxf`]) |
//!
//! Le repère de la façade en millimètres est celui du fichier multiplié par
//! l'échelle ([`crate::project::FacadeScale`]).

pub mod display;
pub mod dxf;
pub mod import;
pub mod pdf;
pub mod svg;

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::project::ProjectError;
use crate::tiles::TileError;

pub use display::{open_for_display, open_project_facade, read_or_render, FacadeDisplay};
pub use import::{import_facade, set_scale};

/// Côté, en pixels, du niveau le plus fin d'un plan vectoriel. Sur un plan
/// A0 en PDF, c'est 19 pixels par point, soit environ 1 400 dpi sur le
/// papier ; sur une façade de 30 m en DXF, un pixel par demi-millimètre.
pub const VECTOR_LEVEL0_SIDE: u32 = 65_536;

/// Les niveaux d'un plan vectoriel qui ne dépassent pas ce nombre de pixels
/// sont rendus dès l'ouverture (environ 85 tuiles) ; les plus fins le sont à
/// la demande, tuile par tuile, puis gardés dans le cache.
pub const VECTOR_EAGER_PIXELS: u64 = 4096 * 4096;

/// Nature d'un fichier de plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Image,
    Pdf,
    Svg,
    Dxf,
}

impl SourceKind {
    /// Nature d'après l'extension du fichier.
    pub fn of(path: &Path) -> Result<Self, FacadeError> {
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        match extension.as_str() {
            "pdf" => Ok(Self::Pdf),
            "svg" | "svgz" => Ok(Self::Svg),
            "dxf" => Ok(Self::Dxf),
            "png" | "jpg" | "jpeg" | "tif" | "tiff" | "heic" | "heif" => Ok(Self::Image),
            _ => Err(FacadeError::Unsupported(extension)),
        }
    }

    /// Nom de l'unité du fichier, pour l'affichage.
    pub fn unit(self) -> &'static str {
        match self {
            Self::Image => "px",
            Self::Pdf => "pt",
            Self::Svg => "px CSS",
            Self::Dxf => "unité de dessin",
        }
    }

    pub fn is_vector(self) -> bool {
        !matches!(self, Self::Image)
    }
}

/// Ce que le moteur sait d'un fichier de plan avant de l'importer.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub kind: SourceKind,
    /// Dimensions dans l'unité du fichier.
    pub width: f64,
    pub height: f64,
    pub unit: &'static str,
    /// PDF : nombre de pages, et page retenue (à partir de 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// DXF : coin haut-gauche du repère du fichier, en coordonnées du DXF.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<[f64; 2]>,
    /// DXF : millimètres par unité de dessin, si le fichier les déclare
    /// (`$INSUNITS`). Indication seulement : la mise à l'échelle reste à
    /// faire par deux points et une cote.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared_mm_per_unit: Option<f64>,
}

#[derive(Debug)]
pub enum FacadeError {
    /// Extension non prise en charge (l'extension est donnée en minuscules).
    Unsupported(String),
    /// Fichier reconnu mais illisible.
    Unreadable(String),
    /// Page demandée hors du PDF.
    BadPage {
        page: u32,
        page_count: u32,
    },
    Io(PathBuf, io::Error),
    Project(ProjectError),
    Tiles(TileError),
    /// Le projet n'a pas encore de façade.
    NoFacade,
    /// Mise à l'échelle impossible ; la raison complète la phrase.
    BadScale(&'static str),
}

impl fmt::Display for FacadeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(extension) => match extension.as_str() {
                "dwg" => f.write_str(
                    "Le format DWG n'est pas lu : l'exporter en PDF ou en DXF depuis le logiciel de l'architecte.",
                ),
                "ai" => f.write_str(
                    "Un fichier Illustrator n'est pas lu comme plan : l'enregistrer en PDF.",
                ),
                "" => f.write_str("Fichier sans extension : format inconnu."),
                other => write!(
                    f,
                    "Format « .{other} » non pris en charge pour la façade (PDF, PNG, JPEG, TIFF, HEIC, SVG ou DXF)."
                ),
            },
            Self::Unreadable(message) => write!(f, "Plan illisible : {message}"),
            Self::BadPage { page, page_count } => write!(
                f,
                "Le PDF a {page_count} page(s) : la page {page} n'existe pas."
            ),
            Self::Io(path, error) => write!(f, "Impossible de lire « {} » : {error}", path.display()),
            Self::Project(error) => error.fmt(f),
            Self::Tiles(error) => write!(f, "Préparation de l'affichage impossible : {error}"),
            Self::NoFacade => f.write_str("Le projet n'a pas encore de façade : importer d'abord un plan."),
            Self::BadScale(reason) => write!(f, "Mise à l'échelle impossible : {reason}."),
        }
    }
}

impl std::error::Error for FacadeError {}

impl From<ProjectError> for FacadeError {
    fn from(error: ProjectError) -> Self {
        Self::Project(error)
    }
}

impl From<TileError> for FacadeError {
    fn from(error: TileError) -> Self {
        Self::Tiles(error)
    }
}

/// Un plan vectoriel qu'on sait rendre par morceaux.
pub trait VectorPlan: Send + Sync {
    /// Dimensions dans l'unité du fichier.
    fn size(&self) -> (f64, f64);

    /// Rend un rectangle de `width` × `height` pixels sur fond blanc.
    ///
    /// Un point (u, v) du fichier tombe au pixel (u · scale − offset_x,
    /// v · scale − offset_y) ; le pixel (i, j) couvre [i, i + 1) × [j, j + 1).
    /// Rend des pixels RVBA de 8 bits, opaques, ligne par ligne.
    fn render(&self, scale: f64, offset_x: f64, offset_y: f64, width: u32, height: u32) -> Vec<u8>;
}

/// Ouvre un plan vectoriel ; `page` (à partir de 1) ne sert qu'au PDF.
pub fn open_vector(
    path: &Path,
    kind: SourceKind,
    page: Option<u32>,
) -> Result<Box<dyn VectorPlan>, FacadeError> {
    Ok(match kind {
        SourceKind::Pdf => Box::new(pdf::PdfPlan::open(read(path)?, page.unwrap_or(1))?.0),
        SourceKind::Svg => Box::new(svg::SvgPlan::open(&read(path)?, path.parent())?),
        SourceKind::Dxf => Box::new(dxf::DxfPlan::open(path)?),
        SourceKind::Image => {
            return Err(FacadeError::Unreadable(
                "une image n'est pas un plan vectoriel".into(),
            ))
        }
    })
}

fn read(path: &Path) -> Result<Vec<u8>, FacadeError> {
    std::fs::read(path).map_err(|error| FacadeError::Io(path.to_path_buf(), error))
}

/// Lit un fichier de plan : nature, dimensions dans son unité, pages.
pub fn inspect(path: &Path, page: Option<u32>) -> Result<SourceInfo, FacadeError> {
    let kind = SourceKind::of(path)?;
    let mut info = SourceInfo {
        kind,
        width: 0.0,
        height: 0.0,
        unit: kind.unit(),
        page_count: None,
        page: None,
        origin: None,
        declared_mm_per_unit: None,
    };
    match kind {
        SourceKind::Image => {
            // Le HEIC des iPhone compris (photo d'ensemble de la façade).
            let (width, height) = crate::photo::dimensions(path)
                .map_err(|error| FacadeError::Unreadable(format!("image illisible : {error}")))?;
            info.width = f64::from(width);
            info.height = f64::from(height);
        }
        SourceKind::Pdf => {
            let page = page.unwrap_or(1);
            let (_, pdf) = pdf::PdfPlan::open(read(path)?, page)?;
            info.width = pdf.width;
            info.height = pdf.height;
            info.page_count = Some(pdf.page_count);
            info.page = Some(page);
        }
        SourceKind::Svg => {
            let plan = svg::SvgPlan::open(&read(path)?, path.parent())?;
            (info.width, info.height) = plan.size();
        }
        SourceKind::Dxf => {
            let plan = dxf::DxfPlan::open(path)?;
            (info.width, info.height) = plan.size();
            info.origin = Some(plan.origin());
            info.declared_mm_per_unit = plan.mm_per_unit();
        }
    }
    if !(info.width > 0.0 && info.height > 0.0 && info.width.is_finite() && info.height.is_finite())
    {
        return Err(FacadeError::Unreadable("dimensions nulles".into()));
    }
    Ok(info)
}

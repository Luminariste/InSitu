//! Plans PDF : lecture et rendu d'une page, par `hayro` (Rust pur).
//!
//! Unité du fichier : le point PDF (1/72 de pouce), sur la page telle
//! qu'elle s'affiche (rotation `/Rotate` et zone de recadrage appliquées),
//! origine en haut à gauche, y vers le bas.

use std::sync::Arc;

use hayro::hayro_interpret::util::TransformExt;
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::kurbo::Affine;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::vello_cpu::{Pixmap, RasterizerSettings, RenderContext, Resources, TargetInit};
use hayro::{render_into, RenderCache, RenderSettings};

use super::{FacadeError, VectorPlan};

/// Une page d'un PDF, prête à être rendue par morceaux.
pub struct PdfPlan {
    /// Contenu du fichier : chaque rendu relit le document à partir d'ici
    /// (les objets de `hayro` ne passent pas d'un fil d'exécution à l'autre).
    data: Arc<Vec<u8>>,
    /// Index de la page, à partir de 0.
    page_index: usize,
    width: f64,
    height: f64,
}

/// Nombre de pages et dimensions (en points) de la page demandée.
pub struct PdfInfo {
    pub page_count: u32,
    pub width: f64,
    pub height: f64,
}

fn load(data: Arc<Vec<u8>>) -> Result<Pdf, FacadeError> {
    Pdf::new(data).map_err(|error| {
        FacadeError::Unreadable(format!(
            "PDF illisible ({error:?}) ; un PDF protégé par mot de passe n'est pas pris en charge"
        ))
    })
}

impl PdfPlan {
    /// `page` compte à partir de 1, comme dans un lecteur de PDF.
    pub fn open(data: Vec<u8>, page: u32) -> Result<(Self, PdfInfo), FacadeError> {
        let data = Arc::new(data);
        let pdf = load(data.clone())?;
        let page_count = pdf.pages().len() as u32;
        if page_count == 0 {
            return Err(FacadeError::Unreadable(
                "le PDF ne contient aucune page".into(),
            ));
        }
        if page == 0 || page > page_count {
            return Err(FacadeError::BadPage { page, page_count });
        }
        let page_index = (page - 1) as usize;
        let (width, height) = pdf.pages()[page_index].render_dimensions();
        let (width, height) = (f64::from(width), f64::from(height));
        drop(pdf);
        let plan = Self {
            data,
            page_index,
            width,
            height,
        };
        Ok((
            plan,
            PdfInfo {
                page_count,
                width,
                height,
            },
        ))
    }
}

impl VectorPlan for PdfPlan {
    fn size(&self) -> (f64, f64) {
        (self.width, self.height)
    }

    fn render(&self, scale: f64, offset_x: f64, offset_y: f64, width: u32, height: u32) -> Vec<u8> {
        let mut out = vec![255u8; width as usize * height as usize * 4];
        let Ok(pdf) = load(self.data.clone()) else {
            return out;
        };
        let Some(page) = pdf.pages().get(self.page_index) else {
            return out;
        };
        let (w, h) = (width as u16, height as u16);
        let mut ctx = RenderContext::new(w, h);
        let transform = Affine::translate((-offset_x, -offset_y))
            * Affine::scale(scale)
            * page.initial_transform(true).to_kurbo();
        let cache = RenderCache::new();
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
                target_init: TargetInit::Clear(WHITE),
                ..Default::default()
            },
        );
        out.copy_from_slice(pixmap.data_as_u8_slice());
        out
    }
}

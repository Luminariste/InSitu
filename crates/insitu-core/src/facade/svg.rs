//! Plans SVG : lecture par `usvg`, rendu par `resvg` (Rust pur).
//!
//! Unité du fichier : l'unité « utilisateur » du SVG, c'est-à-dire le pixel
//! CSS (1/96 de pouce) de sa taille déclarée (`width`, `height`), origine en
//! haut à gauche, y vers le bas. Les textes sont changés en tracés à la
//! lecture, avec les polices installées sur la machine.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use resvg::tiny_skia::{Color, Pixmap, Transform};
use resvg::usvg::{fontdb, Options, Tree};

use super::{FacadeError, VectorPlan};

pub struct SvgPlan {
    tree: Tree,
}

/// Polices de la machine, chargées une seule fois par lancement.
fn system_fonts() -> Arc<fontdb::Database> {
    static FONTS: OnceLock<Arc<fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut database = fontdb::Database::new();
            database.load_system_fonts();
            Arc::new(database)
        })
        .clone()
}

impl SvgPlan {
    /// `resources_dir` : dossier où chercher les images liées par le SVG.
    pub fn open(data: &[u8], resources_dir: Option<&Path>) -> Result<Self, FacadeError> {
        let options = Options {
            resources_dir: resources_dir.map(Path::to_path_buf),
            fontdb: system_fonts(),
            ..Options::default()
        };
        let tree = Tree::from_data(data, &options)
            .map_err(|error| FacadeError::Unreadable(format!("SVG illisible : {error}")))?;
        Ok(Self { tree })
    }
}

impl VectorPlan for SvgPlan {
    fn size(&self) -> (f64, f64) {
        let size = self.tree.size();
        (f64::from(size.width()), f64::from(size.height()))
    }

    fn render(&self, scale: f64, offset_x: f64, offset_y: f64, width: u32, height: u32) -> Vec<u8> {
        let Some(mut pixmap) = Pixmap::new(width, height) else {
            return vec![255; width as usize * height as usize * 4];
        };
        pixmap.fill(Color::WHITE);
        let transform = Transform::from_row(
            scale as f32,
            0.0,
            0.0,
            scale as f32,
            -offset_x as f32,
            -offset_y as f32,
        );
        resvg::render(&self.tree, transform, &mut pixmap.as_mut());
        pixmap.take()
    }
}

//! Moteur de calcul d'InSitu.
//!
//! Ce crate ne dépend d'aucune interface : il se teste en ligne de commande,
//! y compris dans une session cloud. Tout ce qui est calcul (géométrie,
//! détection de mire, déformation, vectorisation, exports pleine résolution)
//! vit ici. La coque Tauri (`src-tauri`) ne fait que l'appeler.
//!
//! Conventions :
//! - les longueurs physiques sont en millimètres (`f64`) ;
//! - l'origine d'une diapositive est son coin haut-gauche, x vers la droite,
//!   y vers le bas ;
//! - les pixels n'apparaissent qu'à l'entrée (images sources) et à la sortie
//!   (exports), jamais dans le modèle.

pub mod facade;
pub mod geometry;
pub mod library;
pub mod photo;
pub mod project;
pub mod rectification;
pub mod registration;
pub mod resample;
pub mod target;
pub mod tiles;
pub mod units;
pub mod vectorize;

use serde::Serialize;

/// Carte d'identité du moteur, affichée par l'interface et par `insitu info`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub name: &'static str,
    pub version: &'static str,
    /// Côté de l'image d'une diapositive, en millimètres.
    pub slide_size_mm: f64,
    /// Côté de la plaque gravée, en millimètres.
    pub plate_size_mm: f64,
    /// Résolution d'export par défaut, en points par pouce.
    pub default_export_dpi: f64,
    /// Côté de l'export par défaut, en pixels.
    pub default_export_px: u32,
}

pub fn engine_info() -> EngineInfo {
    EngineInfo {
        name: "InSitu",
        version: env!("CARGO_PKG_VERSION"),
        slide_size_mm: units::SLIDE_SIZE_MM,
        plate_size_mm: units::PLATE_SIZE_MM,
        default_export_dpi: units::DEFAULT_EXPORT_DPI,
        default_export_px: units::mm_to_px(units::SLIDE_SIZE_MM, units::DEFAULT_EXPORT_DPI),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_info_matches_the_workflow_numbers() {
        let info = engine_info();
        assert_eq!(info.slide_size_mm, 280.0);
        assert_eq!(info.plate_size_mm, 300.0);
        assert_eq!(info.default_export_px, 15_433);
    }
}

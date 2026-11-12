//! Unités et dimensions de référence du workflow.

/// Côté de l'image projetée par une diapositive (et de la mire), en mm.
pub const SLIDE_SIZE_MM: f64 = 280.0;

/// Côté de la plaque de verre gravée, en mm. La marge de 10 mm autour de
/// l'image reçoit le nom de la diapositive et les repères de calage.
pub const PLATE_SIZE_MM: f64 = 300.0;

/// Résolution d'export par défaut (décision du 11 novembre 2026).
pub const DEFAULT_EXPORT_DPI: f64 = 1400.0;

/// Miroir à l'export, par défaut : aucun (le projet d'Épinal a été gravé sans
/// miroir, voir `jeu-essai/LISEZMOI.md`).
pub const DEFAULT_EXPORT_MIRROR: bool = false;

/// Négatif à l'export, par défaut : non. L'image exportée est le dessin blanc
/// sur fond noir ; jusqu'ici c'est LightBurn qui applique le négatif.
pub const DEFAULT_EXPORT_NEGATIVE: bool = false;

/// Marge à l'export, par défaut : non. Le PNG fait 280 mm, l'image seule
/// (choix de Benjamin du 12 novembre 2026, D-3.5-a). Avec la marge, il
/// couvre la plaque de 300 mm, nom et repères gravés dans la marge.
pub const DEFAULT_EXPORT_MARGIN: bool = false;

/// Seuil du noir et blanc à l'export (PNG 1 bit), par défaut : le milieu.
pub const DEFAULT_EXPORT_THRESHOLD: u8 = 128;

/// Largeur de la marge de la plaque autour de l'image, en mm.
pub const PLATE_MARGIN_MM: f64 = (PLATE_SIZE_MM - SLIDE_SIZE_MM) / 2.0;

/// Résolution des photos redressées produites jusqu'ici (A.tiff : 5512 px).
pub const RECTIFIED_PHOTO_DPI: f64 = 500.0;

pub const MM_PER_INCH: f64 = 25.4;

/// Nombre de pixels couvrant `mm` millimètres à `dpi` points par pouce,
/// arrondi au pixel le plus proche.
pub fn mm_to_px(mm: f64, dpi: f64) -> u32 {
    (mm / MM_PER_INCH * dpi).round() as u32
}

/// Taille d'un pixel en millimètres à `dpi` points par pouce.
pub fn px_size_mm(dpi: f64) -> f64 {
    MM_PER_INCH / dpi
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_export_is_15433_px_at_1400_dpi() {
        assert_eq!(mm_to_px(SLIDE_SIZE_MM, DEFAULT_EXPORT_DPI), 15_433);
    }

    #[test]
    fn rectified_photo_is_5512_px_at_500_dpi() {
        assert_eq!(mm_to_px(SLIDE_SIZE_MM, RECTIFIED_PHOTO_DPI), 5_512);
    }

    #[test]
    fn pixel_is_about_18_microns_at_1400_dpi() {
        let px = px_size_mm(DEFAULT_EXPORT_DPI);
        assert!((px - 0.018_142_857).abs() < 1e-6);
    }
}

//! Nom et repères de calage gravés dans la marge de la plaque (tâche 3.5).
//!
//! Tout est dessiné en blanc (gravé) sur l'image en niveaux de gris de la
//! plaque, dans la marge de 10 mm, sans jamais toucher le carré de 280 mm :
//!
//! - aux quatre coins, une équerre qui prolonge les bords de l'image ;
//! - au milieu de chaque côté, un trait perpendiculaire, pour aligner la
//!   plaque sur le projecteur ;
//! - dans la marge du bas, le nom du projet et de la diapositive et les
//!   réglages d'export, en capitales d'une police intégrée (5 × 7 points,
//!   sans accents), pour ne dépendre d'aucune police du système.

use crate::tiles::Raster;
use crate::units::{PLATE_MARGIN_MM, SLIDE_SIZE_MM};

/// Épaisseur des traits, en mm.
pub const STROKE_MM: f64 = 0.25;
/// Les repères commencent à cette distance du carré de l'image, en mm.
const GAP_MM: f64 = 1.0;
/// Longueur des repères, en mm (ils s'arrêtent 1 mm avant le bord du verre).
const MARK_MM: f64 = PLATE_MARGIN_MM - GAP_MM - 1.0;
/// Hauteur des capitales, en mm.
pub const TEXT_HEIGHT_MM: f64 = 3.5;
/// Le texte commence à cette distance du bord gauche de la plaque, en mm.
const TEXT_LEFT_MM: f64 = PLATE_MARGIN_MM + 10.0;

/// Dessine les repères et `text` sur `plate` (image en gris de la plaque
/// entière, `px_mm` mm par pixel).
pub fn draw(plate: &mut Raster, px_mm: f64, text: &str) {
    assert_eq!(plate.channels, 1);
    let m = PLATE_MARGIN_MM;
    let s = SLIDE_SIZE_MM;
    let w = STROKE_MM;
    let (near, far) = (GAP_MM, GAP_MM + MARK_MM);
    // Équerres : aux coins (x, y), deux traits qui prolongent les bords.
    for (cx, dx) in [(m, -1.0), (m + s, 1.0)] {
        for (cy, dy) in [(m, -1.0), (m + s, 1.0)] {
            // Prolongement du bord vertical (vers le haut ou le bas).
            fill_mm(
                plate,
                px_mm,
                cx - w / 2.0,
                cy + dy * near,
                cx + w / 2.0,
                cy + dy * far,
            );
            // Prolongement du bord horizontal (vers la gauche ou la droite).
            fill_mm(
                plate,
                px_mm,
                cx + dx * near,
                cy - w / 2.0,
                cx + dx * far,
                cy + w / 2.0,
            );
        }
    }
    // Traits au milieu de chaque côté.
    let mid = m + s / 2.0;
    for (from, to) in [(m - near, m - far), (m + s + near, m + s + far)] {
        fill_mm(plate, px_mm, mid - w / 2.0, from, mid + w / 2.0, to);
        fill_mm(plate, px_mm, from, mid - w / 2.0, to, mid + w / 2.0);
    }
    // Texte, centré dans la hauteur de la marge du bas.
    let dot = TEXT_HEIGHT_MM / 7.0;
    let top = m + s + (m - TEXT_HEIGHT_MM) / 2.0;
    let max_chars = ((s - 2.0 * (TEXT_LEFT_MM - m)) / (6.0 * dot)).floor() as usize;
    let mut x = TEXT_LEFT_MM;
    for c in fold(text).chars().take(max_chars) {
        let rows = glyph(c);
        for (j, row) in rows.iter().enumerate() {
            for i in 0..5 {
                if row & (0b10000 >> i) != 0 {
                    let (x0, y0) = (x + i as f64 * dot, top + j as f64 * dot);
                    fill_mm(plate, px_mm, x0, y0, x0 + dot, y0 + dot);
                }
            }
        }
        x += 6.0 * dot;
    }
}

/// Largeur du texte écrit par [`draw`], en mm (sans troncature).
pub fn text_width_mm(text: &str) -> f64 {
    fold(text).chars().count() as f64 * 6.0 * TEXT_HEIGHT_MM / 7.0
}

/// Remplit de blanc le rectangle donné en mm (coins dans n'importe quel
/// ordre). Un pixel est rempli si son centre est dedans ; un rectangle plus
/// fin qu'un pixel en remplit quand même une rangée.
fn fill_mm(plate: &mut Raster, px_mm: f64, x0: f64, y0: f64, x1: f64, y1: f64) {
    let span = |a: f64, b: f64, size: u32| {
        let (a, b) = (a.min(b) / px_mm, a.max(b) / px_mm);
        let mut first = (a - 0.5).ceil().max(0.0);
        let mut last = (b - 0.5).floor().min(f64::from(size) - 1.0);
        if last < first {
            first = ((a + b) / 2.0).floor().clamp(0.0, f64::from(size) - 1.0);
            last = first;
        }
        (first as usize, last as usize)
    };
    let (i0, i1) = span(x0, x1, plate.width);
    let (j0, j1) = span(y0, y1, plate.height);
    let width = plate.width as usize;
    for j in j0..=j1 {
        plate.data[j * width + i0..=j * width + i1].fill(255);
    }
}

/// Capitales sans accents ; ce que la police ne connaît pas devient « ? ».
fn fold(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().flat_map(char::to_uppercase) {
        let folded = match c {
            'À' | 'Â' | 'Ä' | 'Á' | 'Ã' | 'Å' => "A",
            'É' | 'È' | 'Ê' | 'Ë' => "E",
            'Î' | 'Ï' | 'Í' | 'Ì' => "I",
            'Ô' | 'Ö' | 'Ó' | 'Ò' | 'Õ' => "O",
            'Ù' | 'Û' | 'Ü' | 'Ú' => "U",
            'Ç' => "C",
            'Ÿ' => "Y",
            'Ñ' => "N",
            'Œ' => "OE",
            'Æ' => "AE",
            '·' | '–' | '—' => "-",
            '\'' | '’' => " ",
            _ => {
                out.push(if glyph_known(c) { c } else { '?' });
                continue;
            }
        };
        out.push_str(folded);
    }
    out
}

fn glyph_known(c: char) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit() || " -._/:()+#?".contains(c)
}

/// Les sept rangées d'un caractère, de haut en bas ; 5 bits par rangée, le
/// bit de poids fort à gauche.
fn glyph(c: char) -> [u8; 7] {
    match c {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'J' => [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x11, 0x0A, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F],
        '/' => [0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x00],
        ':' => [0x00, 0x0C, 0x0C, 0x00, 0x0C, 0x0C, 0x00],
        '(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        '+' => [0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00],
        '#' => [0x0A, 0x0A, 0x1F, 0x0A, 0x1F, 0x0A, 0x0A],
        ' ' => [0x00; 7],
        _ => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accents_are_folded_to_capitals() {
        assert_eq!(fold("Épinal · façade A'"), "EPINAL - FACADE A ");
        assert_eq!(fold("cœur ✓"), "COEUR ?");
    }

    #[test]
    fn marks_stay_in_the_margin() {
        // Plaque de 300 mm à 0,25 mm par pixel.
        let n = 1200;
        let mut plate = Raster::new(n, n, 1, vec![0; (n * n) as usize]);
        draw(&mut plate, 0.25, "Épinal - A - 1400 dpi");
        let inside = |i: u32, j: u32| {
            let (x, y) = ((f64::from(i) + 0.5) * 0.25, (f64::from(j) + 0.5) * 0.25);
            (10.0..=290.0).contains(&x) && (10.0..=290.0).contains(&y)
        };
        let mut lit = 0;
        for j in 0..n {
            for i in 0..n {
                if plate.data[(j * n + i) as usize] == 255 {
                    lit += 1;
                    assert!(!inside(i, j), "repère dans l'image en ({i}, {j})");
                }
            }
        }
        assert!(lit > 1000, "{lit} pixels gravés seulement");
        // L'équerre du coin haut-gauche est là, 2 mm au-dessus du coin.
        assert_eq!(plate.data[(32 * n + 40) as usize], 255);
    }
}

//! Images incorporées d'un `.ai` (demande de Benjamin, 13 novembre 2026) :
//! copiées dans le fichier lui-même, sans fichier lié à côté. Illustrator
//! les range dans un bloc `%AI5_BeginRaster` dont le chemin est vide (ou un
//! fichier temporaire d'Illustrator), suivi de ses pixels bruts :
//!
//! ```text
//! %AI5_BeginRaster
//! () 1 XG
//! /DeviceGray XN
//! [ a b c d tx ty ] largeur hauteur 0 Xh
//! [ a b c d tx ty ] 0 0 l h l h bits type alpha réservé binaire masque … XI
//! %%BeginData: N
//! XI
//! …pixels (N − 3 octets)…
//! %%EndData
//! ```
//!
//! Formats lus (décision D-6.3-i) : masque d'un bit (le cas des dessins de
//! Trévarez : le trait est peint de la couleur de remplissage là où le bit
//! vaut 0, comme `imagemask` en PostScript), gris, RVB et CMJN sur 8 bits.
//! Chaque image devient un PNG en niveaux de gris, trait blanc sur noir,
//! comme les dessins de la bibliothèque.

use image::GrayImage;

/// La façon dont les pixels sont rangés.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelKind {
    /// Un bit par pixel, peint là où il vaut 0.
    Mask,
    /// Un bit par pixel, blanc là où il vaut 1.
    Bitmap,
    Gray,
    Rgb,
    Cmyk,
}

/// Une image incorporée qu'on sait relire.
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedImage {
    /// Calque de premier niveau qui la contient.
    pub layer: String,
    /// Faux si elle ou un de ses calques est masqué dans Illustrator.
    pub visible: bool,
    /// Matrice et rectangle, comme pour une image liée
    /// ([`super::PlacedImage`]).
    pub matrix: [f64; 6],
    pub bounds: [f64; 4],
    pub width: u32,
    pub height: u32,
    pub kind: PixelKind,
    pub pixels: Vec<u8>,
    /// Nombre d'images liées lues avant elle dans le fichier.
    pub after: usize,
}

impl EmbeddedImage {
    /// Octets d'une ligne de pixels.
    fn row_bytes(width: u32, kind: PixelKind) -> usize {
        let w = width as usize;
        match kind {
            PixelKind::Mask | PixelKind::Bitmap => w.div_ceil(8),
            PixelKind::Gray => w,
            PixelKind::Rgb => w * 3,
            PixelKind::Cmyk => w * 4,
        }
    }

    /// L'image en niveaux de gris, trait blanc sur fond noir.
    pub fn to_gray(&self) -> GrayImage {
        let row = Self::row_bytes(self.width, self.kind);
        let mut out = GrayImage::new(self.width, self.height);
        for (y, line) in self
            .pixels
            .chunks(row)
            .take(self.height as usize)
            .enumerate()
        {
            for x in 0..self.width as usize {
                let v = match self.kind {
                    PixelKind::Mask | PixelKind::Bitmap => {
                        let bit = (line[x / 8] >> (7 - (x % 8))) & 1;
                        let painted = if self.kind == PixelKind::Mask {
                            bit == 0
                        } else {
                            bit == 1
                        };
                        if painted {
                            255
                        } else {
                            0
                        }
                    }
                    PixelKind::Gray => line[x],
                    PixelKind::Rgb => {
                        let p = &line[x * 3..x * 3 + 3];
                        (0.299 * f64::from(p[0])
                            + 0.587 * f64::from(p[1])
                            + 0.114 * f64::from(p[2]))
                        .round() as u8
                    }
                    PixelKind::Cmyk => {
                        let p = &line[x * 4..x * 4 + 4];
                        let ink = 0.3 * f64::from(p[0])
                            + 0.59 * f64::from(p[1])
                            + 0.11 * f64::from(p[2])
                            + f64::from(p[3]);
                        (255.0 - ink.min(255.0)).round() as u8
                    }
                };
                out.put_pixel(x as u32, y as u32, image::Luma([v]));
            }
        }
        out
    }
}

/// Lit un bloc `%AI5_BeginRaster` incorporé. `lines` : les lignes du bloc
/// après `%AI5_BeginRaster` ; `data_at` : où commence, dans `text`, la
/// ligne de rang `k` de `lines` (pour retrouver les pixels, binaires).
pub(super) fn read_embedded(
    text: &[u8],
    lines: &[&[u8]],
    line_offset: impl Fn(usize) -> usize,
) -> Option<EmbeddedImage> {
    let numbers = super::numbers;
    let xh = lines.iter().take(8).find(|l| l.ends_with(b"Xh"))?;
    let n = numbers(xh);
    if n.len() < 8 {
        return None;
    }
    let (xi_at, xi) = lines
        .iter()
        .enumerate()
        .take(8)
        .find(|(_, l)| l.starts_with(b"[") && !l.ends_with(b"Xh"))?;
    let h = numbers(xi);
    // [6 nombres] 4 du rectangle, largeur, hauteur, bits, type, alpha,
    // réservé, binaire, masque…
    if h.len() < 18 {
        return None;
    }
    let (width, height) = (h[10] as u32, h[11] as u32);
    let (bits, kind_code, mask) = (h[12] as u32, h[13] as u32, h[17] != 0.0);
    let kind = match (bits, kind_code) {
        (1, 1) if mask => PixelKind::Mask,
        (1, 1) => PixelKind::Bitmap,
        (8, 1) => PixelKind::Gray,
        (8, 3) => PixelKind::Rgb,
        (8, 4) => PixelKind::Cmyk,
        _ => return None,
    };
    if width == 0 || height == 0 || h[16] == 0.0 {
        // Données en hexadécimal (ancien format) : non lues.
        return None;
    }
    let (begin_at, begin) = lines
        .iter()
        .enumerate()
        .skip(xi_at)
        .take(4)
        .find(|(_, l)| l.starts_with(b"%%BeginData:"))?;
    let size = numbers(&begin[b"%%BeginData:".len()..]).first().copied()? as usize;
    let start = line_offset(begin_at) + begin.len() + 1;
    let data = text.get(start..start.checked_add(size)?)?;
    // Les données commencent par « XI » et sa fin de ligne.
    let data = data.strip_prefix(b"XI")?;
    let data = data
        .strip_prefix(b"\r\n")
        .or_else(|| data.strip_prefix(b"\n"))
        .or_else(|| data.strip_prefix(b"\r"))?;
    let needed = EmbeddedImage::row_bytes(width, kind) * height as usize;
    if data.len() < needed {
        return None;
    }
    Some(EmbeddedImage {
        layer: String::new(),
        visible: true,
        matrix: [n[0], -n[1], n[2], -n[3], n[4], n[5]],
        bounds: [0.0, 0.0, f64::from(width), f64::from(height)],
        width,
        height,
        kind,
        pixels: data[..needed].to_vec(),
        after: 0,
    })
}

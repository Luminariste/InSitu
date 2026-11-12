//! Lecture de l'image aplatie d'un fichier Photoshop (PSD, et PSB pour les
//! très grands formats).
//!
//! Tout fichier Photoshop se termine par l'image telle qu'elle apparaît,
//! calques fusionnés (« données de l'image »). C'est elle qu'on lit ; les
//! calques ne le sont pas. Format : spécification Adobe « Photoshop File
//! Formats », sections 1 (en-tête) et 5 (données de l'image).
//!
//! Écrit ici plutôt que repris d'une bibliothèque : celle essayée lisait mal
//! les fichiers non compressés et les 16 bits (D-4.1-c). Formats lus : gris,
//! RVB, et bitmap (1 bit) ; 8, 16 ou 1 bit par canal ; sans compression ou
//! en « PackBits ». Le CMJN, les couleurs indexées et les 32 bits sont
//! refusés avec un message.

use crate::tiles::Raster;

/// Lit l'image aplatie d'un fichier Photoshop.
pub fn decode_psd(data: &[u8]) -> Result<Raster, String> {
    let mut r = Reader { data, at: 0 };
    if r.bytes(4)? != b"8BPS" {
        return Err("ce n'est pas un fichier Photoshop".into());
    }
    let big = match r.u16()? {
        1 => false,
        2 => true, // PSB
        v => return Err(format!("version de fichier Photoshop inconnue ({v})")),
    };
    r.bytes(6)?;
    let channels = usize::from(r.u16()?);
    let height = r.u32()? as usize;
    let width = r.u32()? as usize;
    let depth = r.u16()?;
    let mode = r.u16()?;
    if width == 0 || height == 0 || channels == 0 {
        return Err("image vide".into());
    }
    // Canaux de l'image aplatie gardés : 1 en gris ou bitmap, 3 en RVB. Les
    // canaux suivants (couches alpha, tons directs) sont ignorés.
    let kept = match mode {
        0 | 1 => 1,
        3 => 3,
        2 => {
            return Err(
                "couleurs indexées : enregistrer le PSD en RVB ou en niveaux de gris".into(),
            )
        }
        4 => return Err("CMJN : enregistrer le PSD en RVB ou en niveaux de gris".into()),
        m => {
            return Err(format!(
                "mode de couleur Photoshop non pris en charge ({m})"
            ))
        }
    };
    if channels < kept {
        return Err("canaux manquants".into());
    }
    if !matches!((mode, depth), (0, 1) | (1 | 3, 8 | 16)) {
        return Err(format!(
            "{depth} bits par canal : seuls 1, 8 et 16 bits sont lus"
        ));
    }
    // Sections sautées : couleurs, ressources, calques et masques.
    for section in 0..3 {
        let length = if big && section == 2 {
            r.u64()?
        } else {
            u64::from(r.u32()?)
        };
        r.bytes(usize::try_from(length).map_err(|_| "section trop longue")?)?;
    }
    let compression = r.u16()?;
    let row_bytes = match depth {
        1 => width.div_ceil(8),
        8 => width,
        _ => width * 2,
    };
    let plane = row_bytes * height;
    // Les plans de tous les canaux se suivent ; on lit les `kept` premiers.
    let planes: Vec<u8> = match compression {
        0 => r.bytes(plane * kept)?.to_vec(),
        1 => {
            let rows = height * channels;
            let mut counts = Vec::with_capacity(rows);
            for _ in 0..rows {
                counts.push(if big {
                    r.u32()? as usize
                } else {
                    usize::from(r.u16()?)
                });
            }
            let mut out = Vec::with_capacity(plane * kept);
            for &count in &counts[..height * kept] {
                unpack_bits(r.bytes(count)?, row_bytes, &mut out)?;
            }
            out
        }
        2 | 3 => return Err("image aplatie compressée en ZIP : non prise en charge".into()),
        c => return Err(format!("compression inconnue ({c})")),
    };
    let count = width * height;
    let sample = |channel: usize, index: usize| -> u8 {
        let base = channel * plane;
        match depth {
            // Bitmap : 1 = noir.
            1 => {
                let (y, x) = (index / width, index % width);
                let byte = planes[base + y * row_bytes + x / 8];
                if byte & (0x80 >> (x % 8)) == 0 {
                    255
                } else {
                    0
                }
            }
            8 => planes[base + index],
            _ => planes[base + index * 2], // octet de poids fort
        }
    };
    let mut pixels = Vec::with_capacity(count * kept);
    for index in 0..count {
        for channel in 0..kept {
            pixels.push(sample(channel, index));
        }
    }
    Ok(Raster::new(width as u32, height as u32, kept as u8, pixels))
}

/// Décompresse une ligne « PackBits » de `expected` octets.
fn unpack_bits(mut input: &[u8], expected: usize, out: &mut Vec<u8>) -> Result<(), String> {
    let start = out.len();
    while out.len() - start < expected {
        let (&n, rest) = input.split_first().ok_or("ligne compressée incomplète")?;
        input = rest;
        match n as i8 {
            -128 => {}
            n if n >= 0 => {
                let len = n as usize + 1;
                if input.len() < len {
                    return Err("ligne compressée incomplète".into());
                }
                out.extend_from_slice(&input[..len]);
                input = &input[len..];
            }
            n => {
                let (&value, rest) = input.split_first().ok_or("ligne compressée incomplète")?;
                input = rest;
                out.extend(std::iter::repeat_n(value, 1 + (-(n as isize)) as usize));
            }
        }
    }
    if out.len() - start != expected {
        return Err("ligne compressée trop longue".into());
    }
    Ok(())
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(n)
            .filter(|&e| e <= self.data.len())
            .ok_or("fichier Photoshop tronqué")?;
        let slice = &self.data[self.at..end];
        self.at = end;
        Ok(slice)
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(
            self.bytes(2)?.try_into().expect("2 octets"),
        ))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(
            self.bytes(4)?.try_into().expect("4 octets"),
        ))
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_be_bytes(
            self.bytes(8)?.try_into().expect("8 octets"),
        ))
    }
}

/// Fabrique un PSD minimal (pour les tests) : `compressed` choisit
/// « PackBits » (lignes compressées naïvement) ou aucune compression.
#[cfg(test)]
pub(crate) fn encode_psd(raster: &Raster, compressed: bool) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"8BPS");
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&[0; 6]);
    out.extend_from_slice(&u16::from(raster.channels).to_be_bytes());
    out.extend_from_slice(&raster.height.to_be_bytes());
    out.extend_from_slice(&raster.width.to_be_bytes());
    out.extend_from_slice(&8u16.to_be_bytes());
    let mode: u16 = if raster.channels == 1 { 1 } else { 3 };
    out.extend_from_slice(&mode.to_be_bytes());
    for _ in 0..3 {
        out.extend_from_slice(&0u32.to_be_bytes());
    }
    let c = raster.channels as usize;
    let w = raster.width as usize;
    let rows: Vec<Vec<u8>> = (0..c)
        .flat_map(|channel| {
            (0..raster.height as usize).map(move |y| {
                (0..w)
                    .map(|x| raster.data[(y * w + x) * c + channel])
                    .collect::<Vec<u8>>()
            })
        })
        .collect();
    if !compressed {
        out.extend_from_slice(&0u16.to_be_bytes());
        rows.iter().for_each(|row| out.extend_from_slice(row));
        return out;
    }
    out.extend_from_slice(&1u16.to_be_bytes());
    // Répétitions pour les suites d'octets égaux, copies sinon.
    let packed: Vec<Vec<u8>> = rows
        .iter()
        .map(|row| {
            let mut p = Vec::new();
            let mut i = 0;
            while i < row.len() {
                let run = row[i..]
                    .iter()
                    .take(128)
                    .take_while(|&&v| v == row[i])
                    .count();
                if run >= 2 {
                    p.push((1 - run as isize) as i8 as u8);
                    p.push(row[i]);
                    i += run;
                } else {
                    let len = (row.len() - i).min(128).min(
                        row[i..]
                            .windows(2)
                            .position(|w| w[0] == w[1])
                            .map_or(usize::MAX, |k| k.max(1)),
                    );
                    p.push((len - 1) as u8);
                    p.extend_from_slice(&row[i..i + len]);
                    i += len;
                }
            }
            p
        })
        .collect();
    for row in &packed {
        out.extend_from_slice(&(row.len() as u16).to_be_bytes());
    }
    packed.iter().for_each(|row| out.extend_from_slice(row));
    out
}

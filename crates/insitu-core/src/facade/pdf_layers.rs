//! Calques d'un PDF (« contenus facultatifs », OCG) : les lire, et ne
//! montrer que certains d'entre eux (demande de Benjamin, 13 novembre 2026 :
//! plan détaillé, plan simplifié, guides, tirés d'un même fichier).
//!
//! Un `.ai` enregistré « compatible PDF » garde ses calques d'Illustrator
//! sous cette forme. `hayro` sait masquer un calque, mais ne lit l'état des
//! calques que dans le fichier lui-même (le réglage par défaut du
//! catalogue). Pour choisir les calques montrés, on ajoute donc au fichier,
//! en mémoire seulement, une « mise à jour incrémentale » comme le ferait
//! Acrobat : un nouveau catalogue, identique à l'ancien sauf son réglage des
//! calques, et une table de renvois qui pointe vers lui. Le fichier du
//! projet n'est jamais modifié.

use std::sync::Arc;

use hayro::hayro_syntax::object::{Array, Dict, ObjectIdentifier};
use hayro::hayro_syntax::Pdf;

use super::FacadeError;

/// Un calque du PDF.
struct Layer {
    name: String,
    id: ObjectIdentifier,
}

fn load(data: &[u8]) -> Option<Pdf> {
    Pdf::new(Arc::new(data.to_vec())).ok()
}

/// Les calques déclarés par le catalogue (`/OCProperties /OCGs`), dans
/// l'ordre du fichier.
fn layers_of(pdf: &Pdf) -> Vec<Layer> {
    let xref = pdf.xref();
    let Some(catalog) = xref.get::<Dict<'_>>(xref.root_id()) else {
        return Vec::new();
    };
    let Some(properties) = catalog.get::<Dict<'_>>(b"OCProperties") else {
        return Vec::new();
    };
    let Some(groups) = properties.get::<Array<'_>>(b"OCGs") else {
        return Vec::new();
    };
    let mut layers = Vec::new();
    for item in groups.raw_iter() {
        let Some(reference) = item.as_obj_ref() else {
            continue;
        };
        let id = ObjectIdentifier::new(reference.obj_number, reference.gen_number);
        let name = xref
            .get::<Dict<'_>>(id)
            .and_then(|group| group.get::<hayro::hayro_syntax::object::String<'_>>(b"Name"))
            .map(|name| decode_text(name.as_bytes()))
            .unwrap_or_else(|| format!("Calque {}", layers.len() + 1));
        layers.push(Layer { name, id });
    }
    layers
}

/// Noms des calques du PDF, dans l'ordre du fichier ; vide s'il n'en a pas.
pub fn layer_names(data: &[u8]) -> Vec<String> {
    load(data).map_or_else(Vec::new, |pdf| {
        layers_of(&pdf)
            .into_iter()
            .map(|layer| layer.name)
            .collect()
    })
}

/// Le PDF où seuls les calques nommés dans `visible` sont montrés (tous les
/// calques qui portent un de ces noms). Le contenu qui n'est sur aucun
/// calque reste montré.
pub fn with_layers(data: &[u8], visible: &[String]) -> Result<Vec<u8>, FacadeError> {
    let fail = |why: &str| FacadeError::Unreadable(format!("calques du PDF illisibles : {why}"));
    let pdf = load(data).ok_or_else(|| fail("PDF illisible"))?;
    let layers = layers_of(&pdf);
    if layers.is_empty() {
        return Err(fail("ce PDF n'a pas de calques"));
    }
    if trailer_mentions_encryption(data) {
        return Err(fail("PDF chiffré"));
    }
    let xref = pdf.xref();
    let root = xref.root_id();
    let catalog = xref
        .get::<Dict<'_>>(root)
        .ok_or_else(|| fail("catalogue introuvable"))?;
    let entries = top_level_entries(catalog.data()).ok_or_else(|| fail("catalogue illisible"))?;

    let reference = |id: &ObjectIdentifier| format!("{} {} R", id.obj_number, id.gen_number);
    let all: Vec<String> = layers.iter().map(|l| reference(&l.id)).collect();
    let shown: Vec<String> = layers
        .iter()
        .filter(|l| visible.contains(&l.name))
        .map(|l| reference(&l.id))
        .collect();

    let mut object = format!("\n{} {} obj\n<<", root.obj_number, root.gen_number).into_bytes();
    for (key, value) in &entries {
        if key.as_slice() == b"/OCProperties" {
            continue;
        }
        object.push(b' ');
        object.extend_from_slice(key);
        object.push(b' ');
        object.extend_from_slice(value);
    }
    object.extend_from_slice(
        format!(
            " /OCProperties << /OCGs [{}] /D << /BaseState /OFF /ON [{}] /OFF [] >> >> >>\nendobj\n",
            all.join(" "),
            shown.join(" "),
        )
        .as_bytes(),
    );

    let previous = last_startxref(data).ok_or_else(|| fail("table de renvois introuvable"))?;
    let size = trailer_size(data, previous)
        .unwrap_or(0)
        .max(root.obj_number + 1);

    let mut out = Vec::with_capacity(data.len() + object.len() + 256);
    out.extend_from_slice(data);
    if !out.ends_with(b"\n") {
        out.push(b'\n');
    }
    let object_offset = out.len() + 1; // après le saut de ligne qui ouvre `object`
    out.extend_from_slice(&object);
    let xref_offset = out.len();
    out.extend_from_slice(
        format!(
            "xref\n0 1\n0000000000 65535 f\r\n{} 1\n{:010} {:05} n\r\ntrailer\n<< /Size {size} /Root {} {} R /Prev {previous} >>\nstartxref\n{xref_offset}\n%%EOF\n",
            root.obj_number,
            object_offset,
            root.gen_number.max(0),
            root.obj_number,
            root.gen_number,
        )
        .as_bytes(),
    );
    Ok(out)
}

/// Texte d'une chaîne PDF : UTF-16BE si elle commence par sa marque,
/// sinon un octet par caractère (le codage PDFDoc, proche du Latin-1).
fn decode_text(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xfe, 0xff]) {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    if let Ok(text) = std::str::from_utf8(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes))
    {
        if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
            return text.to_string();
        }
    }
    bytes.iter().map(|&b| char::from(b)).collect()
}

fn trailer_mentions_encryption(data: &[u8]) -> bool {
    find(data, b"/Encrypt").is_some()
}

/// Position donnée par le dernier `startxref` du fichier.
fn last_startxref(data: &[u8]) -> Option<usize> {
    let at = rfind(data, b"startxref")?;
    let rest = &data[at + b"startxref".len()..];
    let digits: Vec<u8> = rest
        .iter()
        .skip_while(|b| b.is_ascii_whitespace())
        .take_while(|b| b.is_ascii_digit())
        .copied()
        .collect();
    std::str::from_utf8(&digits).ok()?.parse().ok()
}

/// `/Size` de la table de renvois qui commence à `offset` (table classique
/// suivie de son `trailer`, ou flux de renvois).
fn trailer_size(data: &[u8], offset: usize) -> Option<u32> {
    let rest = data.get(offset..)?;
    let at = find(rest, b"/Size")?;
    let digits: Vec<u8> = rest[at + 5..]
        .iter()
        .skip_while(|b| b.is_ascii_whitespace())
        .take_while(|b| b.is_ascii_digit())
        .copied()
        .collect();
    std::str::from_utf8(&digits).ok()?.parse().ok()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).rposition(|w| w == needle)
}

// ---------------------------------------------------------------------------
// Lecture minimale d'un dictionnaire PDF, pour le recopier clé par clé
// ---------------------------------------------------------------------------

fn is_delimiter(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

fn is_white(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'\x0c' | b'\0')
}

fn skip_white(data: &[u8], mut i: usize) -> usize {
    while i < data.len() {
        if is_white(data[i]) {
            i += 1;
        } else if data[i] == b'%' {
            while i < data.len() && data[i] != b'\n' && data[i] != b'\r' {
                i += 1;
            }
        } else {
            break;
        }
    }
    i
}

/// Fin d'un mot (nombre, nom sans sa barre, mot-clé).
fn skip_regular(data: &[u8], mut i: usize) -> usize {
    while i < data.len() && !is_white(data[i]) && !is_delimiter(data[i]) {
        i += 1;
    }
    i
}

/// Fin de l'objet qui commence en `i` (espaces déjà passés). Une référence
/// « n g R » compte pour un seul objet.
fn skip_object(data: &[u8], i: usize) -> Option<usize> {
    let byte = *data.get(i)?;
    match byte {
        b'/' => Some(skip_regular(data, i + 1)),
        b'(' => {
            let mut depth = 0usize;
            let mut j = i;
            while j < data.len() {
                match data[j] {
                    b'\\' => j += 1,
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(j + 1);
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            None
        }
        b'<' if data.get(i + 1) == Some(&b'<') => {
            let mut j = skip_white(data, i + 2);
            loop {
                if data.get(j..j + 2)? == b">>" {
                    return Some(j + 2);
                }
                j = skip_white(data, skip_object(data, j)?);
            }
        }
        b'<' => Some(i + data[i..].iter().position(|&b| b == b'>')? + 1),
        b'[' => {
            let mut j = skip_white(data, i + 1);
            loop {
                if *data.get(j)? == b']' {
                    return Some(j + 1);
                }
                j = skip_white(data, skip_object(data, j)?);
            }
        }
        _ => {
            let end = skip_regular(data, i);
            if end == i {
                return None;
            }
            // Une référence : deux entiers suivis de R.
            let word = &data[i..end];
            if word.iter().all(u8::is_ascii_digit) {
                let j = skip_white(data, end);
                let end2 = skip_regular(data, j);
                if end2 > j && data[j..end2].iter().all(u8::is_ascii_digit) {
                    let k = skip_white(data, end2);
                    if data.get(k) == Some(&b'R') && skip_regular(data, k) == k + 1 {
                        return Some(k + 1);
                    }
                }
            }
            Some(end)
        }
    }
}

/// Clés et valeurs (octets bruts) d'un dictionnaire `<< … >>`.
fn top_level_entries(dict: &[u8]) -> Option<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut i = skip_white(dict, 0);
    if dict.get(i..i + 2)? != b"<<" {
        return None;
    }
    i = skip_white(dict, i + 2);
    let mut entries = Vec::new();
    loop {
        if dict.get(i..i + 2)? == b">>" {
            return Some(entries);
        }
        if dict[i] != b'/' {
            return None;
        }
        let key_end = skip_regular(dict, i + 1);
        let key = dict[i..key_end].to_vec();
        let value_start = skip_white(dict, key_end);
        let value_end = skip_object(dict, value_start)?;
        entries.push((key, dict[value_start..value_end].to_vec()));
        i = skip_white(dict, value_end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_dictionary_key_by_key() {
        let dict = b"<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [5 0 R 6 0 R] /D << /Order [5 0 R] /Name (a >> b) >> >> /Lang (fr) /Arr [1 [2 3] <00ff>] >>";
        let entries = top_level_entries(dict).unwrap();
        let keys: Vec<&[u8]> = entries.iter().map(|(k, _)| k.as_slice()).collect();
        assert_eq!(
            keys,
            vec![
                &b"/Type"[..],
                b"/Pages",
                b"/OCProperties",
                b"/Lang",
                b"/Arr"
            ]
        );
        assert_eq!(entries[1].1, b"2 0 R");
        assert_eq!(entries[3].1, b"(fr)");
        assert_eq!(entries[4].1, b"[1 [2 3] <00ff>]");
    }

    #[test]
    fn decodes_layer_names() {
        assert_eq!(decode_text(b"Mordor\xe9"), "Mordoré");
        assert_eq!(decode_text(&[0xfe, 0xff, 0, b'P', 0, b'l']), "Pl");
    }
}

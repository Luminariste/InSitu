//! Import Illustrator (tâche 6.3) : les images placées d'un fichier `.ai`,
//! leurs liens et leurs positions, recréées en dessins posés sur la façade.
//!
//! Un `.ai` récent est un PDF. La composition telle qu'Illustrator la relit
//! est rangée à part, dans les « données privées » du fichier
//! (`/PieceInfo /Illustrator /Private /AIPrivateData1…n`) : un texte
//! PostScript commenté, compressé en Zstandard (Illustrator 2020 et après)
//! ou en zlib (avant). Chaque image liée y est décrite par un bloc
//! `%AI5_BeginPlace … %AI5_EndPlace` : sa matrice de placement, le
//! rectangle de l'image, son chemin absolu et, depuis Illustrator 2024, son
//! chemin relatif au `.ai`. Les anciennes images liées (`%AI5_BeginRaster`
//! avec un chemin) sont lues aussi ; les images incorporées sont comptées,
//! pas importées (décision D-6.3-b).
//!
//! Repère : celui d'Illustrator a y vers le haut. Le plan de travail
//! (`%AI3_Cropmarks`) est ramené à la page, origine en haut à gauche, y
//! vers le bas, comme la façade.

mod embedded;
mod link;
mod plan;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::library::{import_files, ImportOptions, ImportProgress, LibraryError};
use crate::project::model::{new_id, RectMm};
use crate::project::{Layer, Placement, PointMm, Project, ProjectDir, ProjectError};

pub use embedded::{EmbeddedImage, PixelKind};
pub use link::{fold_name, resolve_link};
pub use plan::{
    plan_links, preview_composition, CompositionPreview, LinkPlan, LinkStatus, PreviewImage,
    PreviewLayer, PreviewLink,
};

#[derive(Debug)]
pub enum IllustratorError {
    /// Le fichier n'a pas pu être lu.
    Unreadable(String),
    /// Ni PDF, ni données Illustrator.
    NotIllustrator(String),
    /// La façade n'est pas à l'échelle et aucun rectangle n'est donné.
    NoFacadeScale,
    /// Aucune image liée retrouvée : rien à poser.
    NothingPlaced(String),
    Library(LibraryError),
    Project(ProjectError),
}

impl fmt::Display for IllustratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(m) => write!(f, "fichier illisible : {m}"),
            Self::NotIllustrator(m) => write!(f, "{m}"),
            Self::NoFacadeScale => f.write_str(
                "la façade doit être importée et mise à l'échelle avant d'importer une composition",
            ),
            Self::NothingPlaced(m) => write!(f, "{m}"),
            Self::Library(e) => write!(f, "{e}"),
            Self::Project(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for IllustratorError {}

impl From<ProjectError> for IllustratorError {
    fn from(e: ProjectError) -> Self {
        Self::Project(e)
    }
}

impl From<LibraryError> for IllustratorError {
    fn from(e: LibraryError) -> Self {
        Self::Library(e)
    }
}

// ---------------------------------------------------------------------------
// Lecture du fichier
// ---------------------------------------------------------------------------

/// Une image liée, telle qu'Illustrator la décrit.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacedImage {
    /// Calque de premier niveau qui la contient.
    pub layer: String,
    /// Faux si ce calque, ou un sous-calque qui contient l'image, est masqué
    /// dans Illustrator.
    pub visible: bool,
    /// Matrice d'Illustrator [a b c d tx ty] : un point (x, y) de l'image
    /// tombe en (a·x + c·y + tx, b·x + d·y + ty), y vers le haut.
    pub matrix: [f64; 6],
    /// Rectangle de l'image dans son propre repère (x0, y0, x1, y1) : sa
    /// taille en pixels est (x1 − x0, y1 − y0) ; le pixel (u, v), v vers
    /// le bas, est le point (x0 + u, y0 + v).
    pub bounds: [f64; 4],
    /// Chemin du fichier lié, tel qu'enregistré sur le Mac qui a fait le
    /// `.ai`.
    pub absolute: String,
    /// Dossier du lien relatif au fichier `.ai` lui-même (`../Algues/`),
    /// si Illustrator l'a noté.
    pub relative_dir: Option<String>,
}

impl PlacedImage {
    pub fn file_name(&self) -> &str {
        self.absolute
            .rsplit(['/', '\\', ':'])
            .next()
            .unwrap_or(&self.absolute)
    }

    pub fn size_px(&self) -> (f64, f64) {
        (
            (self.bounds[2] - self.bounds[0]).abs(),
            (self.bounds[3] - self.bounds[1]).abs(),
        )
    }
}

/// Ce que le `.ai` contient d'utile ici.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiDocument {
    /// Plan de travail (x0, y0, x1, y1), repère d'Illustrator (y vers le
    /// haut), en points.
    pub artboard: [f64; 4],
    pub placed: Vec<PlacedImage>,
    /// Images incorporées (sans lien, ou liées à un fichier temporaire
    /// d'Illustrator), qu'on sache les relire ou non.
    pub embedded: usize,
    /// Calque de premier niveau de chaque image incorporée qu'on ne sait pas
    /// relire.
    pub embedded_layers: Vec<String>,
    /// Les images incorporées relues, dans l'ordre du fichier (décision
    /// D-6.3-i). Celles hors de tout calque (réserve de symboles ou de
    /// motifs) n'y sont pas.
    #[serde(skip)]
    pub embedded_images: Vec<EmbeddedImage>,
    /// Calques de premier niveau, dans l'ordre du fichier (le premier est au
    /// fond).
    pub layers: Vec<AiLayer>,
}

/// Un calque de premier niveau d'Illustrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiLayer {
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    /// Couleur du calque dans Illustrator (rouge, vert, bleu), si notée.
    pub color: Option<[u8; 3]>,
}

impl AiDocument {
    /// Toutes les images qu'on peut poser : les liées, puis les incorporées
    /// relues (nommées comme leur fichier extrait). Le rang d'une image dans
    /// cette liste est celui de l'aperçu et de l'import.
    pub fn all_images(&self) -> Vec<PlacedImage> {
        let mut all = self.placed.clone();
        all.extend(
            self.embedded_images
                .iter()
                .enumerate()
                .map(|(j, e)| PlacedImage {
                    layer: e.layer.clone(),
                    visible: e.visible,
                    matrix: e.matrix,
                    bounds: e.bounds,
                    absolute: embedded_name(j, &e.layer),
                    relative_dir: None,
                }),
        );
        all
    }

    /// Rang de l'image `i` de [`Self::all_images`] dans l'ordre du fichier
    /// (le premier est au fond), pour empiler les incorporées à leur place.
    pub fn file_order(&self, i: usize) -> f64 {
        match i.checked_sub(self.placed.len()) {
            None => i as f64,
            Some(j) => self.embedded_images[j].after as f64 - 0.5 + j as f64 * 1e-6,
        }
    }

    pub fn artboard_size(&self) -> (f64, f64) {
        (
            self.artboard[2] - self.artboard[0],
            self.artboard[3] - self.artboard[1],
        )
    }
}

/// Dossier, sous celui du `.ai` dans la bibliothèque, des images
/// incorporées extraites.
pub const EMBEDDED_FOLDER: &str = "Incorporées";

/// Nom du fichier extrait de la `j`-ième image incorporée.
pub fn embedded_name(j: usize, layer: &str) -> String {
    format!("Incorporée {:02} - {}.png", j + 1, clean(layer))
}

/// Lit un `.ai` : son plan de travail et ses images liées.
pub fn read(path: &Path) -> Result<AiDocument, IllustratorError> {
    let bytes = std::fs::read(path)
        .map_err(|e| IllustratorError::Unreadable(format!("{} : {e}", path.display())))?;
    let text = private_data(&bytes)
        .map_err(|m| IllustratorError::NotIllustrator(format!("{} : {m}", path.display())))?;
    parse(&text).ok_or_else(|| {
        IllustratorError::NotIllustrator(format!(
            "{} : plan de travail introuvable dans les données Illustrator",
            path.display()
        ))
    })
}

/// Les données privées d'Illustrator, décompressées.
pub fn private_data(bytes: &[u8]) -> Result<Vec<u8>, String> {
    use hayro::hayro_syntax::object::{Dict, Stream};
    use hayro::hayro_syntax::Pdf;

    let not_ai = || {
        "ce fichier ne contient pas de données Illustrator. Il faut un .ai enregistré par \
         Illustrator avec « Créer un fichier compatible PDF »"
            .to_string()
    };
    if bytes.starts_with(b"%!PS") {
        // Ancien .ai, PostScript : les données sont en clair.
        return Ok(bytes.to_vec());
    }
    let pdf = Pdf::new(bytes.to_vec()).map_err(|_| not_ai())?;
    let page = pdf.pages().first().ok_or_else(not_ai)?;
    let private = page
        .raw()
        .get::<Dict>(b"PieceInfo")
        .and_then(|d| d.get::<Dict>(b"Illustrator"))
        .and_then(|d| d.get::<Dict>(b"Private"))
        .ok_or_else(not_ai)?;
    let mut parts: Vec<(u32, Vec<u8>)> = Vec::new();
    for key in private.keys() {
        let name = key.as_ref().to_vec();
        let Some(n) = name
            .strip_prefix(b"AIPrivateData")
            .and_then(|rest| std::str::from_utf8(rest).ok())
            .and_then(|rest| rest.parse::<u32>().ok())
        else {
            continue;
        };
        let stream = private
            .get::<Stream>(&name)
            .ok_or_else(|| "données Illustrator incomplètes".to_string())?;
        let data = stream
            .decoded()
            .map_err(|_| "données Illustrator illisibles".to_string())?;
        parts.push((n, data.into_owned()));
    }
    if parts.is_empty() {
        return Err(not_ai());
    }
    parts.sort_by_key(|(n, _)| *n);
    let joined: Vec<u8> = parts.into_iter().flat_map(|(_, d)| d).collect();
    decompress(&joined)
}

fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    const ZSTD: &[u8] = b"%AI24_ZStandard_Data";
    const ZLIB: &[u8] = b"%AI12_CompressedData";
    if let Some(rest) = data.strip_prefix(ZSTD) {
        use std::io::Read;
        let mut out = Vec::new();
        ruzstd::decoding::StreamingDecoder::new(rest)
            .map_err(|e| format!("données Illustrator illisibles (zstd : {e})"))?
            .read_to_end(&mut out)
            .map_err(|e| format!("données Illustrator illisibles (zstd : {e})"))?;
        Ok(out)
    } else if let Some(rest) = data.strip_prefix(ZLIB) {
        miniz_oxide::inflate::decompress_to_vec_zlib(rest)
            .map_err(|e| format!("données Illustrator illisibles (zlib : {e:?})"))
    } else {
        Ok(data.to_vec())
    }
}

/// Une chaîne PostScript `(…)` au début de `line` ; rend son contenu.
fn ps_string(line: &[u8]) -> Option<Vec<u8>> {
    let start = line.iter().position(|&c| c == b'(')?;
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut i = start + 1;
    while i < line.len() {
        let c = line[i];
        match c {
            b'\\' if i + 1 < line.len() => {
                i += 1;
                let e = line[i];
                match e {
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'0'..=b'7' => {
                        let mut v = u32::from(e - b'0');
                        for _ in 0..2 {
                            if i + 1 < line.len() && (b'0'..=b'7').contains(&line[i + 1]) {
                                i += 1;
                                v = v * 8 + u32::from(line[i] - b'0');
                            }
                        }
                        out.push(v as u8);
                    }
                    other => out.push(other),
                }
            }
            b'(' => {
                depth += 1;
                out.push(c);
            }
            b')' if depth == 0 => return Some(out),
            b')' => {
                depth -= 1;
                out.push(c);
            }
            _ => out.push(c),
        }
        i += 1;
    }
    None
}

/// Les nombres d'une ligne, dans l'ordre.
fn numbers(line: &[u8]) -> Vec<f64> {
    String::from_utf8_lossy(line)
        .split(|c: char| c.is_whitespace() || c == '[' || c == ']')
        .filter_map(|t| t.parse::<f64>().ok())
        .collect()
}

/// Lit le texte des données privées.
pub fn parse(text: &[u8]) -> Option<AiDocument> {
    let lines: Vec<&[u8]> = text.split(|&c| c == b'\n' || c == b'\r').collect();
    // Où commence chaque ligne dans `text` : les pixels d'une image
    // incorporée se relisent à partir de là.
    let mut offsets = Vec::with_capacity(lines.len());
    let mut at = 0;
    for l in &lines {
        offsets.push(at);
        at += l.len() + 1;
    }
    let mut embedded_images = Vec::new();
    let mut artboard = None;
    let mut placed = Vec::new();
    let mut embedded = 0;
    let mut embedded_layers = Vec::new();
    // Pile des calques ouverts : (nom, visible, verrouillé).
    let mut stack: Vec<AiLayer> = Vec::new();
    let mut top_layers: Vec<AiLayer> = Vec::new();
    let mut pending_layer = false;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if artboard.is_none() {
            if let Some(rest) = line.strip_prefix(b"%AI3_Cropmarks:") {
                let n = numbers(rest);
                if n.len() >= 4 {
                    artboard = Some([n[0], n[1], n[2], n[3]]);
                }
            }
        }
        if line == b"%AI5_BeginLayer" {
            stack.push(AiLayer {
                name: String::new(),
                visible: true,
                locked: false,
                color: None,
            });
            pending_layer = true;
        } else if line.starts_with(b"%AI5_EndLayer") {
            if let Some(layer) = stack.pop() {
                if stack.is_empty() {
                    top_layers.push(layer);
                }
            }
        } else if pending_layer && line.ends_with(b" Lb") {
            // visible, aperçu, modifiable, imprimable, … Lb
            let n = numbers(line);
            if let (Some(last), true) = (stack.last_mut(), n.len() >= 3) {
                last.visible = n[0] != 0.0;
                last.locked = n[2] == 0.0;
                // … numéro de couleur, rouge, vert, bleu, …
                if n.len() >= 11 {
                    let c = |v: f64| v.clamp(0.0, 255.0) as u8;
                    last.color = Some([c(n[8]), c(n[9]), c(n[10])]);
                }
            }
        } else if pending_layer && line.ends_with(b") Ln") {
            if let (Some(name), Some(last)) = (ps_string(line), stack.last_mut()) {
                last.name = String::from_utf8_lossy(&name).into_owned();
            }
            pending_layer = false;
        } else if line == b"%AI5_BeginPlace" || line == b"%AI5_BeginRaster" {
            let raster = line == b"%AI5_BeginRaster";
            let closing: &[u8] = if raster {
                b"%AI5_EndRaster"
            } else {
                b"%AI5_EndPlace"
            };
            let end = (i + 1..lines.len())
                .find(|&k| lines[k] == closing)
                .unwrap_or(lines.len());
            let found = if raster {
                read_raster(&lines[i + 1..end.min(i + 8)])
                    .filter(|p| !link::is_temporary(&p.absolute))
            } else {
                read_place(&lines[i + 1..end])
            };
            match found {
                Some(mut p) => {
                    p.layer = stack.first().map(|l| l.name.clone()).unwrap_or_default();
                    p.visible = stack.iter().all(|l| l.visible);
                    placed.push(p);
                }
                None if raster => {
                    embedded += 1;
                    let layer = stack.first().map(|l| l.name.clone()).unwrap_or_default();
                    let read =
                        embedded::read_embedded(text, &lines[i + 1..end], |k| offsets[i + 1 + k]);
                    match read {
                        Some(mut image) if !layer.trim().is_empty() => {
                            image.layer = layer;
                            image.visible = stack.iter().all(|l| l.visible);
                            image.after = placed.len();
                            embedded_images.push(image);
                        }
                        _ => embedded_layers.push(layer),
                    }
                }
                None => {}
            }
            i = end;
        }
        i += 1;
    }
    Some(AiDocument {
        artboard: artboard?,
        placed,
        embedded,
        embedded_layers,
        embedded_images,
        layers: top_layers,
    })
}

/// Un bloc `%AI5_BeginPlace` (image liée, Illustrator 9 et après).
fn read_place(block: &[&[u8]]) -> Option<PlacedImage> {
    let mut matrix = None;
    let mut absolute = None;
    let mut relative_dir = None;
    let mut k = 0;
    while k < block.len() {
        let line = block[k];
        if matrix.is_none() && line.starts_with(b"[") {
            let n = numbers(line);
            if n.len() >= 10 {
                matrix = Some((
                    [n[0], n[1], n[2], n[3], n[4], n[5]],
                    [n[6], n[7], n[8], n[9]],
                ));
            }
        } else if absolute.is_none() && matrix.is_some() && line.starts_with(b"(") {
            absolute = ps_string(line).map(|s| String::from_utf8_lossy(&s).into_owned());
        } else if line == b"%AI28_BeginPlacedRelativePath" {
            relative_dir = block
                .get(k + 1)
                .and_then(|l| ps_string(l))
                .map(|s| String::from_utf8_lossy(&s).into_owned())
                .filter(|s| !s.is_empty());
        }
        k += 1;
    }
    let (matrix, bounds) = matrix?;
    let absolute = absolute.filter(|s| !s.is_empty())?;
    Some(PlacedImage {
        layer: String::new(),
        visible: true,
        matrix,
        bounds,
        absolute,
        relative_dir,
    })
}

/// Un bloc `%AI5_BeginRaster` : image liée à l'ancienne (son chemin suivi
/// de `XG`) ou incorporée (chemin vide) ; `None` pour une incorporée.
fn read_raster(block: &[&[u8]]) -> Option<PlacedImage> {
    let path = block
        .iter()
        .find(|l| l.ends_with(b"XG"))
        .and_then(|l| ps_string(l))
        .map(|s| String::from_utf8_lossy(&s).into_owned())
        .filter(|s| !s.is_empty())?;
    let line = block.iter().find(|l| l.ends_with(b"Xh"))?;
    let n = numbers(line);
    if n.len() < 8 {
        return None;
    }
    // Ici, la matrice place le pixel (u, v) en (a·u + c·v + tx,
    // ty − b·u − d·v) : même chose qu'un bloc Place aux signes de b et d
    // près.
    Some(PlacedImage {
        layer: String::new(),
        visible: true,
        matrix: [n[0], -n[1], n[2], -n[3], n[4], n[5]],
        bounds: [0.0, 0.0, n[6], n[7]],
        absolute: path,
        relative_dir: None,
    })
}

// ---------------------------------------------------------------------------
// Géométrie : de la page à la façade
// ---------------------------------------------------------------------------

/// Où tombe l'image sur la façade : centre, taille avant rotation,
/// rotation (degrés, sens horaire) et retournement, comme un placement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub center: PointMm,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    pub mirrored: bool,
}

/// Pose de `image` sur `frame` (mm de façade), le plan de travail couvrant
/// tout le rectangle.
pub fn pose(image: &PlacedImage, artboard: [f64; 4], frame: &RectMm) -> Pose {
    let [a, b, c, d, tx, ty] = image.matrix;
    let [x0, y0, x1, y1] = image.bounds;
    let (w, h) = (x1 - x0, y1 - y0);
    let kx = frame.width / (artboard[2] - artboard[0]);
    let ky = frame.height / (artboard[3] - artboard[1]);
    // Point de l'image → mm de façade (y vers le bas).
    let to_facade = |x: f64, y: f64| {
        let (px, py) = (a * x + c * y + tx, b * x + d * y + ty);
        PointMm::new(
            frame.x + (px - artboard[0]) * kx,
            frame.y + (artboard[3] - py) * ky,
        )
    };
    let center = to_facade(x0 + w / 2.0, y0 + h / 2.0);
    // Axes du fichier sur la façade : u (vers la droite), v (vers le bas).
    let u = (a * w * kx, -b * w * ky);
    let v = (c * h * kx, -d * h * ky);
    let width = u.0.hypot(u.1);
    let height = v.0.hypot(v.1);
    let mirrored = u.0 * v.1 - u.1 * v.0 < 0.0;
    let (dx, dy) = if mirrored { (-u.0, -u.1) } else { u };
    let mut rotation = dy.atan2(dx).to_degrees();
    if rotation.abs() < 1e-9 {
        rotation = 0.0;
    }
    Pose {
        center,
        width,
        height,
        rotation,
        mirrored,
    }
}

// ---------------------------------------------------------------------------
// Import dans le projet
// ---------------------------------------------------------------------------

/// Un lien du `.ai` que l'import n'a pas pu poser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingLink {
    /// Le chemin noté dans le `.ai`.
    pub link: String,
    /// Nombre d'images posées avec ce lien.
    pub count: usize,
    pub reason: String,
}

/// Compte rendu de l'import d'une composition.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionReport {
    /// Images liées trouvées dans le `.ai`.
    pub found: usize,
    /// Dessins posés sur la façade.
    pub placed: usize,
    /// Fichiers distincts ajoutés à la bibliothèque (ou déjà présents).
    pub drawings: usize,
    /// Parmi eux, ceux repris d'un dessin déjà dans la bibliothèque (même
    /// fichier, ou même nom quand le fichier lié n'est pas sur ce Mac).
    pub reused: usize,
    /// Dossier de la bibliothèque qui reçoit les nouveaux.
    pub folder: String,
    /// Calques créés ou repris, dans l'ordre d'Illustrator.
    pub layers: Vec<String>,
    /// Liens introuvables ou illisibles.
    pub missing: Vec<MissingLink>,
    /// Images incorporées dans le `.ai` qu'on ne sait pas relire (hors
    /// calque, ou format inconnu) : non importées.
    pub embedded: usize,
    /// Images incorporées extraites en PNG et posées.
    pub extracted: usize,
    /// Images déformées en biais dans Illustrator : posées sans le biais.
    pub skewed: usize,
    /// Images laissées de côté dans la fenêtre d'import.
    pub skipped: usize,
}

/// Rectangle de façade couvert par le plan de travail : `frame` s'il est
/// donné, sinon toute la largeur de la façade à partir du coin haut-gauche,
/// comme la fresque (D-3.5-b).
fn facade_frame(
    project: &Project,
    doc: &AiDocument,
    frame: Option<RectMm>,
) -> Result<RectMm, IllustratorError> {
    if let Some(f) = frame {
        return Ok(f);
    }
    let (aw, ah) = doc.artboard_size();
    let (facade_width, _) = project
        .facade
        .as_ref()
        .and_then(|f| f.size_mm())
        .ok_or(IllustratorError::NoFacadeScale)?;
    Ok(RectMm {
        x: 0.0,
        y: 0.0,
        width: facade_width,
        height: facade_width * ah / aw,
    })
}

/// Nom du `.ai` sans extension.
fn ai_stem(ai: &Path) -> String {
    ai.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Illustrator".into())
}

/// Dossier de la bibliothèque qui reçoit les fichiers liés du `.ai`.
fn library_folder(ai: &Path) -> String {
    format!(
        "{}/{}",
        crate::project::AssetKind::Drawing.directory(),
        clean(&ai_stem(ai))
    )
}

/// Vrai si la matrice déforme l'image en biais.
fn is_skewed(p: &PlacedImage) -> bool {
    let [a, b, c, d, ..] = p.matrix;
    let dot = a * c + b * d;
    dot.abs() > 1e-6 * (a.hypot(b) * c.hypot(d)).max(1e-12)
}

/// Recrée la composition d'un `.ai` : chaque image placée devient un dessin
/// posé, dans un calque du même nom que celui d'Illustrator. Un lien déjà
/// présent dans la bibliothèque (même fichier, ou même nom si le fichier
/// n'est pas sur ce Mac) reprend ce dessin ; les autres fichiers liés entrent
/// dans la bibliothèque, sous `bibliotheque/<nom du .ai>/`. `only` : les
/// images à importer (rang dans [`AiDocument::placed`], comme dans
/// l'aperçu) ; `None` : toutes. Le plan de travail couvre `frame` (mm de
/// façade), voir [`facade_frame`]. Les dessins s'ajoutent à ceux déjà posés.
pub fn import_composition(
    dir: &ProjectDir,
    ai: &Path,
    frame: Option<RectMm>,
    only: Option<&[usize]>,
    options: &ImportOptions,
    progress: &(dyn Fn(ImportProgress) + Sync),
) -> Result<(Project, CompositionReport), IllustratorError> {
    let doc = read(ai)?;
    let project = dir.load()?.project;
    let frame = facade_frame(&project, &doc, frame)?;
    let all = doc.all_images();
    let chosen: Vec<usize> = match only {
        None => (0..all.len()).collect(),
        Some(list) => {
            let mut v: Vec<usize> = list.iter().copied().filter(|&i| i < all.len()).collect();
            v.sort_unstable();
            v.dedup();
            v
        }
    };
    if chosen.is_empty() {
        return Err(IllustratorError::NothingPlaced(
            "aucune image choisie : rien à importer".into(),
        ));
    }
    let plan = plan_links(dir, &project, ai, &doc);
    let mut needed: Vec<usize> = chosen.iter().map(|&i| plan.link_of_image[i]).collect();
    needed.sort_unstable();
    needed.dedup();

    // Fichiers à copier dans la bibliothèque : les liens retrouvés sur ce
    // Mac et absents de la bibliothèque.
    let ai_folder = ai.parent().unwrap_or(Path::new("."));
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    let mut file_of_link: BTreeMap<usize, usize> = BTreeMap::new();
    // Les images incorporées choisies, extraites en PNG dans un dossier
    // provisoire, retiré après l'import.
    let scratch = std::env::temp_dir().join(format!("insitu-incorporees-{}", new_id()));
    let _cleanup = RemoveOnDrop(scratch.clone());
    for &k in &needed {
        let link = &plan.links[k];
        if let (LinkStatus::Embedded, Some(j)) = (link.status, link.embedded) {
            let path = scratch.join(&link.name);
            let written = std::fs::create_dir_all(&scratch)
                .map_err(|e| e.to_string())
                .and_then(|()| {
                    doc.embedded_images[j]
                        .to_gray()
                        .save(&path)
                        .map_err(|e| e.to_string())
                });
            if let Err(e) = written {
                return Err(IllustratorError::Unreadable(format!(
                    "image incorporée {} : {e}",
                    link.name
                )));
            }
            file_of_link.insert(k, files.len());
            files.push((path, format!("{EMBEDDED_FOLDER}/{}", link.name)));
            continue;
        }
        if link.status != LinkStatus::New {
            continue;
        }
        let Some(path) = link.file.as_ref().map(PathBuf::from) else {
            continue;
        };
        let at = match files.iter().position(|(f, _)| *f == path) {
            Some(at) => at,
            None => {
                files.push((path.clone(), inner_path(ai_folder, &path)));
                files.len() - 1
            }
        };
        file_of_link.insert(k, at);
    }
    let base = library_folder(ai);
    let ids = if files.is_empty() {
        Vec::new()
    } else {
        import_files(dir, &base, &files, options, progress)?.1
    };

    // Le dessin de chaque lien, ou pourquoi il manque.
    let count_of = |k: usize| {
        chosen
            .iter()
            .filter(|&&i| plan.link_of_image[i] == k)
            .count()
    };
    let mut drawing_of_link: BTreeMap<usize, String> = BTreeMap::new();
    let mut missing = Vec::new();
    let mut reused = 0;
    for &k in &needed {
        let link = &plan.links[k];
        match link.status {
            LinkStatus::Library | LinkStatus::LibraryByName => {
                if let Some(d) = &link.drawing {
                    drawing_of_link.insert(k, d.clone());
                    reused += 1;
                }
            }
            LinkStatus::New | LinkStatus::Embedded => {
                match file_of_link.get(&k).map(|&at| &ids[at]) {
                    Some(Ok(id)) => {
                        drawing_of_link.insert(k, id.clone());
                    }
                    Some(Err(reason)) => missing.push(MissingLink {
                        link: link.link.clone(),
                        count: count_of(k),
                        reason: reason.clone(),
                    }),
                    None => {}
                }
            }
            LinkStatus::Missing => missing.push(MissingLink {
                link: link.link.clone(),
                count: count_of(k),
                reason: "fichier introuvable".into(),
            }),
        }
    }

    // Relu après l'import de la bibliothèque.
    let stem = ai_stem(ai);
    let mut project = dir.load()?.project;
    let mut layer_names: Vec<String> = Vec::new();
    let mut placements = Vec::new();
    let mut skewed = 0;
    let mut extracted = 0;
    let mut stacked = chosen.clone();
    stacked.sort_by(|&a, &b| doc.file_order(a).total_cmp(&doc.file_order(b)));
    for &i in &stacked {
        let p = &all[i];
        let Some(drawing) = drawing_of_link.get(&plan.link_of_image[i]) else {
            continue;
        };
        let flags = doc.layers.iter().find(|l| l.name == p.layer);
        let top_visible = flags.is_none_or(|l| l.visible);
        let name = if p.layer.trim().is_empty() {
            stem.clone()
        } else {
            p.layer.clone()
        };
        // Masquée dans un sous-calque d'un calque visible : le dessin seul est
        // masqué (l'app n'a pas de sous-calques).
        let hidden = !p.visible && top_visible;
        let layer_id = match project.layers.iter().find(|l| l.name == name) {
            Some(l) => l.id.clone(),
            None => {
                let id = new_id();
                project.layers.push(Layer {
                    id: id.clone(),
                    name: name.clone(),
                    visible: top_visible,
                    locked: flags.is_some_and(|l| l.locked),
                    extra: Default::default(),
                });
                id
            }
        };
        if !layer_names.contains(&name) {
            layer_names.push(name);
        }
        if is_skewed(p) {
            skewed += 1;
        }
        if i >= doc.placed.len() {
            extracted += 1;
        }
        let pose = pose(p, doc.artboard, &frame);
        placements.push(Placement {
            id: new_id(),
            drawing: drawing.clone(),
            center: pose.center,
            width: pose.width,
            height: pose.height,
            rotation: pose.rotation,
            mirrored: pose.mirrored,
            group: None,
            layer: Some(layer_id),
            deformation: None,
            projector: None,
            adjustment: None,
            slide: None,
            hidden,
            locked: false,
            extra: Default::default(),
        });
    }
    if placements.is_empty() {
        return Err(IllustratorError::NothingPlaced(format!(
            "aucune image liée du fichier n'a été retrouvée ({} lien(s), {} image(s) incorporée(s)). \
             Les fichiers liés doivent être à côté du .ai, comme dans Illustrator, ou déjà dans la bibliothèque",
            needed.len(),
            doc.embedded
        )));
    }
    let placed = placements.len();
    // Dans l'ordre d'Illustrator : le premier lu est au fond.
    project.placements.extend(placements);
    crate::placement::check(&mut project)
        .map_err(|e| IllustratorError::NotIllustrator(format!("dessins posés refusés : {e}")))?;
    dir.save(&project)?;
    let mut drawings: Vec<&String> = drawing_of_link.values().collect();
    drawings.sort();
    drawings.dedup();
    let report = CompositionReport {
        found: all.len(),
        placed,
        drawings: drawings.len(),
        reused,
        folder: base,
        layers: layer_names,
        missing,
        embedded: doc.embedded - doc.embedded_images.len(),
        extracted,
        skewed,
        skipped: all.len() - chosen.len(),
    };
    Ok((project, report))
}

fn link_key(p: &PlacedImage) -> String {
    match &p.relative_dir {
        Some(rel) => format!("{}|{}", p.absolute, rel),
        None => p.absolute.clone(),
    }
}

/// Chemin du fichier dans la bibliothèque : relatif au dossier du `.ai`
/// s'il y est (`Algues/algue.png`), sinon `Liens/<nom>`.
fn inner_path(ai_folder: &Path, file: &Path) -> String {
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    match canon(file).strip_prefix(canon(ai_folder)) {
        Ok(rel) => rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => format!("Liens/{name}"),
    }
}

/// Un nom de dossier acceptable dans le projet.
fn clean(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_start_matches('.').to_string();
    if trimmed.is_empty() {
        "Illustrator".into()
    } else {
        trimmed
    }
}

/// Retire un dossier provisoire en fin de portée.
struct RemoveOnDrop(PathBuf);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

//! Aperçu d'un import Illustrator (demande de Benjamin, 13 novembre 2026) :
//! ce que l'import ferait, calque par calque et lien par lien, sans rien
//! écrire dans le projet. L'interface s'en sert pour montrer le résultat et
//! laisser choisir les images à importer.
//!
//! C'est ici aussi que chaque lien est rapproché de la bibliothèque
//! (décision D-6.3-f) : un dessin déjà présent, au fichier identique, est
//! repris tel quel (avec ses retouches et sa silhouette) au lieu d'être
//! copié une seconde fois ; un lien introuvable sur ce Mac reprend le
//! dessin de la bibliothèque qui porte le même nom de fichier.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::link::{fold_name, resolve_link};
use super::{
    ai_stem, facade_frame, is_skewed, library_folder, link_key, pose, read, AiDocument,
    IllustratorError, PlacedImage,
};
use crate::project::model::RectMm;
use crate::project::{Drawing, PointMm, Project, ProjectDir};

/// Ce que l'import ferait d'un fichier lié.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LinkStatus {
    /// Le même fichier est déjà dans la bibliothèque : ce dessin est repris.
    Library,
    /// Le fichier n'est pas sur ce Mac, mais un dessin du même nom est dans
    /// la bibliothèque : il est repris.
    LibraryByName,
    /// Retrouvé sur ce Mac, pas encore dans la bibliothèque : il y entre.
    New,
    /// Introuvable : ses images ne sont pas posées.
    Missing,
}

/// Un fichier lié du `.ai` (plusieurs images peuvent le placer).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewLink {
    /// Le chemin noté dans le `.ai`.
    pub link: String,
    /// Le nom du fichier, sans son dossier.
    pub name: String,
    pub status: LinkStatus,
    /// Le fichier retrouvé sur ce Mac.
    pub file: Option<String>,
    /// Le dessin de la bibliothèque repris (statuts `library…`).
    pub drawing: Option<String>,
    pub drawing_name: Option<String>,
    /// Nombre d'images du `.ai` qui le placent.
    pub count: usize,
}

/// Un calque de premier niveau d'Illustrator, vu par l'aperçu.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewLayer {
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    /// Couleur du calque dans Illustrator.
    pub color: Option<[u8; 3]>,
    /// Images liées qu'il contient.
    pub images: usize,
    /// Images incorporées qu'il contient (non importables).
    pub embedded: usize,
    /// Un calque du même nom existe déjà dans le projet : les dessins s'y
    /// ajouteront.
    pub existing: bool,
}

/// Une image placée, là où l'import la poserait.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewImage {
    /// Rang dans [`CompositionPreview::layers`].
    pub layer: usize,
    /// Rang dans [`CompositionPreview::links`].
    pub link: usize,
    /// Faux si elle est masquée dans Illustrator (elle ou un de ses calques).
    pub visible: bool,
    pub center: PointMm,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    pub mirrored: bool,
    /// Déformée en biais : posée sans le biais.
    pub skewed: bool,
}

/// Ce que l'import d'un `.ai` ferait dans ce projet.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionPreview {
    /// Nom du `.ai`, sans extension.
    pub name: String,
    /// Dossier de la bibliothèque qui recevrait les nouveaux fichiers.
    pub folder: String,
    /// Rectangle de façade couvert par le plan de travail.
    pub frame: RectMm,
    /// Calques de premier niveau, du fond vers l'avant (ordre d'Illustrator).
    pub layers: Vec<PreviewLayer>,
    pub links: Vec<PreviewLink>,
    /// Dans l'ordre du `.ai` : le rang d'une image est celui qu'attend
    /// [`super::import_composition`].
    pub images: Vec<PreviewImage>,
    pub embedded: usize,
}

/// Les liens distincts du `.ai` et le lien de chaque image.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkPlan {
    pub links: Vec<PreviewLink>,
    /// Pour chaque image de [`AiDocument::placed`], son rang dans `links`.
    pub link_of_image: Vec<usize>,
}

/// Les dessins de la bibliothèque, prêts à être comparés aux liens.
struct LibraryIndex<'a> {
    entries: Vec<LibraryFile<'a>>,
}

struct LibraryFile<'a> {
    drawing: &'a Drawing,
    path: PathBuf,
    size: Option<u64>,
    /// Nom du fichier, sans accents ni casse.
    name: String,
    /// Nom de son dossier, sans accents ni casse.
    parent: String,
}

/// Le nom d'un fichier et celui de son dossier, sans accents ni casse, dans
/// un chemin écrit avec `/`, `\` ou `:`.
fn folded_tail(path: &str) -> (String, String) {
    let mut parts = path.rsplit(['/', '\\', ':']).filter(|s| !s.is_empty());
    let name = parts.next().map(fold_name).unwrap_or_default();
    let parent = parts.next().map(fold_name).unwrap_or_default();
    (name, parent)
}

impl<'a> LibraryIndex<'a> {
    fn new(dir: &ProjectDir, project: &'a Project) -> Self {
        let entries = project
            .library
            .iter()
            .map(|drawing| {
                let path = dir.resolve(&drawing.source);
                let size = std::fs::metadata(&path).ok().map(|m| m.len());
                let (name, parent) = folded_tail(drawing.source.as_str());
                LibraryFile {
                    drawing,
                    path,
                    size,
                    name,
                    parent,
                }
            })
            .collect();
        Self { entries }
    }

    /// Le dessin dont le fichier est identique, octet pour octet, à `file`.
    /// À égalité, celui du même nom.
    fn identical(&self, file: &Path) -> Option<&'a Drawing> {
        let size = std::fs::metadata(file).ok()?.len();
        let same_size: Vec<&LibraryFile> = self
            .entries
            .iter()
            .filter(|e| e.size == Some(size))
            .collect();
        if same_size.is_empty() {
            return None;
        }
        let bytes = std::fs::read(file).ok()?;
        let wanted = file
            .file_name()
            .map(|n| fold_name(&n.to_string_lossy()))
            .unwrap_or_default();
        let mut matches: Vec<&LibraryFile> = same_size
            .into_iter()
            .filter(|e| std::fs::read(&e.path).is_ok_and(|b| b == bytes))
            .collect();
        matches.sort_by_key(|e| e.name != wanted);
        matches.first().map(|e| e.drawing)
    }

    /// Le dessin qui porte le nom du fichier lié, de préférence dans un
    /// dossier du même nom.
    fn by_name(&self, image: &PlacedImage) -> Option<&'a Drawing> {
        let (name, parent) = folded_tail(&image.absolute);
        if name.is_empty() {
            return None;
        }
        let candidates: Vec<&LibraryFile> =
            self.entries.iter().filter(|e| e.name == name).collect();
        candidates
            .iter()
            .find(|e| !parent.is_empty() && e.parent == parent)
            .or_else(|| candidates.first())
            .map(|e| e.drawing)
    }
}

/// Retrouve chaque lien du `.ai`, sur ce Mac et dans la bibliothèque du
/// projet. Ne modifie rien.
pub fn plan_links(dir: &ProjectDir, project: &Project, ai: &Path, doc: &AiDocument) -> LinkPlan {
    let library = LibraryIndex::new(dir, project);
    let mut links: Vec<PreviewLink> = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    let mut link_of_image = Vec::with_capacity(doc.placed.len());
    for p in &doc.placed {
        let key = link_key(p);
        let k = match index.get(&key) {
            Some(&k) => k,
            None => {
                let file = resolve_link(ai, p);
                let (status, drawing) = match &file {
                    Some(f) => match library.identical(f) {
                        Some(d) => (LinkStatus::Library, Some(d)),
                        None => (LinkStatus::New, None),
                    },
                    None => match library.by_name(p) {
                        Some(d) => (LinkStatus::LibraryByName, Some(d)),
                        None => (LinkStatus::Missing, None),
                    },
                };
                links.push(PreviewLink {
                    link: p.absolute.clone(),
                    name: p.file_name().to_string(),
                    status,
                    file: file.map(|f| f.display().to_string()),
                    drawing: drawing.map(|d| d.id.clone()),
                    drawing_name: drawing.map(|d| d.name.clone()),
                    count: 0,
                });
                index.insert(key, links.len() - 1);
                links.len() - 1
            }
        };
        links[k].count += 1;
        link_of_image.push(k);
    }
    LinkPlan {
        links,
        link_of_image,
    }
}

/// Ce que [`super::import_composition`] ferait de `ai` dans ce projet, sans
/// rien écrire.
pub fn preview_composition(
    dir: &ProjectDir,
    ai: &Path,
    frame: Option<RectMm>,
) -> Result<CompositionPreview, IllustratorError> {
    let doc = read(ai)?;
    let project = dir.load()?.project;
    let frame = facade_frame(&project, &doc, frame)?;
    let plan = plan_links(dir, &project, ai, &doc);
    let stem = ai_stem(ai);

    // Calques : ceux d'Illustrator, dans leur ordre ; les images hors calque
    // vont dans un calque du nom du fichier, comme à l'import.
    let mut layers: Vec<PreviewLayer> = Vec::new();
    let layer_name = |name: &str| {
        if name.trim().is_empty() {
            stem.clone()
        } else {
            name.to_string()
        }
    };
    let add_layer = |layers: &mut Vec<PreviewLayer>, name: String, visible, locked, color| {
        if let Some(k) = layers.iter().position(|l| l.name == name) {
            return k;
        }
        let existing = project.layers.iter().any(|l| l.name == name);
        layers.push(PreviewLayer {
            name,
            visible,
            locked,
            color,
            images: 0,
            embedded: 0,
            existing,
        });
        layers.len() - 1
    };
    // Une image incorporée hors de tout calque (réserve de symboles ou de
    // motifs d'Illustrator) n'est comptée que dans le total.
    let loose = doc.placed.iter().any(|p| p.layer.trim().is_empty());
    if loose {
        add_layer(&mut layers, stem.clone(), true, false, None);
    }
    for l in &doc.layers {
        add_layer(
            &mut layers,
            layer_name(&l.name),
            l.visible,
            l.locked,
            l.color,
        );
    }
    let images = doc
        .placed
        .iter()
        .zip(&plan.link_of_image)
        .map(|(p, &link)| {
            let layer = add_layer(&mut layers, layer_name(&p.layer), true, false, None);
            layers[layer].images += 1;
            let pose = pose(p, doc.artboard, &frame);
            PreviewImage {
                layer,
                link,
                visible: p.visible,
                center: pose.center,
                width: pose.width,
                height: pose.height,
                rotation: pose.rotation,
                mirrored: pose.mirrored,
                skewed: is_skewed(p),
            }
        })
        .collect();
    for name in doc.embedded_layers.iter().filter(|n| !n.trim().is_empty()) {
        let layer = add_layer(&mut layers, layer_name(name), true, false, None);
        layers[layer].embedded += 1;
    }
    Ok(CompositionPreview {
        name: stem.clone(),
        folder: library_folder(ai),
        frame,
        layers,
        links: plan.links,
        images,
        embedded: doc.embedded,
    })
}

//! Les copies en conflit de Dropbox : les trouver et les réunir au projet.
//!
//! Quand deux Mac modifient `projet.json` avant que Dropbox ait fait passer
//! la version de l'autre, Dropbox garde l'une sous le nom `projet.json` et
//! renomme l'autre, à côté : `projet (Copie en conflit de Benjamin Nesme
//! 2026-11-15).json` (« conflicted copy » en anglais). Rien n'est perdu, mais
//! l'app n'ouvre que `projet.json`.
//!
//! Ce module :
//!
//! - trouve ces copies ([`find_conflicts`]) ;
//! - compare la copie et la version actuelle élément par élément : chaque
//!   projecteur, dessin de la bibliothèque, dessin posé, calque, groupe de
//!   plans par son identifiant, et chaque autre rubrique en bloc
//!   ([`compare`]) ;
//! - les réunit ([`resolve`]) : ce qui n'existe que d'un côté est gardé
//!   d'office ; ce qui a changé des deux côtés prend la version choisie.
//!
//! Il n'y a pas de version commune d'avant le conflit dans le dossier. Pour
//! proposer un choix, on regarde l'historique d'annulation de ce Mac : la
//! valeur qu'il a connue en premier est l'ancienne, l'autre est le
//! changement (simple suggestion, voir [`Difference::suggested`]).
//!
//! Après réunion, la copie est rangée dans `copies-en-conflit/` (gardée, plus
//! signalée), et la réunion s'annule par ⌘Z comme toute action.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::error::ProjectError;
use super::migrate;
use super::model::Project;
use super::store::ProjectDir;
use super::time::Timestamp;

/// Sous-dossier où sont rangées les copies déjà réunies.
pub const RESOLVED_DIR: &str = "copies-en-conflit";

/// Vrai si `name` est le nom d'une copie en conflit de Dropbox (français,
/// anglais, allemand, espagnol, italien).
pub fn is_conflict_copy_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains('(')
        && ["conflit", "conflict", "konflikt", "conflitto"]
            .iter()
            .any(|word| lower.contains(word))
}

/// Une copie en conflit de `projet.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictCopy {
    /// Nom du fichier, dans le dossier projet.
    pub file: String,
    /// Dernière modification du fichier.
    pub modified: Option<Timestamp>,
}

/// Ce que Dropbox a laissé en double dans un projet.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictScan {
    /// Copies de `projet.json`, à réunir.
    pub projects: Vec<ConflictCopy>,
    /// Autres fichiers en double (photos, dessins…), chemins relatifs au
    /// projet : seulement signalés.
    pub others: Vec<String>,
}

fn modified(path: &Path) -> Option<Timestamp> {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| Timestamp::from_unix_seconds(d.as_secs()))
}

/// Cherche les copies en conflit du projet. Le verrou et les copies déjà
/// rangées dans [`RESOLVED_DIR`] ne comptent pas.
pub fn find_conflicts(dir: &ProjectDir) -> Result<ConflictScan, ProjectError> {
    let mut scan = ConflictScan::default();
    let root = dir.root();
    let mut pending = vec![(root.to_path_buf(), String::new())];
    while let Some((folder, prefix)) = pending.pop() {
        let entries =
            fs::read_dir(&folder).map_err(ProjectError::io("de lire le dossier", &folder))?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let relative = format!("{prefix}{name}");
            let path = entry.path();
            if path.is_dir() {
                if prefix.is_empty() && name == RESOLVED_DIR {
                    continue;
                }
                pending.push((path, format!("{relative}/")));
                continue;
            }
            if !is_conflict_copy_name(&name) {
                continue;
            }
            if prefix.is_empty() && name.starts_with("projet (") && name.ends_with(".json") {
                scan.projects.push(ConflictCopy {
                    file: name,
                    modified: modified(&path),
                });
            } else if !(prefix.is_empty() && name.starts_with("verrou (")) {
                scan.others.push(relative);
            }
        }
    }
    scan.projects.sort_by(|a, b| a.file.cmp(&b.file));
    scan.others.sort();
    Ok(scan)
}

/// Quelle version garder pour un élément changé des deux côtés.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    /// `projet.json`, celle que l'app ouvre.
    Current,
    /// La copie en conflit.
    Copy,
}

/// Un élément différent dans les deux versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Difference {
    /// Clé du choix, à rendre à [`resolve`] : `placements/<id>`, ou le nom
    /// d'une rubrique (`export`).
    pub key: String,
    /// Rubrique, en français (« Dessins posés »).
    pub section: String,
    /// L'élément (« Daurade », « Projecteur B »).
    pub label: String,
    /// Ce qui diffère (« position, taille »).
    pub detail: String,
    /// Version probablement la plus récente, d'après l'historique de ce
    /// Mac ; aucune si l'historique ne dit rien.
    pub suggested: Option<Side>,
}

/// Nombre d'éléments d'une rubrique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionCount {
    pub section: String,
    pub count: usize,
}

/// Comparaison de `projet.json` et d'une copie en conflit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictReport {
    pub copy: ConflictCopy,
    /// Dernière modification de `projet.json`.
    pub current_modified: Option<Timestamp>,
    /// Ajouts présents seulement dans `projet.json` : gardés.
    pub only_current: Vec<SectionCount>,
    /// Ajouts présents seulement dans la copie : gardés.
    pub only_copy: Vec<SectionCount>,
    /// Éléments différents des deux côtés : à choisir.
    pub differences: Vec<Difference>,
}

/// Rubriques dont les éléments portent un identifiant, dans l'ordre montré.
const KEYED: [&str; 5] = [
    "projectors",
    "library",
    "placements",
    "layers",
    "planGroups",
];

/// Champs qui disent quel projet c'est : ils doivent être les mêmes.
const IDENTITY: [&str; 3] = ["id", "formatVersion", "createdAt"];

fn section_name(key: &str) -> &'static str {
    match key {
        "projectors" => "Projecteurs",
        "library" => "Bibliothèque",
        "placements" => "Dessins posés",
        "layers" => "Calques",
        "planGroups" => "Plans",
        "libraryFolders" => "Dossiers de la bibliothèque",
        "sharedLinks" => "Liens avec la bibliothèque commune",
        "facade" => "Façade",
        "fresco" => "Fresque",
        "export" => "Réglages d'export",
        "facadeAssembly" => "Assemblage de photos",
        "name" => "Nom du projet",
        _ => "Autre",
    }
}

fn field_name(key: &str) -> String {
    match key {
        "center" => "position",
        "width" | "height" => "taille",
        "rotation" => "rotation",
        "mirrored" => "miroir",
        "deformation" => "déformation",
        "adjustment" => "ajustement",
        "mask" => "masque",
        "registration" | "registrationLines" => "recalage",
        "rectification" => "redressement",
        "rectifiedPhoto" => "photo redressée",
        "rawPhoto" => "photo de mire",
        "slideMask" => "masque de diapositive",
        "folder" => "dossier",
        "name" | "label" => "nom",
        "silhouette" => "silhouette",
        "retouch" => "gomme et crayon",
        "hidden" => "masqué",
        "locked" => "verrouillé",
        "layer" => "calque",
        "projector" => "diapositive",
        "drawing" => "dessin",
        "source" => "fichier",
        other => other,
    }
    .to_string()
}

/// Ce qui diffère entre deux éléments : leurs champs, en français.
fn describe(a: &Value, b: &Value) -> String {
    let (Value::Object(a), Value::Object(b)) = (a, b) else {
        return "contenu".to_string();
    };
    let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut names: Vec<String> = Vec::new();
    for key in keys {
        if a.get(key) != b.get(key) {
            let name = field_name(key);
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    if names.is_empty() {
        "contenu".to_string()
    } else {
        names.join(", ")
    }
}

fn id_of(value: &Value) -> Option<&str> {
    value.get("id").and_then(Value::as_str)
}

fn keyed(document: &Map<String, Value>, key: &str) -> Vec<Value> {
    document
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// Lit un `projet.json` (ou sa copie) comme le fait l'app, et le rend sous sa
/// forme écrite : deux versions se comparent alors champ par champ.
fn read_normalized(path: &Path) -> Result<Map<String, Value>, ProjectError> {
    let text = fs::read_to_string(path).map_err(ProjectError::io("de lire", path))?;
    let mut document: Value =
        serde_json::from_str(&text).map_err(|e| ProjectError::InvalidJson {
            path: path.to_path_buf(),
            message: format!("ce n'est pas du JSON valide ({e})"),
        })?;
    migrate::upgrade_with(
        &mut document,
        path,
        migrate::FIRST_FORMAT_VERSION,
        migrate::MIGRATIONS,
    )?;
    let project: Project =
        serde_json::from_value(document).map_err(|e| ProjectError::InvalidJson {
            path: path.to_path_buf(),
            message: format!("la description du projet est incorrecte ({e})"),
        })?;
    match serde_json::to_value(&project) {
        Ok(Value::Object(map)) => Ok(map),
        _ => Err(ProjectError::InvalidJson {
            path: path.to_path_buf(),
            message: "le projet ne peut pas être relu".to_string(),
        }),
    }
}

/// Un dessin importé des deux côtés depuis le même fichier reçoit deux
/// identifiants. Dans la copie, il prend celui de la version actuelle, et les
/// dessins posés de la copie qui l'utilisent suivent.
fn align_duplicate_drawings(current: &Map<String, Value>, copy: &mut Map<String, Value>) {
    let current_library = keyed(current, "library");
    let copy_library = keyed(copy, "library");
    let copy_ids: HashSet<&str> = copy_library.iter().filter_map(id_of).collect();
    let current_ids: HashSet<&str> = current_library.iter().filter_map(id_of).collect();
    // Fichier → identifiant, pour les dessins de la version actuelle absents
    // de la copie.
    let mut by_source: HashMap<&str, &str> = HashMap::new();
    for drawing in &current_library {
        if let (Some(id), Some(source)) = (
            id_of(drawing),
            drawing.get("source").and_then(Value::as_str),
        ) {
            if !copy_ids.contains(id) {
                by_source.entry(source).or_insert(id);
            }
        }
    }
    let mut renamed: HashMap<String, String> = HashMap::new();
    for drawing in &copy_library {
        let (Some(id), Some(source)) = (
            id_of(drawing),
            drawing.get("source").and_then(Value::as_str),
        ) else {
            continue;
        };
        if current_ids.contains(id) {
            continue;
        }
        if let Some(target) = by_source.remove(source) {
            renamed.insert(id.to_string(), target.to_string());
        }
    }
    if renamed.is_empty() {
        return;
    }
    if let Some(Value::Array(library)) = copy.get_mut("library") {
        for drawing in library.iter_mut() {
            if let Some(target) = id_of(drawing).and_then(|id| renamed.get(id)).cloned() {
                drawing["id"] = Value::String(target);
            }
        }
    }
    if let Some(Value::Array(placements)) = copy.get_mut("placements") {
        for placement in placements.iter_mut() {
            let target = placement
                .get("drawing")
                .and_then(Value::as_str)
                .and_then(|id| renamed.get(id))
                .cloned();
            if let Some(target) = target {
                placement["drawing"] = Value::String(target);
            }
        }
    }
}

/// Les deux versions, prêtes à comparer.
struct Pair {
    current: Map<String, Value>,
    copy: Map<String, Value>,
}

fn load_pair(dir: &ProjectDir, copy_file: &str) -> Result<(Pair, ConflictCopy), ProjectError> {
    if copy_file.contains('/') || copy_file.contains('\\') || !is_conflict_copy_name(copy_file) {
        return Err(ProjectError::InvalidJson {
            path: dir.root().join(copy_file),
            message: "ce n'est pas une copie en conflit du projet".to_string(),
        });
    }
    let copy_path = dir.root().join(copy_file);
    let current = read_normalized(&dir.project_file())?;
    let mut copy = read_normalized(&copy_path)?;
    for key in IDENTITY {
        if key != "createdAt" && current.get(key) != copy.get(key) {
            return Err(ProjectError::InvalidJson {
                path: copy_path,
                message: "cette copie est celle d'un autre projet : elle n'est pas réunie"
                    .to_string(),
            });
        }
    }
    align_duplicate_drawings(&current, &mut copy);
    let info = ConflictCopy {
        file: copy_file.to_string(),
        modified: modified(&copy_path),
    };
    Ok((Pair { current, copy }, info))
}

/// Nom affiché d'un élément.
fn element_label(section: &str, element: &Value, drawings: &HashMap<String, String>) -> String {
    let text = |key: &str| element.get(key).and_then(Value::as_str).map(str::to_string);
    match section {
        "projectors" => text("label")
            .map(|l| format!("Projecteur {l}"))
            .unwrap_or_else(|| "Projecteur".to_string()),
        "placements" => text("drawing")
            .and_then(|d| drawings.get(&d).cloned())
            .unwrap_or_else(|| "Dessin posé".to_string()),
        _ => text("name")
            .or_else(|| text("label"))
            .unwrap_or_else(|| section_name(section).to_string()),
    }
}

/// Dessin → nom, des deux côtés.
fn drawing_names(pair: &Pair) -> HashMap<String, String> {
    let mut names = HashMap::new();
    for side in [&pair.copy, &pair.current] {
        for drawing in keyed(side, "library") {
            if let (Some(id), Some(name)) =
                (id_of(&drawing), drawing.get("name").and_then(Value::as_str))
            {
                names.insert(id.to_string(), name.to_string());
            }
        }
    }
    names
}

/// Différences brutes, avant suggestion.
struct Raw {
    only_current: Vec<SectionCount>,
    only_copy: Vec<SectionCount>,
    /// Clé, rubrique (clé du JSON), valeur actuelle, valeur de la copie.
    changed: Vec<(String, String, Value, Value)>,
}

fn all_top_keys(pair: &Pair) -> Vec<String> {
    let mut keys: Vec<String> = KEYED.iter().map(|k| k.to_string()).collect();
    let mut others: Vec<String> = pair
        .current
        .keys()
        .chain(pair.copy.keys())
        .filter(|k| !KEYED.contains(&k.as_str()) && !IDENTITY.contains(&k.as_str()))
        .cloned()
        .collect();
    others.sort();
    others.dedup();
    keys.extend(others);
    keys
}

/// Vrai si la rubrique se réunit par union de ses éléments (liste sans
/// identifiants : dossiers de la bibliothèque, liens).
fn is_union(a: Option<&Value>, b: Option<&Value>) -> bool {
    let list = |v: Option<&Value>| matches!(v, None | Some(Value::Array(_)));
    list(a) && list(b) && (a.is_some() || b.is_some())
}

fn raw_differences(pair: &Pair) -> Raw {
    let mut raw = Raw {
        only_current: Vec::new(),
        only_copy: Vec::new(),
        changed: Vec::new(),
    };
    for key in all_top_keys(pair) {
        let section = section_name(&key).to_string();
        if KEYED.contains(&key.as_str()) {
            let current = keyed(&pair.current, &key);
            let copy = keyed(&pair.copy, &key);
            let current_by_id: HashMap<&str, &Value> = current
                .iter()
                .filter_map(|v| id_of(v).map(|id| (id, v)))
                .collect();
            let copy_by_id: HashMap<&str, &Value> = copy
                .iter()
                .filter_map(|v| id_of(v).map(|id| (id, v)))
                .collect();
            let only_current = current_by_id
                .keys()
                .filter(|id| !copy_by_id.contains_key(*id))
                .count();
            let only_copy = copy_by_id
                .keys()
                .filter(|id| !current_by_id.contains_key(*id))
                .count();
            if only_current > 0 {
                raw.only_current.push(SectionCount {
                    section: section.clone(),
                    count: only_current,
                });
            }
            if only_copy > 0 {
                raw.only_copy.push(SectionCount {
                    section: section.clone(),
                    count: only_copy,
                });
            }
            for element in &current {
                let Some(id) = id_of(element) else { continue };
                if let Some(other) = copy_by_id.get(id) {
                    if *other != element {
                        raw.changed.push((
                            format!("{key}/{id}"),
                            key.clone(),
                            element.clone(),
                            (*other).clone(),
                        ));
                    }
                }
            }
        } else if is_union(pair.current.get(&key), pair.copy.get(&key)) {
            let current = keyed(&pair.current, &key);
            let copy = keyed(&pair.copy, &key);
            let extra_current = current.iter().filter(|v| !copy.contains(v)).count();
            let extra_copy = copy.iter().filter(|v| !current.contains(v)).count();
            if extra_current > 0 {
                raw.only_current.push(SectionCount {
                    section: section.clone(),
                    count: extra_current,
                });
            }
            if extra_copy > 0 {
                raw.only_copy.push(SectionCount {
                    section: section.clone(),
                    count: extra_copy,
                });
            }
        } else {
            let current = pair.current.get(&key).cloned().unwrap_or(Value::Null);
            let copy = pair.copy.get(&key).cloned().unwrap_or(Value::Null);
            if current != copy {
                raw.changed.push((key.clone(), key.clone(), current, copy));
            }
        }
    }
    raw
}

/// La valeur d'un élément (clé `rubrique/id` ou `rubrique`) dans une version.
fn element_in<'a>(document: &'a Value, key: &str) -> Option<&'a Value> {
    match key.split_once('/') {
        Some((section, id)) => document
            .get(section)?
            .as_array()?
            .iter()
            .find(|v| id_of(v) == Some(id)),
        None => document.get(key),
    }
}

/// Suggestions d'après l'historique de ce Mac : pour chaque élément, la
/// valeur vue en premier est l'ancienne ; on propose l'autre.
fn suggestions(
    dir: &ProjectDir,
    project_id: &str,
    changed: &[(String, String, Value, Value)],
) -> HashMap<String, Side> {
    let mut found: HashMap<String, Side> = HashMap::new();
    let Some(root) = dir.history_root() else {
        return found;
    };
    let mut pending: Vec<&(String, String, Value, Value)> = changed.iter().collect();
    for snapshot in super::history::snapshots(root, project_id) {
        if pending.is_empty() {
            break;
        }
        pending.retain(|(key, _, current, copy)| {
            let Some(old) = element_in(&snapshot, key) else {
                return true;
            };
            // Un élément absent est écrit `null` dans une rubrique en bloc.
            if old == current {
                found.insert(key.clone(), Side::Copy);
                false
            } else if old == copy {
                found.insert(key.clone(), Side::Current);
                false
            } else {
                true
            }
        });
    }
    found
}

/// Compare `projet.json` et la copie en conflit `copy_file` (nom du fichier,
/// dans le dossier projet).
pub fn compare(dir: &ProjectDir, copy_file: &str) -> Result<ConflictReport, ProjectError> {
    let (pair, copy) = load_pair(dir, copy_file)?;
    let raw = raw_differences(&pair);
    let names = drawing_names(&pair);
    let project_id = pair
        .current
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let suggested = suggestions(dir, &project_id, &raw.changed);
    let differences = raw
        .changed
        .iter()
        .map(|(key, section, current, other)| Difference {
            key: key.clone(),
            section: section_name(section).to_string(),
            label: if key.contains('/') {
                element_label(section, current, &names)
            } else {
                section_name(section).to_string()
            },
            detail: if key.contains('/') {
                describe(current, other)
            } else {
                "contenu".to_string()
            },
            suggested: suggested.get(key).copied(),
        })
        .collect();
    Ok(ConflictReport {
        copy,
        current_modified: modified(&dir.project_file()),
        only_current: raw.only_current,
        only_copy: raw.only_copy,
        differences,
    })
}

/// Réunit deux listes d'éléments à identifiant. L'ordre de la version
/// actuelle est gardé ; un élément propre à la copie se place juste après
/// celui qui le précède dans la copie.
fn merge_keyed(current: &[Value], copy: &[Value], take_copy: &dyn Fn(&str) -> bool) -> Vec<Value> {
    let copy_by_id: HashMap<&str, &Value> = copy
        .iter()
        .filter_map(|v| id_of(v).map(|id| (id, v)))
        .collect();
    let mut merged: Vec<Value> = current
        .iter()
        .map(
            |element| match id_of(element).and_then(|id| copy_by_id.get(id).map(|c| (id, c))) {
                Some((id, other)) if take_copy(id) => (*other).clone(),
                _ => element.clone(),
            },
        )
        .collect();
    let mut previous: Option<String> = None;
    for element in copy {
        let Some(id) = id_of(element) else { continue };
        let exists = merged.iter().any(|v| id_of(v) == Some(id));
        if !exists {
            let at = previous
                .as_deref()
                .and_then(|p| merged.iter().position(|v| id_of(v) == Some(p)))
                .map_or(0, |i| i + 1);
            merged.insert(at, element.clone());
        }
        previous = Some(id.to_string());
    }
    merged
}

fn merge_union(current: &[Value], copy: &[Value]) -> Vec<Value> {
    let mut merged = current.to_vec();
    for element in copy {
        if !merged.contains(element) {
            merged.push(element.clone());
        }
    }
    merged
}

/// Réunit la copie en conflit `copy_file` et `projet.json`, enregistre le
/// résultat (annulable par ⌘Z) et range la copie dans [`RESOLVED_DIR`].
///
/// `choices` donne, pour une clé de [`Difference`], la version à garder ;
/// sans choix, la suggestion, ou à défaut la version actuelle.
pub fn resolve(
    dir: &ProjectDir,
    copy_file: &str,
    choices: &BTreeMap<String, Side>,
) -> Result<Project, ProjectError> {
    let (pair, _) = load_pair(dir, copy_file)?;
    let report = compare(dir, copy_file)?;
    let side_of = |key: &str| {
        choices
            .get(key)
            .copied()
            .or_else(|| {
                report
                    .differences
                    .iter()
                    .find(|d| d.key == key)
                    .and_then(|d| d.suggested)
            })
            .unwrap_or(Side::Current)
    };
    let mut merged = pair.current.clone();
    for key in all_top_keys(&pair) {
        let value = if KEYED.contains(&key.as_str()) {
            let section = key.clone();
            let take_copy = |id: &str| side_of(&format!("{section}/{id}")) == Side::Copy;
            Value::Array(merge_keyed(
                &keyed(&pair.current, &key),
                &keyed(&pair.copy, &key),
                &take_copy,
            ))
        } else if is_union(pair.current.get(&key), pair.copy.get(&key)) {
            Value::Array(merge_union(
                &keyed(&pair.current, &key),
                &keyed(&pair.copy, &key),
            ))
        } else if side_of(&key) == Side::Copy {
            pair.copy.get(&key).cloned().unwrap_or(Value::Null)
        } else {
            pair.current.get(&key).cloned().unwrap_or(Value::Null)
        };
        if value.is_null() || value.as_array().is_some_and(Vec::is_empty) {
            merged.remove(&key);
        } else {
            merged.insert(key, value);
        }
    }
    let project: Project =
        serde_json::from_value(Value::Object(merged)).map_err(|e| ProjectError::InvalidJson {
            path: dir.project_file(),
            message: format!("la réunion des deux versions est incorrecte ({e})"),
        })?;
    dir.save(&project)?;
    let resolved = dir.root().join(RESOLVED_DIR);
    fs::create_dir_all(&resolved).map_err(ProjectError::io("de créer le dossier", &resolved))?;
    let from = dir.root().join(copy_file);
    let mut to = resolved.join(copy_file);
    let mut n = 2;
    while to.exists() {
        to = resolved.join(format!("{n} - {copy_file}"));
        n += 1;
    }
    fs::rename(&from, &to).map_err(ProjectError::io("de ranger", &from))?;
    Ok(project)
}

#[cfg(test)]
mod tests;

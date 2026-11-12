//! Annulation illimitée (tâche 4.3).
//!
//! Chaque enregistrement d'un projet ouvert avec un historique
//! ([`ProjectDir::with_history`]) garde le `projet.json` d'avant dans un
//! dossier de l'app (jamais dans le projet : rien ne passe par Dropbox). Toute
//! action qui modifie le projet, quel que soit l'écran, s'annule donc de la
//! même façon, et l'historique survit à la fermeture de l'app.
//!
//! Garde-fou : si `projet.json` a été changé par ailleurs depuis la dernière
//! action (l'autre Mac, par Dropbox), annuler écraserait ce changement ; le
//! moteur refuse alors et vide l'historique.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::ProjectError;
use super::model::{new_id, Project};
use super::store::{write_atomic, ProjectDir};
use crate::tiles::fnv1a_64;

/// Nom du fichier qui décrit les deux piles.
const STACK_FILE: &str = "pile.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    /// Ce que l'action a changé, en français (« les dessins posés »).
    label: String,
    /// Fichier qui contient le `projet.json` à remettre.
    snapshot: String,
    /// Empreinte du `projet.json` attendu sur le disque avant de le remettre.
    expected: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Stacks {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
}

/// Ce que l'on peut annuler et rétablir, pour les menus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryState {
    /// Ce que ⌘Z annulerait, ou rien.
    pub undo: Option<String>,
    /// Ce que ⇧⌘Z rétablirait, ou rien.
    pub redo: Option<String>,
    /// Nombre d'actions qu'on peut annuler.
    pub depth: usize,
}

#[derive(Debug)]
pub enum HistoryError {
    /// Rien à annuler ou à rétablir.
    Nothing {
        redo: bool,
    },
    /// Le projet a changé hors de l'app depuis la dernière action.
    ChangedElsewhere,
    /// Projet ouvert sans historique.
    NoHistory,
    Project(ProjectError),
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nothing { redo: false } => write!(f, "Rien à annuler."),
            Self::Nothing { redo: true } => write!(f, "Rien à rétablir."),
            Self::ChangedElsewhere => write!(
                f,
                "Le projet a été modifié ailleurs (l'autre Mac ?) depuis la dernière action : \
                 l'annulation écraserait ce changement. L'historique est vidé ; le projet reste tel quel."
            ),
            Self::NoHistory => write!(f, "Ce projet est ouvert sans historique."),
            Self::Project(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for HistoryError {}

impl From<ProjectError> for HistoryError {
    fn from(e: ProjectError) -> Self {
        Self::Project(e)
    }
}

/// Dossier de l'historique d'un projet, d'après son identifiant.
fn folder(root: &Path, text: &str) -> Option<PathBuf> {
    let value: Value = serde_json::from_str(text).ok()?;
    let id = value.get("id")?.as_str()?;
    // L'identifiant vient du fichier : on n'en garde que des caractères sûrs.
    let safe: String = id.chars().filter(char::is_ascii_alphanumeric).collect();
    (!safe.is_empty()).then(|| root.join(safe))
}

fn read_stacks(dir: &Path) -> Stacks {
    fs::read_to_string(dir.join(STACK_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn write_stacks(dir: &Path, stacks: &Stacks) -> Result<(), ProjectError> {
    let text = serde_json::to_string_pretty(stacks).expect("sérialisable");
    write_atomic(&dir.join(STACK_FILE), text.as_bytes())
}

fn drop_entries(dir: &Path, entries: &[Entry]) {
    for entry in entries {
        let _ = fs::remove_file(dir.join(&entry.snapshot));
    }
}

fn store_snapshot(dir: &Path, text: &str) -> Result<String, ProjectError> {
    let name = format!("{}.json", new_id());
    write_atomic(&dir.join(&name), text.as_bytes())?;
    Ok(name)
}

/// Ce qui a changé entre deux versions, d'après les champs de premier niveau.
pub(crate) fn describe(before: &str, after: &str) -> String {
    let parse = |t: &str| serde_json::from_str::<Value>(t).ok();
    let (Some(Value::Object(a)), Some(Value::Object(b))) = (parse(before), parse(after)) else {
        return "le projet".to_string();
    };
    let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut changed: Vec<&str> = keys
        .into_iter()
        .filter(|k| a.get(*k) != b.get(*k))
        .map(|k| match k.as_str() {
            "placements" => "les dessins posés",
            "layers" => "les calques",
            "projectors" => "les projecteurs",
            "facade" => "la façade",
            "fresco" => "la fresque",
            "export" => "les réglages d'export",
            "library" => "la bibliothèque",
            "facadeAssembly" => "l'assemblage de photos",
            "name" => "le nom du projet",
            _ => "le projet",
        })
        .collect();
    changed.dedup();
    match changed.as_slice() {
        [one] => one.to_string(),
        [first, second] => format!("{first} et {second}"),
        _ => "le projet".to_string(),
    }
}

/// Note un enregistrement : `before` est le `projet.json` qu'il remplace.
/// Une nouvelle action efface ce qu'on pouvait rétablir.
pub(crate) fn record(root: &Path, before: &str, after: &str) -> Result<(), ProjectError> {
    if before == after {
        return Ok(());
    }
    let Some(dir) = folder(root, after) else {
        return Ok(());
    };
    fs::create_dir_all(&dir).map_err(ProjectError::io("de créer le dossier", &dir))?;
    let mut stacks = read_stacks(&dir);
    let snapshot = store_snapshot(&dir, before)?;
    drop_entries(&dir, &stacks.redo);
    stacks.redo.clear();
    stacks.undo.push(Entry {
        label: describe(before, after),
        snapshot,
        expected: fnv1a_64(after.as_bytes()),
    });
    write_stacks(&dir, &stacks)
}

fn history_dir(dir: &ProjectDir) -> Result<(PathBuf, String), HistoryError> {
    let root = dir.history_root().ok_or(HistoryError::NoHistory)?;
    let file = dir.project_file();
    let current = fs::read_to_string(&file).map_err(ProjectError::io("de lire", &file))?;
    let folder = folder(root, &current).ok_or(HistoryError::NoHistory)?;
    Ok((folder, current))
}

/// Ce qu'on peut annuler et rétablir.
pub fn state(dir: &ProjectDir) -> Result<HistoryState, HistoryError> {
    let (folder, _) = history_dir(dir)?;
    let stacks = read_stacks(&folder);
    Ok(HistoryState {
        undo: stacks.undo.last().map(|e| e.label.clone()),
        redo: stacks.redo.last().map(|e| e.label.clone()),
        depth: stacks.undo.len(),
    })
}

/// Annule la dernière action (`redo` faux) ou rétablit la dernière annulée.
/// Rend le projet remis en place et ce qui a été annulé ou rétabli.
pub fn step(dir: &ProjectDir, redo: bool) -> Result<(Project, String), HistoryError> {
    let (folder, current) = history_dir(dir)?;
    let mut stacks = read_stacks(&folder);
    let entry = if redo {
        stacks.redo.pop()
    } else {
        stacks.undo.pop()
    }
    .ok_or(HistoryError::Nothing { redo })?;
    if entry.expected != fnv1a_64(current.as_bytes()) {
        let all: Vec<Entry> = stacks.undo.drain(..).chain(stacks.redo.drain(..)).collect();
        drop_entries(&folder, &all);
        drop_entries(&folder, std::slice::from_ref(&entry));
        write_stacks(&folder, &stacks)?;
        return Err(HistoryError::ChangedElsewhere);
    }
    let path = folder.join(&entry.snapshot);
    let restored = fs::read_to_string(&path).map_err(ProjectError::io("de lire", &path))?;
    // Le projet remis doit être lisible avant d'écraser celui en place.
    let project: Project =
        serde_json::from_str(&restored).map_err(|e| ProjectError::InvalidJson {
            path: path.clone(),
            message: format!("étape d'historique illisible ({e})"),
        })?;
    let back = Entry {
        label: entry.label.clone(),
        snapshot: store_snapshot(&folder, &current)?,
        expected: fnv1a_64(restored.as_bytes()),
    };
    write_atomic(&dir.project_file(), restored.as_bytes())?;
    let _ = fs::remove_file(&path);
    if redo {
        stacks.undo.push(back);
    } else {
        stacks.redo.push(back);
    }
    write_stacks(&folder, &stacks)?;
    // Relu par le chemin habituel (mise à niveau d'un format ancien comprise).
    let project = dir.load().map(|l| l.project).unwrap_or(project);
    Ok((project, entry.label))
}

#[cfg(test)]
mod tests;

//! Retrouver sur ce Mac le fichier lié par un `.ai` fait sur un autre.
//!
//! Le chemin absolu noté dans le `.ai` est celui du Mac qui l'a enregistré
//! (« /Users/marcsicard/Library/CloudStorage/Dropbox/… ») : il ne sert
//! presque jamais tel quel. Dans l'ordre (décision D-6.3-c) :
//!
//! 1. le dossier relatif noté par Illustrator (`../Algues/`), compté depuis
//!    le fichier `.ai` lui-même ;
//! 2. le chemin absolu, s'il existe ici ;
//! 3. un fichier du même nom dans le dossier du `.ai` et ses sous-dossiers,
//!    de préférence dans un dossier du même nom que celui du lien.
//!
//! Les noms sont comparés sans accents ni casse : le `.ai` les écrit tantôt
//! décomposés (« e » + accent grave), tantôt sans accents, et le disque
//! peut les écrire autrement.

use std::path::{Component, Path, PathBuf};

use super::PlacedImage;

/// Profondeur de recherche sous le dossier du `.ai`.
const SEARCH_DEPTH: usize = 5;

/// Le fichier lié par `image`, retrouvé à partir du `.ai` ; `None` s'il
/// n'est nulle part.
pub fn resolve_link(ai: &Path, image: &PlacedImage) -> Option<PathBuf> {
    let name = image.file_name();
    if name.is_empty() || is_temporary(&image.absolute) {
        return None;
    }
    let wanted = fold_name(name);
    if let Some(rel) = &image.relative_dir {
        let folder = normalize(&ai.join(rel.replace('\\', "/")));
        if let Some(found) = in_folder(&folder, &wanted) {
            return Some(found);
        }
    }
    let absolute = Path::new(&image.absolute);
    if absolute.is_absolute() && absolute.is_file() {
        return Some(absolute.to_path_buf());
    }
    let parent_name = image
        .absolute
        .rsplit(['/', '\\', ':'])
        .nth(1)
        .map(fold_name)
        .unwrap_or_default();
    let root = ai.parent()?;
    let mut candidates = Vec::new();
    search(root, &wanted, SEARCH_DEPTH, &mut candidates);
    candidates.sort();
    candidates
        .iter()
        .find(|c| {
            c.parent()
                .and_then(Path::file_name)
                .is_some_and(|n| fold_name(&n.to_string_lossy()) == parent_name)
        })
        .or_else(|| candidates.first())
        .cloned()
}

/// Fichier temporaire d'Illustrator (`/var/folders/…`) : jamais retrouvé.
pub(super) fn is_temporary(path: &str) -> bool {
    path.starts_with("/var/folders/") || path.starts_with("/private/var/folders/")
}

/// Le fichier de `folder` dont le nom, sans accents ni casse, vaut
/// `wanted`.
fn in_folder(folder: &Path, wanted: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(folder).ok()?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| fold_name(&n.to_string_lossy()) == wanted)
        })
        .collect();
    found.sort();
    found.into_iter().next()
}

fn search(folder: &Path, wanted: &str, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() {
            if depth > 0 && !name.ends_with(".insitu") {
                search(&path, wanted, depth - 1, out);
            }
        } else if kind.is_file() && fold_name(&name) == wanted {
            out.push(path);
        }
    }
}

/// Résout `.` et `..` sans toucher au disque.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Un nom sans accents ni casse : « Murène.png », « Mure\u{300}ne.png » et
/// « murene.png » donnent le même résultat.
pub fn fold_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        // Accents combinants (forme décomposée) : retirés.
        if ('\u{300}'..='\u{36f}').contains(&c) {
            continue;
        }
        let base = match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => {
                'a'
            }
            'ç' | 'Ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'i',
            'ñ' | 'Ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'u',
            'ý' | 'ÿ' | 'Ý' => 'y',
            'œ' | 'Œ' => {
                out.push_str("oe");
                continue;
            }
            'æ' | 'Æ' => {
                out.push_str("ae");
                continue;
            }
            other => other,
        };
        out.extend(base.to_lowercase());
    }
    out
}

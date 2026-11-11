//! Erreurs du modèle de projet, avec des messages en français destinés à
//! l'utilisateur.

use std::fmt;
use std::io;
use std::path::PathBuf;

use super::lock::LockInfo;

#[derive(Debug)]
pub enum ProjectError {
    /// Lecture ou écriture impossible. `action` complète la phrase
    /// « Impossible … » : « de lire », « d'enregistrer ».
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    /// Le dossier n'existe pas ou ne contient pas de `projet.json`.
    NotAProject { path: PathBuf },
    /// Le nom du dossier ne se termine pas par `.insitu`.
    BadExtension { path: PathBuf },
    /// Un projet existe déjà à cet endroit.
    AlreadyExists { path: PathBuf },
    /// Le dossier existe, n'est pas un projet et n'est pas vide.
    DirectoryNotEmpty { path: PathBuf },
    /// `projet.json` n'est pas du JSON, ou ne décrit pas un projet.
    InvalidJson { path: PathBuf, message: String },
    /// `projet.json` n'a pas de champ `formatVersion` exploitable.
    MissingFormatVersion { path: PathBuf },
    /// Le projet a été enregistré par une version plus récente de l'app.
    FormatTooRecent {
        path: PathBuf,
        found: u64,
        supported: u32,
    },
    /// Une étape de mise à niveau a échoué.
    Migration { from: u32, message: String },
    /// Un chemin de fichier n'est pas un chemin relatif acceptable.
    InvalidPath { value: String, reason: &'static str },
    /// Une date n'est pas au format `AAAA-MM-JJTHH:MM:SSZ`.
    InvalidTimestamp { value: String },
    /// Le modèle contient un nombre impossible à écrire (infini, indéfini).
    NonFiniteNumber { field: String },
    /// Le fichier à importer n'existe pas ou n'est pas un fichier.
    ImportSourceMissing { path: PathBuf },
    /// Le projet est ouvert par quelqu'un d'autre.
    Locked { holder: LockInfo },
    /// Le verrou qu'on tenait a été repris par quelqu'un d'autre.
    LockLost { holder: LockInfo },
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                action,
                path,
                source,
            } => write!(f, "Impossible {action} « {} » : {source}", path.display()),
            Self::NotAProject { path } => write!(
                f,
                "« {} » n'est pas un projet InSitu : le dossier n'existe pas ou ne contient pas de fichier projet.json.",
                path.display()
            ),
            Self::BadExtension { path } => write!(
                f,
                "Le nom d'un dossier projet doit se terminer par .insitu (reçu : « {} »).",
                path.display()
            ),
            Self::AlreadyExists { path } => write!(
                f,
                "Un projet existe déjà dans « {} ».",
                path.display()
            ),
            Self::DirectoryNotEmpty { path } => write!(
                f,
                "Le dossier « {} » existe déjà et n'est pas vide : choisir un autre nom.",
                path.display()
            ),
            Self::InvalidJson { path, message } => write!(
                f,
                "Le fichier « {} » est illisible : {message}",
                path.display()
            ),
            Self::MissingFormatVersion { path } => write!(
                f,
                "Le fichier « {} » n'indique pas de version de format (champ formatVersion) : ce n'est pas un projet InSitu.",
                path.display()
            ),
            Self::FormatTooRecent {
                path,
                found,
                supported,
            } => write!(
                f,
                "Le projet « {} » a été enregistré par une version plus récente d'InSitu (format {found}) ; cette version ne lit que jusqu'au format {supported}. Mettre l'app à jour pour l'ouvrir. Le projet n'a pas été modifié.",
                path.display()
            ),
            Self::Migration { from, message } => write!(
                f,
                "La mise à niveau du projet depuis le format {from} a échoué : {message}"
            ),
            Self::InvalidPath { value, reason } => {
                write!(f, "Chemin de fichier refusé « {value} » : {reason}.")
            }
            Self::InvalidTimestamp { value } => write!(
                f,
                "Date illisible « {value} » : le format attendu est AAAA-MM-JJTHH:MM:SSZ."
            ),
            Self::NonFiniteNumber { field } => write!(
                f,
                "Le projet ne peut pas être enregistré : la valeur de « {field} » n'est pas un nombre fini."
            ),
            Self::ImportSourceMissing { path } => write!(
                f,
                "Le fichier à importer « {} » est introuvable.",
                path.display()
            ),
            Self::Locked { holder } => write!(
                f,
                "Le projet est déjà ouvert par {} sur « {} » depuis le {} (dernier signe de vie : {}). Pas d'édition simultanée : attendre sa fermeture, ou forcer l'ouverture si l'on est sûr qu'il est fermé.",
                holder.user,
                holder.machine,
                holder.opened_at.to_iso8601(),
                holder.refreshed_at.to_iso8601()
            ),
            Self::LockLost { holder } => write!(
                f,
                "Le projet a été ouvert entre-temps par {} sur « {} ». Ne plus enregistrer depuis ce poste sans s'être mis d'accord.",
                holder.user, holder.machine
            ),
        }
    }
}

impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl ProjectError {
    /// Fabrique de fermeture pour `map_err` sur une erreur d'entrée-sortie.
    pub(crate) fn io(
        action: &'static str,
        path: impl Into<PathBuf>,
    ) -> impl FnOnce(io::Error) -> ProjectError {
        let path = path.into();
        move |source| ProjectError::Io {
            action,
            path,
            source,
        }
    }
}

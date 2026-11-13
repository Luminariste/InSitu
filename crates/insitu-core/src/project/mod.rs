//! Modèle de projet : le dossier `<nom>.insitu`.
//!
//! Un projet est un dossier partagé par Dropbox entre deux Mac. Il contient
//! un fichier de description lisible (`projet.json`) et des sous-dossiers
//! d'images. Tous les chemins écrits dans la description sont relatifs au
//! dossier. Le format est décrit dans `docs/format-projet.md`.
//!
//! - [`model`] : les données d'un projet (façade, projecteurs, fresque,
//!   réglages d'export) ;
//! - [`store`] : création, lecture, écriture atomique, import de fichiers ;
//! - [`migrate`] : numéro de version du format et mises à niveau ;
//! - [`lock`] : le verrou qui avertit quand l'autre personne a le projet
//!   ouvert ;
//! - [`validate`] : contrôle d'un projet et des fichiers qu'il référence.

pub mod error;
pub mod history;
pub mod lock;
pub mod migrate;
pub mod model;
pub mod path;
pub mod store;
pub mod time;
pub mod validate;

#[cfg(test)]
pub(crate) mod test_support;

pub use error::ProjectError;
pub use lock::{
    read_lock, LockIdentity, LockInfo, LockMode, LockOutcome, ProjectLock, LOCK_FILE,
    LOCK_REFRESH_INTERVAL_SECONDS, LOCK_STALE_AFTER_SECONDS,
};
pub use migrate::CURRENT_FORMAT_VERSION;
pub use model::{retouch_problem, silhouette_problem};
pub use model::{
    AssemblyOutput, AssemblyPair, AssemblyPhoto, Deformation, DeformationFrame, Drawing,
    ExportSettings, ExtraFields, Facade, FacadeAssembly, FacadeScale, FrescoImage, FrescoVector,
    Layer, Mirror, Placement, Plan, PlanGroup, PointMm, Project, Projector, RectMm, Rectification,
    RectificationMethod, RectificationPoint, RegistrationPair, RetouchStroke, RetouchTool,
    SilhouettePolygon, SlideAdjustment, SourcePoint, VectorSettings,
};
pub use path::RelativePath;
pub use store::{AssetKind, LoadedProject, ProjectDir, PROJECT_EXTENSION, PROJECT_FILE};
pub use time::Timestamp;
pub use validate::{validate, Issue, Severity, ValidationReport};

//! Les données d'un projet, telles qu'elles sont écrites dans `projet.json`.
//!
//! Conventions :
//! - les longueurs sont en millimètres (`f64`), origine en haut à gauche, y
//!   vers le bas ;
//! - les seules coordonnées qui ne sont pas des millimètres sont celles d'un
//!   point repéré dans un fichier importé ([`SourcePoint`]) : elles sont dans
//!   l'unité de ce fichier (pixels pour une image) ;
//! - les fichiers sont désignés par un chemin relatif au dossier projet.
//!
//! Champs inconnus : chaque objet du modèle garde dans `extra` les champs
//! qu'il ne connaît pas et les réécrit tels quels à l'enregistrement. Un
//! projet touché par une version qui ignore un champ ne le perd donc pas. Les
//! petits objets géométriques (points, rectangle) font exception : un champ
//! inconnu y est refusé à la lecture, pour ne jamais être perdu en silence.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::migrate::CURRENT_FORMAT_VERSION;
use super::path::RelativePath;
use super::time::Timestamp;
use crate::units;

/// Champs présents dans le fichier mais inconnus de cette version de l'app.
pub type ExtraFields = BTreeMap<String, serde_json::Value>;

/// Un point en millimètres, sur une diapositive ou sur la façade.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointMm {
    pub x: f64,
    pub y: f64,
}

impl PointMm {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Un point repéré dans un fichier importé, dans l'unité de ce fichier :
/// pixels pour une image (disposition des pixels du fichier, sans tenir
/// compte de la rotation EXIF), unité native pour un plan vectoriel.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePoint {
    pub x: f64,
    pub y: f64,
}

impl SourcePoint {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Un rectangle en millimètres : coin haut-gauche et dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RectMm {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Un projet : tout ce que contient `projet.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// Version du format ; voir [`super::migrate`].
    pub format_version: u32,
    /// Identifiant stable, attribué à la création, indépendant du nom.
    pub id: String,
    /// Nom affiché, par exemple « Épinal ».
    pub name: String,
    pub created_at: Timestamp,
    /// La façade ; absente tant qu'aucun plan n'a été importé.
    #[serde(default)]
    pub facade: Option<Facade>,
    #[serde(default)]
    pub projectors: Vec<Projector>,
    /// La fresque importée en une seule image (jalon 3).
    #[serde(default)]
    pub fresco: Option<FrescoImage>,
    #[serde(default)]
    pub export: ExportSettings,
    /// Bibliothèque de dessins (tâche 4.1). Absente du fichier tant qu'elle
    /// est vide.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub library: Vec<Drawing>,
    /// Dessins posés sur la façade (tâche 4.2), du plus au fond au plus en
    /// avant. Absent du fichier tant qu'il est vide.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placements: Vec<Placement>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl Project {
    /// Un projet vide, au format courant, avec un nouvel identifiant.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            format_version: CURRENT_FORMAT_VERSION,
            id: new_id(),
            name: name.into(),
            created_at: Timestamp::now(),
            facade: None,
            projectors: Vec::new(),
            fresco: None,
            export: ExportSettings::default(),
            library: Vec::new(),
            placements: Vec::new(),
            extra: ExtraFields::new(),
        }
    }

    pub fn drawing(&self, id: &str) -> Option<&Drawing> {
        self.library.iter().find(|d| d.id == id)
    }

    /// Ajoute un projecteur vide et le renvoie pour qu'on le remplisse.
    pub fn add_projector(&mut self, label: impl Into<String>) -> &mut Projector {
        let index = self.projectors.len();
        self.projectors.push(Projector::new(label));
        &mut self.projectors[index]
    }

    pub fn projector(&self, id: &str) -> Option<&Projector> {
        self.projectors.iter().find(|p| p.id == id)
    }

    pub fn projector_mut(&mut self, id: &str) -> Option<&mut Projector> {
        self.projectors.iter_mut().find(|p| p.id == id)
    }

    /// Tous les fichiers référencés, avec l'endroit du modèle qui les cite.
    pub fn referenced_files(&self) -> Vec<(String, &RelativePath)> {
        let mut files = Vec::new();
        if let Some(facade) = &self.facade {
            files.push(("facade.source".to_string(), &facade.source));
        }
        for (i, projector) in self.projectors.iter().enumerate() {
            if let Some(path) = &projector.raw_photo {
                files.push((format!("projectors[{i}].rawPhoto"), path));
            }
            if let Some(path) = &projector.rectified_photo {
                files.push((format!("projectors[{i}].rectifiedPhoto"), path));
            }
        }
        if let Some(fresco) = &self.fresco {
            files.push(("fresco.source".to_string(), &fresco.source));
        }
        for (i, drawing) in self.library.iter().enumerate() {
            files.push((format!("library[{i}].source"), &drawing.source));
        }
        files
    }

    /// Champs présents dans le fichier mais inconnus de cette version, sous
    /// la forme `projectors[0].truc`. Ils sont conservés à l'enregistrement.
    pub fn unknown_fields(&self) -> Vec<String> {
        let mut found = Vec::new();
        let mut collect = |prefix: &str, extra: &ExtraFields| {
            found.extend(extra.keys().map(|key| format!("{prefix}{key}")));
        };
        collect("", &self.extra);
        if let Some(facade) = &self.facade {
            collect("facade.", &facade.extra);
            if let Some(scale) = &facade.scale {
                collect("facade.scale.", &scale.extra);
            }
        }
        for (i, projector) in self.projectors.iter().enumerate() {
            collect(&format!("projectors[{i}]."), &projector.extra);
            if let Some(rectification) = &projector.rectification {
                collect(
                    &format!("projectors[{i}].rectification."),
                    &rectification.extra,
                );
                for (j, point) in rectification.points.iter().enumerate() {
                    collect(
                        &format!("projectors[{i}].rectification.points[{j}]."),
                        &point.extra,
                    );
                }
            }
            for (j, pair) in projector.registration.iter().enumerate() {
                collect(&format!("projectors[{i}].registration[{j}]."), &pair.extra);
            }
        }
        if let Some(fresco) = &self.fresco {
            collect("fresco.", &fresco.extra);
        }
        collect("export.", &self.export.extra);
        for (i, drawing) in self.library.iter().enumerate() {
            collect(&format!("library[{i}]."), &drawing.extra);
        }
        for (i, placement) in self.placements.iter().enumerate() {
            collect(&format!("placements[{i}]."), &placement.extra);
        }
        found
    }

    /// Nombres du modèle, avec leur emplacement. Sert à refuser
    /// l'enregistrement d'une valeur infinie ou indéfinie, que JSON ne sait
    /// pas écrire.
    pub(crate) fn numbers(&self) -> Vec<(String, f64)> {
        let mut out = Vec::new();
        if let Some(facade) = &self.facade {
            out.push(("facade.sourceWidth".to_string(), facade.source_width));
            out.push(("facade.sourceHeight".to_string(), facade.source_height));
            if let Some(origin) = &facade.origin {
                out.push(("facade.origin.x".to_string(), origin.x));
                out.push(("facade.origin.y".to_string(), origin.y));
            }
            if let Some(scale) = &facade.scale {
                out.push(("facade.scale.pointA.x".to_string(), scale.point_a.x));
                out.push(("facade.scale.pointA.y".to_string(), scale.point_a.y));
                out.push(("facade.scale.pointB.x".to_string(), scale.point_b.x));
                out.push(("facade.scale.pointB.y".to_string(), scale.point_b.y));
                out.push(("facade.scale.distanceMm".to_string(), scale.distance_mm));
            }
        }
        for (i, projector) in self.projectors.iter().enumerate() {
            if let Some(rectification) = &projector.rectification {
                let base = format!("projectors[{i}].rectification");
                if let Some(residual) = rectification.residual_mm {
                    out.push((format!("{base}.residualMm"), residual));
                }
                for (j, point) in rectification.points.iter().enumerate() {
                    out.push((format!("{base}.points[{j}].photo.x"), point.photo.x));
                    out.push((format!("{base}.points[{j}].photo.y"), point.photo.y));
                    out.push((format!("{base}.points[{j}].slide.x"), point.slide.x));
                    out.push((format!("{base}.points[{j}].slide.y"), point.slide.y));
                }
            }
            for (j, pair) in projector.registration.iter().enumerate() {
                let base = format!("projectors[{i}].registration[{j}]");
                out.push((format!("{base}.slide.x"), pair.slide.x));
                out.push((format!("{base}.slide.y"), pair.slide.y));
                out.push((format!("{base}.facade.x"), pair.facade.x));
                out.push((format!("{base}.facade.y"), pair.facade.y));
            }
        }
        if let Some(fresco) = &self.fresco {
            out.push(("fresco.frame.x".to_string(), fresco.frame.x));
            out.push(("fresco.frame.y".to_string(), fresco.frame.y));
            out.push(("fresco.frame.width".to_string(), fresco.frame.width));
            out.push(("fresco.frame.height".to_string(), fresco.frame.height));
        }
        out.push(("export.dpi".to_string(), self.export.dpi));
        for (i, drawing) in self.library.iter().enumerate() {
            out.push((format!("library[{i}].width"), drawing.width));
            out.push((format!("library[{i}].height"), drawing.height));
        }
        for (i, p) in self.placements.iter().enumerate() {
            out.push((format!("placements[{i}].center.x"), p.center.x));
            out.push((format!("placements[{i}].center.y"), p.center.y));
            out.push((format!("placements[{i}].width"), p.width));
            out.push((format!("placements[{i}].height"), p.height));
            out.push((format!("placements[{i}].rotation"), p.rotation));
        }
        out
    }
}

/// La façade : le fichier importé (plan d'architecte, déroulé ou photo
/// d'ensemble) et sa mise à l'échelle.
///
/// Le repère de la façade, en millimètres réels, est celui du fichier source
/// multiplié par l'échelle : origine au coin haut-gauche du fichier, y vers
/// le bas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Facade {
    pub source: RelativePath,
    /// Largeur du fichier source, dans son unité (pixels pour une image).
    pub source_width: f64,
    /// Hauteur du fichier source, dans son unité.
    pub source_height: f64,
    /// Mise à l'échelle ; absente tant qu'elle n'a pas été faite.
    #[serde(default)]
    pub scale: Option<FacadeScale>,
    /// Page retenue, à partir de 1, pour un PDF ; absente sinon (tâche 3.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Pour un DXF : coin haut-gauche du repère du fichier, en coordonnées du
    /// DXF (y vers le haut). Absent pour les autres formats (tâche 3.1,
    /// D-3.1-d).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<SourcePoint>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl Facade {
    /// Dimensions de la façade en millimètres, une fois mise à l'échelle.
    pub fn size_mm(&self) -> Option<(f64, f64)> {
        let factor = self.scale.as_ref()?.mm_per_source_unit()?;
        Some((self.source_width * factor, self.source_height * factor))
    }

    /// Passe d'un point du fichier source à un point de la façade en mm.
    pub fn to_mm(&self, point: SourcePoint) -> Option<PointMm> {
        let factor = self.scale.as_ref()?.mm_per_source_unit()?;
        Some(PointMm::new(point.x * factor, point.y * factor))
    }
}

/// Mise à l'échelle de la façade « par deux points et une cote » : deux
/// points repérés dans le fichier source et la distance réelle qui les
/// sépare.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FacadeScale {
    pub point_a: SourcePoint,
    pub point_b: SourcePoint,
    /// Distance réelle entre les deux points, en millimètres.
    pub distance_mm: f64,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl FacadeScale {
    /// Millimètres réels par unité du fichier source. `None` si les deux
    /// points sont confondus ou la cote nulle.
    pub fn mm_per_source_unit(&self) -> Option<f64> {
        let dx = self.point_b.x - self.point_a.x;
        let dy = self.point_b.y - self.point_a.y;
        let length = dx.hypot(dy);
        let factor = self.distance_mm / length;
        (length > 0.0 && factor.is_finite() && factor > 0.0).then_some(factor)
    }
}

/// Un rétroprojecteur et ce qu'on sait de lui.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Projector {
    /// Identifiant stable : ne change jamais, même si le libellé change.
    pub id: String,
    /// Libellé affiché et gravé, par exemple « A ».
    pub label: String,
    /// Photo de la mire en chambre noire.
    #[serde(default)]
    pub raw_photo: Option<RelativePath>,
    /// Photo redressée : elle couvre exactement le carré de la diapositive.
    #[serde(default)]
    pub rectified_photo: Option<RelativePath>,
    /// Comment la photo brute a été redressée.
    #[serde(default)]
    pub rectification: Option<Rectification>,
    /// Recalage : points communs entre la diapositive et la façade.
    #[serde(default)]
    pub registration: Vec<RegistrationPair>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl Projector {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            label: label.into(),
            raw_photo: None,
            rectified_photo: None,
            rectification: None,
            registration: Vec::new(),
            extra: ExtraFields::new(),
        }
    }
}

/// Qui a placé les points du redressement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RectificationMethod {
    /// Détection automatique de la mire.
    Automatic,
    /// Points posés ou corrigés à la main.
    Manual,
}

/// Résultat du redressement d'une photo brute : des correspondances entre la
/// photo et la mire. Quatre points (les coins) suffisent en mode manuel ; la
/// détection automatique en fournit davantage (intersections de la grille).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rectification {
    pub method: RectificationMethod,
    #[serde(default)]
    pub points: Vec<RectificationPoint>,
    /// Écart résiduel du redressement, en mm de diapositive, s'il est connu.
    #[serde(default)]
    pub residual_mm: Option<f64>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// Un point de la mire : où il est dans la photo brute, où il est sur la
/// diapositive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RectificationPoint {
    /// Position dans la photo brute, en pixels.
    pub photo: SourcePoint,
    /// Position sur la diapositive, en mm (0 à 280).
    pub slide: PointMm,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl RectificationPoint {
    pub fn new(photo: SourcePoint, slide: PointMm) -> Self {
        Self {
            photo,
            slide,
            extra: ExtraFields::new(),
        }
    }
}

/// Un point commun entre la diapositive d'un projecteur et la façade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationPair {
    /// Position sur la diapositive, en mm (0 à 280).
    pub slide: PointMm,
    /// Position sur la façade, en mm réels.
    pub facade: PointMm,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl RegistrationPair {
    pub fn new(slide: PointMm, facade: PointMm) -> Self {
        Self {
            slide,
            facade,
            extra: ExtraFields::new(),
        }
    }
}

/// La fresque importée en une seule image, et le rectangle de façade qu'elle
/// occupe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrescoImage {
    pub source: RelativePath,
    /// Rectangle occupé sur la façade, en mm réels.
    pub frame: RectMm,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// Un dessin de la bibliothèque : un fichier copié dans le dossier
/// `bibliotheque/` du projet (tâche 4.1). Son fond noir est rendu
/// transparent à l'affichage ; le fichier n'est jamais modifié.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Drawing {
    /// Identifiant stable, que citeront les placements sur la façade.
    pub id: String,
    /// Nom affiché : celui du fichier, sans son extension.
    pub name: String,
    /// Le fichier, par exemple `bibliotheque/Oiseaux/merle.png`. Le dossier
    /// d'origine est conservé sous `bibliotheque/`.
    pub source: RelativePath,
    /// Taille du fichier dans son unité : pixels pour une image, pixels CSS
    /// (1/96 de pouce) pour un SVG.
    pub width: f64,
    pub height: f64,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl Drawing {
    /// Dossier du dessin dans la bibliothèque, sans `bibliotheque/` ni le
    /// nom du fichier : `Oiseaux/Grands`, ou vide.
    pub fn folder(&self) -> &str {
        let path = self.source.as_str();
        let inside = path.split_once('/').map_or("", |(_, rest)| rest);
        inside.rsplit_once('/').map_or("", |(folder, _)| folder)
    }
}

/// Un dessin posé sur la façade (tâche 4.2).
///
/// Le dessin est d'abord mis à la taille `width` × `height` (mm réels), son
/// centre à l'origine ; il est ensuite retourné de gauche à droite si
/// `mirrored`, tourné de `rotation` degrés (sens des aiguilles d'une montre à
/// l'écran, puisque y descend), puis posé en `center`. Une symétrie de haut
/// en bas s'écrit `mirrored` plus un demi-tour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    /// Identifiant stable de ce placement.
    pub id: String,
    /// Le dessin posé : identifiant d'un dessin de la bibliothèque.
    pub drawing: String,
    /// Centre du dessin sur la façade, en mm réels.
    pub center: PointMm,
    /// Largeur et hauteur du dessin posé, en mm réels, avant rotation.
    pub width: f64,
    pub height: f64,
    /// Rotation en degrés, sens des aiguilles d'une montre.
    #[serde(default)]
    pub rotation: f64,
    /// Retourné de gauche à droite.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mirrored: bool,
    /// Groupe auquel appartient le dessin ; les dessins d'un même groupe se
    /// sélectionnent et se transforment ensemble.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl Placement {
    /// Point de la façade où tombe le point (`u`, `v`) du dessin, en
    /// fractions de sa largeur et de sa hauteur : (0, 0) est le coin
    /// haut-gauche du fichier, (1, 1) le coin bas-droit.
    pub fn facade_point(&self, u: f64, v: f64) -> PointMm {
        let mut x = (u - 0.5) * self.width;
        let y = (v - 0.5) * self.height;
        if self.mirrored {
            x = -x;
        }
        let (sin, cos) = self.rotation.to_radians().sin_cos();
        PointMm::new(
            self.center.x + x * cos - y * sin,
            self.center.y + x * sin + y * cos,
        )
    }

    /// Les quatre coins du fichier sur la façade : haut-gauche, haut-droit,
    /// bas-droit, bas-gauche (coins du fichier, pas de l'écran).
    pub fn corners(&self) -> [PointMm; 4] {
        [
            self.facade_point(0.0, 0.0),
            self.facade_point(1.0, 0.0),
            self.facade_point(1.0, 1.0),
            self.facade_point(0.0, 1.0),
        ]
    }

    /// Rectangle droit qui contient le dessin posé.
    pub fn bounds(&self) -> RectMm {
        let corners = self.corners();
        let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
        let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for c in corners {
            x0 = x0.min(c.x);
            y0 = y0.min(c.y);
            x1 = x1.max(c.x);
            y1 = y1.max(c.y);
        }
        RectMm {
            x: x0,
            y: y0,
            width: x1 - x0,
            height: y1 - y0,
        }
    }
}

/// Réglages d'export des diapositives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExportSettings {
    /// Résolution, en points par pouce.
    pub dpi: f64,
    /// Image retournée gauche-droite.
    pub mirror: bool,
    /// Noir et blanc inversés.
    pub negative: bool,
    /// Image de la plaque entière (300 mm), nom et repères gravés dans la
    /// marge de 10 mm ; sinon, l'image de 280 mm seule (tâche 3.5).
    pub margin: bool,
    /// PNG en noir et blanc pur (1 bit), seuillé à `threshold` ; sinon en
    /// niveaux de gris (tâche 3.5).
    pub one_bit: bool,
    /// Seuil du noir et blanc, de 1 à 255 : un pixel au moins aussi clair
    /// devient blanc.
    pub threshold: u8,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            dpi: units::DEFAULT_EXPORT_DPI,
            mirror: units::DEFAULT_EXPORT_MIRROR,
            negative: units::DEFAULT_EXPORT_NEGATIVE,
            margin: units::DEFAULT_EXPORT_MARGIN,
            one_bit: false,
            threshold: units::DEFAULT_EXPORT_THRESHOLD,
            extra: ExtraFields::new(),
        }
    }
}

/// Fabrique un identifiant de 16 caractères hexadécimaux, tiré au hasard.
/// Sans dépendance externe : l'aléa vient de la graine que la bibliothèque
/// standard tire pour ses tables de hachage, mêlée à l'heure et à un compteur.
pub(crate) fn new_id() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(nanos);
    hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    hasher.write_u32(std::process::id());
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_project_uses_the_reference_export_settings() {
        let project = Project::new("Essai");
        assert_eq!(project.format_version, CURRENT_FORMAT_VERSION);
        assert_eq!(project.export.dpi, 1400.0);
        assert!(!project.export.mirror);
        assert!(!project.export.negative);
        assert!(project.projectors.is_empty());
    }

    #[test]
    fn identifiers_are_distinct_and_stable_in_shape() {
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..500 {
            let id = new_id();
            assert_eq!(id.len(), 16);
            assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
            assert!(seen.insert(id), "identifiant tiré deux fois");
        }
    }

    #[test]
    fn renaming_a_projector_keeps_its_identifier() {
        let mut project = Project::new("Essai");
        let id = project.add_projector("A").id.clone();
        if let Some(projector) = project.projector_mut(&id) {
            projector.label = "Gauche".to_string();
        }
        assert_eq!(
            project.projector(&id).map(|p| p.label.as_str()),
            Some("Gauche")
        );
    }

    #[test]
    fn facade_scale_gives_millimetres() {
        let facade = Facade {
            source: RelativePath::new("facade/plan.png").expect("chemin"),
            source_width: 1000.0,
            source_height: 500.0,
            scale: Some(FacadeScale {
                point_a: SourcePoint::new(100.0, 100.0),
                point_b: SourcePoint::new(400.0, 500.0),
                distance_mm: 2500.0,
                extra: ExtraFields::new(),
            }),
            page: None,
            origin: None,
            extra: ExtraFields::new(),
        };
        // 300-400-500 : 500 unités pour 2 500 mm, soit 5 mm par unité.
        assert_eq!(facade.size_mm(), Some((5000.0, 2500.0)));
        assert_eq!(
            facade.to_mm(SourcePoint::new(10.0, 20.0)),
            Some(PointMm::new(50.0, 100.0))
        );
    }

    #[test]
    fn a_degenerate_scale_gives_nothing() {
        let scale = FacadeScale {
            point_a: SourcePoint::new(1.0, 1.0),
            point_b: SourcePoint::new(1.0, 1.0),
            distance_mm: 100.0,
            extra: ExtraFields::new(),
        };
        assert_eq!(scale.mm_per_source_unit(), None);
    }

    #[test]
    fn missing_optional_parts_take_their_defaults() {
        let json = r#"{
            "formatVersion": 1,
            "id": "0123456789abcdef",
            "name": "Minimal",
            "createdAt": "2026-11-11T10:00:00Z",
            "export": { "mirror": true }
        }"#;
        let project: Project = serde_json::from_str(json).expect("projet minimal");
        assert!(project.facade.is_none());
        assert!(project.projectors.is_empty());
        assert_eq!(project.export.dpi, 1400.0);
        assert!(project.export.mirror);
        assert!(project.unknown_fields().is_empty());
    }

    #[test]
    fn an_unknown_field_in_a_point_is_refused_not_dropped() {
        let json = r#"{ "x": 1.0, "y": 2.0, "z": 3.0 }"#;
        assert!(serde_json::from_str::<PointMm>(json).is_err());
    }
}

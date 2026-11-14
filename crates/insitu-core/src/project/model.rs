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
    /// Calques de la composition (tâche 4.3), du fond vers l'avant. Absent
    /// tant qu'il n'y en a pas : tous les dessins sont alors sur un seul
    /// calque implicite.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<Layer>,
    /// Assemblage de photos de façade (tâche 6.4). Absent tant qu'aucune
    /// photo n'a été ajoutée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facade_assembly: Option<FacadeAssembly>,
    /// Plans ajoutés à l'espace de travail de la façade, par-dessus le plan
    /// de référence (`facade`), du fond vers l'avant (demande de Benjamin,
    /// 13 novembre 2026, D-PL-a). Absent tant qu'il n'y en a pas.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan_groups: Vec<PlanGroup>,
    /// Dossiers de la bibliothèque créés à la main (`Oiseaux/Mer`), gardés
    /// même vides ; les autres dossiers viennent des dessins eux-mêmes
    /// (demande de Benjamin, 13 novembre 2026, D-4.1-g). Absent tant qu'il
    /// n'y en a pas.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub library_folders: Vec<String>,
    /// Dessins copiés depuis la bibliothèque commune, ou envoyés vers elle
    /// (demande de Benjamin, 13 novembre 2026, D-4.1-i) : de quel dessin
    /// commun chacun vient, et dans quel état il a été copié. Absent tant
    /// qu'il n'y en a pas.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_links: Vec<SharedLink>,
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
            layers: Vec::new(),
            facade_assembly: None,
            plan_groups: Vec::new(),
            library_folders: Vec::new(),
            shared_links: Vec::new(),
            extra: ExtraFields::new(),
        }
    }

    pub fn plan_group(&self, id: &str) -> Option<&PlanGroup> {
        self.plan_groups.iter().find(|g| g.id == id)
    }

    pub fn plan_group_mut(&mut self, id: &str) -> Option<&mut PlanGroup> {
        self.plan_groups.iter_mut().find(|g| g.id == id)
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
            if let Some(vector) = &fresco.vector {
                files.push(("fresco.vector.source".to_string(), &vector.source));
            }
        }
        for (i, drawing) in self.library.iter().enumerate() {
            files.push((format!("library[{i}].source"), &drawing.source));
        }
        if let Some(assembly) = &self.facade_assembly {
            for (i, photo) in assembly.photos.iter().enumerate() {
                files.push((format!("facadeAssembly.photos[{i}].source"), &photo.source));
            }
        }
        for (i, group) in self.plan_groups.iter().enumerate() {
            files.push((format!("planGroups[{i}].source"), &group.source));
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
        for (i, layer) in self.layers.iter().enumerate() {
            collect(&format!("layers[{i}]."), &layer.extra);
        }
        for (i, group) in self.plan_groups.iter().enumerate() {
            collect(&format!("planGroups[{i}]."), &group.extra);
            for (j, plan) in group.plans.iter().enumerate() {
                collect(&format!("planGroups[{i}].plans[{j}]."), &plan.extra);
            }
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
        for (i, group) in self.plan_groups.iter().enumerate() {
            out.push((format!("planGroups[{i}].origin.x"), group.origin.x));
            out.push((format!("planGroups[{i}].origin.y"), group.origin.y));
            out.push((format!("planGroups[{i}].mmPerUnit"), group.mm_per_unit));
            for (j, plan) in group.plans.iter().enumerate() {
                out.push((
                    format!("planGroups[{i}].plans[{j}].sourceWidth"),
                    plan.source_width,
                ));
                out.push((
                    format!("planGroups[{i}].plans[{j}].sourceHeight"),
                    plan.source_height,
                ));
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
/// Le repère de la façade, en millimètres réels, est celui du plan tel
/// qu'il est affiché (fichier source tourné de `rotation` quarts de tour)
/// multiplié par l'échelle : origine au coin haut-gauche, y vers le bas.
/// Les dimensions et les points de la mise à l'échelle sont dans ce plan
/// tourné (D15-k).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Facade {
    pub source: RelativePath,
    /// Largeur du plan affiché, dans l'unité du fichier (pixels pour une
    /// image) : celle du fichier, ou sa hauteur s'il est tourné d'un quart
    /// de tour.
    pub source_width: f64,
    /// Hauteur du plan affiché, dans l'unité du fichier.
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
    /// Quarts de tour, dans le sens des aiguilles d'une montre, appliqués au
    /// fichier pour l'afficher (0 à 3 ; 0 si absent).
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub rotation: u8,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

fn is_zero_u8(value: &u8) -> bool {
    *value == 0
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

/// Un groupe de plans : un même fichier (PDF, `.ai`, image, SVG, DXF) posé
/// sur la façade, dont on montre une ou plusieurs vues (pages ou calques).
/// Toutes ses vues partagent sa place et son échelle : tirées du même
/// fichier, elles restent calées entre elles (D-PL-b).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanGroup {
    /// Identifiant stable.
    pub id: String,
    /// Nom affiché ; celui du fichier, sans son extension, à l'import.
    pub name: String,
    /// Le fichier, sous `facade/plans/`.
    pub source: RelativePath,
    /// Coin haut-gauche des vues, en mm de façade.
    pub origin: PointMm,
    /// Millimètres de façade par unité du fichier.
    pub mm_per_unit: f64,
    /// Quarts de tour, sens des aiguilles d'une montre (0 à 3).
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub rotation: u8,
    /// Groupe caché : aucune de ses vues n'est montrée.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    /// Les vues du fichier, du fond vers l'avant.
    pub plans: Vec<Plan>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// Une vue d'un groupe : une page, éventuellement réduite à certains
/// calques.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub id: String,
    /// Nom affiché : le calque ou la page, par exemple « Plan simplifié ».
    pub name: String,
    /// Page d'un PDF, à partir de 1 ; absente sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Calques montrés d'un PDF ; absent : tous.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<String>>,
    /// Dimensions de la vue affichée (rotation du groupe appliquée), dans
    /// l'unité du fichier.
    pub source_width: f64,
    pub source_height: f64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

impl PlanGroup {
    /// Taille du groupe sur la façade, en mm : celle de sa première vue.
    pub fn size_mm(&self) -> (f64, f64) {
        self.plans.first().map_or((0.0, 0.0), |plan| {
            (
                plan.source_width * self.mm_per_unit,
                plan.source_height * self.mm_per_unit,
            )
        })
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
    /// Recalage par lignes (D-3.2-k) : traits communs entre la diapositive
    /// et la façade (bord de fenêtre, corniche). L'écran s'en sert pour
    /// placer les coins guides ; le calcul de l'emprise reste fait sur les
    /// paires de points.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub registration_lines: Vec<RegistrationLine>,
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
            registration_lines: Vec::new(),
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
    /// Miroir appliqué à la diapositive (D-2.5-j) : les coordonnées `slide`
    /// des points en tiennent déjà compte ; ce réglage sert à le garder
    /// quand la mire est détectée de nouveau. Absent : aucun miroir.
    #[serde(default, skip_serializing_if = "Mirror::is_none")]
    pub mirror: Mirror,
    /// Quarts de tour de la diapositive dans le sens des aiguilles d'une
    /// montre (0 à 3), appliqués après le miroir (D-2.5-m) : comme pour le
    /// miroir, les points en tiennent déjà compte, et le réglage est
    /// réappliqué quand la mire est détectée de nouveau. Absent : 0.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub rotation: u8,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// Retournement de la diapositive par rapport à la mire photographiée.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mirror {
    /// Gauche et droite échangées.
    #[serde(default)]
    pub horizontal: bool,
    /// Haut et bas échangés.
    #[serde(default)]
    pub vertical: bool,
}

impl Mirror {
    pub fn is_none(&self) -> bool {
        !self.horizontal && !self.vertical
    }

    /// Image d'un point de la diapositive (mm) par ce miroir.
    pub fn apply(&self, p: PointMm, size_mm: f64) -> PointMm {
        PointMm::new(
            if self.horizontal { size_mm - p.x } else { p.x },
            if self.vertical { size_mm - p.y } else { p.y },
        )
    }
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

/// Une ligne commune entre la diapositive d'un projecteur et la façade :
/// deux points de chaque côté, qui ne se correspondent pas forcément (seule
/// la droite compte).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationLine {
    /// Deux points de la ligne sur la diapositive, en mm (0 à 280).
    pub slide: [PointMm; 2],
    /// Deux points de la ligne sur la façade, en mm réels.
    pub facade: [PointMm; 2],
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// La fresque importée en une seule image, et le rectangle de façade qu'elle
/// occupe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrescoImage {
    /// L'image affichée et lue par les aperçus. Pour une fresque vectorielle
    /// (PDF, Illustrator, SVG), c'est son rendu en niveaux de gris.
    pub source: RelativePath,
    /// Rectangle occupé sur la façade, en mm réels.
    pub frame: RectMm,
    /// Le fichier vectoriel d'origine, s'il y en a un : l'export des
    /// diapositives le relit à la résolution de chaque diapositive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<FrescoVector>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// Le fichier vectoriel d'origine d'une fresque.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrescoVector {
    /// Le fichier copié dans `fresque/` : PDF, Illustrator (`.ai`) ou SVG.
    pub source: RelativePath,
    /// Page lue (à partir de 1), pour un PDF ou un `.ai` ; absente : la
    /// première.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
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
    /// Réglages de vectorisation propres à ce dessin (tâche 5.2) ; absent :
    /// réglages par défaut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorSettings>,
    /// Silhouette retouchée à la main (tâche 4.7) : polygones en fractions
    /// du dessin. Absente : la silhouette est détectée automatiquement ;
    /// liste vide : le dessin n'en a pas (il ne cache rien).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub silhouette: Option<Vec<SilhouettePolygon>>,
    /// Gomme et crayon (tâche 4.8) : retouches posées sur le dessin, dans
    /// l'ordre où elles ont été faites. Vide : le dessin tel qu'importé.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retouch: Vec<RetouchStroke>,
    /// Dossier choisi dans la bibliothèque (`Oiseaux/Mer`, vide : à la
    /// racine), quand le dessin a été rangé à la main (D-4.1-g). Absent : le
    /// dossier où le fichier a été copié. Le fichier, lui, ne bouge pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// L'outil d'une retouche : la gomme peint du noir (le trait disparaît), le
/// crayon du blanc (un trait apparaît).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RetouchTool {
    Erase,
    Draw,
}

/// Une retouche à la gomme ou au crayon (tâche 4.8), en fractions du dessin
/// comme la silhouette : elle suit le dessin à toute résolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetouchStroke {
    pub tool: RetouchTool,
    /// Vrai : une zone, le polygone `points` rempli ; faux : un trait qui
    /// passe par `points`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fill: bool,
    /// Épaisseur d'un trait, en fraction du plus grand côté du dessin
    /// (ignorée pour une zone).
    #[serde(default)]
    pub size: f64,
    /// Points `[u, v]` en fractions du dessin.
    pub points: Vec<[f64; 2]>,
}

/// Ce qui ne va pas dans des retouches, s'il y a quelque chose.
pub fn retouch_problem(strokes: &[RetouchStroke]) -> Option<String> {
    for (i, stroke) in strokes.iter().enumerate() {
        let n = i + 1;
        if stroke.points.is_empty() {
            return Some(format!("la retouche {n} n'a aucun point"));
        }
        if stroke.fill && stroke.points.len() < 3 {
            return Some(format!("la zone {n} a moins de trois points"));
        }
        if !stroke.fill && !(stroke.size.is_finite() && stroke.size > 0.0 && stroke.size <= 1.0) {
            return Some(format!("le trait {n} a une épaisseur invalide"));
        }
        if stroke
            .points
            .iter()
            .flatten()
            .any(|c| !c.is_finite() || !(-1.0..=2.0).contains(c))
        {
            return Some(format!(
                "la retouche {n} a un point hors du dessin ou invalide"
            ));
        }
    }
    None
}

/// Un polygone de silhouette : points `[u, v]` en fractions du dessin,
/// (0, 0) en haut à gauche, (1, 1) en bas à droite ; le dernier est relié
/// au premier.
pub type SilhouettePolygon = Vec<[f64; 2]>;

/// Ce qui ne va pas dans une silhouette, s'il y a quelque chose.
pub fn silhouette_problem(polygons: &[SilhouettePolygon]) -> Option<String> {
    for (i, polygon) in polygons.iter().enumerate() {
        if polygon.len() < 3 {
            return Some(format!("le contour {} a moins de trois points", i + 1));
        }
        if polygon
            .iter()
            .flatten()
            .any(|c| !c.is_finite() || !(-1.0..=2.0).contains(c))
        {
            return Some(format!(
                "le contour {} a un point hors du dessin ou invalide",
                i + 1
            ));
        }
    }
    None
}

/// Assemblage de plusieurs photos de façade en une seule image (tâche 6.4).
/// La première photo est la **référence** : l'image assemblée est dans son
/// repère (ses pixels), agrandi pour contenir les autres. Chaque autre photo
/// est recalée sur une photo déjà placée (son **appui**) par des paires de
/// points communs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FacadeAssembly {
    pub photos: Vec<AssemblyPhoto>,
    /// La dernière image assemblée ; absente tant qu'elle n'a pas été faite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<AssemblyOutput>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// L'image assemblée et sa place dans le repère de la photo de référence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssemblyOutput {
    /// Le fichier, `facade/assemblage.jpg`.
    pub source: RelativePath,
    /// Coin haut-gauche de l'image, en pixels de la photo de référence.
    pub origin: SourcePoint,
    /// Pixels de l'image par pixel de la photo de référence (1 sauf
    /// réduction).
    pub scale: f64,
}

/// Une photo de l'assemblage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyPhoto {
    pub id: String,
    /// Nom affiché : celui du fichier, sans son extension.
    pub name: String,
    /// Le fichier, sous `facade/assemblage/`.
    pub source: RelativePath,
    /// Taille du fichier en pixels (pixels tels qu'enregistrés : l'orientation
    /// EXIF n'est pas appliquée, comme dans la visionneuse).
    pub width: f64,
    pub height: f64,
    /// Photo d'appui (son identifiant) ; absente pour la référence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    /// Points communs avec la photo d'appui.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pairs: Vec<AssemblyPair>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// Un point commun entre une photo (`here`) et sa photo d'appui (`there`),
/// en pixels de chacune.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssemblyPair {
    pub here: SourcePoint,
    pub there: SourcePoint,
}

/// Réglages de vectorisation d'un dessin (tâche 5.2). Les mêmes que ceux du
/// moteur de vectorisation (tâche 5.1), sans le nombre de fils de calcul.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VectorSettings {
    /// Seuil, de 1 à 255 : un pixel au moins aussi clair est dans le trait.
    pub threshold: u8,
    /// Écart maximal toléré entre les courbes et le contour de l'image, en
    /// pixels du dessin.
    pub tolerance_px: f64,
    /// Au-delà de cet angle (degrés), un sommet est un angle vif.
    pub corner_angle_deg: f64,
    /// Taches et trous plus petits que cette aire (pixels carrés) ignorés.
    pub min_area_px: f64,
    /// Lissage des contours avant l'ajustement, en pixels ; 0 : aucun
    /// (D-5.2-g). Absent du fichier quand il vaut 0.
    #[serde(skip_serializing_if = "is_zero")]
    pub smoothing_px: f64,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

impl Default for VectorSettings {
    fn default() -> Self {
        Self {
            threshold: 128,
            tolerance_px: 0.3,
            corner_angle_deg: 70.0,
            min_area_px: 0.0,
            smoothing_px: 0.0,
        }
    }
}

/// Lien entre un dessin du projet et un dessin de la bibliothèque commune
/// (D-4.1-i).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedLink {
    /// Le dessin du projet.
    pub drawing: String,
    /// Identifiant de la bibliothèque commune (son `projet.json`).
    pub library: String,
    /// Le dessin dans la bibliothèque commune.
    pub origin: String,
    /// Empreinte du dessin commun au moment de la copie : fichier, gomme et
    /// crayon, silhouette, réglages des vecteurs.
    pub fingerprint: String,
}

impl Drawing {
    /// Dossier du dessin dans la bibliothèque, sans `bibliotheque/` ni le
    /// nom du fichier : `Oiseaux/Grands`, ou vide. Celui choisi à la main
    /// s'il y en a un, sinon celui du fichier.
    pub fn folder(&self) -> &str {
        match &self.folder {
            Some(folder) => folder,
            None => self.file_folder(),
        }
    }

    /// Dossier où le fichier du dessin a été copié, sous `bibliotheque/`.
    pub fn file_folder(&self) -> &str {
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
    /// Calque du dessin (tâche 4.3) ; absent : le premier calque.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    /// Déformation libre (tâche 4.4) ; absente : le dessin reste droit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deformation: Option<Deformation>,
    /// Diapositive choisie à la main (tâche 4.5) : identifiant d'un
    /// projecteur. Absente, ou projecteur inconnu : la diapositive proposée
    /// par le moteur (`crate::placement::assign`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projector: Option<String>,
    /// Retouche du dessin dans une diapositive (tâche 4.6), sans toucher à
    /// la façade ; absente : aucune retouche.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjustment: Option<SlideAdjustment>,
    /// Posé dans la diapositive de ce projecteur, sans passer par la façade
    /// (D-3.6-g) : centre, taille, rotation et déformation sont alors en mm
    /// de diapositive. Le dessin n'est pas sur la façade ; il est gravé dans
    /// cette diapositive, recalée ou non.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slide: Option<String>,
    /// Masqué dans l'arborescence des diapositives : ni affiché, ni exporté,
    /// comme un dessin d'un calque masqué (D-4.5-g).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    /// Verrouillé : affiché et exporté, mais on ne peut plus le choisir sur
    /// la façade (D-4.5-g).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// Retouche locale d'un dessin posé dans la diapositive d'un projecteur
/// (tâche 4.6) : une grille de déformation, comme la déformation libre,
/// mais tendue en mm de diapositive. Elle ne vaut que pour ce projecteur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideAdjustment {
    /// Le projecteur dont la diapositive est retouchée.
    pub projector: String,
    /// La grille ; son cadre est en mm de diapositive.
    pub deformation: Deformation,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// Déformation libre d'un dessin posé (tâche 4.4) : une grille tendue sur
/// un cadre de la façade. Calculs dans `crate::placement::mesh`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deformation {
    pub frame: DeformationFrame,
    /// Nombre de cases ; une seule case : quatre coins en perspective.
    pub columns: u32,
    pub rows: u32,
    /// Position de chaque nœud en fractions du cadre, ligne par ligne :
    /// `(columns + 1) × (rows + 1)` couples ; au repos, le nœud (i, j) est
    /// en (i / columns, j / rows).
    pub nodes: Vec<[f64; 2]>,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

/// Rectangle tourné de la façade sur lequel la grille est tendue ; il suit
/// le dessin (ou le groupe) quand on le déplace, le tourne ou l'agrandit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeformationFrame {
    pub center: PointMm,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub rotation: f64,
}

/// Un calque de la composition (tâche 4.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub id: String,
    pub name: String,
    /// Faux : ses dessins ne sont pas montrés.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub visible: bool,
    /// Vrai : ses dessins ne se choisissent plus à la souris.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
    #[serde(flatten)]
    pub extra: ExtraFields,
}

fn yes() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

impl Project {
    /// Les dessins posés dans l'ordre d'empilement réel : calque par calque
    /// (du fond vers l'avant), puis dans l'ordre de la liste. Un dessin sans
    /// calque, ou d'un calque inconnu, compte dans le premier.
    pub fn stacked_placements(&self) -> Vec<&Placement> {
        let rank = |p: &Placement| {
            p.layer
                .as_deref()
                .and_then(|id| self.layers.iter().position(|l| l.id == id))
                .unwrap_or(0)
        };
        let mut out: Vec<&Placement> = self.placements.iter().collect();
        out.sort_by_key(|p| rank(p));
        out
    }

    /// Les dessins posés à exporter : dans l'ordre d'affichage, sans ceux
    /// des calques masqués ni ceux masqués un à un (un dessin sans calque,
    /// ou sur un calque inconnu, suit le premier calque).
    pub fn visible_placements(&self) -> Vec<&Placement> {
        let shown = |p: &Placement| {
            let layer = p
                .layer
                .as_deref()
                .and_then(|id| self.layers.iter().find(|l| l.id == id))
                .or(self.layers.first());
            !p.hidden && layer.is_none_or(|l| l.visible)
        };
        self.stacked_placements()
            .into_iter()
            .filter(|p| shown(p))
            .collect()
    }

    /// Les dessins posés visibles qui sont sur la façade : sans ceux posés
    /// directement dans une diapositive (D-3.6-g), dont la position est en
    /// mm de diapositive.
    pub fn facade_placements(&self) -> Vec<&Placement> {
        self.visible_placements()
            .into_iter()
            .filter(|p| p.slide.is_none())
            .collect()
    }
}

impl Placement {
    /// Point de la façade où tombe le point (`u`, `v`) du dessin, en
    /// fractions de sa largeur et de sa hauteur : (0, 0) est le coin
    /// haut-gauche du fichier, (1, 1) le coin bas-droit.
    pub fn facade_point(&self, u: f64, v: f64) -> PointMm {
        let p = self.rigid_point(u, v);
        match &self.deformation {
            Some(d) => crate::placement::mesh::forward(d, p),
            None => p,
        }
    }

    /// Comme [`Self::facade_point`], sans la déformation libre.
    pub fn rigid_point(&self, u: f64, v: f64) -> PointMm {
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

    /// Inverse de [`Self::facade_point`] : la position (`u`, `v`) dans le
    /// dessin, en fractions de sa largeur et de sa hauteur, du point `p` de
    /// la façade. Hors du dessin, `u` ou `v` sort de [0, 1] ; `None` si la
    /// déformation n'a pas d'antécédent pour ce point.
    /// La retouche du dessin dans la diapositive du projecteur `projector`,
    /// s'il en a une (tâche 4.6).
    pub fn adjustment_for(&self, projector: &str) -> Option<&Deformation> {
        self.adjustment
            .as_ref()
            .filter(|a| a.projector == projector)
            .map(|a| &a.deformation)
    }

    pub fn drawing_point(&self, p: PointMm) -> Option<(f64, f64)> {
        let p = match &self.deformation {
            Some(d) => crate::placement::mesh::inverse(d, p)?,
            None => p,
        };
        Some(self.rigid_fractions(p))
    }

    /// Inverse de [`Self::rigid_point`].
    pub fn rigid_fractions(&self, p: PointMm) -> (f64, f64) {
        let (dx, dy) = (p.x - self.center.x, p.y - self.center.y);
        let (sin, cos) = self.rotation.to_radians().sin_cos();
        let mut x = dx * cos + dy * sin;
        let y = -dx * sin + dy * cos;
        if self.mirrored {
            x = -x;
        }
        (x / self.width + 0.5, y / self.height + 0.5)
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
    /// Un SVG en millimètres à côté de chaque PNG : les courbes des dessins
    /// posés (tâche 5.3). La fresque importée en une image n'y est pas.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub svg: bool,
    /// La fresque importée en une image (ou en PDF, SVG) est gravée avec
    /// les dessins posés ; sinon, les dessins seuls (D-3.5-n). Oui par
    /// défaut, comme avant.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub fresco: bool,
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
            svg: false,
            fresco: true,
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
            rotation: 0,
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

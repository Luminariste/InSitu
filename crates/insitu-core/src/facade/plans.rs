//! L'espace de travail de la façade : des plans ajoutés par-dessus le plan
//! de référence (demande de Benjamin, 13 novembre 2026 ; décisions D-PL-a
//! à g).
//!
//! Le plan de référence (`project.facade`) donne l'échelle et le repère en
//! millimètres ; dessins posés et recalages y restent attachés. Les autres
//! plans sont rangés en groupes ([`PlanGroup`]) : un groupe est un fichier,
//! posé sur la façade à une place et une échelle, dont on montre une ou
//! plusieurs vues (pages d'un PDF, ou calques d'un PDF ou d'un `.ai`). Les
//! vues d'un groupe bougent ensemble : tirées du même fichier, elles
//! restent calées entre elles.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::display::{open_for_display_layers, FacadeDisplay};
use super::{inspect, FacadeError, SourceKind};
use crate::project::model::new_id;
use crate::project::{ExtraFields, Plan, PlanGroup, PointMm, Project, ProjectDir};

/// Sous-dossier du projet où sont copiés les plans ajoutés.
pub const PLANS_DIR: &str = "facade/plans";

/// Une vue à tirer d'un fichier : une page (PDF), et éventuellement
/// certains calques seulement.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanChoice {
    /// Page d'un PDF, à partir de 1 (1 par défaut).
    #[serde(default)]
    pub page: Option<u32>,
    /// Calques montrés ; absent : tous.
    #[serde(default)]
    pub layers: Option<Vec<String>>,
    /// Nom affiché ; à défaut, le calque, la page ou le fichier.
    #[serde(default)]
    pub name: Option<String>,
}

/// Résultat d'un calage par deux points.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alignment {
    pub project: Project,
    /// Écart d'angle entre les deux segments, en degrés : un plan ne tourne
    /// pas librement, cet écart reste.
    pub turn_degrees: f64,
    /// Facteur appliqué à l'échelle du groupe.
    pub factor: f64,
}

fn group_mut<'a>(project: &'a mut Project, id: &str) -> Result<&'a mut PlanGroup, FacadeError> {
    project
        .plan_group_mut(id)
        .ok_or_else(|| FacadeError::UnknownPlan(id.to_string()))
}

/// Millimètres de façade par unité du plan de référence.
fn reference_scale(project: &Project) -> Result<f64, FacadeError> {
    let facade = project.facade.as_ref().ok_or(FacadeError::NoFacade)?;
    facade
        .scale
        .as_ref()
        .and_then(|s| s.mm_per_source_unit())
        .ok_or(FacadeError::ReferenceNotScaled)
}

/// Fabrique les vues demandées d'un fichier, tourné de `rotation` quarts de
/// tour. Chaque vue est lue avant d'entrer dans le projet.
fn make_plans(file: &Path, choices: &[PlanChoice], rotation: u8) -> Result<Vec<Plan>, FacadeError> {
    let defaults = [PlanChoice::default()];
    let choices = if choices.is_empty() {
        &defaults[..]
    } else {
        choices
    };
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Plan".into());
    let mut plans = Vec::new();
    for choice in choices {
        let info = inspect(file, choice.page)?;
        let pdf = info.kind == SourceKind::Pdf;
        let layers = choice.layers.clone().filter(|_| pdf);
        if let Some(wanted) = &layers {
            if let Some(missing) = wanted.iter().find(|name| !info.layers.contains(name)) {
                return Err(FacadeError::Unreadable(format!(
                    "le calque « {missing} » n'existe pas dans ce fichier"
                )));
            }
        }
        let (width, height) = if rotation % 2 == 1 {
            (info.height, info.width)
        } else {
            (info.width, info.height)
        };
        let name = choice
            .name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| match (&layers, info.page_count) {
                (Some(names), _) if !names.is_empty() => names.join(", "),
                (_, Some(count)) if count > 1 => format!("Page {}", info.page.unwrap_or(1)),
                _ => stem.clone(),
            });
        plans.push(Plan {
            id: new_id(),
            name,
            page: info.page.filter(|_| pdf),
            layers,
            source_width: width,
            source_height: height,
            hidden: false,
            extra: ExtraFields::new(),
        });
    }
    Ok(plans)
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (fs::metadata(a), fs::metadata(b)) {
        (Ok(ma), Ok(mb)) if ma.len() == mb.len() => {
            matches!((fs::read(a), fs::read(b)), (Ok(x), Ok(y)) if x == y)
        }
        _ => false,
    }
}

/// Ajoute un plan à l'espace de travail : le fichier `source` est copié
/// dans `facade/plans/`, et chaque choix en devient une vue. Le projet est
/// enregistré.
///
/// Place de départ (D-PL-d) : si c'est le fichier du plan de référence, à
/// la même place, à la même échelle et dans le même sens que lui (ses
/// calques ou ses autres pages tombent pile dessus) ; sinon, en haut à
/// gauche, à la largeur du plan de référence.
pub fn add_plan_group(
    dir: &ProjectDir,
    source: &Path,
    choices: &[PlanChoice],
) -> Result<Project, FacadeError> {
    let mut project = dir.load()?.project;
    let k = reference_scale(&project)?;
    let facade = project.facade.as_ref().expect("vérifiée");
    let reference = dir.resolve(&facade.source);
    let aligned = same_file(source, &reference);
    let rotation = if aligned { facade.rotation % 4 } else { 0 };
    let plans = make_plans(source, choices, rotation)?;
    let mm_per_unit = if aligned {
        k
    } else {
        facade.source_width * k / plans[0].source_width
    };
    let relative = dir.import_file_into(source, PLANS_DIR)?;
    let name = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Plan".into());
    project.plan_groups.push(PlanGroup {
        id: new_id(),
        name,
        source: relative,
        origin: PointMm::new(0.0, 0.0),
        mm_per_unit,
        rotation,
        hidden: false,
        plans,
        extra: ExtraFields::new(),
    });
    dir.save(&project)?;
    Ok(project)
}

/// Ajoute des vues (pages, calques) du même fichier à un groupe.
pub fn add_plans(
    dir: &ProjectDir,
    group_id: &str,
    choices: &[PlanChoice],
) -> Result<Project, FacadeError> {
    let mut project = dir.load()?.project;
    let group = group_mut(&mut project, group_id)?;
    let file = dir.resolve(&group.source);
    let mut plans = make_plans(&file, choices, group.rotation)?;
    group.plans.append(&mut plans);
    dir.save(&project)?;
    Ok(project)
}

fn check_placement(origin: PointMm, mm_per_unit: f64) -> Result<(), FacadeError> {
    if !(origin.x.is_finite() && origin.y.is_finite()) {
        return Err(FacadeError::BadPlacement("la position n'est pas un nombre"));
    }
    if !(mm_per_unit.is_finite() && mm_per_unit > 0.0) {
        return Err(FacadeError::BadPlacement("l'échelle doit être positive"));
    }
    Ok(())
}

/// Pose un groupe : coin haut-gauche en mm de façade et échelle (mm par
/// unité du fichier). Sert au déplacement à la souris et aux champs de
/// l'inspecteur.
pub fn place_plan_group(
    dir: &ProjectDir,
    group_id: &str,
    origin: PointMm,
    mm_per_unit: f64,
) -> Result<Project, FacadeError> {
    check_placement(origin, mm_per_unit)?;
    let mut project = dir.load()?.project;
    let group = group_mut(&mut project, group_id)?;
    group.origin = origin;
    group.mm_per_unit = mm_per_unit;
    dir.save(&project)?;
    Ok(project)
}

/// Calage par deux points (D-PL-e) : les points `from`, pris sur le plan
/// du groupe (en mm de façade, là où ils sont maintenant), doivent venir
/// sur les points `to` de la façade. Renvoie la nouvelle origine et la
/// nouvelle échelle : agrandissement et déplacement seulement, sans
/// rotation. `None` si les points sont confondus.
pub fn alignment(
    origin: PointMm,
    mm_per_unit: f64,
    from: [PointMm; 2],
    to: [PointMm; 2],
) -> Option<(PointMm, f64, f64)> {
    let (fx, fy) = (from[1].x - from[0].x, from[1].y - from[0].y);
    let (tx, ty) = (to[1].x - to[0].x, to[1].y - to[0].y);
    let (lf, lt) = (fx.hypot(fy), tx.hypot(ty));
    if !(lf > 1e-9 && lt > 1e-9) {
        return None;
    }
    let factor = lt / lf;
    let scale = mm_per_unit * factor;
    // Le premier point, en unités du fichier, va sur sa cible.
    let u = (from[0].x - origin.x) / mm_per_unit;
    let v = (from[0].y - origin.y) / mm_per_unit;
    let new_origin = PointMm::new(to[0].x - u * scale, to[0].y - v * scale);
    let mut turn = (ty.atan2(tx) - fy.atan2(fx)).to_degrees();
    while turn > 180.0 {
        turn -= 360.0;
    }
    while turn < -180.0 {
        turn += 360.0;
    }
    Some((new_origin, scale, turn))
}

/// Cale un groupe sur la façade par deux paires de points (voir
/// [`alignment`]) et enregistre le projet.
pub fn align_plan_group(
    dir: &ProjectDir,
    group_id: &str,
    from: [PointMm; 2],
    to: [PointMm; 2],
) -> Result<Alignment, FacadeError> {
    let mut project = dir.load()?.project;
    let group = group_mut(&mut project, group_id)?;
    let (origin, scale, turn) = alignment(group.origin, group.mm_per_unit, from, to)
        .ok_or(FacadeError::BadPlacement("deux points sont confondus"))?;
    check_placement(origin, scale)?;
    let factor = scale / group.mm_per_unit;
    group.origin = origin;
    group.mm_per_unit = scale;
    dir.save(&project)?;
    Ok(Alignment {
        project,
        turn_degrees: turn,
        factor,
    })
}

/// Un quart de tour du groupe dans le sens des aiguilles d'une montre,
/// autour du centre de sa première vue.
fn turn_group_in_place(group: &mut PlanGroup) {
    let (w, h) = group.size_mm();
    let center = PointMm::new(group.origin.x + w / 2.0, group.origin.y + h / 2.0);
    turn_dims(group);
    group.origin = PointMm::new(center.x - h / 2.0, center.y - w / 2.0);
}

fn turn_dims(group: &mut PlanGroup) {
    group.rotation = (group.rotation + 1) % 4;
    for plan in &mut group.plans {
        (plan.source_width, plan.source_height) = (plan.source_height, plan.source_width);
    }
}

/// Un quart de tour de tout l'espace de travail (le plan de référence
/// tourne, D15-k) : le groupe tourne avec lui autour du coin haut-gauche de
/// la façade, dont la hauteur était `height_mm`. Un point (x, y) va en
/// (h − y, x) ; le coin haut-gauche du groupe tourné est l'image de son
/// coin bas-gauche.
pub(crate) fn turn_group_with_facade(group: &mut PlanGroup, height_mm: f64) {
    let (_, h) = group.size_mm();
    let origin = PointMm::new(height_mm - (group.origin.y + h), group.origin.x);
    turn_dims(group);
    group.origin = origin;
}

/// Tourne un groupe de `quarter_turns` quarts de tour (négatif : sens
/// inverse), autour de son centre, et enregistre le projet.
pub fn rotate_plan_group(
    dir: &ProjectDir,
    group_id: &str,
    quarter_turns: i32,
) -> Result<Project, FacadeError> {
    let mut project = dir.load()?.project;
    let group = group_mut(&mut project, group_id)?;
    for _ in 0..quarter_turns.rem_euclid(4) {
        turn_group_in_place(group);
    }
    dir.save(&project)?;
    Ok(project)
}

/// Montre ou cache un groupe (`plan_id` absent) ou une de ses vues.
pub fn set_plan_hidden(
    dir: &ProjectDir,
    group_id: &str,
    plan_id: Option<&str>,
    hidden: bool,
) -> Result<Project, FacadeError> {
    let mut project = dir.load()?.project;
    let group = group_mut(&mut project, group_id)?;
    match plan_id {
        None => group.hidden = hidden,
        Some(id) => {
            group
                .plans
                .iter_mut()
                .find(|p| p.id == id)
                .ok_or_else(|| FacadeError::UnknownPlan(id.to_string()))?
                .hidden = hidden;
        }
    }
    dir.save(&project)?;
    Ok(project)
}

/// Renomme un groupe (`plan_id` absent) ou une de ses vues.
pub fn rename_plan(
    dir: &ProjectDir,
    group_id: &str,
    plan_id: Option<&str>,
    name: &str,
) -> Result<Project, FacadeError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(FacadeError::BadPlacement("un plan doit avoir un nom"));
    }
    let mut project = dir.load()?.project;
    let group = group_mut(&mut project, group_id)?;
    match plan_id {
        None => group.name = name.to_string(),
        Some(id) => {
            group
                .plans
                .iter_mut()
                .find(|p| p.id == id)
                .ok_or_else(|| FacadeError::UnknownPlan(id.to_string()))?
                .name = name.to_string();
        }
    }
    dir.save(&project)?;
    Ok(project)
}

/// Retire un groupe (`plan_id` absent) ou une de ses vues ; un groupe sans
/// vue est retiré. Le fichier reste dans `facade/plans/`.
pub fn remove_plan(
    dir: &ProjectDir,
    group_id: &str,
    plan_id: Option<&str>,
) -> Result<Project, FacadeError> {
    let mut project = dir.load()?.project;
    let index = project
        .plan_groups
        .iter()
        .position(|g| g.id == group_id)
        .ok_or_else(|| FacadeError::UnknownPlan(group_id.to_string()))?;
    let remove_group = match plan_id {
        None => true,
        Some(id) => {
            let group = &mut project.plan_groups[index];
            let before = group.plans.len();
            group.plans.retain(|p| p.id != id);
            if group.plans.len() == before {
                return Err(FacadeError::UnknownPlan(id.to_string()));
            }
            group.plans.is_empty()
        }
    };
    if remove_group {
        project.plan_groups.remove(index);
    }
    dir.save(&project)?;
    Ok(project)
}

/// Change l'ordre des groupes : `to` est la nouvelle place (0 : le plus au
/// fond, juste au-dessus du plan de référence).
pub fn move_plan_group(
    dir: &ProjectDir,
    group_id: &str,
    to: usize,
) -> Result<Project, FacadeError> {
    let mut project = dir.load()?.project;
    let index = project
        .plan_groups
        .iter()
        .position(|g| g.id == group_id)
        .ok_or_else(|| FacadeError::UnknownPlan(group_id.to_string()))?;
    let group = project.plan_groups.remove(index);
    let to = to.min(project.plan_groups.len());
    project.plan_groups.insert(to, group);
    dir.save(&project)?;
    Ok(project)
}

/// Prépare l'affichage d'une vue d'un groupe (pyramide de tuiles, comme le
/// plan de référence). Pour la placer en mm : coin haut-gauche à
/// `group.origin`, `mmPerPixel = sourceUnitsPerPixel × group.mmPerUnit`.
pub fn open_plan_display(
    dir: &ProjectDir,
    group_id: &str,
    plan_id: &str,
    cache_root: &Path,
) -> Result<FacadeDisplay, FacadeError> {
    let project = dir.load()?.project;
    let group = project
        .plan_group(group_id)
        .ok_or_else(|| FacadeError::UnknownPlan(group_id.to_string()))?;
    let plan = group
        .plans
        .iter()
        .find(|p| p.id == plan_id)
        .ok_or_else(|| FacadeError::UnknownPlan(plan_id.to_string()))?;
    open_for_display_layers(
        &dir.resolve(&group.source),
        plan.page,
        plan.layers.as_deref(),
        group.rotation,
        cache_root,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_points_give_position_and_scale() {
        // Un plan à 2 mm par unité, posé en (100, 50) ; ses points (110, 60)
        // et (130, 60) doivent aller en (0, 0) et (60, 0) : trois fois plus
        // grand.
        let (origin, scale, turn) = alignment(
            PointMm::new(100.0, 50.0),
            2.0,
            [PointMm::new(110.0, 60.0), PointMm::new(130.0, 60.0)],
            [PointMm::new(0.0, 0.0), PointMm::new(60.0, 0.0)],
        )
        .unwrap();
        assert!((scale - 6.0).abs() < 1e-12);
        assert!(turn.abs() < 1e-12);
        // Le point (110, 60) est l'unité (5, 5) du fichier : à 6 mm par
        // unité, il tombe en (0, 0) si l'origine est (−30, −30).
        assert!((origin.x + 30.0).abs() < 1e-9 && (origin.y + 30.0).abs() < 1e-9);
        // Et le second point aussi : unité (15, 5) → (−30 + 90, −30 + 30).
        assert!((origin.x + 15.0 * scale - 60.0).abs() < 1e-9);
    }

    #[test]
    fn alignment_reports_the_turn_it_cannot_do() {
        let (_, _, turn) = alignment(
            PointMm::new(0.0, 0.0),
            1.0,
            [PointMm::new(0.0, 0.0), PointMm::new(10.0, 0.0)],
            [PointMm::new(0.0, 0.0), PointMm::new(0.0, 10.0)],
        )
        .unwrap();
        assert!((turn - 90.0).abs() < 1e-9);
        assert!(alignment(
            PointMm::new(0.0, 0.0),
            1.0,
            [PointMm::new(1.0, 1.0), PointMm::new(1.0, 1.0)],
            [PointMm::new(0.0, 0.0), PointMm::new(0.0, 10.0)],
        )
        .is_none());
    }

    #[test]
    fn a_turn_in_place_keeps_the_center() {
        let mut group = PlanGroup {
            id: "g".into(),
            name: "g".into(),
            source: crate::project::RelativePath::new("facade/plans/a.png").unwrap(),
            origin: PointMm::new(10.0, 20.0),
            mm_per_unit: 2.0,
            rotation: 0,
            hidden: false,
            plans: vec![Plan {
                id: "p".into(),
                name: "p".into(),
                page: None,
                layers: None,
                source_width: 50.0,
                source_height: 20.0,
                hidden: false,
                extra: ExtraFields::new(),
            }],
            extra: ExtraFields::new(),
        };
        // 100 × 40 mm, centre (60, 40).
        turn_group_in_place(&mut group);
        assert_eq!(group.rotation, 1);
        assert_eq!(group.size_mm(), (40.0, 100.0));
        assert_eq!(group.origin, PointMm::new(40.0, -10.0));
        // Avec la façade (hauteur 300 mm) : le coin bas-gauche (40, 90)
        // devient le coin haut-gauche (300 − 90, 40).
        turn_group_with_facade(&mut group, 300.0);
        assert_eq!(group.rotation, 2);
        assert_eq!(group.origin, PointMm::new(210.0, 40.0));
    }
}

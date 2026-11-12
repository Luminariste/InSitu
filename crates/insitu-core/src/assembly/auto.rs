//! Raccord automatique des photos de l'assemblage (tâche 6.6) : le moteur
//! trouve lui-même les points communs entre les photos, choisit l'appui de
//! chacune et enregistre ces points comme si on les avait cliqués.
//!
//! Tout le reste ne change pas : diagnostic des points, correction à la
//! main, calcul de l'image assemblée (tâche 6.4). Voir D-6.6-a à d.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;

use super::features::{detect, match_pair, Features, Matched};
use super::{fit_pairs, report, AssemblyError, AssemblyReport};
use crate::geometry::Point;
use crate::photo;
use crate::project::{AssemblyPair, Project, ProjectDir, SourcePoint};

/// Appariements justes qu'il faut au moins pour raccorder deux photos.
pub const MIN_MATCHES: usize = 20;

/// Points communs enregistrés au plus par photo (bien répartis).
pub const MAX_PAIRS: usize = 24;

/// Cases de répartition des points enregistrés (colonnes, rangées).
const SPREAD: (usize, usize) = (6, 4);

/// Fils de calcul au plus : chaque photo lue occupe de la mémoire (une
/// photo de 48 Mpx pèse près de 50 Mo en niveaux de gris).
const MAX_THREADS: usize = 4;

/// Ce que l'analyse dit d'une photo.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoPhotoReport {
    pub id: String,
    pub name: String,
    /// Photo d'appui choisie ; `None` pour la référence et pour une photo
    /// non raccordée.
    pub anchor: Option<String>,
    /// Appariements justes trouvés avec l'appui.
    pub matches: usize,
    /// Points communs enregistrés.
    pub pairs: usize,
    /// Vrai si l'analyse a raccordé la photo (ses points ont été remplacés).
    pub linked: bool,
}

/// Compte rendu de l'analyse automatique.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoReport {
    pub photos: Vec<AutoPhotoReport>,
    /// Noms des photos qu'aucun détail commun ne relie aux autres (leurs
    /// points éventuels sont gardés).
    pub unlinked: Vec<String>,
    pub seconds: f64,
    /// Diagnostic de l'assemblage après l'analyse.
    pub assembly: AssemblyReport,
}

/// Fait tourner `task` sur chaque élément, sur quelques fils.
fn parallel<T: Sync, R: Send>(items: &[T], task: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, MAX_THREADS);
    let next = Mutex::new(0usize);
    let results: Mutex<Vec<Option<R>>> = Mutex::new((0..items.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..threads.min(items.len().max(1)) {
            scope.spawn(|| loop {
                let k = {
                    let mut n = next.lock().expect("verrou");
                    let k = *n;
                    *n += 1;
                    k
                };
                if k >= items.len() {
                    break;
                }
                let r = task(&items[k]);
                results.lock().expect("verrou")[k] = Some(r);
            });
        }
    });
    results
        .into_inner()
        .expect("verrou")
        .into_iter()
        .map(|r| r.expect("calculé"))
        .collect()
}

/// Les points communs à enregistrer pour `child` appuyée sur `anchor`,
/// bien répartis sur la zone de recouvrement, en pixels de chaque photo.
fn spread_pairs(child: &Features, anchor: &Features, inliers: &[Matched]) -> Vec<AssemblyPair> {
    let points: Vec<_> = inliers
        .iter()
        .map(|m| {
            (
                Point::new(m.a.x * child.scale, m.a.y * child.scale),
                Point::new(m.b.x * anchor.scale, m.b.y * anchor.scale),
                m.error,
            )
        })
        .collect();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (p, _, _) in &points {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    let (cols, rows) = SPREAD;
    let cell = |p: Point| {
        let cx = (((p.x - min_x) / (max_x - min_x).max(1.0)) * cols as f64).floor() as usize;
        let cy = (((p.y - min_y) / (max_y - min_y).max(1.0)) * rows as f64).floor() as usize;
        cy.min(rows - 1) * cols + cx.min(cols - 1)
    };
    // Le meilleur de chaque case, puis les meilleurs restants.
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by(|&a, &b| points[a].2.total_cmp(&points[b].2));
    let mut taken = vec![false; points.len()];
    let mut used = vec![false; cols * rows];
    let mut chosen = Vec::new();
    for &k in &order {
        let c = cell(points[k].0);
        if !used[c] {
            used[c] = true;
            taken[k] = true;
            chosen.push(k);
        }
    }
    for &k in &order {
        // Au moins 12 points quand la zone commune est étroite.
        if chosen.len() >= 12 {
            break;
        }
        if !taken[k] {
            taken[k] = true;
            chosen.push(k);
        }
    }
    chosen.truncate(MAX_PAIRS);
    chosen
        .into_iter()
        .map(|k| AssemblyPair {
            here: SourcePoint::new(points[k].0.x, points[k].0.y),
            there: SourcePoint::new(points[k].1.x, points[k].1.y),
        })
        .collect()
}

/// Analyse automatique de l'assemblage du projet : pour chaque photo, sauf
/// la référence, cherche la photo déjà raccordée qui partage le plus de
/// détails avec elle, en fait son appui et remplace ses points communs par
/// ceux trouvés. Une photo qu'aucun détail ne relie garde ses points et son
/// appui. Le projet est enregistré.
pub fn analyze(dir: &ProjectDir) -> Result<(Project, AutoReport), AssemblyError> {
    let start = Instant::now();
    let mut project = dir.load()?.project;
    let assembly = project
        .facade_assembly
        .as_ref()
        .ok_or(AssemblyError::Empty)?;
    let photos = &assembly.photos;
    if photos.is_empty() {
        return Err(AssemblyError::Empty);
    }
    let paths: Vec<PathBuf> = photos.iter().map(|p| dir.resolve(&p.source)).collect();
    let features: Vec<Features> = parallel(&paths, |path| {
        photo::read(path)
            .map(|image| detect(&image.to_luma8()))
            .map_err(|e| AssemblyError::Unreadable(e.to_string()))
    })
    .into_iter()
    .collect::<Result<_, _>>()?;

    // Toutes les paires de photos.
    let n = photos.len();
    let couples: Vec<(usize, usize)> = (0..n)
        .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
        .collect();
    let matched = parallel(&couples, |&(i, j)| {
        match_pair(&features[i], &features[j], MIN_MATCHES)
    });
    let weight = |i: usize, j: usize| -> usize {
        let (a, b) = (i.min(j), i.max(j));
        let k = couples
            .iter()
            .position(|&c| c == (a, b))
            .expect("toutes les paires");
        matched[k].as_ref().map_or(0, |m| m.inliers.len())
    };

    // Arbre des meilleurs raccords depuis la référence (Prim) : chaque
    // photo s'appuie sur la photo déjà raccordée avec laquelle elle partage
    // le plus d'appariements justes.
    let mut linked = vec![false; n];
    linked[0] = true;
    let mut parent: Vec<Option<usize>> = vec![None; n];
    loop {
        let best = (0..n)
            .filter(|&c| !linked[c])
            .flat_map(|c| (0..n).filter(|&p| linked[p]).map(move |p| (c, p)))
            .map(|(c, p)| (c, p, weight(c, p)))
            .filter(|&(_, _, w)| w > 0)
            .max_by_key(|&(c, _, w)| (w, std::cmp::Reverse(c)));
        let Some((child, anchor, _)) = best else {
            break;
        };
        linked[child] = true;
        parent[child] = Some(anchor);
    }

    let assembly = project.facade_assembly.as_mut().expect("vérifié plus haut");
    let mut photo_reports = Vec::with_capacity(n);
    let mut unlinked = Vec::new();
    for c in 0..n {
        let (id, name) = (
            assembly.photos[c].id.clone(),
            assembly.photos[c].name.clone(),
        );
        let Some(a) = parent[c] else {
            if c > 0 {
                unlinked.push(name.clone());
            }
            photo_reports.push(AutoPhotoReport {
                id,
                name,
                anchor: None,
                matches: 0,
                pairs: 0,
                linked: c == 0,
            });
            continue;
        };
        let k = couples
            .iter()
            .position(|&cp| cp == (c.min(a), c.max(a)))
            .expect("toutes les paires");
        let m = matched[k].as_ref().expect("raccord trouvé");
        // Appariements dans le sens photo → appui.
        let inliers: Vec<Matched> = if c < a {
            m.inliers.clone()
        } else {
            m.inliers
                .iter()
                .map(|m| Matched {
                    a: m.b,
                    b: m.a,
                    error: m.error,
                })
                .collect()
        };
        let mut pairs = spread_pairs(&features[c], &features[a], &inliers);
        // Les points restent dans leur photo.
        let (w, h) = (assembly.photos[c].width, assembly.photos[c].height);
        let (aw, ah) = (assembly.photos[a].width, assembly.photos[a].height);
        pairs.retain(|p| {
            (0.0..=w).contains(&p.here.x)
                && (0.0..=h).contains(&p.here.y)
                && (0.0..=aw).contains(&p.there.x)
                && (0.0..=ah).contains(&p.there.y)
        });
        if pairs.len() < 4 || fit_pairs(&pairs).is_err() {
            unlinked.push(name.clone());
            photo_reports.push(AutoPhotoReport {
                id,
                name,
                anchor: None,
                matches: inliers.len(),
                pairs: 0,
                linked: false,
            });
            continue;
        }
        let anchor_id = assembly.photos[a].id.clone();
        let photo = &mut assembly.photos[c];
        photo.anchor = Some(anchor_id.clone());
        photo.pairs = pairs;
        photo_reports.push(AutoPhotoReport {
            id,
            name,
            anchor: Some(anchor_id),
            matches: inliers.len(),
            pairs: photo.pairs.len(),
            linked: true,
        });
    }
    dir.save(&project)?;
    let assembly = report(&project);
    Ok((
        project,
        AutoReport {
            photos: photo_reports,
            unlinked,
            seconds: start.elapsed().as_secs_f64(),
            assembly,
        },
    ))
}

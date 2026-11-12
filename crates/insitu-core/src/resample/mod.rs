//! Déformation d'images entières, à pleine résolution (tâche 3.4).
//!
//! [`warp_raster`] fabrique une image de sortie (typiquement une diapositive
//! de 15 433 px) en allant chercher, pour chaque pixel de sortie, la valeur
//! correspondante dans une image source (la fresque, un dessin, une photo).
//! La correspondance est une fonction quelconque « pixel de sortie → pixel
//! source » : trapèze, grille de déformation, ou leur enchaînement avec des
//! changements d'unité (pixels de diapositive → mm de diapositive → mm de
//! façade → pixels de la fresque).
//!
//! Trois choix font la vitesse et la qualité :
//!
//! - **un treillis** : la correspondance n'est évaluée exactement qu'aux
//!   nœuds d'un treillis (un nœud tous les 8 pixels de sortie par défaut),
//!   puis interpolée linéairement entre eux. L'écart introduit est mesuré par
//!   les tests (moins d'un centième de pixel pour un trapèze) ;
//! - **un filtre adapté à l'échelle locale** : voir [`filter`] ; les traits
//!   fins restent nets sans crénelage, et une réduction ne fait pas de moiré ;
//! - **des bandes en parallèle** : l'image est découpée en bandes de 256
//!   lignes, calculées sur tous les cœurs.
//!
//! Conventions de pixels : le pixel `(i, j)` couvre le carré
//! `[i, i+1] × [j, j+1]`, son centre est en `(i + 0,5 ; j + 0,5)`. Une
//! correspondance qui envoie chaque centre de sortie sur un centre source
//! (identité, translation entière) recopie la source à l'identique.

mod filter;

use std::fmt;
use std::sync::Mutex;

use crate::geometry::{Jacobian, Point};
use crate::tiles::Raster;
use filter::{Footprint, Pyramid};

/// Pas du treillis par défaut, en pixels de sortie.
pub const DEFAULT_LATTICE_STEP: u32 = 8;

/// Hauteur des bandes calculées d'un seul tenant, et largeur des tuiles
/// dans une bande, en pixels de sortie.
pub const DEFAULT_TILE_SIZE: u32 = 256;

/// Réglages de [`warp_raster`]. Les valeurs par défaut conviennent à
/// l'export ; les autres servent surtout aux tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarpOptions {
    /// Pas du treillis en pixels de sortie. 0 : la correspondance est
    /// évaluée exactement en chaque pixel (trois fois plus lent au moins,
    /// sert de référence).
    pub lattice_step: u32,
    /// Côté des tuiles, en pixels de sortie.
    pub tile_size: u32,
    /// Nombre de fils de calcul ; 0 = autant que de cœurs.
    pub threads: usize,
    /// Valeur des pixels hors de l'image source, sur chaque canal. 0 : noir
    /// (rien n'est gravé), ou transparent pour une image avec transparence.
    pub background: u8,
}

impl Default for WarpOptions {
    fn default() -> Self {
        Self {
            lattice_step: DEFAULT_LATTICE_STEP,
            tile_size: DEFAULT_TILE_SIZE,
            threads: 0,
            background: 0,
        }
    }
}

/// Ce qui peut empêcher une déformation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarpError {
    /// L'image de sortie demandée n'a aucun pixel.
    EmptyOutput,
    /// L'image de sortie dépasserait la mémoire adressable.
    OutputTooLarge,
    /// Un réglage est hors de son domaine ; le texte dit lequel.
    InvalidOption(&'static str),
}

impl fmt::Display for WarpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyOutput => f.write_str("l'image de sortie demandée est vide"),
            Self::OutputTooLarge => f.write_str("l'image de sortie demandée est trop grande"),
            Self::InvalidOption(what) => write!(f, "réglage invalide : {what}"),
        }
    }
}

impl std::error::Error for WarpError {}

/// Déforme `source` vers une image de `width × height` pixels, de même
/// nombre de canaux.
///
/// `map` reçoit la position d'un point de l'image de sortie, en pixels, et
/// rend la position correspondante dans la source, en pixels (conventions
/// du module) ; `None` si ce point n'a pas d'antécédent (il reçoit alors la
/// valeur de fond). Elle est appelée depuis plusieurs fils à la fois.
///
/// Mémoire : l'image de sortie, plus, seulement en cas de réduction de plus
/// de moitié, des versions réduites de la source (un tiers de sa taille au
/// plus).
pub fn warp_raster<F>(
    source: &Raster,
    width: u32,
    height: u32,
    map: F,
    options: &WarpOptions,
) -> Result<Raster, WarpError>
where
    F: Fn(Point) -> Option<Point> + Sync,
{
    if width == 0 || height == 0 {
        return Err(WarpError::EmptyOutput);
    }
    if options.tile_size == 0 {
        return Err(WarpError::InvalidOption(
            "les tuiles doivent faire au moins un pixel",
        ));
    }
    if options.lattice_step > options.tile_size {
        return Err(WarpError::InvalidOption(
            "le pas du treillis ne peut pas dépasser la taille des tuiles",
        ));
    }
    let channels = source.channels as usize;
    let row_bytes = (width as usize)
        .checked_mul(channels)
        .ok_or(WarpError::OutputTooLarge)?;
    let total = row_bytes
        .checked_mul(height as usize)
        .ok_or(WarpError::OutputTooLarge)?;

    let mut data = vec![0u8; total];
    let pyramid = Pyramid::new(source);
    let job = Job {
        pyramid: &pyramid,
        map: &map,
        width,
        options,
        channels,
    };
    let band_rows = options.tile_size as usize;
    let bands = Mutex::new(data.chunks_mut(row_bytes * band_rows).enumerate());
    let threads = match options.threads {
        0 => std::thread::available_parallelism().map_or(1, |n| n.get()),
        n => n,
    };
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let next = bands.lock().expect("verrou des bandes").next();
                let Some((index, band)) = next else { break };
                job.render_band((index * band_rows) as u32, band);
            });
        }
    });
    Ok(Raster::new(width, height, source.channels, data))
}

/// Ce que partagent les fils de calcul.
struct Job<'a, F> {
    pyramid: &'a Pyramid<'a>,
    map: &'a F,
    width: u32,
    options: &'a WarpOptions,
    channels: usize,
}

impl<F> Job<'_, F>
where
    F: Fn(Point) -> Option<Point> + Sync,
{
    /// Calcule une bande de lignes, qui commence à la ligne `y0`.
    fn render_band(&self, y0: u32, band: &mut [u8]) {
        let row_bytes = self.width as usize * self.channels;
        let rows = (band.len() / row_bytes) as u32;
        let tile = self.options.tile_size;
        let mut x0 = 0;
        while x0 < self.width {
            let columns = tile.min(self.width - x0);
            let area = Area {
                x0,
                y0,
                columns,
                rows,
            };
            if self.options.lattice_step == 0 {
                self.render_exact(area, y0, band);
            } else {
                self.render_lattice(area, band);
            }
            x0 += tile;
        }
    }

    /// Une tuile par le treillis : la correspondance n'est évaluée qu'aux
    /// nœuds, interpolée linéairement entre eux.
    /// La dérivée, estimée aux nœuds, est elle aussi interpolée : le filtre
    /// varie continûment d'un pixel à l'autre, sans effet de quadrillage.
    /// Les derniers nœuds sont posés sur la dernière ligne et la
    /// dernière colonne de la tuile, jamais au-delà : une correspondance
    /// n'est donc jamais évaluée hors de l'image de sortie (où elle peut
    /// changer d'allure, par exemple au bord d'une grille de déformation).
    fn render_lattice(&self, area: Area, band: &mut [u8]) {
        if area.columns < 2 || area.rows < 2 {
            self.render_exact(area, area.y0, band);
            return;
        }
        let xs = lattice_offsets(area.columns, self.options.lattice_step);
        let ys = lattice_offsets(area.rows, self.options.lattice_step);
        let nx = xs.len();
        let mut nodes = Vec::with_capacity(nx * ys.len());
        for &oy in &ys {
            for &ox in &xs {
                nodes.push((self.map)(Point::new(
                    f64::from(area.x0 + ox) + 0.5,
                    f64::from(area.y0 + oy) + 0.5,
                )));
            }
        }
        // Dérivée en chaque nœud, par différences entre nœuds voisins
        // (centrées à l'intérieur, d'un seul côté au bord).
        let ny = ys.len();
        let jacobians: Vec<Option<Jacobian>> = (0..ny)
            .flat_map(|j| (0..nx).map(move |i| (i, j)))
            .map(|(i, j)| {
                let (il, ir) = (i.saturating_sub(1), (i + 1).min(nx - 1));
                let (jt, jb) = (j.saturating_sub(1), (j + 1).min(ny - 1));
                let dx = (nodes[j * nx + ir]? - nodes[j * nx + il]?)
                    * (1.0 / f64::from(xs[ir] - xs[il]));
                let dy = (nodes[jb * nx + i]? - nodes[jt * nx + i]?)
                    * (1.0 / f64::from(ys[jb] - ys[jt]));
                Some([[dx.x, dy.x], [dx.y, dy.y]])
            })
            .collect();
        let levels = self.pyramid.level_count();
        for j in 0..ny - 1 {
            // La dernière maille inclut la dernière ligne (son nœud).
            let last_row = j + 2 == ny;
            let rows = ys[j + 1] - ys[j] + u32::from(last_row);
            let h = f64::from(ys[j + 1] - ys[j]);
            for i in 0..nx - 1 {
                let last_column = i + 2 == nx;
                let columns = xs[i + 1] - xs[i] + u32::from(last_column);
                let w = f64::from(xs[i + 1] - xs[i]);
                let cell = Area {
                    x0: area.x0 + xs[i],
                    y0: area.y0 + ys[j],
                    columns,
                    rows,
                };
                let at = |di: usize, dj: usize| {
                    let k = (j + dj) * nx + i + di;
                    Some((nodes[k]?, jacobians[k]?))
                };
                let (Some(c00), Some(c10), Some(c01), Some(c11)) =
                    (at(0, 0), at(1, 0), at(0, 1), at(1, 1))
                else {
                    // Une partie de la maille n'a pas d'antécédent (horizon
                    // d'un trapèze) : on la calcule pixel par pixel.
                    self.render_exact(cell, area.y0, band);
                    continue;
                };
                for y in 0..cell.rows {
                    let fy = f64::from(y) / h;
                    let left = lerp(c00, c01, fy);
                    let right = lerp(c10, c11, fy);
                    for x in 0..cell.columns {
                        let (p, jacobian) = lerp(left, right, f64::from(x) / w);
                        let footprint = Footprint::from_jacobian(&jacobian, levels);
                        self.put(cell.x0 + x, cell.y0 - area.y0 + y, p, footprint, band);
                    }
                }
            }
        }
    }

    /// Une zone pixel par pixel : position exacte, dérivée par différences
    /// centrées d'un demi-pixel.
    /// `band_y0` : première ligne de la bande.
    fn render_exact(&self, area: Area, band_y0: u32, band: &mut [u8]) {
        for y in 0..area.rows {
            for x in 0..area.columns {
                let c = Point::new(f64::from(area.x0 + x) + 0.5, f64::from(area.y0 + y) + 0.5);
                let estimate = (|| {
                    let p = (self.map)(c)?;
                    let px = (self.map)(c + Point::new(0.5, 0.0))?
                        - (self.map)(c - Point::new(0.5, 0.0))?;
                    let py = (self.map)(c + Point::new(0.0, 0.5))?
                        - (self.map)(c - Point::new(0.0, 0.5))?;
                    let j: Jacobian = [[px.x, py.x], [px.y, py.y]];
                    Some((p, Footprint::from_jacobian(&j, self.pyramid.level_count())))
                })();
                let row = area.y0 - band_y0 + y;
                match estimate {
                    Some((p, footprint)) => self.put(area.x0 + x, row, p, footprint, band),
                    None => self.fill_background(area.x0 + x, row, band),
                }
            }
        }
    }

    /// Écrit le pixel `(x, row)` de la bande (row compté depuis le haut de
    /// la bande), de position source `p` (niveau 0).
    #[inline]
    fn put(&self, x: u32, row: u32, p: Point, footprint: Option<Footprint>, band: &mut [u8]) {
        let Some(footprint) = footprint else {
            self.fill_background(x, row, band);
            return;
        };
        let c = self.channels;
        let offset = (row as usize * self.width as usize + x as usize) * c;
        let out = &mut band[offset..offset + c];
        let image = self.pyramid.level(footprint.level);
        let (px, py) = (p.x * footprint.level_scale, p.y * footprint.level_scale);
        let background = self.options.background;
        match c {
            1 => filter::sample::<1>(image, px, py, &footprint, background, out),
            3 => filter::sample::<3>(image, px, py, &footprint, background, out),
            _ => filter::sample::<4>(image, px, py, &footprint, background, out),
        }
    }

    fn fill_background(&self, x: u32, row: u32, band: &mut [u8]) {
        let c = self.channels;
        let offset = (row as usize * self.width as usize + x as usize) * c;
        band[offset..offset + c].fill(self.options.background);
    }
}

/// Interpolation linéaire d'une position et de sa dérivée.
#[inline]
fn lerp(a: (Point, Jacobian), b: (Point, Jacobian), t: f64) -> (Point, Jacobian) {
    let m = |u: f64, v: f64| u + (v - u) * t;
    (
        a.0 + (b.0 - a.0) * t,
        [
            [m(a.1[0][0], b.1[0][0]), m(a.1[0][1], b.1[0][1])],
            [m(a.1[1][0], b.1[1][0]), m(a.1[1][1], b.1[1][1])],
        ],
    )
}

/// Positions des nœuds du treillis sur un côté de `length ≥ 2` pixels :
/// tous les `step` pixels, plus le dernier pixel.
fn lattice_offsets(length: u32, step: u32) -> Vec<u32> {
    let mut offsets: Vec<u32> = (0..length - 1).step_by(step as usize).collect();
    offsets.push(length - 1);
    offsets
}

/// Rectangle de pixels de sortie.
#[derive(Debug, Clone, Copy)]
struct Area {
    x0: u32,
    y0: u32,
    columns: u32,
    rows: u32,
}

#[cfg(test)]
mod tests;

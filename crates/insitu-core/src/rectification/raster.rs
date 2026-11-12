//! Images de travail de la détection : un canal en nombres flottants, lu par
//! interpolation, et les quelques filtres dont la détection a besoin.
//!
//! Convention des coordonnées (décision D-2.4-c) : le pixel de colonne `i`
//! et de ligne `j` couvre le carré `[i, i + 1] × [j, j + 1]` ; son centre est
//! en `(i + 0,5 ; j + 0,5)`. C'est la convention de la visionneuse (un calque
//! est posé par son coin) : un point mesuré ici s'y affiche sans décalage.

/// Image à un canal, valeurs flottantes, rangée ligne par ligne.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Plane {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
}

/// Que lire hors de l'image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Border {
    /// La valeur du bord le plus proche.
    Replicate,
    /// Zéro.
    Zero,
}

impl Plane {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0.0; width * height],
        }
    }

    pub fn at(&self, i: usize, j: usize) -> f32 {
        self.data[j * self.width + i]
    }

    fn texel(&self, i: isize, j: isize, border: Border) -> f32 {
        let inside = i >= 0 && j >= 0 && (i as usize) < self.width && (j as usize) < self.height;
        if inside {
            return self.data[j as usize * self.width + i as usize];
        }
        match border {
            Border::Zero => 0.0,
            Border::Replicate => {
                let i = i.clamp(0, self.width as isize - 1) as usize;
                let j = j.clamp(0, self.height as isize - 1) as usize;
                self.data[j * self.width + i]
            }
        }
    }

    /// Valeur interpolée (bilinéaire) au point `(x, y)`, en coordonnées
    /// continues (centre du premier pixel en 0,5).
    pub fn sample(&self, x: f64, y: f64, border: Border) -> f32 {
        let u = x - 0.5;
        let v = y - 0.5;
        if !(u.is_finite() && v.is_finite()) {
            return 0.0;
        }
        // Très loin de l'image : inutile de calculer.
        let far = 4.0 + (self.width + self.height) as f64;
        if u < -far || v < -far || u > far || v > far {
            return match border {
                Border::Zero => 0.0,
                Border::Replicate => self.texel(u as isize, v as isize, border),
            };
        }
        let i0 = u.floor();
        let j0 = v.floor();
        let fx = (u - i0) as f32;
        let fy = (v - j0) as f32;
        let (i0, j0) = (i0 as isize, j0 as isize);
        let a = self.texel(i0, j0, border);
        let b = self.texel(i0 + 1, j0, border);
        let c = self.texel(i0, j0 + 1, border);
        let d = self.texel(i0 + 1, j0 + 1, border);
        let top = a + (b - a) * fx;
        let bottom = c + (d - c) * fx;
        top + (bottom - top) * fy
    }

    /// Moitié de la taille, chaque pixel étant la moyenne d'un carré de 2 × 2
    /// (une ligne ou une colonne impaire du bord est ignorée).
    pub fn half(&self) -> Plane {
        let w = self.width / 2;
        let h = self.height / 2;
        let mut out = Plane::new(w, h);
        for j in 0..h {
            for i in 0..w {
                let s = self.at(2 * i, 2 * j)
                    + self.at(2 * i + 1, 2 * j)
                    + self.at(2 * i, 2 * j + 1)
                    + self.at(2 * i + 1, 2 * j + 1);
                out.data[j * w + i] = 0.25 * s;
            }
        }
        out
    }

    /// Flou gaussien d'écart type `sigma` pixels (séparable, bords répétés).
    pub fn gaussian_blur(&self, sigma: f64) -> Plane {
        let radius = (4.0 * sigma).ceil().max(1.0) as usize;
        let mut kernel: Vec<f32> = (0..=2 * radius)
            .map(|k| {
                let x = k as f64 - radius as f64;
                (-x * x / (2.0 * sigma * sigma)).exp() as f32
            })
            .collect();
        let sum: f32 = kernel.iter().sum();
        for k in &mut kernel {
            *k /= sum;
        }
        let horizontal = self.convolve_rows(&kernel);
        horizontal.transpose().convolve_rows(&kernel).transpose()
    }

    fn convolve_rows(&self, kernel: &[f32]) -> Plane {
        let r = kernel.len() / 2;
        let w = self.width;
        let mut out = Plane::new(w, self.height);
        let mut padded = vec![0.0f32; w + 2 * r];
        for j in 0..self.height {
            let row = &self.data[j * w..(j + 1) * w];
            for (k, p) in padded.iter_mut().enumerate() {
                *p = row[(k as isize - r as isize).clamp(0, w as isize - 1) as usize];
            }
            let dst = &mut out.data[j * w..(j + 1) * w];
            for (i, d) in dst.iter_mut().enumerate() {
                *d = kernel
                    .iter()
                    .zip(&padded[i..i + kernel.len()])
                    .map(|(a, b)| a * b)
                    .sum();
            }
        }
        out
    }

    pub fn transpose(&self) -> Plane {
        let mut out = Plane::new(self.height, self.width);
        for j in 0..self.height {
            for i in 0..self.width {
                out.data[i * self.height + j] = self.data[j * self.width + i];
            }
        }
        out
    }

    /// Minimum (ou maximum) sur un carré de `2·radius + 1` pixels de côté.
    /// Hors de l'image, rien n'est lu (le carré est rogné).
    fn extremum_filter(&self, radius: usize, maximum: bool) -> Plane {
        let rows = self.extremum_rows(radius, maximum);
        rows.transpose().extremum_rows(radius, maximum).transpose()
    }

    fn extremum_rows(&self, radius: usize, maximum: bool) -> Plane {
        let w = self.width;
        let mut out = Plane::new(w, self.height);
        let better = |a: f32, b: f32| if maximum { a >= b } else { a <= b };
        let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
        for j in 0..self.height {
            let row = &self.data[j * w..(j + 1) * w];
            queue.clear();
            let mut next = 0usize;
            for i in 0..w {
                let last = (i + radius).min(w - 1);
                while next <= last {
                    while let Some(&back) = queue.back() {
                        if better(row[next], row[back]) {
                            queue.pop_back();
                        } else {
                            break;
                        }
                    }
                    queue.push_back(next);
                    next += 1;
                }
                while let Some(&front) = queue.front() {
                    if front + radius < i {
                        queue.pop_front();
                    } else {
                        break;
                    }
                }
                out.data[j * w + i] = row[*queue.front().expect("fenêtre non vide")];
            }
        }
        out
    }

    /// « Chapeau haut-de-forme » clair : ce qui dépasse de l'ouverture par un
    /// carré de `2·radius + 1` px. Fait ressortir les traits clairs fins.
    pub fn white_top_hat(&self, radius: usize) -> Plane {
        let opening = self
            .extremum_filter(radius, false)
            .extremum_filter(radius, true);
        self.zip_map(&opening, |a, o| a - o)
    }

    /// « Chapeau haut-de-forme » sombre : ce qui manque pour atteindre la
    /// fermeture par un carré de `2·radius + 1` px. Fait ressortir les traits
    /// sombres fins.
    pub fn black_top_hat(&self, radius: usize) -> Plane {
        let closing = self
            .extremum_filter(radius, true)
            .extremum_filter(radius, false);
        closing.zip_map(self, |c, a| c - a)
    }

    fn zip_map(&self, other: &Plane, f: impl Fn(f32, f32) -> f32) -> Plane {
        Plane {
            width: self.width,
            height: self.height,
            data: self
                .data
                .iter()
                .zip(&other.data)
                .map(|(a, b)| f(*a, *b))
                .collect(),
        }
    }
}

/// Image couleur décomposée en trois plans, valeurs de 0 à 255.
pub(crate) struct ColorPlanes {
    pub red: Plane,
    pub green: Plane,
    pub blue: Plane,
}

impl ColorPlanes {
    pub fn from_rgb(image: &image::RgbImage) -> Self {
        let (w, h) = (image.width() as usize, image.height() as usize);
        let mut planes = [Plane::new(w, h), Plane::new(w, h), Plane::new(w, h)];
        for (k, px) in image.pixels().enumerate() {
            for (c, plane) in planes.iter_mut().enumerate() {
                plane.data[k] = f32::from(px.0[c]);
            }
        }
        let [red, green, blue] = planes;
        Self { red, green, blue }
    }

    /// Luminance (pondération habituelle 0,299 / 0,587 / 0,114).
    pub fn gray(&self) -> Plane {
        Plane {
            width: self.red.width,
            height: self.red.height,
            data: self
                .red
                .data
                .iter()
                .zip(&self.green.data)
                .zip(&self.blue.data)
                .map(|((r, g), b)| 0.299 * r + 0.587 * g + 0.114 * b)
                .collect(),
        }
    }

    pub fn half(&self) -> Self {
        Self {
            red: self.red.half(),
            green: self.green.half(),
            blue: self.blue.half(),
        }
    }

    pub fn gaussian_blur(&self, sigma: f64) -> Self {
        Self {
            red: self.red.gaussian_blur(sigma),
            green: self.green.gaussian_blur(sigma),
            blue: self.blue.gaussian_blur(sigma),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(w: usize, h: usize) -> Plane {
        let mut p = Plane::new(w, h);
        for j in 0..h {
            for i in 0..w {
                p.data[j * w + i] = (i + 10 * j) as f32;
            }
        }
        p
    }

    #[test]
    fn sampling_at_a_pixel_centre_returns_the_pixel() {
        let p = ramp(5, 4);
        assert_eq!(p.sample(2.5, 1.5, Border::Zero), 12.0);
        assert_eq!(p.sample(3.0, 1.5, Border::Zero), 12.5);
        assert_eq!(p.sample(-3.0, 1.5, Border::Replicate), 10.0);
        assert_eq!(p.sample(-3.0, 1.5, Border::Zero), 0.0);
    }

    #[test]
    fn extremum_filters_match_a_naive_window() {
        let mut p = Plane::new(7, 5);
        let mut x = 7u32;
        for v in &mut p.data {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            *v = (x >> 16) as f32;
        }
        let r = 2;
        let fast = p.extremum_filter(r, true);
        for j in 0..5usize {
            for i in 0..7usize {
                let mut m = f32::MIN;
                for jj in j.saturating_sub(r)..=(j + r).min(4) {
                    for ii in i.saturating_sub(r)..=(i + r).min(6) {
                        m = m.max(p.at(ii, jj));
                    }
                }
                assert_eq!(fast.at(i, j), m);
            }
        }
    }

    #[test]
    fn blur_keeps_a_constant_image_constant() {
        let mut p = Plane::new(9, 6);
        p.data.iter_mut().for_each(|v| *v = 3.0);
        let b = p.gaussian_blur(1.5);
        assert!(b.data.iter().all(|v| (v - 3.0).abs() < 1e-5));
    }

    #[test]
    fn black_top_hat_finds_a_dark_line() {
        let mut p = Plane::new(30, 30);
        p.data.iter_mut().for_each(|v| *v = 100.0);
        for j in 0..30 {
            p.data[j * 30 + 15] = 40.0;
        }
        let t = p.black_top_hat(3);
        assert_eq!(t.at(15, 10), 60.0);
        assert_eq!(t.at(5, 10), 0.0);
    }
}

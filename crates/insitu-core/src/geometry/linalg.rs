//! Algèbre linéaire minimale : systèmes denses et valeurs propres de petites
//! matrices symétriques. Écrit à la main pour ne pas ajouter de dépendance
//! (voir la décision D-2.3-g).

/// Matrice carrée dense, rangée ligne par ligne.
#[derive(Debug, Clone)]
pub(crate) struct SquareMatrix {
    pub n: usize,
    pub data: Vec<f64>,
}

impl SquareMatrix {
    pub fn zeros(n: usize) -> Self {
        Self {
            n,
            data: vec![0.0; n * n],
        }
    }

    pub fn at(&self, i: usize, j: usize) -> f64 {
        self.data[i * self.n + j]
    }

    pub fn set(&mut self, i: usize, j: usize, v: f64) {
        self.data[i * self.n + j] = v;
    }

    pub fn row(&self, i: usize) -> &[f64] {
        &self.data[i * self.n..(i + 1) * self.n]
    }
}

/// Décomposition LU avec pivot partiel.
pub(crate) struct LuFactors {
    n: usize,
    lu: Vec<f64>,
    /// La ligne `i` de `lu` est la ligne `perm[i]` de la matrice d'origine.
    perm: Vec<usize>,
}

/// Seuil relatif en dessous duquel un pivot est tenu pour nul.
const SINGULAR_PIVOT: f64 = 1e-12;

/// Factorise `a`. `None` si la matrice est singulière (ou non finie).
pub(crate) fn factorize(a: SquareMatrix) -> Option<LuFactors> {
    let n = a.n;
    let mut lu = a.data;
    let magnitude = lu.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    if !magnitude.is_finite() || magnitude == 0.0 {
        return None;
    }
    let tiny = magnitude * SINGULAR_PIVOT;
    let mut perm: Vec<usize> = (0..n).collect();
    for k in 0..n {
        let mut best = k;
        let mut best_abs = lu[k * n + k].abs();
        for i in k + 1..n {
            let v = lu[i * n + k].abs();
            if v > best_abs {
                best = i;
                best_abs = v;
            }
        }
        if best_abs.is_nan() || best_abs <= tiny {
            return None;
        }
        if best != k {
            for j in 0..n {
                lu.swap(k * n + j, best * n + j);
            }
            perm.swap(k, best);
        }
        let (upper, lower) = lu.split_at_mut((k + 1) * n);
        let pivot_row = &upper[k * n..];
        let pivot = pivot_row[k];
        for row in lower.chunks_exact_mut(n) {
            let factor = row[k] / pivot;
            row[k] = factor;
            if factor != 0.0 {
                for (x, p) in row[k + 1..].iter_mut().zip(&pivot_row[k + 1..]) {
                    *x -= factor * p;
                }
            }
        }
    }
    Some(LuFactors { n, lu, perm })
}

impl LuFactors {
    /// Résout `A · x = b`.
    pub fn solve(&self, b: &[f64]) -> Vec<f64> {
        let n = self.n;
        let mut x: Vec<f64> = self.perm.iter().map(|&i| b[i]).collect();
        for i in 1..n {
            let row = &self.lu[i * n..i * n + i];
            let s: f64 = row.iter().zip(&x[..i]).map(|(a, b)| a * b).sum();
            x[i] -= s;
        }
        for i in (0..n).rev() {
            let row = &self.lu[i * n + i + 1..(i + 1) * n];
            let s: f64 = row.iter().zip(&x[i + 1..]).map(|(a, b)| a * b).sum();
            x[i] = (x[i] - s) / self.lu[i * n + i];
        }
        x
    }

    /// Inverse de la matrice d'origine.
    pub fn inverse(&self) -> SquareMatrix {
        let n = self.n;
        let mut inv = SquareMatrix::zeros(n);
        let mut unit = vec![0.0; n];
        for j in 0..n {
            unit[j] = 1.0;
            let column = self.solve(&unit);
            unit[j] = 0.0;
            for (i, v) in column.into_iter().enumerate() {
                inv.set(i, j, v);
            }
        }
        inv
    }
}

/// Valeurs et vecteurs propres d'une matrice symétrique (méthode de Jacobi).
/// Renvoie `(valeurs, vecteurs)` ; le vecteur `k` est la colonne `k`.
pub(crate) fn symmetric_eigen(mut a: SquareMatrix) -> (Vec<f64>, SquareMatrix) {
    let n = a.n;
    let mut v = SquareMatrix::zeros(n);
    for i in 0..n {
        v.set(i, i, 1.0);
    }
    for _sweep in 0..100 {
        let mut off = 0.0;
        let mut total = 0.0;
        for i in 0..n {
            for j in 0..n {
                let x = a.at(i, j) * a.at(i, j);
                total += x;
                if i != j {
                    off += x;
                }
            }
        }
        if off <= 1e-40 * total {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                let apq = a.at(p, q);
                if apq == 0.0 {
                    continue;
                }
                let theta = (a.at(q, q) - a.at(p, p)) / (2.0 * apq);
                let sign = if theta < 0.0 { -1.0 } else { 1.0 };
                let t = sign / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let (akp, akq) = (a.at(k, p), a.at(k, q));
                    a.set(k, p, c * akp - s * akq);
                    a.set(k, q, s * akp + c * akq);
                }
                for k in 0..n {
                    let (apk, aqk) = (a.at(p, k), a.at(q, k));
                    a.set(p, k, c * apk - s * aqk);
                    a.set(q, k, s * apk + c * aqk);
                }
                for k in 0..n {
                    let (vkp, vkq) = (v.at(k, p), v.at(k, q));
                    v.set(k, p, c * vkp - s * vkq);
                    v.set(k, q, s * vkp + c * vkq);
                }
            }
        }
    }
    ((0..n).map(|i| a.at(i, i)).collect(), v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(n: usize, values: &[f64]) -> SquareMatrix {
        SquareMatrix {
            n,
            data: values.to_vec(),
        }
    }

    #[test]
    fn solves_a_system_that_needs_pivoting() {
        // Le premier pivot est nul : sans échange de lignes, division par 0.
        let a = matrix(3, &[0.0, 2.0, 1.0, 1.0, 1.0, 1.0, 2.0, 0.0, -1.0]);
        let x = factorize(a).unwrap().solve(&[7.0, 6.0, -1.0]);
        for (got, want) in x.iter().zip([1.0, 2.0, 3.0]) {
            assert!((got - want).abs() < 1e-12, "{x:?}");
        }
    }

    #[test]
    fn inverse_times_matrix_is_identity() {
        let a = matrix(3, &[4.0, 1.0, 0.5, 1.0, 3.0, -1.0, 0.5, -1.0, 2.0]);
        let inv = factorize(a.clone()).unwrap().inverse();
        for i in 0..3 {
            let row = inv.row(i);
            for j in 0..3 {
                let p: f64 = (0..3).map(|k| row[k] * a.at(k, j)).sum();
                let want = if i == j { 1.0 } else { 0.0 };
                assert!((p - want).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn singular_matrix_is_refused() {
        let a = matrix(3, &[1.0, 2.0, 3.0, 2.0, 4.0, 6.0, 1.0, 0.0, 1.0]);
        assert!(factorize(a).is_none());
        assert!(factorize(SquareMatrix::zeros(2)).is_none());
    }

    #[test]
    fn eigen_decomposition_of_a_known_matrix() {
        // Valeurs propres connues : 1 et 3, vecteurs (1, -1) et (1, 1).
        let (values, vectors) = symmetric_eigen(matrix(2, &[2.0, 1.0, 1.0, 2.0]));
        let (lo, hi) = if values[0] < values[1] {
            (0, 1)
        } else {
            (1, 0)
        };
        assert!((values[lo] - 1.0).abs() < 1e-12);
        assert!((values[hi] - 3.0).abs() < 1e-12);
        assert!((vectors.at(0, lo) + vectors.at(1, lo)).abs() < 1e-12);
        assert!((vectors.at(0, hi) - vectors.at(1, hi)).abs() < 1e-12);
    }
}

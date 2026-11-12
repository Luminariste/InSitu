//! Les seize repères codés de la mire : famille AprilTag 36h11, numéros 0 à 15.
//!
//! Un repère fait 8 × 8 modules : un pourtour noir d'un module, et 6 × 6
//! modules de données. Autour, il faut un module de blanc pour qu'il soit lu ;
//! la mire le lui donne (voir `super::TargetLayout`).
//!
//! Les codes viennent du dictionnaire `DICT_APRILTAG_36h11` d'OpenCV, celui
//! qu'a utilisé l'étude 2.1 : ils y ont été relus depuis l'image que dessine
//! OpenCV pour chaque numéro. Ils sont écrits ici ligne par ligne, du module
//! en haut à gauche (bit de poids fort) au module en bas à droite ; un bit à 1
//! est un module blanc. Cet ordre est celui de l'image, pas celui de la
//! bibliothèque AprilTag d'origine, qui range ses bits autrement : les
//! nombres diffèrent, les dessins sont les mêmes.

/// Nombre de modules de données par côté.
pub const DATA_MODULES: usize = 6;

/// Nombre de modules par côté, pourtour noir compris.
pub const MARKER_MODULES: usize = DATA_MODULES + 2;

/// Codes des repères 0 à 15 (voir l'en-tête du module pour l'ordre des bits).
pub const TAG36H11_CODES: [u64; 16] = [
    0x21a146bab,
    0x92d18fe9b,
    0x7089014bb,
    0x193979e27,
    0x44153d3d7,
    0x35cd5b8cf,
    0xa10ba56a0,
    0x2b874a608,
    0xb57fb8d44,
    0x4e20b5a64,
    0x61d897f2c,
    0xab3469ffc,
    0x594ca45c2,
    0xfa1c2d2e2,
    0x97c24b972,
    0x75928624a,
];

/// Dessin d'un repère, pourtour compris : `true` = module noir.
/// `[ligne][colonne]`, ligne 0 en haut.
pub fn marker_modules(id: usize) -> [[bool; MARKER_MODULES]; MARKER_MODULES] {
    let code = TAG36H11_CODES[id];
    let mut modules = [[true; MARKER_MODULES]; MARKER_MODULES];
    for row in 0..DATA_MODULES {
        for col in 0..DATA_MODULES {
            let bit = 35 - (row * DATA_MODULES + col);
            let white = (code >> bit) & 1 == 1;
            modules[row + 1][col + 1] = !white;
        }
    }
    modules
}

/// Le code tourné d'un quart de tour dans le sens des aiguilles d'une montre.
#[cfg(test)]
fn rotate(code: u64) -> u64 {
    let n = DATA_MODULES;
    let mut out = 0u64;
    for row in 0..n {
        for col in 0..n {
            // Le module (row, col) de l'image tournée vient de (n-1-col, row).
            let src_bit = 35 - ((n - 1 - col) * n + row);
            let bit = (code >> src_bit) & 1;
            out = (out << 1) | bit;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une famille « 36h11 » garantit au moins 11 bits de différence entre
    /// deux repères, quelle que soit la façon dont ils sont tournés. Un code
    /// mal recopié ou dans le mauvais ordre de bits casserait presque
    /// sûrement cette propriété.
    #[test]
    fn codes_keep_the_family_hamming_distance_in_every_rotation() {
        let mut min = u32::MAX;
        for (i, &a) in TAG36H11_CODES.iter().enumerate() {
            // Un repère ne doit pas ressembler à lui-même tourné.
            let mut r = a;
            for _ in 0..3 {
                r = rotate(r);
                min = min.min((a ^ r).count_ones());
            }
            for &b in &TAG36H11_CODES[i + 1..] {
                let mut r = b;
                for _ in 0..4 {
                    min = min.min((a ^ r).count_ones());
                    r = rotate(r);
                }
            }
        }
        assert!(min >= 11, "distance minimale {min}");
    }

    #[test]
    fn rotating_four_times_gives_the_code_back() {
        for &code in &TAG36H11_CODES {
            assert_eq!(rotate(rotate(rotate(rotate(code)))), code);
        }
    }

    #[test]
    fn markers_have_a_black_border() {
        for id in 0..16 {
            let m = marker_modules(id);
            let last = MARKER_MODULES - 1;
            assert!(m[0].iter().chain(&m[last]).all(|&black| black));
            assert!(m.iter().all(|row| row[0] && row[last]));
        }
    }

    /// Repère 0 tel que le dessine OpenCV (`generateImageMarker`, 8 px :
    /// un pixel par module, pourtour compris ; 1 = blanc).
    #[test]
    fn marker_zero_matches_opencv_drawing() {
        let expected = ["001000", "011010", "000101", "000110", "101110", "101011"];
        let m = marker_modules(0);
        for (row, line) in expected.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                assert_eq!(m[row + 1][col + 1], ch == '0', "ligne {row}, colonne {col}");
            }
        }
    }
}

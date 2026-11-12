//! Critère d'acceptation de la tâche 4.1 : 200 dessins de 4 096 px importés
//! et parcourus sans attente.
//!
//! Les dessins sont fabriqués par le test (traits blancs sur fond noir, des
//! tracés au hasard de 9 px d'épaisseur), enregistrés en PNG en gris comme
//! les verres d'Épinal. Le test rapide (lancé à chaque vérification) en fait
//! 200 de 512 px ; la mesure en 4 096 px ne tourne qu'en version optimisée
//! (la vérification GitHub la lance ainsi) :
//!
//! ```text
//! cargo test --release -p insitu-core --test bibliotheque -- --nocapture
//! ```
//!
//! « Parcourus sans attente » : une fois l'import fini, relire toute la
//! bibliothèque avec ses vignettes ne demande que des lectures de dates de
//! fichiers (moins d'une seconde pour 200 dessins, quelques millisecondes en
//! pratique), et chaque vignette pèse quelques kilo-octets.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder};
use insitu_core::library::vectors::vectorize_drawings;
use insitu_core::library::{import_folder, list, ImportOptions, THUMBNAIL_SIDE};
use insitu_core::project::ProjectDir;

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Dessin d'essai : quelques tracés au hasard, reproductibles.
fn drawing(side: u32, seed: u64) -> Vec<u8> {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1u64 << 53) as f64
    };
    let s = f64::from(side);
    let radius = (s / 900.0).max(1.0); // 9 px d'épaisseur à 4 096 px
    let mut pixels = vec![0u8; (side * side) as usize];
    for _ in 0..12 {
        let (mut x, mut y) = (s * (0.1 + 0.8 * next()), s * (0.1 + 0.8 * next()));
        let mut heading = next() * std::f64::consts::TAU;
        for _ in 0..(s * 0.6) as usize {
            heading += (next() - 0.5) * 0.25;
            x = (x + heading.cos()).clamp(radius, s - 1.0 - radius);
            y = (y + heading.sin()).clamp(radius, s - 1.0 - radius);
            let r = radius.ceil() as i64;
            for dy in -r..=r {
                for dx in -r..=r {
                    if ((dx * dx + dy * dy) as f64) <= radius * radius {
                        let (px, py) = (x as i64 + dx, y as i64 + dy);
                        pixels[(py as u32 * side + px as u32) as usize] = 255;
                    }
                }
            }
        }
    }
    pixels
}

/// Fabrique `count` dessins de `side` px dans `folder`, sur tous les cœurs.
fn make_drawings(folder: &Path, count: u64, side: u32) {
    fs::create_dir_all(folder.join("Série B")).unwrap();
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()) as u64;
    std::thread::scope(|scope| {
        for t in 0..threads {
            scope.spawn(move || {
                for i in (t..count).step_by(threads as usize) {
                    let sub = if i % 2 == 0 { "" } else { "Série B/" };
                    let path = folder.join(format!("{sub}dessin-{i:03}.png"));
                    let file = std::io::BufWriter::new(fs::File::create(path).unwrap());
                    PngEncoder::new_with_quality(file, CompressionType::Fast, FilterType::Adaptive)
                        .write_image(&drawing(side, i + 1), side, side, ExtendedColorType::L8)
                        .unwrap();
                }
            });
        }
    });
}

fn run(side: u32) {
    let root =
        std::env::temp_dir().join(format!("insitu-bibliotheque-{side}-{}", std::process::id()));
    let _cleanup = Scratch(root.clone());
    let source = root.join("Dessins de Marc");
    let cache = root.join("cache");
    let start = Instant::now();
    make_drawings(&source, 200, side);
    println!(
        "200 dessins de {side} px fabriqués en {:.1} s",
        start.elapsed().as_secs_f64()
    );

    let (dir, _) = ProjectDir::create(root.join("Essai.insitu"), Some("Essai")).unwrap();
    let options = ImportOptions {
        cache_root: Some(cache.clone()),
        threads: 0,
    };
    let start = Instant::now();
    let (project, report) = import_folder(&dir, &source, &options, &|_| {}).unwrap();
    let import_seconds = start.elapsed().as_secs_f64();
    assert_eq!(report.added.len(), 200);
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert_eq!(project.library.len(), 200);
    assert_eq!(
        project
            .library
            .iter()
            .filter(|d| d.folder() == "Dessins de Marc/Série B")
            .count(),
        100
    );

    // Les vecteurs des nouveaux dessins (tâche 5.2) sont faits après
    // l'import, pendant qu'on parcourt déjà : mesurés à part, sans seuil.
    let start = Instant::now();
    let vectors = vectorize_drawings(&dir, &report.added, &cache, 0, &|_| {}).unwrap();
    assert_eq!(vectors.done, 200, "{:?}", vectors.failures);
    println!(
        "vectorisation après l'import : {:.1} s ({:.0} ms par dessin)",
        start.elapsed().as_secs_f64(),
        start.elapsed().as_secs_f64() * 1000.0 / 200.0
    );

    let start = Instant::now();
    let entries = list(&dir, Some(&cache), 0).unwrap();
    let browse_seconds = start.elapsed().as_secs_f64();
    let mut thumbnail_bytes = 0;
    for entry in &entries {
        assert_eq!(entry.width, f64::from(side));
        let path = cache.join(
            entry
                .thumbnail
                .as_ref()
                .expect("vignette prête dès l'import"),
        );
        thumbnail_bytes += fs::metadata(&path).unwrap().len();
        let (w, h) = image::image_dimensions(&path).unwrap();
        assert!(w.max(h) <= THUMBNAIL_SIDE);
    }
    println!(
        "import : {import_seconds:.1} s ({:.0} ms par dessin, {} cœurs) ; relecture avec vignettes : {:.0} ms ; vignettes : {} Ko en tout",
        import_seconds * 1000.0 / 200.0,
        std::thread::available_parallelism().map_or(1, |n| n.get()),
        browse_seconds * 1000.0,
        thumbnail_bytes / 1024
    );
    assert!(
        browse_seconds < 1.0,
        "relecture trop lente : {browse_seconds:.2} s"
    );
    assert!(thumbnail_bytes / 200 < 64 * 1024, "vignettes trop lourdes");
}

#[test]
fn two_hundred_drawings_are_imported_and_browsed() {
    run(512);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "mesure en 4 096 px : en version optimisée seulement (--release)"
)]
fn two_hundred_full_size_drawings_are_imported_and_browsed() {
    run(4096);
}

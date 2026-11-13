use super::psd::{decode_psd, encode_psd};
use super::*;
use crate::project::test_support::TempDir;
use crate::project::validate;

/// Un dessin d'essai : traits blancs de 3 px sur fond noir (un cercle et une
/// diagonale), en gris.
pub(crate) fn line_drawing(side: u32, seed: u32) -> Raster {
    let s = side as f64;
    let (cx, cy) = (s * (0.4 + 0.02 * f64::from(seed % 5)), s * 0.5);
    let r = s * 0.3;
    let mut data = vec![0u8; (side * side) as usize];
    for y in 0..side {
        for x in 0..side {
            let (fx, fy) = (f64::from(x), f64::from(y));
            let ring = ((fx - cx).hypot(fy - cy) - r).abs() < 1.5;
            let diagonal = (fx - fy).abs() < 1.5;
            if ring || diagonal {
                data[(y * side + x) as usize] = 255;
            }
        }
    }
    Raster::new(side, side, 1, data)
}

fn save(raster: &Raster, path: &Path) {
    let color = match raster.channels {
        1 => image::ColorType::L8,
        3 => image::ColorType::Rgb8,
        _ => image::ColorType::Rgba8,
    };
    image::save_buffer(path, &raster.data, raster.width, raster.height, color)
        .expect("écriture de l'image");
}

fn to_rgb(raster: &Raster) -> Raster {
    let data = raster.data.iter().flat_map(|&v| [v, v / 2, v]).collect();
    Raster::new(raster.width, raster.height, 3, data)
}

const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="200">
  <rect width="400" height="200" fill="black"/>
  <path d="M20 100 C 120 0, 280 200, 380 100" stroke="white" stroke-width="12" fill="none"/>
</svg>"#;

/// Un projet et un dossier « Oiseaux » de dessins de tous les formats.
struct Fixture {
    _temp: TempDir,
    dir: ProjectDir,
    source: PathBuf,
    cache: PathBuf,
}

fn fixture() -> Fixture {
    let temp = TempDir::new("bibliotheque");
    let (dir, _) =
        ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).expect("projet");
    let source = temp.path().join("Oiseaux");
    let deep = source.join("Grands").join("Rapaces");
    fs::create_dir_all(&deep).expect("dossiers");
    save(&line_drawing(96, 1), &source.join("merle.png"));
    save(&to_rgb(&line_drawing(80, 2)), &source.join("pie.bmp"));
    save(
        &line_drawing(64, 3),
        &source.join("Grands").join("héron.tif"),
    );
    save(
        &to_rgb(&line_drawing(72, 4)),
        &source.join("Grands").join("cygne.JPG"),
    );
    fs::write(
        deep.join("buse.psd"),
        encode_psd(&to_rgb(&line_drawing(70, 5)), true),
    )
    .expect("psd");
    fs::write(deep.join("aigle.svg"), SVG).expect("svg");
    fs::write(source.join("casse.png"), b"pas une image").expect("faux png");
    fs::write(source.join(".DS_Store"), b"x").expect("fichier cache");
    fs::write(source.join("notes.txt"), b"x").expect("texte");
    let cache = temp.path().join("cache");
    Fixture {
        dir,
        source,
        cache,
        _temp: temp,
    }
}

fn import(f: &Fixture) -> (Project, ImportReport) {
    let options = ImportOptions {
        cache_root: Some(f.cache.clone()),
        threads: 3,
    };
    import_folder(&f.dir, &f.source, &options, &|_| {}).expect("import")
}

#[test]
fn black_becomes_transparent_and_composites_back_exactly() {
    let gray = Raster::new(3, 1, 1, vec![0, 128, 255]);
    assert_eq!(
        black_to_alpha(&gray),
        vec![255, 255, 255, 0, 255, 255, 255, 128, 255, 255, 255, 255]
    );
    // Toute couleur, posée sur du noir, redonne le pixel d'origine (à une
    // unité près, par arrondi).
    let mut data = Vec::new();
    for r in (0..=255u8).step_by(15) {
        for g in (0..=255u8).step_by(17) {
            for b in (0..=255u8).step_by(51) {
                data.extend([r, g, b]);
            }
        }
    }
    let n = data.len() / 3;
    let rgb = Raster::new(n as u32, 1, 3, data.clone());
    let rgba = black_to_alpha(&rgb);
    for (i, px) in rgba.as_chunks::<4>().0.iter().enumerate() {
        let a = u32::from(px[3]);
        for k in 0..3 {
            let back = (u32::from(px[k]) * a + 127) / 255;
            let original = u32::from(data[i * 3 + k]);
            assert!(
                back.abs_diff(original) <= 1,
                "pixel {i} : {back} au lieu de {original}"
            );
        }
    }
    // Une image déjà transparente est d'abord posée sur du noir.
    let with_alpha = Raster::new(1, 1, 4, vec![255, 255, 255, 64]);
    assert_eq!(black_to_alpha(&with_alpha), vec![255, 255, 255, 64]);
}

#[test]
fn a_rendition_halves_until_it_fits_and_keeps_thin_lines_visible() {
    let small = rendition(line_drawing(1024, 0), THUMBNAIL_SIDE);
    assert_eq!((small.width, small.height, small.channels), (256, 256, 4));
    // Un trait de 3 px réduit 4 fois couvre 3/4 de pixel : la moyenne en
    // lumière le garde nettement visible.
    let strongest = small
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| p[3])
        .max()
        .unwrap();
    assert!(
        strongest > 180,
        "trait trop pâle dans la vignette : {strongest}"
    );
    let top_right = &small.data[(255 * 4)..(256 * 4)];
    assert_eq!(top_right[3], 0, "le fond doit être transparent");
    let untouched = rendition(line_drawing(200, 0), THUMBNAIL_SIDE);
    assert_eq!((untouched.width, untouched.height), (200, 200));
}

#[test]
fn every_format_is_read_with_its_size() {
    let f = fixture();
    let deep = f.source.join("Grands").join("Rapaces");
    let psd = decode(&deep.join("buse.psd"), THUMBNAIL_SIDE).expect("psd");
    assert_eq!(
        (psd.width, psd.height, psd.raster.channels),
        (70.0, 70.0, 3)
    );
    assert_eq!(
        psd.raster,
        to_rgb(&line_drawing(70, 5)),
        "image aplatie lue à l'identique"
    );
    let svg = decode(&deep.join("aigle.svg"), THUMBNAIL_SIDE).expect("svg");
    assert_eq!((svg.width, svg.height), (400.0, 200.0));
    assert_eq!((svg.raster.width, svg.raster.height), (320, 160));
    let bmp = decode(&f.source.join("pie.bmp"), THUMBNAIL_SIDE).expect("bmp");
    assert_eq!(bmp.raster, to_rgb(&line_drawing(80, 2)));
    let broken = decode(&f.source.join("casse.png"), THUMBNAIL_SIDE);
    assert!(matches!(broken, Err(LibraryError::Unreadable { .. })));
}

#[test]
fn a_folder_is_imported_with_its_subfolders_and_thumbnails() {
    let f = fixture();
    let calls = Mutex::new(Vec::new());
    let options = ImportOptions {
        cache_root: Some(f.cache.clone()),
        threads: 3,
    };
    let (project, report) = import_folder(&f.dir, &f.source, &options, &|p| {
        calls.lock().unwrap().push(p)
    })
    .expect("import");

    assert_eq!(report.folder, "bibliotheque/Oiseaux");
    assert_eq!(report.added.len(), 6);
    assert_eq!(report.already_present, 0);
    assert_eq!(report.failures.len(), 1, "{:?}", report.failures);
    assert_eq!(report.failures[0].file, "casse.png");
    let calls = calls.into_inner().unwrap();
    assert_eq!(calls.first(), Some(&ImportProgress { done: 0, total: 7 }));
    assert_eq!(calls.iter().map(|p| p.done).max(), Some(7));

    let sources: Vec<&str> = project.library.iter().map(|d| d.source.as_str()).collect();
    assert_eq!(
        sources,
        [
            "bibliotheque/Oiseaux/Grands/Rapaces/aigle.svg",
            "bibliotheque/Oiseaux/Grands/Rapaces/buse.psd",
            "bibliotheque/Oiseaux/Grands/cygne.JPG",
            "bibliotheque/Oiseaux/Grands/héron.tif",
            "bibliotheque/Oiseaux/merle.png",
            "bibliotheque/Oiseaux/pie.bmp",
        ]
    );
    let heron = project
        .library
        .iter()
        .find(|d| d.name == "héron")
        .expect("héron");
    assert_eq!(heron.folder(), "Oiseaux/Grands");
    assert_eq!((heron.width, heron.height), (64.0, 64.0));
    // Copies à l'identique, et le projet relu est le même.
    for drawing in &project.library {
        assert!(f.dir.resolve(&drawing.source).is_file());
    }
    assert!(same_bytes(
        &f.source.join("merle.png"),
        &f.dir.root().join("bibliotheque/Oiseaux/merle.png")
    ));
    assert_eq!(f.dir.load().expect("relecture").project, project);
    assert!(validate(&f.dir).is_valid());

    // Vignettes : déjà là, transparentes, au bon format.
    let entries = list(&f.dir, Some(&f.cache), 2).expect("liste");
    assert_eq!(entries.len(), 6);
    for entry in &entries {
        assert_eq!(entry.problem, None);
        let path = f.cache.join(entry.thumbnail.as_deref().expect("vignette"));
        let thumb = image::open(&path).expect("vignette lisible").into_rgba8();
        assert!(thumb.width().max(thumb.height()) <= THUMBNAIL_SIDE);
        assert_eq!(
            thumb.get_pixel(0, thumb.height() - 1)[3],
            0,
            "{} : coin opaque",
            entry.name
        );
    }
}

#[test]
fn importing_again_adds_nothing_but_a_changed_file_comes_in_beside() {
    let f = fixture();
    import(&f);
    let (project, report) = import(&f);
    assert_eq!((report.added.len(), report.already_present), (0, 6));
    assert_eq!(project.library.len(), 6);

    save(&line_drawing(96, 9), &f.source.join("merle.png"));
    let (project, report) = import(&f);
    assert_eq!((report.added.len(), report.already_present), (1, 5));
    let added = project.drawing(&report.added[0]).expect("nouveau");
    assert_eq!(added.source.as_str(), "bibliotheque/Oiseaux/merle-2.png");
    assert_eq!(added.name, "merle");
}

#[test]
fn missing_thumbnails_are_made_again_and_missing_files_reported() {
    let f = fixture();
    let (project, _) = import(&f);
    fs::remove_dir_all(&f.cache).expect("cache vidé");
    fs::remove_file(f.dir.root().join("bibliotheque/Oiseaux/pie.bmp")).expect("fichier retiré");
    let entries = list(&f.dir, Some(&f.cache), 0).expect("liste");
    assert_eq!(entries.len(), project.library.len());
    for entry in entries {
        if entry.name == "pie" {
            assert!(entry.thumbnail.is_none());
            assert!(entry.problem.is_some());
        } else {
            assert!(f
                .cache
                .join(entry.thumbnail.expect("vignette refaite"))
                .is_file());
        }
    }
    let merle = project.library.iter().find(|d| d.name == "merle").unwrap();
    let request = preview(&f.dir, &merle.id, &f.cache).expect("aperçu");
    let big = image::open(f.cache.join(request)).expect("aperçu lisible");
    assert_eq!((big.width(), big.height()), (96, 96));
}

#[test]
fn bad_folders_are_refused() {
    let f = fixture();
    let options = ImportOptions::default();
    let inside = f.dir.root().join("bibliotheque");
    assert!(matches!(
        import_folder(&f.dir, &inside, &options, &|_| {}),
        Err(LibraryError::InsideProject(_))
    ));
    let empty = f.source.parent().unwrap().join("Vide");
    fs::create_dir_all(&empty).unwrap();
    assert!(matches!(
        import_folder(&f.dir, &empty, &options, &|_| {}),
        Err(LibraryError::NoDrawings(_))
    ));
    assert!(matches!(
        import_folder(&f.dir, &empty.join("absent"), &options, &|_| {}),
        Err(LibraryError::NotAFolder(_))
    ));
}

#[test]
fn awkward_names_stay_inside_the_project() {
    assert_eq!(clean_segment("a:b"), "a-b");
    assert_eq!(clean_segment(".."), "_");
    assert_eq!(clean_segment("Été 2026"), "Été 2026");
}

#[test]
fn photoshop_files_are_read_compressed_or_not() {
    for raster in [line_drawing(37, 1), to_rgb(&line_drawing(150, 2))] {
        for compressed in [false, true] {
            let back = decode_psd(&encode_psd(&raster, compressed)).expect("psd relu");
            assert_eq!(back, raster, "compressé : {compressed}");
        }
    }
    let mut cmyk = encode_psd(&line_drawing(8, 0), false);
    cmyk[24..26].copy_from_slice(&4u16.to_be_bytes());
    assert!(decode_psd(&cmyk).unwrap_err().contains("CMJN"));
    let truncated = encode_psd(&line_drawing(8, 0), true);
    assert!(decode_psd(&truncated[..truncated.len() - 3]).is_err());
    assert!(decode_psd(b"GIF89a").is_err());
}

#[test]
fn sixteen_bit_and_bitmap_photoshop_files_are_read() {
    let header = |depth: u16, mode: u16| {
        let mut out = b"8BPS".to_vec();
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&[0; 6]);
        out.extend_from_slice(&1u16.to_be_bytes()); // un canal
        out.extend_from_slice(&1u32.to_be_bytes()); // hauteur
        out.extend_from_slice(&3u32.to_be_bytes()); // largeur
        out.extend_from_slice(&depth.to_be_bytes());
        out.extend_from_slice(&mode.to_be_bytes());
        out.extend_from_slice(&[0; 12]);
        out.extend_from_slice(&0u16.to_be_bytes());
        out
    };
    let mut sixteen = header(16, 1);
    sixteen.extend_from_slice(&[0x00, 0x10, 0x80, 0x00, 0xff, 0xff]);
    assert_eq!(
        decode_psd(&sixteen).expect("16 bits").data,
        vec![0, 128, 255]
    );
    let mut bitmap = header(1, 0);
    bitmap.push(0b0100_0000); // 1 = noir
    assert_eq!(decode_psd(&bitmap).expect("bitmap").data, vec![255, 0, 255]);
}

// ---------------------------------------------------------------------------
// Vecteurs en bibliothèque (tâche 5.2)
// ---------------------------------------------------------------------------

use crate::project::VectorSettings;
use vectors::{drawing_vectors, set_vector_settings, vector_request_prefix, vectorize_drawings};

#[test]
fn every_imported_drawing_is_vectorized_within_half_a_pixel() {
    let f = fixture();
    let (project, report) = import(&f);
    let calls = Mutex::new(Vec::new());
    let done = vectorize_drawings(&f.dir, &report.added, &f.cache, 3, &|p| {
        calls.lock().unwrap().push(p)
    })
    .expect("vectorisation");
    assert_eq!(done.done, 6);
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(
        calls.into_inner().unwrap().iter().map(|p| p.done).max(),
        Some(6)
    );
    for drawing in &project.library {
        let path = f.dir.resolve(&drawing.source);
        // Vectorisé juste après l'import : le compte rendu est déjà en cache.
        let prefix = vector_request_prefix(&path, &VectorSettings::default()).expect("clé");
        assert!(
            f.cache.join(format!("{prefix}.json")).is_file(),
            "{} pas vectorisé après l'import",
            drawing.name
        );
        let summary = drawing_vectors(&f.dir, &drawing.id, &f.cache).expect("vecteurs");
        assert!(summary.default_settings);
        assert!(
            summary.stats.contours > 0,
            "{} : aucun contour",
            drawing.name
        );
        assert!(
            summary.comparison.max_deviation_px < 0.5,
            "{} : écart {} px",
            drawing.name,
            summary.comparison.max_deviation_px
        );
        let filled = fs::read_to_string(f.cache.join(&summary.filled_svg)).expect("trait");
        let outline = fs::read_to_string(f.cache.join(&summary.outline_svg)).expect("contour");
        let view_box = format!("viewBox=\"0 0 {} {}\"", summary.width, summary.height);
        assert!(filled.contains(&view_box) && outline.contains(&view_box));
        assert!(filled.contains("fill=\"white\"") && !filled.contains("<rect"));
        assert!(outline.contains("fill=\"none\"") && outline.contains("stroke="));
        assert!(
            f.cache.join(&summary.image).is_file(),
            "image de comparaison"
        );
    }
    // Un SVG est vectorisé depuis son rendu à 4 096 px.
    let aigle = project.library.iter().find(|d| d.name == "aigle").unwrap();
    let summary = drawing_vectors(&f.dir, &aigle.id, &f.cache).unwrap();
    assert_eq!((summary.width, summary.height), (4096, 2048));
    // Une image, à sa taille.
    let merle = project.library.iter().find(|d| d.name == "merle").unwrap();
    let summary = drawing_vectors(&f.dir, &merle.id, &f.cache).unwrap();
    assert_eq!((summary.width, summary.height), (96, 96));
}

#[test]
fn settings_are_kept_per_drawing_and_change_the_vectors() {
    let f = fixture();
    let (project, _) = import(&f);
    let merle = project
        .library
        .iter()
        .find(|d| d.name == "merle")
        .unwrap()
        .id
        .clone();
    let pie = project
        .library
        .iter()
        .find(|d| d.name == "pie")
        .unwrap()
        .id
        .clone();
    let before = drawing_vectors(&f.dir, &merle, &f.cache).unwrap();

    let custom = VectorSettings {
        threshold: 200,
        tolerance_px: 1.5,
        corner_angle_deg: 45.0,
        min_area_px: 4.0,
        smoothing_px: 2.5,
    };
    let (saved, after) = set_vector_settings(&f.dir, &merle, Some(custom), &f.cache).unwrap();
    assert_eq!(after.settings, custom);
    assert!(!after.default_settings);
    assert_ne!(
        after.filled_svg, before.filled_svg,
        "nouveau fichier en cache"
    );
    // Plus de tolérance : moins de morceaux.
    let pieces = |s: &vectors::VectorSummary| s.stats.lines + s.stats.cubics;
    assert!(
        pieces(&after) < pieces(&before),
        "{} ≥ {}",
        pieces(&after),
        pieces(&before)
    );
    // Enregistré dans le projet, pour ce dessin seulement.
    let reread = f.dir.load().unwrap().project;
    assert_eq!(reread, saved);
    assert_eq!(reread.drawing(&merle).unwrap().vector, Some(custom));
    assert_eq!(reread.drawing(&pie).unwrap().vector, None);
    let text = fs::read_to_string(f.dir.root().join("projet.json")).unwrap();
    assert_eq!(text.matches("\"vector\"").count(), 1);
    assert!(text.contains("\"tolerancePx\": 1.5"));
    let entries = list(&f.dir, Some(&f.cache), 2).unwrap();
    assert_eq!(
        entries.iter().find(|e| e.id == merle).unwrap().vector,
        Some(custom)
    );
    assert!(validate(&f.dir).is_valid());
    // Relu : les mêmes vecteurs, depuis le cache.
    assert_eq!(drawing_vectors(&f.dir, &merle, &f.cache).unwrap(), after);

    // Retour au défaut, explicite ou par des réglages égaux au défaut : rien
    // n'est écrit dans le projet, et les premiers vecteurs reviennent.
    let (_, back) = set_vector_settings(&f.dir, &merle, None, &f.cache).unwrap();
    assert!(back.default_settings);
    assert_eq!(back.filled_svg, before.filled_svg);
    set_vector_settings(&f.dir, &merle, Some(custom), &f.cache).unwrap();
    set_vector_settings(&f.dir, &merle, Some(VectorSettings::default()), &f.cache).unwrap();
    assert_eq!(
        f.dir
            .load()
            .unwrap()
            .project
            .drawing(&merle)
            .unwrap()
            .vector,
        None
    );
    assert!(!fs::read_to_string(f.dir.root().join("projet.json"))
        .unwrap()
        .contains("\"vector\""));
}

#[test]
fn refused_settings_leave_the_project_untouched() {
    let f = fixture();
    let (project, _) = import(&f);
    let id = project.library[0].id.clone();
    let before = fs::read(f.dir.root().join("projet.json")).unwrap();
    for wrong in [
        VectorSettings {
            threshold: 0,
            ..VectorSettings::default()
        },
        VectorSettings {
            tolerance_px: 0.0,
            ..VectorSettings::default()
        },
        VectorSettings {
            corner_angle_deg: 180.0,
            ..VectorSettings::default()
        },
        VectorSettings {
            min_area_px: -1.0,
            ..VectorSettings::default()
        },
        VectorSettings {
            smoothing_px: 25.0,
            ..VectorSettings::default()
        },
    ] {
        let refused = set_vector_settings(&f.dir, &id, Some(wrong), &f.cache);
        assert!(
            matches!(refused, Err(LibraryError::Settings(_))),
            "{wrong:?}"
        );
    }
    assert_eq!(fs::read(f.dir.root().join("projet.json")).unwrap(), before);
    assert!(matches!(
        drawing_vectors(&f.dir, "inconnu", &f.cache),
        Err(LibraryError::UnknownDrawing(_))
    ));
}

#[test]
fn the_threshold_decides_whether_grey_strokes_are_kept() {
    // Un trait blanc et un trait gris (100) : au seuil 128, seul le blanc
    // reste ; au seuil 64, les deux.
    let side = 120u32;
    let mut data = vec![0u8; (side * side) as usize];
    for y in 20..100 {
        for x in 30..36 {
            data[(y * side + x) as usize] = 255;
        }
        for x in 80..86 {
            data[(y * side + x) as usize] = 100;
        }
    }
    let temp = TempDir::new("vecteurs-seuil");
    let (dir, _) = ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).unwrap();
    let source = temp.path().join("Gris");
    fs::create_dir_all(&source).unwrap();
    save(
        &Raster::new(side, side, 1, data),
        &source.join("deux-traits.png"),
    );
    let cache = temp.path().join("cache");
    let options = ImportOptions {
        cache_root: Some(cache.clone()),
        threads: 1,
    };
    let (project, _) = import_folder(&dir, &source, &options, &|_| {}).unwrap();
    let id = project.library[0].id.clone();
    let bright = drawing_vectors(&dir, &id, &cache).unwrap();
    assert_eq!(bright.stats.contours, 1);
    let low = VectorSettings {
        threshold: 64,
        ..VectorSettings::default()
    };
    let (_, both) = set_vector_settings(&dir, &id, Some(low), &cache).unwrap();
    assert_eq!(both.stats.contours, 2);
    assert!(both.comparison.max_deviation_px < 0.5);
}

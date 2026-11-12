use super::*;
use crate::project::test_support::TempDir;

/// Texte de données privées d'essai : un plan de travail de 1 000 × 500 pt
/// (x de 0 à 1 000, y de −500 à 0, y vers le haut), deux calques dont un
/// masqué, trois images liées (droite, tournée, retournée), une image liée
/// à l'ancienne et une incorporée.
fn sample_text(oiseau_link: &str) -> String {
    format!(
        "%!PS-Adobe-3.0 \n\
         %%Creator: Adobe Illustrator(R) 24.0\n\
         %AI3_Cropmarks: 0 -500 1000 0\n\
         %%EndSetup\n\
         %AI5_BeginLayer\n\
         1 1 1 1 0 0 1 0 79 128 255 0 50 0 Lb\n\
         (Fond) Ln\n\
         %AI5_BeginPlace\n\
         %%FileType: 1\n\
         [0.1 0 0 -0.1 100 -100] 0 0 1000 500 4 4 0\n\
         ({oiseau_link})`\n\
         %%IncludeFile:/Users/autre/Dessins/Oiseaux/merle.png\n\
         %AI28_BeginPlacedRelativePath\n\
         (../Oiseaux/)\n\
         %AI28_EndPlacedRelativePath\n\
         %AI5_EndPlace\n\
         %AI5_BeginPlace\n\
         %%FileType: 1\n\
         [0 0.2 0.2 0 500 -250] 0 0 400 400 4 4 0\n\
         (/Users/autre/Dessins/Poissons/Mure\u{300}ne.png)`\n\
         %AI28_BeginPlacedRelativePath\n\
         (../Poissons/)\n\
         %AI28_EndPlacedRelativePath\n\
         %AI5_EndPlace\n\
         %AI5_BeginRaster\n\
         () 1 XG\n\
         [ 1 0 0 1 0 0 ] 10 10 0 Xh\n\
         %AI5_EndRaster\n\
         %AI5_EndLayer--\n\
         %AI5_BeginLayer\n\
         0 1 0 1 0 0 1 0 79 128 255 0 50 0 Lb\n\
         (Esquisses) Ln\n\
         %AI5_BeginPlace\n\
         %%FileType: 1\n\
         [-0.1 0 0 -0.1 900 -300] 0 0 1000 500 4 4 0\n\
         (/Users/autre/Dessins/Oiseaux/merle.png)`\n\
         %AI28_BeginPlacedRelativePath\n\
         (../Oiseaux/)\n\
         %AI28_EndPlacedRelativePath\n\
         %AI5_EndPlace\n\
         %AI5_BeginRaster\n\
         (/Users/autre/Dessins/absent.png) 0 XG\n\
         [ 1 0 0 1 10 -10 ] 20 30 0 Xh\n\
         %AI5_EndRaster\n\
         %AI5_EndLayer--\n"
    )
}

/// Un `.ai` d'essai : une page PDF dont les données Illustrator sont
/// compressées (zlib ou zstd), coupées en deux morceaux.
fn sample_ai(text: &str, zstd: bool) -> Vec<u8> {
    use pdf_writer::{Name, Pdf, Rect, Ref};
    let compressed = if zstd {
        let mut v = b"%AI24_ZStandard_Data".to_vec();
        v.extend(ruzstd::encoding::compress_to_vec(
            text.as_bytes(),
            ruzstd::encoding::CompressionLevel::Fastest,
        ));
        v
    } else {
        let mut v = b"%AI12_CompressedData".to_vec();
        v.extend(miniz_oxide::deflate::compress_to_vec_zlib(
            text.as_bytes(),
            6,
        ));
        v
    };
    let (first, second) = compressed.split_at(compressed.len() / 2);
    let mut pdf = Pdf::new();
    let (catalog, tree, page, content, p1, p2) = (
        Ref::new(1),
        Ref::new(2),
        Ref::new(3),
        Ref::new(4),
        Ref::new(5),
        Ref::new(6),
    );
    pdf.catalog(catalog).pages(tree);
    pdf.pages(tree).kids([page]).count(1);
    {
        let mut p = pdf.page(page);
        p.media_box(Rect::new(0.0, 0.0, 1000.0, 500.0));
        p.parent(tree);
        p.contents(content);
        let mut info = p.insert(Name(b"PieceInfo")).dict();
        let mut illustrator = info.insert(Name(b"Illustrator")).dict();
        let mut private = illustrator.insert(Name(b"Private")).dict();
        // Dans le désordre : l'ordre vient du numéro.
        private.pair(Name(b"AIPrivateData2"), p2);
        private.pair(Name(b"AIPrivateData1"), p1);
    }
    pdf.stream(content, b"");
    pdf.stream(p1, first);
    pdf.stream(p2, second);
    pdf.finish()
}

#[test]
fn private_data_is_read_and_parsed() {
    for zstd in [false, true] {
        let text = sample_text("/Users/autre/Dessins/Oiseaux/merle.png");
        let ai = sample_ai(&text, zstd);
        assert_eq!(private_data(&ai).unwrap(), text.as_bytes(), "zstd : {zstd}");
        let doc = parse(text.as_bytes()).unwrap();
        assert_eq!(doc.artboard, [0.0, -500.0, 1000.0, 0.0]);
        assert_eq!(doc.embedded, 1);
        assert_eq!(doc.placed.len(), 4);
        let layers: Vec<_> = doc
            .layers
            .iter()
            .map(|l| (l.name.as_str(), l.visible))
            .collect();
        assert_eq!(layers, [("Fond", true), ("Esquisses", false)]);
        let p = &doc.placed[0];
        assert_eq!(p.layer, "Fond");
        assert!(p.visible);
        assert_eq!(p.file_name(), "merle.png");
        assert_eq!(p.relative_dir.as_deref(), Some("../Oiseaux/"));
        assert_eq!(p.size_px(), (1000.0, 500.0));
        assert_eq!(doc.placed[1].file_name(), "Mure\u{300}ne.png");
        assert_eq!(doc.placed[2].layer, "Esquisses");
        assert!(!doc.placed[2].visible);
        // L'image liée à l'ancienne : chemin, taille, matrice ramenée.
        let old = &doc.placed[3];
        assert_eq!(old.absolute, "/Users/autre/Dessins/absent.png");
        assert_eq!(old.size_px(), (20.0, 30.0));
        assert_eq!(old.matrix, [1.0, -0.0, 0.0, -1.0, 10.0, -10.0]);
    }
    // Un PDF ordinaire n'a pas de données Illustrator.
    let plain = {
        use pdf_writer::{Pdf, Rect, Ref};
        let mut pdf = Pdf::new();
        pdf.catalog(Ref::new(1)).pages(Ref::new(2));
        pdf.pages(Ref::new(2)).kids([Ref::new(3)]).count(1);
        pdf.page(Ref::new(3))
            .media_box(Rect::new(0.0, 0.0, 10.0, 10.0))
            .parent(Ref::new(2));
        pdf.finish()
    };
    let message = private_data(&plain).unwrap_err();
    assert!(message.contains("compatible PDF"), "{message}");
}

/// Les coins du dessin posé tombent sur les coins de l'image, tels que la
/// matrice d'Illustrator les place, quels que soient l'échelle, la
/// rotation et le retournement.
#[test]
fn the_pose_puts_the_drawing_corners_where_illustrator_does() {
    let artboard = [-200.0, -900.0, 1_200.0, 100.0];
    let frame = RectMm {
        x: 50.0,
        y: 20.0,
        width: 28_000.0,
        height: 20_000.0,
    };
    let k = 20.0; // mm de façade par point
    let cases = [
        (0.0, 0.03, false),
        (30.0, 0.05, false),
        (-120.0, 0.02, true),
        (90.0, 0.04, true),
        (200.0, 0.01, false),
    ];
    for (degrees, scale, flip) in cases {
        let (sin, cos) = f64::to_radians(degrees).sin_cos();
        // Dans le repère d'Illustrator (y vers le haut), une image droite a
        // d < 0 ; on la tourne (sens trigonométrique) et on la retourne.
        let sx = if flip { -scale } else { scale };
        let (a, b) = (sx * cos, sx * sin);
        let (c, d) = (scale * sin, -scale * cos);
        let image = PlacedImage {
            layer: String::new(),
            visible: true,
            matrix: [a, b, c, d, 400.0, -300.0],
            bounds: [7_056.0, 7_056.0, 11_152.0, 9_104.0],
            absolute: "x.png".into(),
            relative_dir: None,
        };
        let pose = pose(&image, artboard, &frame);
        let placement = Placement {
            id: "p".into(),
            drawing: "d".into(),
            center: pose.center,
            width: pose.width,
            height: pose.height,
            rotation: pose.rotation,
            mirrored: pose.mirrored,
            group: None,
            layer: None,
            deformation: None,
            projector: None,
            adjustment: None,
            extra: Default::default(),
        };
        assert_eq!(pose.mirrored, flip, "{degrees}°");
        let [x0, y0, x1, y1] = image.bounds;
        let expected = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| {
            let (px, py) = (a * x + c * y + 400.0, b * x + d * y - 300.0);
            (
                frame.x + (px - artboard[0]) * k,
                frame.y + (artboard[3] - py) * k,
            )
        });
        for (got, want) in placement.corners().iter().zip(expected) {
            assert!(
                (got.x - want.0).abs() < 1e-6 && (got.y - want.1).abs() < 1e-6,
                "{degrees}° {flip} : {got:?} au lieu de {want:?}"
            );
        }
    }
}

#[test]
fn names_are_compared_without_accents_or_case() {
    assert_eq!(fold_name("Murène.png"), "murene.png");
    assert_eq!(fold_name("Mure\u{300}ne.png"), "murene.png");
    assert_eq!(fold_name("MURENE.PNG"), "murene.png");
    assert_eq!(fold_name("Cœur"), "coeur");
}

fn write_png(path: &Path, width: u32, height: u32) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut data = vec![0u8; (width * height) as usize];
    for (i, v) in data.iter_mut().enumerate() {
        if i % 7 == 0 {
            *v = 255;
        }
    }
    image::save_buffer(path, &data, width, height, image::ExtendedColorType::L8).unwrap();
}

/// La composition A recréée : liens retrouvés à côté du `.ai` (chemin
/// relatif, nom accentué écrit autrement, chemin absolu d'un autre Mac),
/// dessins posés à leur place, calques repris, lien manquant signalé.
#[test]
fn a_composition_is_recreated_from_its_links() {
    let temp = TempDir::new("import-illustrator");
    let (dir, project) =
        ProjectDir::create(temp.path().join("Essai.insitu"), Some("Essai")).unwrap();
    dir.save(&project).unwrap();
    let work = temp.path().join("Fresque");
    write_png(&work.join("Oiseaux/merle.png"), 1000, 500);
    // Nom écrit en un seul caractère sur le disque, décomposé dans le .ai.
    write_png(&work.join("Poissons/Murène.png"), 400, 400);
    let ai = work.join("Composition.ai");
    std::fs::write(
        &ai,
        sample_ai(&sample_text("/Users/autre/Dessins/Oiseaux/merle.png"), true),
    )
    .unwrap();

    // Sans façade à l'échelle, il faut un rectangle.
    assert!(matches!(
        import_composition(&dir, &ai, None, &ImportOptions::default(), &|_| {}),
        Err(IllustratorError::NoFacadeScale)
    ));
    let frame = RectMm {
        x: 0.0,
        y: 0.0,
        width: 20_000.0,
        height: 10_000.0,
    };
    let (project, report) =
        import_composition(&dir, &ai, Some(frame), &ImportOptions::default(), &|_| {}).unwrap();
    assert_eq!(report.found, 4);
    assert_eq!(report.placed, 3);
    assert_eq!(report.drawings, 2);
    assert_eq!(report.folder, "bibliotheque/Composition");
    assert_eq!(report.embedded, 1);
    assert_eq!(report.layers, ["Fond", "Esquisses"]);
    assert_eq!(report.missing.len(), 1);
    assert_eq!(report.missing[0].link, "/Users/autre/Dessins/absent.png");

    let mut sources: Vec<_> = project.library.iter().map(|d| d.source.as_str()).collect();
    sources.sort();
    assert_eq!(
        sources,
        [
            "bibliotheque/Composition/Oiseaux/merle.png",
            "bibliotheque/Composition/Poissons/Murène.png"
        ]
    );
    // Calques : « Esquisses » masqué, comme dans Illustrator.
    let esquisses = project
        .layers
        .iter()
        .find(|l| l.name == "Esquisses")
        .unwrap();
    assert!(!esquisses.visible);
    // Le merle : 1 000 × 500 px à 0,1 pt, 20 mm par point → 2 000 × 1 000 mm,
    // coin haut-gauche en (100 pt, 100 pt) depuis le haut → (2 000, 2 000) mm.
    let merle = &project.placements[0];
    assert!((merle.width - 2_000.0).abs() < 1e-6 && (merle.height - 1_000.0).abs() < 1e-6);
    assert!((merle.center.x - 3_000.0).abs() < 1e-6);
    assert!((merle.center.y - 2_500.0).abs() < 1e-6);
    assert_eq!(merle.rotation, 0.0);
    assert!(!merle.mirrored);
    // La murène, tournée d'un quart de tour ; le merle d'« Esquisses »,
    // retourné.
    assert!(project.placements[1].rotation.abs() > 1.0);
    assert!(project.placements[2].mirrored);
    assert_eq!(project.placements[2].drawing, merle.drawing);

    // Relu, le projet est le même ; un second import ne double pas la
    // bibliothèque.
    assert_eq!(dir.load().unwrap().project, project);
    let (again, _) =
        import_composition(&dir, &ai, Some(frame), &ImportOptions::default(), &|_| {}).unwrap();
    assert_eq!(again.library.len(), 2);
    assert_eq!(again.placements.len(), 6);
    assert_eq!(again.layers.len(), 2);
}

#[test]
fn a_link_is_found_by_its_absolute_path_or_by_name() {
    let temp = TempDir::new("liens-illustrator");
    let elsewhere = temp.path().join("Ailleurs/merle.png");
    write_png(&elsewhere, 4, 4);
    let ai = temp.path().join("Fresque/Composition.ai");
    std::fs::create_dir_all(ai.parent().unwrap()).unwrap();
    let mut image = PlacedImage {
        layer: String::new(),
        visible: true,
        matrix: [1.0, 0.0, 0.0, -1.0, 0.0, 0.0],
        bounds: [0.0, 0.0, 4.0, 4.0],
        absolute: elsewhere.display().to_string(),
        relative_dir: Some("../Introuvable/".into()),
    };
    assert_eq!(resolve_link(&ai, &image), Some(elsewhere.clone()));
    // Chemin d'un autre Mac : retrouvé par son nom sous le dossier du .ai,
    // de préférence dans un dossier du même nom.
    write_png(&temp.path().join("Fresque/Brouillons/merle.png"), 4, 4);
    let good = temp.path().join("Fresque/Oiseaux/merle.png");
    write_png(&good, 4, 4);
    image.absolute = "/Users/autre/Dropbox/Oiseaux/merle.png".into();
    assert_eq!(resolve_link(&ai, &image), Some(good));
    // Un fichier temporaire d'Illustrator n'est jamais cherché.
    image.absolute = "/var/folders/xy/T/merle.png".into();
    assert_eq!(resolve_link(&ai, &image), None);
}

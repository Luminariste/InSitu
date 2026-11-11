//! Détecte les repères AprilTag 36h11 d'images données en argument et écrit
//! une ligne JSON par image : identifiants et coins en pixels.
//!
//! Sert uniquement à vérifier qu'un détecteur en Rust pur (crate
//! `kornia-apriltag`) trouve les mêmes repères qu'OpenCV sur les photos
//! simulées de l'étude 2.1.
//!
//! Avec la variable d'environnement `ESSAI_PASSES=1`, une seule passe aux
//! réglages par défaut de la crate. Sinon trois passes, de plus en plus
//! floutées et réduites, dont on réunit les résultats : c'est ce qui rend
//! la détection tolérante au flou et au bruit.

use std::collections::BTreeMap;

use kornia_apriltag::{family::TagFamilyKind, AprilTagDecoder, DecodeTagsConfig};
use kornia_image::{Image, ImageSize};

/// (flou gaussien en pixels, facteur de réduction)
const PASSES: [(f32, usize); 3] = [(0.0, 2), (1.5, 3), (3.0, 4)];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let single = std::env::var("ESSAI_PASSES").map(|v| v == "1").unwrap_or(false);
    let passes = if single { &PASSES[..1] } else { &PASSES[..] };
    for path in std::env::args().skip(1) {
        let gray = image::open(&path)?.to_luma8();
        let (width, height) = (gray.width() as usize, gray.height() as usize);
        let start = std::time::Instant::now();
        // identifiant -> (erreurs de bits, coins)
        let mut found: BTreeMap<u16, (u8, [[f32; 2]; 4])> = BTreeMap::new();
        for &(sigma, downscale) in passes {
            let pixels = if sigma > 0.0 { image::imageops::blur(&gray, sigma).into_raw() } else { gray.clone().into_raw() };
            let img = Image::<u8, 1>::new(ImageSize { width, height }, pixels)?;
            let mut config = DecodeTagsConfig::new(vec![TagFamilyKind::Tag36H11])?;
            config.downscale_factor = downscale;
            let mut decoder = AprilTagDecoder::new(config, img.size())?;
            for d in decoder.decode(&img)? {
                // Ordre de la crate : haut-droit, bas-droit, bas-gauche, haut-gauche.
                let c = &d.quad.corners;
                let corners = [[c[3].x, c[3].y], [c[0].x, c[0].y], [c[1].x, c[1].y], [c[2].x, c[2].y]];
                // La première passe (la plus fine) garde la priorité.
                found.entry(d.id).or_insert((d.hamming, corners));
            }
        }
        let ms = start.elapsed().as_millis();
        let tags: Vec<String> = found
            .iter()
            .map(|(id, (hamming, c))| {
                format!(
                    "{{\"id\":{},\"hamming\":{},\"coins\":[[{:.3},{:.3}],[{:.3},{:.3}],[{:.3},{:.3}],[{:.3},{:.3}]]}}",
                    id, hamming, c[0][0], c[0][1], c[1][0], c[1][1], c[2][0], c[2][1], c[3][0], c[3][1]
                )
            })
            .collect();
        println!("{{\"fichier\":\"{}\",\"ms\":{},\"reperes\":[{}]}}", path, ms, tags.join(","));
    }
    Ok(())
}

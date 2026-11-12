//! Mire imprimable en PDF, entièrement vectorielle.
//!
//! Le fichier est écrit à la main (une page, police Helvetica intégrée aux
//! lecteurs PDF, aucune image) : pas de dépendance, et les coordonnées sont
//! exactement celles de [`TargetLayout`]. Imprimé à 100 % (« taille réelle »),
//! le carré de la grille mesure 280 mm.

use std::fmt::Write as _;

use super::{Color, Shape, TargetLayout, RED};
use crate::geometry::Point;
use crate::units::MM_PER_INCH;

/// Points PDF (1/72 de pouce) par millimètre.
pub const PT_PER_MM: f64 = 72.0 / MM_PER_INCH;

/// Côté de la feuille découpée autour de la mire, en mm : un A3 recoupé en
/// carré, comme les feuilles d'Épinal (marge blanche de 8,5 mm).
pub const SHEET_SIZE_MM: f64 = 297.0;

/// Format de la page du PDF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageFormat {
    /// A3 portrait (297 × 420 mm) : la mire au milieu, deux pointillés pour
    /// recouper la feuille en carré de 297 mm, et la consigne d'impression.
    #[default]
    A3,
    /// Page carrée de 297 mm : la feuille déjà découpée, sans consigne.
    Square,
}

impl PageFormat {
    /// Largeur et hauteur de la page, en mm.
    pub fn size_mm(self) -> (f64, f64) {
        match self {
            PageFormat::A3 => (297.0, 420.0),
            PageFormat::Square => (SHEET_SIZE_MM, SHEET_SIZE_MM),
        }
    }

    /// Position du coin haut-gauche de la mire sur la page, en mm.
    pub fn target_origin_mm(self, layout: &TargetLayout) -> Point {
        let (w, h) = self.size_mm();
        Point::new((w - layout.size_mm) / 2.0, (h - layout.size_mm) / 2.0)
    }
}

/// Fabrique le PDF de la mire.
pub fn render(layout: &TargetLayout, format: PageFormat) -> Vec<u8> {
    let (page_w, page_h) = format.size_mm();
    let origin = format.target_origin_mm(layout);
    let mut c = String::new();

    // Repère de travail : millimètres, origine en haut à gauche, y vers le bas.
    // Échelle écrite avec 9 décimales : 4 laisseraient 5 µm d'erreur sur 280 mm.
    let _ = writeln!(
        c,
        "q {s:.9} 0 0 {:.9} 0 {:.6} cm",
        -PT_PER_MM,
        page_h * PT_PER_MM,
        s = PT_PER_MM
    );
    c.push_str("0 J 0 j\n");

    // La mire.
    let _ = writeln!(c, "q 1 0 0 1 {} {} cm", num(origin.x), num(origin.y));
    for shape in layout.drawing() {
        write_shape(&mut c, &shape);
    }
    // Identifiant, dans la marge blanche sous la grille (reste sur la
    // feuille recoupée).
    write_text(&mut c, &layout.identifier(), 0.0, layout.size_mm + 4.6, 2.6);
    c.push_str("Q\n");

    if format == PageFormat::A3 {
        let sheet_top = (page_h - SHEET_SIZE_MM) / 2.0;
        // Pointillés de coupe.
        c.push_str("0.5 G 0.25 w [3 2] 0 d\n");
        for y in [sheet_top, sheet_top + SHEET_SIZE_MM] {
            let _ = writeln!(c, "0 {} m {} {} l S", num(y), num(page_w), num(y));
        }
        c.push_str("[] 0 d 0 G\n");
        let lines = [
            (5.0, format!("Mire InSitu, version {}", super::TARGET_VERSION)),
            (4.0, "Imprimer à 100 % (taille réelle), sans « ajuster à la page ».".to_string()),
            (
                4.0,
                "Contrôle à la règle : le carré de la grille mesure 280 mm de côté, chaque repère noir 16 mm."
                    .to_string(),
            ),
            (4.0, "Couper la feuille le long des deux pointillés (carré de 297 mm).".to_string()),
        ];
        let mut y = 18.0;
        for (size, text) in &lines {
            write_text(&mut c, text, 15.0, y, *size);
            y += size * 1.9;
        }
    }
    c.push_str("Q\n");

    assemble(c.as_bytes(), page_w, page_h, &layout.identifier())
}

fn color_op(color: Color) -> String {
    match color {
        Color::Black => "0 G".to_string(),
        Color::Red => format!("{} {} {} RG", num(RED[0]), num(RED[1]), num(RED[2])),
    }
}

fn write_shape(c: &mut String, shape: &Shape) {
    match shape {
        Shape::Line {
            from,
            to,
            width,
            color,
        } => {
            let _ = writeln!(
                c,
                "{} {} w {} {} m {} {} l S",
                color_op(*color),
                num(*width),
                num(from.x),
                num(from.y),
                num(to.x),
                num(to.y)
            );
        }
        Shape::Circle {
            center,
            radius,
            width,
            color,
        } => {
            // Huit arcs de Bézier : écart au vrai cercle < 0,000 01 du rayon.
            let n = 8;
            let step = std::f64::consts::TAU / n as f64;
            let k = 4.0 / 3.0 * (step / 4.0).tan() * radius;
            let at = |a: f64| (center.x + radius * a.cos(), center.y + radius * a.sin());
            let _ = write!(c, "{} {} w ", color_op(*color), num(*width));
            let (x0, y0) = at(0.0);
            let _ = write!(c, "{} {} m", num(x0), num(y0));
            for i in 0..n {
                let (a0, a1) = (i as f64 * step, (i + 1) as f64 * step);
                let (p0, p3) = (at(a0), at(a1));
                let p1 = (p0.0 - k * a0.sin(), p0.1 + k * a0.cos());
                let p2 = (p3.0 + k * a1.sin(), p3.1 - k * a1.cos());
                let _ = write!(
                    c,
                    " {} {} {} {} {} {} c",
                    num(p1.0),
                    num(p1.1),
                    num(p2.0),
                    num(p2.1),
                    num(p3.0),
                    num(p3.1)
                );
            }
            c.push_str(" h S\n");
        }
        Shape::Rects { rects } => {
            // Un seul remplissage pour tout le repère : pas de liseré entre
            // deux modules voisins à l'affichage.
            c.push_str("0 g");
            for r in rects {
                let _ = write!(
                    c,
                    " {} {} {} {} re",
                    num(r.x),
                    num(r.y),
                    num(r.width),
                    num(r.height)
                );
            }
            c.push_str(" f\n");
        }
    }
}

/// Texte en Helvetica, ligne de base en (x, y) mm, corps `size` mm.
fn write_text(c: &mut String, text: &str, x: f64, y: f64, size: f64) {
    // Le repère est retourné (y vers le bas) : la matrice de texte le
    // retourne à nouveau pour que les lettres soient à l'endroit.
    let _ = writeln!(
        c,
        "0 g BT /F1 {} Tf 1 0 0 -1 {} {} Tm ({}) Tj ET",
        num(size),
        num(x),
        num(y),
        escape_text(text)
    );
}

/// Texte en codage WinAnsi, échappé pour une chaîne PDF. Les caractères
/// absents du codage sont remplacés par « ? ».
fn escape_text(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        let byte: u8 = match ch {
            '(' | ')' | '\\' => {
                out.push('\\');
                ch as u8
            }
            ' '..='~' => ch as u8,
            '\u{a0}'..='\u{ff}' => ch as u32 as u8,
            '’' => 0x92,
            '–' => 0x96,
            '—' => 0x97,
            _ => b'?',
        };
        if byte.is_ascii() && !byte.is_ascii_control() {
            out.push(byte as char);
        } else {
            let _ = write!(out, "\\{byte:03o}");
        }
    }
    out
}

/// Nombre écrit avec au plus 4 décimales (0,1 µm en mm), sans zéros inutiles.
fn num(v: f64) -> String {
    let s = format!("{:.4}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// Assemble les objets du PDF et la table des positions.
fn assemble(content: &[u8], page_w_mm: f64, page_h_mm: f64, title: &str) -> Vec<u8> {
    let media = format!(
        "[0 0 {} {}]",
        num(page_w_mm * PT_PER_MM),
        num(page_h_mm * PT_PER_MM)
    );
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox {media} /CropBox {media} \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
        )
        .into_bytes(),
        {
            let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
            stream.extend_from_slice(content);
            stream.extend_from_slice(b"\nendstream");
            stream
        },
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
        // Pas de date ni de numéro de version de l'app : le même dessin donne
        // toujours le même fichier, octet pour octet (voir `docs/mire/`).
        format!("<< /Title ({}) /Creator (InSitu) >>", escape_text(title)).into_bytes(),
    ];

    let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {} /Root 1 0 R /Info 6 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    out.extend_from_slice(table.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rejoue le flux de dessin du PDF comme le ferait une imprimante :
    /// matrices `cm`, `q`/`Q`, et relève en points PDF les extrémités de
    /// chaque trait noir et les coins de chaque rectangle rempli.
    struct Replay {
        strokes: Vec<(f64, f64)>,
        fills: Vec<(f64, f64)>,
    }

    fn content_stream(pdf: &[u8]) -> String {
        let text = String::from_utf8_lossy(pdf);
        let start = text.find("stream\n").unwrap() + 7;
        let end = text.find("\nendstream").unwrap();
        text[start..end].to_string()
    }

    fn replay(content: &str) -> Replay {
        type M = [f64; 6];
        let mul = |m: M, n: M| -> M {
            [
                m[0] * n[0] + m[1] * n[2],
                m[0] * n[1] + m[1] * n[3],
                m[2] * n[0] + m[3] * n[2],
                m[2] * n[1] + m[3] * n[3],
                m[4] * n[0] + m[5] * n[2] + n[4],
                m[4] * n[1] + m[5] * n[3] + n[5],
            ]
        };
        let apply = |m: M, x: f64, y: f64| (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]);
        let mut ctm: M = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let mut stack = Vec::new();
        let mut operands: Vec<f64> = Vec::new();
        let mut path: Vec<(f64, f64)> = Vec::new();
        let mut gray_black = true;
        let mut in_text = false;
        let mut out = Replay {
            strokes: Vec::new(),
            fills: Vec::new(),
        };
        for token in content.split_whitespace() {
            if in_text {
                in_text = token != "ET";
                continue;
            }
            if let Ok(v) = token.parse::<f64>() {
                operands.push(v);
                continue;
            }
            match token {
                "q" => stack.push(ctm),
                "Q" => ctm = stack.pop().unwrap(),
                "cm" => {
                    let o = &operands[operands.len() - 6..];
                    ctm = mul([o[0], o[1], o[2], o[3], o[4], o[5]], ctm);
                }
                "G" => gray_black = operands.last() == Some(&0.0),
                "RG" => gray_black = false,
                "m" | "l" => {
                    let o = &operands[operands.len() - 2..];
                    path.push(apply(ctm, o[0], o[1]));
                }
                "re" => {
                    let o = &operands[operands.len() - 4..];
                    path.push(apply(ctm, o[0], o[1]));
                    path.push(apply(ctm, o[0] + o[2], o[1] + o[3]));
                }
                "S" => {
                    if gray_black {
                        out.strokes.append(&mut path);
                    }
                    path.clear();
                }
                "f" => out.fills.append(&mut path),
                "BT" => in_text = true,
                "c" | "h" => {}
                _ => {}
            }
            if !token.starts_with('[') {
                operands.clear();
            }
        }
        out
    }

    fn extent(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
        let xs = points.iter().map(|p| p.0);
        let ys = points.iter().map(|p| p.1);
        (
            xs.clone().fold(f64::MAX, f64::min),
            xs.fold(f64::MIN, f64::max),
            ys.clone().fold(f64::MAX, f64::min),
            ys.fold(f64::MIN, f64::max),
        )
    }

    /// Critère d'acceptation, partie vérifiable sans imprimante : dans le
    /// PDF, la grille va d'un trait extrême à l'autre sur exactement 280 mm
    /// dans les deux sens, et la page a la taille annoncée.
    #[test]
    fn printed_grid_spans_exactly_280_mm() {
        let layout = TargetLayout::coded();
        for format in [PageFormat::A3, PageFormat::Square] {
            let pdf = render(&layout, format);
            let (w, h) = format.size_mm();
            let media = format!(
                "/MediaBox [0 0 {} {}]",
                num(w * PT_PER_MM),
                num(h * PT_PER_MM)
            );
            assert!(String::from_utf8_lossy(&pdf).contains(&media), "{media}");

            let replay = replay(&content_stream(&pdf));
            // Traits de la mire seulement : les pointillés de coupe sont gris.
            let (x0, x1, y0, y1) = extent(&replay.strokes);
            let to_mm = |pt: f64| pt / PT_PER_MM;
            assert!(
                (to_mm(x1 - x0) - 280.0).abs() < 1e-3,
                "largeur {}",
                to_mm(x1 - x0)
            );
            assert!(
                (to_mm(y1 - y0) - 280.0).abs() < 1e-3,
                "hauteur {}",
                to_mm(y1 - y0)
            );
            // Centrée en largeur : 8,5 mm de chaque côté.
            assert!((to_mm(x0) - 8.5).abs() < 1e-3);
            assert!((to_mm(w * PT_PER_MM - x1) - 8.5).abs() < 1e-3);
        }
    }

    #[test]
    fn markers_are_16_mm_black_squares() {
        let layout = TargetLayout::coded();
        let pdf = render(&layout, PageFormat::Square);
        let replay = replay(&content_stream(&pdf));
        // Le repère 0 : son pourtour noir couvre 12 → 28 mm, plus la marge.
        let first: Vec<_> = replay
            .fills
            .iter()
            .copied()
            .filter(|p| p.0 / PT_PER_MM < 8.5 + 40.0 && p.1 / PT_PER_MM > 297.0 - 8.5 - 40.0)
            .collect();
        let (x0, x1, y0, y1) = extent(&first);
        assert!(((x1 - x0) / PT_PER_MM - 16.0).abs() < 1e-3);
        assert!(((y1 - y0) / PT_PER_MM - 16.0).abs() < 1e-3);
        assert!((x0 / PT_PER_MM - (8.5 + 12.0)).abs() < 1e-3);
    }

    #[test]
    fn cross_reference_table_points_at_each_object() {
        let pdf = render(&TargetLayout::coded(), PageFormat::A3);
        let text = String::from_utf8_lossy(&pdf).into_owned();
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.ends_with("%%EOF\n"));
        let startxref: usize = text
            .rsplit("startxref\n")
            .next()
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(pdf[startxref..].starts_with(b"xref\n0 7\n"));
        let table = String::from_utf8_lossy(&pdf[startxref..]).into_owned();
        for (i, line) in table.lines().skip(3).take(6).enumerate() {
            let offset: usize = line[..10].parse().unwrap();
            let expected = format!("{} 0 obj", i + 1);
            assert!(
                pdf[offset..].starts_with(expected.as_bytes()),
                "objet {}",
                i + 1
            );
        }
        // Longueur du flux exacte.
        let content = content_stream(&pdf);
        assert!(text.contains(&format!("/Length {} >>", content.len())));
    }

    #[test]
    fn text_is_encoded_in_winansi() {
        assert_eq!(escape_text("à 100 % (taille)"), "\\340 100 % \\(taille\\)");
        assert_eq!(escape_text("n° é"), "n\\260 \\351");
    }

    #[test]
    fn numbers_are_short_and_exact_to_a_tenth_of_a_micron() {
        assert_eq!(num(280.0), "280");
        assert_eq!(num(8.5), "8.5");
        assert_eq!(num(2.834_645_669), "2.8346");
        assert_eq!(num(-0.00001), "0");
    }
}

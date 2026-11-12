//! Fiche de calepinage (tâche 6.2) : un PDF A3 à emporter sur le site.
//!
//! - **Page 1, le plan** : le plan de la façade (éclairci), l'emprise et le
//!   nom de chaque projecteur, les zones de recouvrement (tirets rouges), le
//!   contour de chaque dessin posé dans la couleur de sa diapositive (rouge
//!   s'il est coupé, gris s'il est hors emprise) ; à droite, la liste des
//!   diapositives, les réglages d'export et les alertes de la simulation.
//! - **Pages suivantes, les diapositives**, six par page : la vignette de la
//!   diapositive calculée (traits noirs sur blanc, pour l'impression), son
//!   nom, son fichier d'export, son recalage, sa photo et ses dessins.
//!
//! A3 couché (420 × 297 mm). Textes en Helvetica (police des lecteurs PDF,
//! codage WinAnsi), jamais plus petits que 3 mm de corps : lisible à bout
//! de bras sur le chantier.

#[cfg(test)]
mod tests;

use std::fs;
use std::io::Cursor;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, GrayImage, ImageEncoder};
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref, Str};
use serde::Serialize;

use crate::export::{slide_file_name, ExportError};
use crate::facade::{display::render_whole, SourceKind};
use crate::overview::overview;
use crate::placement::assign::assignments;
use crate::project::{ExportSettings, Placement, PointMm, Project, ProjectDir};
use crate::simulation::{simulate, AlertKind, Simulation};
use crate::tiles;
use crate::units::{self, MM_PER_INCH};

/// Page A3 couchée, en mm.
pub const PAGE_MM: (f64, f64) = (420.0, 297.0);
/// Plus petit corps de texte de la fiche, en mm (environ 8,5 points).
pub const MIN_TEXT_MM: f64 = 3.0;

const PT_PER_MM: f64 = 72.0 / MM_PER_INCH;
const MARGIN: f64 = 12.0;
/// Côté des vignettes de diapositive dans le PDF, en pixels.
const THUMB_PX: u32 = 640;
/// Plus grand côté du plan de façade dans le PDF, en pixels.
const PLAN_PX: u32 = 2400;
/// Diapositives par page (3 colonnes, 2 rangées).
const SLIDES_PER_PAGE: usize = 6;

/// Couleurs des projecteurs sur papier blanc, dans l'ordre du projet (les
/// mêmes teintes que l'écran, plus foncées pour l'impression).
const PRINT_COLORS: [[f32; 3]; 12] = [
    [0.00, 0.47, 0.75],
    [0.85, 0.45, 0.00],
    [0.18, 0.55, 0.22],
    [0.80, 0.10, 0.45],
    [0.50, 0.25, 0.65],
    [0.65, 0.58, 0.00],
    [0.00, 0.55, 0.50],
    [0.80, 0.30, 0.15],
    [0.35, 0.30, 0.70],
    [0.40, 0.55, 0.10],
    [0.15, 0.40, 0.80],
    [0.70, 0.15, 0.15],
];
const RED: [f32; 3] = [0.85, 0.05, 0.05];
const GRAY: [f32; 3] = [0.55, 0.55, 0.55];
const BLACK: [f32; 3] = [0.0, 0.0, 0.0];

/// Ce que la fiche contient.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetReport {
    pub pages: usize,
    /// Diapositives montrées (tous les projecteurs du projet).
    pub slides: usize,
    pub alerts: usize,
    /// Plus petit corps de texte de la fiche, en mm.
    pub smallest_text_mm: f64,
}

/// Écrit la fiche de calepinage du projet dans `out` (PDF). Les aperçus des
/// diapositives sont pris dans le cache (fabriqués s'ils manquent). Les
/// réglages d'export montrés sont `settings` s'ils sont donnés (ceux de
/// l'écran, pas encore enregistrés), sinon ceux du projet.
pub fn write_sheet(
    dir: &ProjectDir,
    cache_root: &Path,
    out: &Path,
    settings: Option<&ExportSettings>,
) -> Result<SheetReport, ExportError> {
    let (bytes, report) = sheet(dir, cache_root, settings)?;
    fs::write(out, bytes)
        .map_err(|e| ExportError::Destination(format!("{} : {e}", out.display())))?;
    Ok(report)
}

/// La fiche de calepinage, en PDF.
pub fn sheet(
    dir: &ProjectDir,
    cache_root: &Path,
    settings: Option<&ExportSettings>,
) -> Result<(Vec<u8>, SheetReport), ExportError> {
    let mut project = dir.load()?.project;
    if let Some(s) = settings {
        project.export = s.clone();
    }
    let simulation = simulate(dir, cache_root)?;
    let plan = plan_image(dir, &project);
    let mut thumbs = Vec::new();
    for slide in &simulation.slides {
        let thumb = match &slide.preview {
            Some(request) => Some(thumbnail(&cache_root.join(request))?),
            None => None,
        };
        thumbs.push(thumb);
    }
    let today = today();
    Ok(build(
        &project,
        &simulation,
        plan.as_ref(),
        &thumbs,
        &today,
        dir,
    ))
}

/// Le plan de la façade en niveaux de gris, et sa taille sur la façade (mm).
pub struct PlanImage {
    pub image: GrayImage,
    pub size_mm: (f64, f64),
}

/// Le plan de la façade, réduit et éclairci pour l'impression ; `None` sans
/// plan à l'échelle ou s'il est illisible (la fiche se fait sans fond).
fn plan_image(dir: &ProjectDir, project: &Project) -> Option<PlanImage> {
    let facade = project.facade.as_ref()?;
    let size_mm = facade.size_mm()?;
    let path = dir.resolve(&facade.source);
    let rgba = match SourceKind::of(&path).ok()? {
        SourceKind::Image => {
            let image = crate::photo::read(&path).ok()?;
            match facade.rotation % 4 {
                1 => image.rotate90(),
                2 => image.rotate180(),
                3 => image.rotate270(),
                _ => image,
            }
            .into_rgba8()
        }
        _ => {
            let (w, h, data) = render_whole(&path, facade.page, facade.rotation, PLAN_PX).ok()?;
            image::RgbaImage::from_raw(w, h, data)?
        }
    };
    // Sur du blanc, en gris, éclairci : les tracés de la fiche restent lisibles.
    let gray = GrayImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let lum = (299 * u32::from(r) + 587 * u32::from(g) + 114 * u32::from(b)) / 1000;
        let on_white = (lum * u32::from(a) + 255 * (255 - u32::from(a))) / 255;
        image::Luma([(255 - (255 - on_white) * 45 / 100) as u8])
    });
    let scale = f64::from(PLAN_PX) / f64::from(gray.width().max(gray.height()));
    let image = if scale < 1.0 {
        image::imageops::resize(
            &gray,
            ((f64::from(gray.width()) * scale).round() as u32).max(1),
            ((f64::from(gray.height()) * scale).round() as u32).max(1),
            image::imageops::FilterType::Triangle,
        )
    } else {
        gray
    };
    Some(PlanImage { image, size_mm })
}

/// La vignette d'une diapositive (aperçu du cache) : traits blancs sur noir,
/// comme la plaque gravée (D-6.2-e).
fn thumbnail(path: &Path) -> Result<GrayImage, ExportError> {
    let raster = tiles::read_raster(path).map_err(|e| ExportError::Image(e.to_string()))?;
    let lit = crate::simulation::LitSlide::from_raster(&raster);
    let side = lit.side;
    let gray = GrayImage::from_fn(side, side, |x, y| {
        image::Luma([lit.data[(y * side + x) as usize]])
    });
    Ok(image::imageops::resize(
        &gray,
        THUMB_PX,
        THUMB_PX,
        image::imageops::FilterType::Triangle,
    ))
}

fn jpeg(image: &GrayImage) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    JpegEncoder::new_with_quality(&mut out, 88)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::L8,
        )
        .expect("encodage JPEG en mémoire");
    out.into_inner()
}

/// La date du jour, « 12/11/2026 » (temps universel).
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format!("{d:02}/{m:02}/{y}")
}

/// Jour, mois, année d'un nombre de jours depuis le 1er janvier 1970.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Texte en codage WinAnsi ; un caractère absent du codage devient « ? ».
fn winansi(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| match c {
            ' '..='~' => c as u8,
            '\u{a0}' | '\u{202f}' => b' ',
            '\u{a1}'..='\u{ff}' => c as u32 as u8,
            '’' => 0x92,
            '–' => 0x96,
            '—' => 0x97,
            '…' => 0x85,
            _ => b'?',
        })
        .collect()
}

/// Largeur approchée d'un texte en Helvetica, en mm, pour couper les lignes.
fn text_width(text: &str, size: f64, bold: bool) -> f64 {
    let factor = if bold { 0.60 } else { 0.55 };
    text.chars().count() as f64 * size * factor
}

/// Coupe un texte en lignes d'au plus `width` mm.
fn wrap(text: &str, size: f64, width: f64) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let next = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if text_width(&next, size, false) > width && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        } else {
            line = next;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Une page en cours d'écriture, en mm, origine en haut à gauche.
struct Page {
    content: Content,
    images: Vec<(Vec<u8>, u32, u32)>,
    /// Plus petit corps de texte écrit sur la page (mm).
    smallest_text: f64,
}

impl Page {
    fn new() -> Self {
        Self {
            content: Content::new(),
            images: Vec::new(),
            smallest_text: f64::INFINITY,
        }
    }

    fn pt(x: f64, y: f64) -> (f32, f32) {
        ((x * PT_PER_MM) as f32, ((PAGE_MM.1 - y) * PT_PER_MM) as f32)
    }

    fn stroke(&mut self, color: [f32; 3], width: f64, dash: Option<f64>) -> &mut Self {
        self.content
            .set_stroke_rgb(color[0], color[1], color[2])
            .set_line_width((width * PT_PER_MM) as f32);
        match dash {
            Some(d) => self
                .content
                .set_dash_pattern([(d * PT_PER_MM) as f32; 2], 0.0),
            None => self.content.set_dash_pattern([], 0.0),
        };
        self
    }

    fn fill(&mut self, color: [f32; 3]) -> &mut Self {
        self.content.set_fill_rgb(color[0], color[1], color[2]);
        self
    }

    fn polygon(&mut self, points: &[(f64, f64)], close: bool) {
        let Some(&(x, y)) = points.first() else {
            return;
        };
        let (x, y) = Self::pt(x, y);
        self.content.move_to(x, y);
        for &(x, y) in &points[1..] {
            let (x, y) = Self::pt(x, y);
            self.content.line_to(x, y);
        }
        if close {
            self.content.close_and_stroke();
        } else {
            self.content.stroke();
        }
    }

    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, fill: bool) {
        let (px, py) = Self::pt(x, y + h);
        self.content
            .rect(px, py, (w * PT_PER_MM) as f32, (h * PT_PER_MM) as f32);
        if fill {
            self.content.fill_nonzero();
        } else {
            self.content.stroke();
        }
    }

    /// Texte, ligne de base en (x, y) mm, corps `size` mm.
    fn text(&mut self, text: &str, x: f64, y: f64, size: f64, bold: bool, color: [f32; 3]) {
        self.smallest_text = self.smallest_text.min(size);
        let (px, py) = Self::pt(x, y);
        self.content.set_fill_rgb(color[0], color[1], color[2]);
        self.content
            .begin_text()
            .set_font(
                Name(if bold { b"F2" } else { b"F1" }),
                (size * PT_PER_MM) as f32,
            )
            .next_line(px, py);
        self.content.show(Str(&winansi(text))).end_text();
    }

    /// Une image en niveaux de gris dans le rectangle (x, y, w, h) mm.
    fn image(&mut self, image: &GrayImage, x: f64, y: f64, w: f64, h: f64) {
        let index = self.images.len();
        self.images
            .push((jpeg(image), image.width(), image.height()));
        let (px, py) = Self::pt(x, y + h);
        self.content.save_state();
        self.content.transform([
            (w * PT_PER_MM) as f32,
            0.0,
            0.0,
            (h * PT_PER_MM) as f32,
            px,
            py,
        ]);
        let name = format!("Im{index}");
        self.content.x_object(Name(name.as_bytes()));
        self.content.restore_state();
    }
}

/// Couleur d'impression du projecteur de rang `index`.
fn color(index: usize) -> [f32; 3] {
    PRINT_COLORS[index % PRINT_COLORS.len()]
}

/// Assemble la fiche : renvoie le PDF et son résumé.
pub fn build(
    project: &Project,
    simulation: &Simulation,
    plan: Option<&PlanImage>,
    thumbs: &[Option<GrayImage>],
    today: &str,
    dir: &ProjectDir,
) -> (Vec<u8>, SheetReport) {
    let mut pages = vec![plan_page(project, simulation, plan, today, dir)];
    let chunks: Vec<_> = (0..project.projectors.len()).collect();
    let total = chunks.chunks(SLIDES_PER_PAGE).count();
    for (k, chunk) in chunks.chunks(SLIDES_PER_PAGE).enumerate() {
        pages.push(slides_page(
            project,
            simulation,
            thumbs,
            chunk,
            k + 1,
            total,
        ));
    }
    let report = SheetReport {
        pages: pages.len(),
        slides: project.projectors.len(),
        alerts: simulation.alerts.len(),
        smallest_text_mm: pages
            .iter()
            .map(|p| p.smallest_text)
            .fold(f64::INFINITY, f64::min),
    };
    (write_pdf(pages), report)
}

fn write_pdf(pages: Vec<Page>) -> Vec<u8> {
    let mut pdf = Pdf::new();
    let mut next = 1;
    let mut new_ref = || {
        next += 1;
        Ref::new(next - 1)
    };
    let catalog = new_ref();
    let tree = new_ref();
    let regular = new_ref();
    let bold = new_ref();
    pdf.catalog(catalog).pages(tree);
    pdf.type1_font(regular)
        .base_font(Name(b"Helvetica"))
        .encoding_predefined(Name(b"WinAnsiEncoding"));
    pdf.type1_font(bold)
        .base_font(Name(b"Helvetica-Bold"))
        .encoding_predefined(Name(b"WinAnsiEncoding"));
    let mut kids = Vec::new();
    for page in pages {
        let page_id = new_ref();
        let content_id = new_ref();
        let image_ids: Vec<Ref> = page.images.iter().map(|_| new_ref()).collect();
        kids.push(page_id);
        let mut p = pdf.page(page_id);
        p.media_box(Rect::new(
            0.0,
            0.0,
            (PAGE_MM.0 * PT_PER_MM) as f32,
            (PAGE_MM.1 * PT_PER_MM) as f32,
        ));
        p.parent(tree);
        p.contents(content_id);
        let mut resources = p.resources();
        resources
            .fonts()
            .pair(Name(b"F1"), regular)
            .pair(Name(b"F2"), bold);
        {
            let mut objects = resources.x_objects();
            for (i, id) in image_ids.iter().enumerate() {
                let name = format!("Im{i}");
                objects.pair(Name(name.as_bytes()), *id);
            }
        }
        resources.finish();
        p.finish();
        pdf.stream(content_id, &page.content.finish());
        for ((data, w, h), id) in page.images.iter().zip(&image_ids) {
            let mut image = pdf.image_xobject(*id, data);
            image.filter(Filter::DctDecode);
            image.width(*w as i32);
            image.height(*h as i32);
            image.color_space().device_gray();
            image.bits_per_component(8);
        }
    }
    let count = kids.len() as i32;
    pdf.pages(tree).kids(kids).count(count);
    pdf.finish()
}

/// Le rectangle de la façade à montrer : plan, emprises et dessins posés.
fn facade_extent(
    project: &Project,
    plan: Option<&PlanImage>,
    footprints: &[Vec<PointMm>],
) -> Option<(f64, f64, f64, f64)> {
    let mut points: Vec<PointMm> = footprints.iter().flatten().copied().collect();
    if let Some(plan) = plan {
        points.push(PointMm::new(0.0, 0.0));
        points.push(PointMm::new(plan.size_mm.0, plan.size_mm.1));
    }
    for p in project.visible_placements() {
        points.extend(p.corners());
    }
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in points {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1, y1))
}

/// Contour d'un dessin posé sur la façade, déformation comprise.
fn outline(p: &Placement) -> Vec<PointMm> {
    let n = 8;
    let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
    (0..4)
        .flat_map(|k| {
            let (a, b) = (corners[k], corners[(k + 1) % 4]);
            (0..n).map(move |s| {
                let t = f64::from(s) / f64::from(n);
                p.facade_point(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
            })
        })
        .collect()
}

fn plan_page(
    project: &Project,
    simulation: &Simulation,
    plan: Option<&PlanImage>,
    today: &str,
    dir: &ProjectDir,
) -> Page {
    let mut page = Page::new();
    let view = overview(dir, project, None);
    let n = project.projectors.len();
    page.text(
        &format!("Calepinage — {}", project.name),
        MARGIN,
        22.0,
        8.0,
        true,
        BLACK,
    );
    page.text(
        &format!(
            "{n} diapositive{} · fiche du {today} · InSitu {}",
            if n > 1 { "s" } else { "" },
            env!("CARGO_PKG_VERSION")
        ),
        MARGIN,
        30.0,
        4.0,
        false,
        BLACK,
    );

    // Le plan, dans le cadre de gauche.
    let (bx, by, bw, bh) = (MARGIN, 38.0, 280.0, PAGE_MM.1 - 38.0 - MARGIN);
    page.stroke(GRAY, 0.2, None).rect(bx, by, bw, bh, false);
    let footprints: Vec<Vec<PointMm>> = view
        .projectors
        .iter()
        .map(|p| p.footprint.clone())
        .collect();
    if let Some((x0, y0, x1, y1)) = facade_extent(project, plan, &footprints) {
        let pad = 4.0;
        let k = ((bw - 2.0 * pad) / (x1 - x0)).min((bh - 2.0 * pad) / (y1 - y0));
        let ox = bx + pad + ((bw - 2.0 * pad) - (x1 - x0) * k) / 2.0;
        let oy = by + pad + ((bh - 2.0 * pad) - (y1 - y0) * k) / 2.0;
        let at = |p: PointMm| (ox + (p.x - x0) * k, oy + (p.y - y0) * k);
        if let Some(plan) = plan {
            let (x, y) = at(PointMm::new(0.0, 0.0));
            page.image(&plan.image, x, y, plan.size_mm.0 * k, plan.size_mm.1 * k);
        }
        // Recouvrements en tirets rouges.
        page.stroke(RED, 0.35, Some(1.5));
        for o in &view.overlaps {
            for outline in &o.outlines {
                let pts: Vec<_> = outline.iter().map(|&p| at(p)).collect();
                page.polygon(&pts, true);
            }
        }
        // Emprises.
        for (i, p) in view.projectors.iter().enumerate() {
            if p.footprint.len() < 3 {
                continue;
            }
            let pts: Vec<_> = p.footprint.iter().map(|&q| at(q)).collect();
            page.stroke(color(i), 0.7, None).polygon(&pts, true);
        }
        // Dessins posés, dans la couleur de leur diapositive.
        let all = assignments(project);
        let alerted = |id: &str, kind: AlertKind| {
            simulation
                .alerts
                .iter()
                .any(|a| a.kind == kind && a.placement.as_deref() == Some(id))
        };
        for p in project.visible_placements() {
            let index = all
                .iter()
                .find(|a| a.placement == p.id)
                .and_then(|a| a.projector.as_deref())
                .and_then(|id| project.projectors.iter().position(|q| q.id == id));
            let (c, dash) = if alerted(&p.id, AlertKind::Outside) {
                (GRAY, Some(0.8))
            } else if alerted(&p.id, AlertKind::Cut) {
                (RED, None)
            } else {
                (index.map_or(GRAY, color), None)
            };
            let pts: Vec<_> = outline(p).into_iter().map(at).collect();
            page.stroke(c, 0.3, dash).polygon(&pts, true);
        }
        // Noms des projecteurs, sur fond blanc, au centre de l'emprise.
        for (i, p) in view.projectors.iter().enumerate() {
            if p.footprint.len() < 3 {
                continue;
            }
            let m = p.footprint.len() - 1;
            let (cx, cy) = p.footprint[..m].iter().fold((0.0, 0.0), |(x, y), q| {
                (x + q.x / m as f64, y + q.y / m as f64)
            });
            let (x, y) = at(PointMm::new(cx, cy));
            let size = 7.0;
            let w = text_width(&p.label, size, true);
            page.fill([1.0, 1.0, 1.0]).rect(
                x - w / 2.0 - 1.5,
                y - size * 0.85,
                w + 3.0,
                size * 1.2,
                true,
            );
            page.text(&p.label, x - w / 2.0, y, size, true, color(i));
        }
    } else {
        page.text(
            "Rien à montrer : ni plan à l'échelle, ni projecteur recalé.",
            bx + 6.0,
            by + 12.0,
            4.5,
            false,
            BLACK,
        );
    }

    // Colonne de droite.
    let x = bx + bw + 10.0;
    let width = PAGE_MM.0 - MARGIN - x;
    let mut y = 44.0;
    page.text("DIAPOSITIVES", x, y, 3.5, true, BLACK);
    y += 6.5;
    let all = assignments(project);
    let shown = 16;
    for (i, p) in project.projectors.iter().enumerate().take(shown) {
        let count = all
            .iter()
            .filter(|a| a.projector.as_deref() == Some(p.id.as_str()))
            .count();
        page.fill(color(i)).rect(x, y - 3.6, 4.0, 4.0, true);
        page.text(&p.label, x + 6.0, y, 4.0, true, BLACK);
        let file = slide_file_name(&project.name, &p.label);
        let detail = format!(
            "{count} dessin{} · {file}",
            if count > 1 { "s" } else { "" }
        );
        let line = wrap(&detail, MIN_TEXT_MM, width - 26.0)
            .into_iter()
            .next()
            .unwrap_or_default();
        page.text(&line, x + 26.0, y, MIN_TEXT_MM, false, BLACK);
        y += 6.0;
    }
    if n > shown {
        page.text(
            &format!("… et {} autres (pages suivantes)", n - shown),
            x,
            y,
            MIN_TEXT_MM,
            false,
            BLACK,
        );
        y += 6.0;
    }

    y += 4.0;
    page.text("RÉGLAGES D'EXPORT", x, y, 3.5, true, BLACK);
    y += 6.5;
    for line in settings_lines(&project.export) {
        page.text(&line, x, y, 3.5, false, BLACK);
        y += 5.2;
    }

    y += 4.0;
    page.text("ALERTES", x, y, 3.5, true, BLACK);
    y += 6.5;
    if simulation.alerts.is_empty() {
        page.text(
            "Aucune : chaque dessin tient dans sa",
            x,
            y,
            3.5,
            false,
            BLACK,
        );
        page.text(
            "diapositive, rien n'est projeté deux fois.",
            x,
            y + 5.0,
            3.5,
            false,
            BLACK,
        );
    } else {
        let bottom = PAGE_MM.1 - MARGIN - 30.0;
        let mut written = 0;
        for alert in &simulation.alerts {
            let lines = wrap(&alert.message, MIN_TEXT_MM, width - 4.0);
            if y + lines.len() as f64 * 4.2 > bottom {
                break;
            }
            page.fill(if alert.kind == AlertKind::Outside {
                RED
            } else {
                [0.85, 0.45, 0.0]
            })
            .rect(x, y - 2.6, 2.4, 2.4, true);
            for line in lines {
                page.text(&line, x + 4.0, y, MIN_TEXT_MM, false, BLACK);
                y += 4.2;
            }
            y += 1.2;
            written += 1;
        }
        if written < simulation.alerts.len() {
            page.text(
                &format!(
                    "… et {} autres : voir la simulation dans l'app.",
                    simulation.alerts.len() - written
                ),
                x,
                y,
                MIN_TEXT_MM,
                false,
                BLACK,
            );
        }
    }

    // Légende, en bas de la colonne.
    let mut y = PAGE_MM.1 - MARGIN - 20.0;
    page.text("LÉGENDE", x, y, 3.5, true, BLACK);
    y += 5.5;
    let legend: [([f32; 3], f64, Option<f64>, &str); 4] = [
        (color(0), 0.7, None, "emprise d'un projecteur"),
        (RED, 0.35, Some(1.5), "recouvrement de deux emprises"),
        (RED, 0.3, None, "dessin coupé"),
        (GRAY, 0.3, Some(0.8), "dessin hors emprise"),
    ];
    for (c, w, dash, label) in legend {
        page.stroke(c, w, dash)
            .polygon(&[(x, y - 1.2), (x + 8.0, y - 1.2)], false);
        page.text(label, x + 11.0, y, MIN_TEXT_MM, false, BLACK);
        y += 4.0;
    }
    page
}

/// Les réglages d'export, en clair.
pub fn settings_lines(s: &ExportSettings) -> Vec<String> {
    let (slide, margin) = crate::export::sizes_px(s.dpi);
    let plate = slide + 2 * margin;
    let yes = |b: bool| if b { "oui" } else { "non" };
    vec![
        format!("Résolution : {} dpi", s.dpi.round()),
        if s.margin {
            format!("Plaque de {} mm : {plate} px", units::PLATE_SIZE_MM)
        } else {
            format!("Image de {} mm : {slide} px", units::SLIDE_SIZE_MM)
        },
        format!("Miroir : {} · Négatif : {}", yes(s.mirror), yes(s.negative)),
        if s.one_bit {
            format!("Noir et blanc pur, seuil {}", s.threshold)
        } else {
            "Niveaux de gris".to_string()
        },
        format!("SVG en plus : {}", yes(s.svg)),
    ]
}

fn slides_page(
    project: &Project,
    simulation: &Simulation,
    thumbs: &[Option<GrayImage>],
    indices: &[usize],
    number: usize,
    total: usize,
) -> Page {
    let mut page = Page::new();
    page.text(
        &format!("Diapositives — {}", project.name),
        MARGIN,
        22.0,
        8.0,
        true,
        BLACK,
    );
    page.text(
        &format!("page {number} sur {total}"),
        PAGE_MM.0 - MARGIN - 30.0,
        22.0,
        4.0,
        false,
        BLACK,
    );
    let all = assignments(project);
    let gap = 8.0;
    let cell_w = (PAGE_MM.0 - 2.0 * MARGIN - 2.0 * gap) / 3.0;
    let top = 32.0;
    let cell_h = (PAGE_MM.1 - top - MARGIN - gap) / 2.0;
    let thumb = 82.0;
    for (slot, &i) in indices.iter().enumerate() {
        let p = &project.projectors[i];
        let x = MARGIN + (slot % 3) as f64 * (cell_w + gap);
        let y = top + (slot / 3) as f64 * (cell_h + gap);
        match &thumbs[i] {
            Some(image) => page.image(image, x, y, thumb, thumb),
            None => {
                page.fill([0.93, 0.93, 0.93]).rect(x, y, thumb, thumb, true);
                let problem = simulation
                    .slides
                    .iter()
                    .find(|s| s.projector_id == p.id)
                    .and_then(|s| s.problem.clone())
                    .unwrap_or_else(|| "pas d'aperçu".into());
                for (k, line) in wrap(&problem, 3.5, thumb - 8.0).iter().enumerate() {
                    page.text(
                        line,
                        x + 4.0,
                        y + thumb / 2.0 + k as f64 * 5.0,
                        3.5,
                        false,
                        BLACK,
                    );
                }
            }
        }
        page.stroke(color(i), 0.8, None)
            .rect(x, y, thumb, thumb, false);
        // À droite de la vignette : nom, fichier, recalage, photo.
        let tx = x + thumb + 4.0;
        let tw = cell_w - thumb - 4.0;
        page.text(&p.label, tx, y + 7.0, 7.0, true, color(i));
        let mut ty = y + 15.0;
        let photo = p
            .rectified_photo
            .as_ref()
            .map(|r| {
                r.as_str()
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .to_string()
            })
            .unwrap_or_else(|| "pas de photo redressée".into());
        let facts = [
            format!("Fichier : {}", slide_file_name(&project.name, &p.label)),
            format!("Recalage : {} points", p.registration.len()),
            format!("Photo : {photo}"),
        ];
        for fact in facts {
            for line in wrap(&fact, MIN_TEXT_MM, tw) {
                page.text(&line, tx, ty, MIN_TEXT_MM, false, BLACK);
                ty += 4.2;
            }
            ty += 1.0;
        }
        // Sous la vignette : les dessins de la diapositive.
        let names: Vec<String> = project
            .visible_placements()
            .into_iter()
            .filter(|q| {
                all.iter()
                    .any(|a| a.placement == q.id && a.projector.as_deref() == Some(p.id.as_str()))
            })
            .map(|q| {
                project
                    .library
                    .iter()
                    .find(|d| d.id == q.drawing)
                    .map_or_else(|| q.drawing.clone(), |d| d.name.clone())
            })
            .collect();
        let mut ly = y + thumb + 6.0;
        let heading = match names.len() {
            0 => "Aucun dessin posé".to_string(),
            1 => "1 dessin :".to_string(),
            k => format!("{k} dessins :"),
        };
        page.text(&heading, x, ly, 3.5, true, BLACK);
        ly += 4.8;
        let lines = wrap(&names.join(", "), MIN_TEXT_MM, cell_w);
        let room = ((y + cell_h - ly) / 4.2).floor().max(0.0) as usize;
        for (k, line) in lines.iter().enumerate() {
            if k + 1 == room && lines.len() > room {
                page.text("…", x, ly, MIN_TEXT_MM, false, BLACK);
                break;
            }
            page.text(line, x, ly, MIN_TEXT_MM, false, BLACK);
            ly += 4.2;
        }
    }
    page
}

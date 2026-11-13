// Logique pure de l'écran Recalage (tâche 3.2) : paires de points, pose en
// deux clics, textes affichés. Testée sans écran (`logic.test.ts`).

import type { FacadeOverview, PairReport, RegistrationLine, RegistrationPair, RegistrationReport } from "../engine";
import type { LayerSpec } from "../viewer";

export interface XYPoint {
  x: number;
  y: number;
}

export const SLIDE_MM = 280;

/** Côté d'un point en cours de pose. */
export type Side = "slide" | "facade";

/** Un point posé d'un seul côté, en attente de l'autre. */
export interface Pending {
  slide: XYPoint | null;
  facade: XYPoint | null;
}

export const NO_PENDING: Pending = { slide: null, facade: null };

/** Vrai si le point est sur la diapositive (0 à 280 mm). */
export function onSlide(p: XYPoint): boolean {
  return p.x >= 0 && p.x <= SLIDE_MM && p.y >= 0 && p.y <= SLIDE_MM;
}

/**
 * Un clic sur un côté. Sans paire sélectionnée, il complète le point en
 * attente ; quand les deux côtés sont posés, la paire est ajoutée. Avec une
 * paire sélectionnée, il déplace le point de ce côté.
 */
export function click(
  pairs: RegistrationPair[],
  pending: Pending,
  selected: number | null,
  side: Side,
  at: XYPoint,
): { pairs: RegistrationPair[]; pending: Pending; changed: boolean } {
  const point = { x: at.x, y: at.y };
  if (selected !== null && selected >= 0 && selected < pairs.length) {
    const next = pairs.map((pair, i) => (i === selected ? { ...pair, [side]: point } : pair));
    return { pairs: next, pending, changed: true };
  }
  const placed: Pending = { ...pending, [side]: point };
  if (placed.slide && placed.facade) {
    return { pairs: [...pairs, { slide: placed.slide, facade: placed.facade }], pending: NO_PENDING, changed: true };
  }
  return { pairs, pending: placed, changed: false };
}

/** Déplace le point d'une paire, d'un côté ; sur la diapositive, il reste dans le carré. */
export function movePoint(pairs: RegistrationPair[], index: number, side: Side, at: XYPoint): RegistrationPair[] {
  const point =
    side === "slide"
      ? { x: Math.min(Math.max(at.x, 0), SLIDE_MM), y: Math.min(Math.max(at.y, 0), SLIDE_MM) }
      : { x: at.x, y: at.y };
  return pairs.map((pair, i) => (i === index ? { ...pair, [side]: point } : pair));
}

/** Rayon, en pixels d'écran, dans lequel on saisit un point pour le glisser. */
export const GRAB_RADIUS_PX = 14;

export function removePair(pairs: RegistrationPair[], index: number): RegistrationPair[] {
  return pairs.filter((_, i) => i !== index);
}

/** État d'un point pour l'affichage. */
export type PairStatus = "aberrant" | "bon" | "moyen" | "inconnu";

/** Écart sur la diapositive au-delà duquel un point est signalé en orange. */
export const WARN_SLIDE_MM = 1;

export function pairStatus(report: PairReport | undefined): PairStatus {
  if (!report) return "inconnu";
  if (report.outlier) return "aberrant";
  if (report.slideResidualMm === null) return "inconnu";
  return report.slideResidualMm <= WARN_SLIDE_MM ? "bon" : "moyen";
}

const mm1 = new Intl.NumberFormat("fr-FR", { minimumFractionDigits: 1, maximumFractionDigits: 1 });
const mm2 = new Intl.NumberFormat("fr-FR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

/** Écart sur la diapositive : « 0,35 mm ». */
export function formatSlideMm(value: number | null | undefined): string {
  return value === null || value === undefined || !Number.isFinite(value) ? "—" : `${mm2.format(value)} mm`;
}

/** Écart sur la façade, en centimètres : « 2,4 cm ». */
export function formatFacadeCm(value: number | null | undefined): string {
  return value === null || value === undefined || !Number.isFinite(value) ? "—" : `${mm1.format(value / 10)} cm`;
}

/** Ce qu'il faut cliquer, en clair : des détails communs, pas les coins de la photo. */
export const POINT_ADVICE =
  "Choisir un détail d'architecture visible à la fois sur la photo et sur la façade : coin de fenêtre, pied de colonne, pointe d'un arc.";

/** Phrase de résumé du recalage, sous la vue. */
export function summary(report: RegistrationReport | null, count: number): string {
  if (count === 0) return "Poser la photo sur la façade, puis glisser ses coins jusqu'à ce que les fenêtres tombent sur celles du plan.";
  if (count < 4) {
    const missing = 4 - count;
    return `Encore ${missing} point${missing > 1 ? "s" : ""} pour le trapèze : un détail sur la photo, puis le même sur la façade, bien répartis vers les quatre coins de la photo.`;
  }
  if (!report) return `${count} points.`;
  if (report.stage === "trapeze") {
    return count === 4
      ? "Trapèze calculé (4 points, sans écart mesurable). Pour affiner : attraper un détail de la photo sur la façade et le lâcher sur le même détail du plan."
      : "Trapèze ajusté au mieux sur 5 points. Au 6e, les points douteux sont signalés.";
  }
  const outliers = report.pairs.filter((p) => p.outlier).length;
  const mean = `écart moyen ${formatSlideMm(report.rmsSlideMm)} sur la diapositive, ${formatFacadeCm(report.rmsFacadeMm)} sur la façade`;
  const flagged =
    outliers === 0
      ? "aucun point douteux"
      : `${outliers} point${outliers > 1 ? "s" : ""} douteux à vérifier (en rouge)`;
  return `Relief corrigé sur ${count} points : ${mean} ; ${flagged}.`;
}

/** Consigne selon ce qui reste à poser. */
export function hint(pending: Pending, selected: number | null, side: Side = "facade"): string | null {
  if (selected !== null)
    return `Point ${selected + 1} : le glisser, ou cliquer sur la photo ou sur la façade pour l'y amener. Flèches : déplacer son point ${side === "facade" ? "de la façade" : "de la photo"} (⇧ : pas plus grand).`;
  if (pending.slide && !pending.facade) return "Cliquer maintenant le même point sur la façade.";
  if (pending.facade && !pending.slide) return "Cliquer maintenant le même point sur la photo.";
  return null;
}

/**
 * La photo redressée du projecteur posée sur la façade, d'après son
 * recalage enregistré (le même calque que l'écran Façade), ou rien s'il
 * n'est pas encore recalé. Fondu normal : sur un plan clair, on voit la
 * photo et le plan l'un à travers l'autre.
 */
export function facadePhotoLayer(
  overview: FacadeOverview | null,
  projectorId: string | null,
  show: boolean,
  opacity: number,
  photoUrl: (request: string) => string,
  live: MeshLike | null = null,
): LayerSpec | null {
  const p = overview?.projectors.find((q) => q.projectorId === projectorId);
  // Pendant un glisser, le maillage provisoire remplace celui enregistré.
  const mesh = live ?? p?.mesh ?? null;
  if (!mesh || !p?.photoPreview) return null;
  return {
    id: "photo-recalee",
    kind: "mesh-image",
    url: photoUrl(p.photoPreview),
    divisions: mesh.divisions,
    points: mesh.facade,
    opacity,
    visible: show,
  };
}

// --- Pose directe sur la façade (D-3.2-h) -------------------------------------
//
// Au lieu de cliquer chaque point deux fois (photo, puis façade), on pose la
// photo redressée sur la façade d'un clic, par ses quatre coins, puis on la
// cale en glissant directement sur la façade :
// - un coin (ou tout point déjà posé) se glisse ;
// - la poignée du centre déplace toute la photo ;
// - ailleurs sur la photo, on attrape un détail et on le lâche sur le même
//   détail du plan : c'est une nouvelle paire (une « épingle »).
// Les paires restent celles du recalage par points : rien ne change dans le
// projet ni dans le calcul du moteur.

/** Coins de la diapositive, dans l'ordre haut-gauche, haut-droit, bas-droit, bas-gauche. */
export const SLIDE_CORNERS: readonly XYPoint[] = [
  { x: 0, y: 0 },
  { x: SLIDE_MM, y: 0 },
  { x: SLIDE_MM, y: SLIDE_MM },
  { x: 0, y: SLIDE_MM },
];
const CORNER_NAMES = ["haut-gauche", "haut-droit", "bas-droit", "bas-gauche"];

/** « Coin haut-gauche » si la paire tient un coin de la diapositive, sinon rien. */
export function cornerName(pair: RegistrationPair): string | null {
  const k = SLIDE_CORNERS.findIndex((c) => Math.abs(c.x - pair.slide.x) < 1e-6 && Math.abs(c.y - pair.slide.y) < 1e-6);
  return k < 0 ? null : `Coin ${CORNER_NAMES[k]}`;
}

/** Nom d'un point dans la liste : « Coin bas-droit (guide) » ou « Point 5 ». */
export function pairTitle(pairs: RegistrationPair[], index: number): string {
  const corner = cornerName(pairs[index]);
  return corner ? `${corner} (guide)` : `Point ${index + 1}`;
}

/** Une zone rectangulaire de la façade, en mm. */
export interface MmRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Côté du carré où poser la photo la première fois : la taille moyenne des
 * emprises déjà recalées (les projecteurs d'un site se ressemblent), sinon
 * 40 % de la vue (on y voit la photo entière et de la marge pour la caler).
 */
export function initialSide(view: MmRect, otherFootprints: XYPoint[][]): number {
  const sizes = otherFootprints
    .filter((f) => f.length >= 3)
    .map((f) => {
      const xs = f.map((p) => p.x);
      const ys = f.map((p) => p.y);
      return Math.sqrt((Math.max(...xs) - Math.min(...xs)) * (Math.max(...ys) - Math.min(...ys)));
    })
    .filter((s) => Number.isFinite(s) && s > 0);
  if (sizes.length > 0) return sizes.reduce((a, b) => a + b, 0) / sizes.length;
  return 0.4 * Math.min(view.width, view.height);
}

/** Les quatre coins de la diapositive posés en carré de côté `side`, centré sur `center`. */
export function cornerPairs(center: XYPoint, side: number): RegistrationPair[] {
  return SLIDE_CORNERS.map((c) => ({
    slide: { x: c.x, y: c.y },
    facade: { x: center.x + ((c.x / SLIDE_MM) - 0.5) * side, y: center.y + ((c.y / SLIDE_MM) - 0.5) * side },
  }));
}

/** Toute la photo déplacée sur la façade : chaque point de façade glisse de (dx, dy). */
export function translateFacade(pairs: RegistrationPair[], dx: number, dy: number): RegistrationPair[] {
  return pairs.map((p) => ({ slide: p.slide, facade: { x: p.facade.x + dx, y: p.facade.y + dy } }));
}

/** Le maillage de la photo sur la façade (même forme que celui du moteur). */
export interface MeshLike {
  divisions: number;
  facade: XYPoint[];
}

/** Centre de la diapositive posé sur la façade : là où se dessine la poignée de déplacement. */
export function meshCenter(mesh: MeshLike): XYPoint | null {
  const n = mesh.divisions;
  if (n % 2 !== 0) return null;
  return mesh.facade[(n / 2) * (n + 1) + n / 2] ?? null;
}

/**
 * Le point de la diapositive (mm) qui s'affiche au point `at` de la façade,
 * ou rien s'il est hors de la photo. Même découpage en triangles que
 * l'affichage (`gridTriangles`) : le détail attrapé reste sous le pointeur.
 */
export function locateOnSlide(mesh: MeshLike, at: XYPoint): XYPoint | null {
  const n = mesh.divisions;
  const pts = mesh.facade;
  if (n < 1 || pts.length !== (n + 1) * (n + 1)) return null;
  const step = SLIDE_MM / n;
  const slideOf = (k: number) => ({ x: (k % (n + 1)) * step, y: Math.floor(k / (n + 1)) * step });
  for (let j = 0; j < n; j++) {
    for (let i = 0; i < n; i++) {
      const a = j * (n + 1) + i;
      const b = a + 1;
      const c = a + n + 1;
      const d = c + 1;
      for (const [u, v, w] of [
        [a, b, c],
        [b, d, c],
      ]) {
        const weights = barycentric(pts[u], pts[v], pts[w], at);
        if (!weights) continue;
        const [wu, wv, ww] = weights;
        const su = slideOf(u);
        const sv = slideOf(v);
        const sw = slideOf(w);
        return { x: wu * su.x + wv * sv.x + ww * sw.x, y: wu * su.y + wv * sv.y + ww * sw.y };
      }
    }
  }
  return null;
}

/** Poids barycentriques de `p` dans le triangle (a, b, c), ou rien s'il est dehors. */
function barycentric(a: XYPoint, b: XYPoint, c: XYPoint, p: XYPoint): [number, number, number] | null {
  const det = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
  if (Math.abs(det) < 1e-12) return null;
  const wa = ((b.y - c.y) * (p.x - c.x) + (c.x - b.x) * (p.y - c.y)) / det;
  const wb = ((c.y - a.y) * (p.x - c.x) + (a.x - c.x) * (p.y - c.y)) / det;
  const wc = 1 - wa - wb;
  const eps = -1e-9;
  return wa >= eps && wb >= eps && wc >= eps ? [wa, wb, wc] : null;
}

/** Ce que fait un appui sur la façade. */
export type FacadeGrab =
  | { kind: "point"; index: number }
  | { kind: "move" }
  | { kind: "pin"; slide: XYPoint }
  | { kind: "none" };

/**
 * Décide ce qu'attrape un appui sur la façade : un point posé (le plus
 * proche, à moins de `GRAB_RADIUS_PX`), la poignée du centre, un détail de la
 * photo (épingle), ou rien (la vue se déplace, ou c'est un clic de pose).
 * Barre d'espace enfoncée : toujours rien, pour déplacer la vue comme dans
 * Illustrator.
 */
export function facadeGrab(input: {
  pointHit: number | null;
  centerDistancePx: number | null;
  mesh: MeshLike | null;
  at: XYPoint;
  photoShown: boolean;
  spaceHeld: boolean;
}): FacadeGrab {
  if (input.spaceHeld) return { kind: "none" };
  if (input.pointHit !== null) return { kind: "point", index: input.pointHit };
  if (!input.photoShown || !input.mesh) return { kind: "none" };
  if (input.centerDistancePx !== null && input.centerDistancePx <= GRAB_RADIUS_PX) return { kind: "move" };
  const slide = locateOnSlide(input.mesh, input.at);
  return slide ? { kind: "pin", slide } : { kind: "none" };
}

/**
 * Maillage d'un simple trapèze par les 4 premières paires, calculé ici
 * seulement pour l'exemple du navigateur, où le moteur n'est pas joignable
 * (dans l'app, c'est toujours le moteur qui pose la photo).
 */
export function demoTrapezeMesh(pairs: RegistrationPair[], divisions = 16): MeshLike | null {
  if (pairs.length < 4) return null;
  const h = homography(
    pairs.slice(0, 4).map((p) => p.slide),
    pairs.slice(0, 4).map((p) => p.facade),
  );
  if (!h) return null;
  const step = SLIDE_MM / divisions;
  const facade: XYPoint[] = [];
  for (let j = 0; j <= divisions; j++) {
    for (let i = 0; i <= divisions; i++) {
      const x = i * step;
      const y = j * step;
      const w = h[6] * x + h[7] * y + 1;
      facade.push({ x: (h[0] * x + h[1] * y + h[2]) / w, y: (h[3] * x + h[4] * y + h[5]) / w });
    }
  }
  return { divisions, facade };
}

/** Homographie exacte par 4 points (8 inconnues), ou rien si elle est dégénérée. */
function homography(src: XYPoint[], dst: XYPoint[]): number[] | null {
  const a: number[][] = [];
  for (let k = 0; k < 4; k++) {
    const { x, y } = src[k];
    const { x: u, y: v } = dst[k];
    a.push([x, y, 1, 0, 0, 0, -u * x, -u * y, u]);
    a.push([0, 0, 0, x, y, 1, -v * x, -v * y, v]);
  }
  for (let c = 0; c < 8; c++) {
    let pivot = c;
    for (let r = c + 1; r < 8; r++) if (Math.abs(a[r][c]) > Math.abs(a[pivot][c])) pivot = r;
    if (Math.abs(a[pivot][c]) < 1e-12) return null;
    [a[c], a[pivot]] = [a[pivot], a[c]];
    for (let r = 0; r < 8; r++) {
      if (r === c) continue;
      const f = a[r][c] / a[c][c];
      for (let k = c; k < 9; k++) a[r][k] -= f * a[c][k];
    }
  }
  return a.map((row, r) => row[8] / row[r]);
}

/**
 * Bouton « Ajuster sur le plan » montré ou non. Retiré pour l'instant
 * (D-3.2-j) : sur les photos de Trévarez et le plan détaillé, aucun calcul
 * essayé ne tombait juste. Le moteur et `insitu registration adjust` restent.
 */
export const ADJUST_SHOWN = false;

// --- Points sûrs libres (D-3.2-j) ------------------------------------------
//
// Les quatre coins posés par « Poser la photo » (ou rendus par l'ajustement)
// ne sont qu'un guide : ils donnent la forme de départ. Les autres paires
// sont les points sûrs de l'utilisateur, posés n'importe où, même groupés
// ou alignés. Quand un point sûr bouge, les coins guides suivent, avec ce
// que les points sûrs permettent de décider :
// - 1 point : la photo glisse ;
// - 2 points (ou plus, mais alignés) : elle glisse, tourne et change d'échelle ;
// - 3 points (ou plus, mais peu étalés) : elle se penche aussi ;
// - 4 points bien étalés : la forme vient des seuls points sûrs.
// Les coins guides restent des paires (le moteur n'en sait rien), toujours
// d'accord avec les points sûrs : ils ne les tirent plus en arrière.

/** Écart type, en mm de diapositive, en dessous duquel des points sont dits alignés. */
export const PIN_LINE_MM = 15;
/** Écart type, en mm de diapositive, au-delà duquel 4 points sûrs décident seuls de la forme. */
export const PIN_SPREAD_MM = 25;

/** Vrai pour un coin guide (paire tenue par un coin de la diapositive). */
export function isGuide(pair: RegistrationPair): boolean {
  return cornerName(pair) !== null;
}

/** Ce que les points sûrs décident, du moins au plus. */
export type PinFit = "aucun" | "glisse" | "similitude" | "affine" | "libre";

/** Étalement des points (écarts types sur les deux axes principaux, du plus grand au plus petit). */
function spread(points: XYPoint[]): [number, number] {
  const n = points.length;
  const mx = points.reduce((s, p) => s + p.x, 0) / n;
  const my = points.reduce((s, p) => s + p.y, 0) / n;
  let sxx = 0;
  let syy = 0;
  let sxy = 0;
  for (const p of points) {
    sxx += (p.x - mx) ** 2;
    syy += (p.y - my) ** 2;
    sxy += (p.x - mx) * (p.y - my);
  }
  sxx /= n;
  syy /= n;
  sxy /= n;
  const t = (sxx + syy) / 2;
  const d = Math.sqrt(Math.max(0, ((sxx - syy) / 2) ** 2 + sxy ** 2));
  return [Math.sqrt(Math.max(0, t + d)), Math.sqrt(Math.max(0, t - d))];
}

/** Ce que des points sûrs posés à ces endroits de la photo permettent de décider. */
export function pinFit(slidePoints: XYPoint[]): PinFit {
  const n = slidePoints.length;
  if (n === 0) return "aucun";
  const [long, thin] = spread(slidePoints);
  if (n >= 4 && thin >= PIN_SPREAD_MM) return "libre";
  if (n >= 3 && thin >= PIN_LINE_MM) return "affine";
  if (n >= 2 && long >= 1) return "similitude";
  return "glisse";
}

/**
 * Les coins guides recalculés pour suivre les points sûrs (voir plus haut).
 * Rien ne change s'il n'y a pas exactement quatre coins guides, ou aucun
 * point sûr.
 */
export function followPins(pairs: RegistrationPair[]): RegistrationPair[] {
  const guides = pairs.map((p, i) => (isGuide(p) ? i : -1)).filter((i) => i >= 0);
  const pins = pairs.filter((p) => !isGuide(p));
  if (guides.length !== 4 || pins.length === 0) return pairs;
  const fit = pinFit(pins.map((p) => p.slide));
  let place: ((slide: XYPoint, facade: XYPoint) => XYPoint) | null = null;
  if (fit === "libre") {
    const h = homographyLeastSquares(
      pins.map((p) => p.slide),
      pins.map((p) => p.facade),
    );
    if (h) place = (slide) => applyHomography(h, slide);
  }
  if (!place) {
    const h0 = homography(
      guides.map((i) => pairs[i].slide),
      guides.map((i) => pairs[i].facade),
    );
    if (!h0) return pairs;
    const from = pins.map((p) => applyHomography(h0, p.slide));
    const to = pins.map((p) => p.facade);
    const t = fit === "affine" || fit === "libre" ? affineFit(from, to) : fit === "similitude" ? similarityFit(from, to) : null;
    const shift = translationFit(from, to);
    place = (_, facade) => (t ? t(facade) : { x: facade.x + shift.x, y: facade.y + shift.y });
  }
  const move = place;
  return pairs.map((p) => (isGuide(p) ? { ...p, facade: move(p.slide, p.facade) } : p));
}

function applyHomography(h: number[], p: XYPoint): XYPoint {
  const w = h[6] * p.x + h[7] * p.y + 1;
  return { x: (h[0] * p.x + h[1] * p.y + h[2]) / w, y: (h[3] * p.x + h[4] * p.y + h[5]) / w };
}

function translationFit(from: XYPoint[], to: XYPoint[]): XYPoint {
  const n = from.length;
  return {
    x: to.reduce((s, p) => s + p.x, 0) / n - from.reduce((s, p) => s + p.x, 0) / n,
    y: to.reduce((s, p) => s + p.y, 0) / n - from.reduce((s, p) => s + p.y, 0) / n,
  };
}

/** Similitude (glisse, rotation, échelle) au sens des moindres carrés. */
function similarityFit(from: XYPoint[], to: XYPoint[]): ((p: XYPoint) => XYPoint) | null {
  const n = from.length;
  const fx = from.reduce((s, p) => s + p.x, 0) / n;
  const fy = from.reduce((s, p) => s + p.y, 0) / n;
  const tx = to.reduce((s, p) => s + p.x, 0) / n;
  const ty = to.reduce((s, p) => s + p.y, 0) / n;
  let num_a = 0;
  let num_b = 0;
  let den = 0;
  for (let k = 0; k < n; k++) {
    const ux = from[k].x - fx;
    const uy = from[k].y - fy;
    const vx = to[k].x - tx;
    const vy = to[k].y - ty;
    num_a += ux * vx + uy * vy;
    num_b += ux * vy - uy * vx;
    den += ux * ux + uy * uy;
  }
  if (den < 1e-9) return null;
  const a = num_a / den;
  const b = num_b / den;
  return (p) => ({ x: tx + a * (p.x - fx) - b * (p.y - fy), y: ty + b * (p.x - fx) + a * (p.y - fy) });
}

/** Transformation affine au sens des moindres carrés. */
function affineFit(from: XYPoint[], to: XYPoint[]): ((p: XYPoint) => XYPoint) | null {
  const rows = from.map((p) => [p.x, p.y, 1]);
  const ata = [0, 1, 2].map((i) => [0, 1, 2].map((j) => rows.reduce((s, r) => s + r[i] * r[j], 0)));
  const cx = solve(
    ata,
    [0, 1, 2].map((i) => rows.reduce((s, r, k) => s + r[i] * to[k].x, 0)),
  );
  const cy = solve(
    ata,
    [0, 1, 2].map((i) => rows.reduce((s, r, k) => s + r[i] * to[k].y, 0)),
  );
  if (!cx || !cy) return null;
  return (p) => ({ x: cx[0] * p.x + cx[1] * p.y + cx[2], y: cy[0] * p.x + cy[1] * p.y + cy[2] });
}

/** Homographie au sens des moindres carrés (4 points ou plus), ou rien si elle est dégénérée. */
function homographyLeastSquares(src: XYPoint[], dst: XYPoint[]): number[] | null {
  // Coordonnées ramenées autour de 0 pour un calcul bien conditionné.
  const norm = (pts: XYPoint[]) => {
    const n = pts.length;
    const mx = pts.reduce((s, p) => s + p.x, 0) / n;
    const my = pts.reduce((s, p) => s + p.y, 0) / n;
    const d = pts.reduce((s, p) => s + Math.hypot(p.x - mx, p.y - my), 0) / n || 1;
    return { mx, my, k: Math.SQRT2 / d };
  };
  const ns = norm(src);
  const nd = norm(dst);
  const rows: number[][] = [];
  const rhs: number[] = [];
  src.forEach((p, i) => {
    const x = (p.x - ns.mx) * ns.k;
    const y = (p.y - ns.my) * ns.k;
    const u = (dst[i].x - nd.mx) * nd.k;
    const v = (dst[i].y - nd.my) * nd.k;
    rows.push([x, y, 1, 0, 0, 0, -u * x, -u * y]);
    rhs.push(u);
    rows.push([0, 0, 0, x, y, 1, -v * x, -v * y]);
    rhs.push(v);
  });
  const ata = [...Array(8).keys()].map((i) => [...Array(8).keys()].map((j) => rows.reduce((s, r) => s + r[i] * r[j], 0)));
  const atb = [...Array(8).keys()].map((i) => rows.reduce((s, r, k) => s + r[i] * rhs[k], 0));
  const g = solve(ata, atb);
  if (!g) return null;
  // Retour aux coordonnées d'origine : H = Td⁻¹ · G · Ts.
  const G = [
    [g[0], g[1], g[2]],
    [g[3], g[4], g[5]],
    [g[6], g[7], 1],
  ];
  const Ts = [
    [ns.k, 0, -ns.k * ns.mx],
    [0, ns.k, -ns.k * ns.my],
    [0, 0, 1],
  ];
  const TdInv = [
    [1 / nd.k, 0, nd.mx],
    [0, 1 / nd.k, nd.my],
    [0, 0, 1],
  ];
  const mul = (a: number[][], b: number[][]) => a.map((r) => [0, 1, 2].map((j) => r[0] * b[0][j] + r[1] * b[1][j] + r[2] * b[2][j]));
  const H = mul(TdInv, mul(G, Ts));
  if (Math.abs(H[2][2]) < 1e-12) return null;
  const h = H.flat().map((v) => v / H[2][2]);
  return h.every(Number.isFinite) ? h.slice(0, 8) : null;
}

/** Résout le système carré `a · x = b` (pivot de Gauss), ou rien s'il est singulier. */
function solve(a: number[][], b: number[]): number[] | null {
  const n = b.length;
  const m = a.map((row, i) => [...row, b[i]]);
  for (let c = 0; c < n; c++) {
    let pivot = c;
    for (let r = c + 1; r < n; r++) if (Math.abs(m[r][c]) > Math.abs(m[pivot][c])) pivot = r;
    if (Math.abs(m[pivot][c]) < 1e-12) return null;
    [m[c], m[pivot]] = [m[pivot], m[c]];
    for (let r = 0; r < n; r++) {
      if (r === c) continue;
      const f = m[r][c] / m[c][c];
      for (let k = c; k <= n; k++) m[r][k] -= f * m[c][k];
    }
  }
  return m.map((row, r) => row[n] / row[r]);
}

/** Phrase qui dit ce que les points sûrs commandent, sous les boutons. */
export function pinFitText(pairs: RegistrationPair[]): string {
  const pins = pairs.filter((p) => !isGuide(p));
  const guides = pairs.length - pins.length;
  if (guides !== 4) return "";
  switch (pinFit(pins.map((p) => p.slide))) {
    case "aucun":
      return "Aucun point sûr : la photo tient par ses coins guides.";
    case "glisse":
      return "1 point sûr : la photo glisse pour le suivre.";
    case "similitude":
      return `${pins.length} points sûrs${pins.length > 2 ? " presque alignés" : ""} : la photo glisse, tourne et change de taille ; sa forme vient des coins guides.`;
    case "affine":
      return `${pins.length} points sûrs : la photo se penche aussi ; il en faut 4 bien répartis pour qu'elle se déforme librement.`;
    case "libre":
      return `${pins.length} points sûrs bien répartis : la forme de la photo ne dépend plus que d'eux ; les coins guides suivent.`;
  }
}

// --- Lignes communes (D-3.2-k) ---------------------------------------------
//
// Idée de Benjamin : un bord de fenêtre (vertical), une corniche
// (horizontale), tracés sur la photo et sur le plan. Une ligne donne deux
// équations, comme un point : 4 lignes (dont 3 jamais concourantes)
// suffisent au trapèze. Points sûrs et lignes se combinent. Ce que les
// contraintes ne décident pas reste celui des coins guides : selon leur
// nombre d'équations, la photo glisse (2), fait une similitude (4), une
// affine (6) ou prend la forme libre (8 et plus) ; un faible rappel vers la
// pose des guides lève les directions que les contraintes laissent libres
// (deux corniches ne disent rien de la position gauche-droite).

/** Poids du rappel vers la pose des guides, en coordonnées réduites. */
const GUIDE_PRIOR = 1e-6;

/** Vrai si une paire de points d'une ligne est tracée (deux points distincts). */
export function lineDrawn(a: XYPoint, b: XYPoint): boolean {
  return Math.hypot(b.x - a.x, b.y - a.y) > 1e-3;
}

/**
 * Les coins guides recalculés pour suivre les points sûrs et les lignes.
 * Sans ligne, c'est `followPins`. Rien ne change sans les quatre coins
 * guides ni contrainte.
 */
export function followConstraints(pairs: RegistrationPair[], lines: RegistrationLine[]): RegistrationPair[] {
  const usable = lines.filter((l) => lineDrawn(l.slide[0], l.slide[1]) && lineDrawn(l.facade[0], l.facade[1]));
  if (usable.length === 0) return followPins(pairs);
  const guides = pairs.map((p, i) => (isGuide(p) ? i : -1)).filter((i) => i >= 0);
  const pins = pairs.filter((p) => !isGuide(p));
  if (guides.length !== 4) return pairs;
  const h0 = homography(
    guides.map((i) => pairs[i].slide),
    guides.map((i) => pairs[i].facade),
  );
  if (!h0) return pairs;
  // Coordonnées de façade réduites autour des coins guides.
  const gf = guides.map((i) => pairs[i].facade);
  const cx = gf.reduce((s, p) => s + p.x, 0) / 4;
  const cy = gf.reduce((s, p) => s + p.y, 0) / 4;
  const k = 1 / (gf.reduce((s, p) => s + Math.hypot(p.x - cx, p.y - cy), 0) / 4 || 1);
  const norm = (p: XYPoint) => ({ x: (p.x - cx) * k, y: (p.y - cy) * k });
  const denorm = (p: XYPoint) => ({ x: p.x / k + cx, y: p.y / k + cy });

  // Contraintes : un point P (pose actuelle) doit aller en Q ; ou un point P
  // doit tomber sur la droite a·x + b·y + c = 0 (a² + b² = 1).
  const pointsC = pins.map((p) => ({ from: norm(applyHomography(h0, p.slide)), to: norm(p.facade) }));
  const linesC = usable.flatMap((l) => {
    const f0 = norm(l.facade[0]);
    const f1 = norm(l.facade[1]);
    const a = f1.y - f0.y;
    const b = f0.x - f1.x;
    const n = Math.hypot(a, b);
    const line = { a: a / n, b: b / n, c: -(a * f0.x + b * f0.y) / n };
    return l.slide.map((s) => ({ from: norm(applyHomography(h0, s)), line }));
  });
  const equations = 2 * pointsC.length + linesC.length;
  const model = equations >= 8 ? "libre" : equations >= 6 ? "affine" : equations >= 4 ? "similitude" : "glisse";

  // T(p) = (x, y) en fonction des paramètres θ : x = ax·θ + dx, y = ay·θ + dy
  // (linéaire), sauf pour la forme libre (homographie, équations de type DLT).
  type Row = { coef: number[]; rhs: number };
  const rows: Row[] = [];
  let identity: number[];
  let apply: (theta: number[], p: XYPoint) => XYPoint;
  if (model === "libre") {
    identity = [1, 0, 0, 0, 1, 0, 0, 0];
    for (const { from: p, to: q } of pointsC) {
      rows.push({ coef: [p.x, p.y, 1, 0, 0, 0, -q.x * p.x, -q.x * p.y], rhs: q.x });
      rows.push({ coef: [0, 0, 0, p.x, p.y, 1, -q.y * p.x, -q.y * p.y], rhs: q.y });
    }
    for (const { from: p, line: l } of linesC) {
      rows.push({ coef: [l.a * p.x, l.a * p.y, l.a, l.b * p.x, l.b * p.y, l.b, l.c * p.x, l.c * p.y], rhs: -l.c });
    }
    apply = (t, p) => {
      const w = t[6] * p.x + t[7] * p.y + 1;
      return { x: (t[0] * p.x + t[1] * p.y + t[2]) / w, y: (t[3] * p.x + t[4] * p.y + t[5]) / w };
    };
  } else {
    const parts = (p: XYPoint): { ax: number[]; ay: number[] } =>
      model === "glisse"
        ? { ax: [1, 0], ay: [0, 1] }
        : model === "similitude"
          ? { ax: [p.x, -p.y, 1, 0], ay: [p.y, p.x, 0, 1] }
          : { ax: [p.x, p.y, 1, 0, 0, 0], ay: [0, 0, 0, p.x, p.y, 1] };
    // Pour la glisse, T(p) = p + θ : le point lui-même est la partie fixe.
    const fixed = (p: XYPoint) => (model === "glisse" ? p : { x: 0, y: 0 });
    identity = model === "glisse" ? [0, 0] : model === "similitude" ? [1, 0, 0, 0] : [1, 0, 0, 0, 1, 0];
    for (const { from: p, to: q } of pointsC) {
      const { ax, ay } = parts(p);
      const d = fixed(p);
      rows.push({ coef: ax, rhs: q.x - d.x });
      rows.push({ coef: ay, rhs: q.y - d.y });
    }
    for (const { from: p, line: l } of linesC) {
      const { ax, ay } = parts(p);
      const d = fixed(p);
      rows.push({ coef: ax.map((v, i) => l.a * v + l.b * ay[i]), rhs: -(l.a * d.x + l.b * d.y + l.c) });
    }
    apply = (t, p) => {
      const { ax, ay } = parts(p);
      const d = fixed(p);
      return { x: d.x + ax.reduce((s, v, i) => s + v * t[i], 0), y: d.y + ay.reduce((s, v, i) => s + v * t[i], 0) };
    };
  }
  // Rappel faible vers la pose des guides (θ = identité).
  const w = Math.sqrt(GUIDE_PRIOR);
  identity.forEach((v, i) => rows.push({ coef: identity.map((_, j) => (j === i ? w : 0)), rhs: w * v }));
  const n = identity.length;
  const ata = [...Array(n).keys()].map((i) => [...Array(n).keys()].map((j) => rows.reduce((s, r) => s + r.coef[i] * r.coef[j], 0)));
  const atb = [...Array(n).keys()].map((i) => rows.reduce((s, r) => s + r.coef[i] * r.rhs, 0));
  const theta = solve(ata, atb);
  if (!theta || !theta.every(Number.isFinite)) return pairs;
  const moved = pairs.map((p) => (isGuide(p) ? { ...p, facade: denorm(apply(theta, norm(p.facade))) } : p));
  return moved.every((p) => Number.isFinite(p.facade.x) && Number.isFinite(p.facade.y)) ? moved : pairs;
}

/** Phrase qui dit ce que points sûrs et lignes commandent. */
export function constraintsText(pairs: RegistrationPair[], lines: RegistrationLine[]): string {
  if (lines.length === 0) return pinFitText(pairs);
  const pins = pairs.filter((p) => !isGuide(p)).length;
  if (pairs.length - pins !== 4) return "";
  const equations = 2 * pins + 2 * lines.length;
  const what = `${lines.length} ligne${lines.length > 1 ? "s" : ""}${pins > 0 ? ` et ${pins} point${pins > 1 ? "s" : ""} sûr${pins > 1 ? "s" : ""}` : ""}`;
  if (equations >= 8) return `${what} : assez pour décider de la forme de la photo ; les coins guides suivent.`;
  if (equations >= 6) return `${what} : la photo glisse, tourne, change de taille et se penche ; il manque une ligne ou un point pour la forme libre.`;
  if (equations >= 4) return `${what} : la photo glisse, tourne et change de taille ; sa forme vient des coins guides.`;
  return `${what} : la photo glisse pour la suivre ; le reste vient des coins guides.`;
}

/**
 * Remet les quatre coins guides quand ils manquent (points posés un à un,
 * ou coin déplacé sur la photo, qui n'est alors plus un coin) : la pose
 * actuelle, trapèze ajusté au mieux sur tous les points, donne la place des
 * coins ; les points sûrs restent. Il faut au moins 4 points. Sans quoi,
 * ou si les quatre coins sont déjà là, rien ne change (D-3.2-k).
 */
export function ensureGuides(pairs: RegistrationPair[]): RegistrationPair[] {
  if (pairs.filter(isGuide).length === 4 || pairs.length < 4) return pairs;
  const h = homographyLeastSquares(
    pairs.map((p) => p.slide),
    pairs.map((p) => p.facade),
  );
  if (!h) return pairs;
  const corners = SLIDE_CORNERS.map((c) => ({ slide: { x: c.x, y: c.y }, facade: applyHomography(h, c) }));
  if (!corners.every((p) => Number.isFinite(p.facade.x) && Number.isFinite(p.facade.y))) return pairs;
  return [...corners, ...pairs.filter((p) => !isGuide(p))];
}

// Masque d'un dessin posé (demande de Benjamin du 14 novembre, D-4.2-u) :
// calculs purs, testés dans mask.test.ts, et rendu de l'aperçu masqué.
//
// Le masque s'écrit comme la gomme de la bibliothèque (tâche 4.8) : des
// coups de pinceau ronds ou des zones, en fractions du dessin (`erase`
// cache, `draw` fait réapparaître). Il suit donc le dessin partout. Un
// geste se trace à l'écran (mm de façade ou de diapositive) ; chaque point
// est ramené dans le dessin par l'inverse de la chaîne qui pose le dessin.

import type { RetouchStroke } from "../engine";
import { paintStrokes } from "../library/retouchLogic";
import type { Point } from "./placement";

export type MaskStroke = RetouchStroke;
/** Pinceau : on peint ; lasso : on entoure une zone. */
export type MaskShape = "brush" | "lasso";

/** Épaisseur du pinceau, en points d'écran : réglages proposés et valeur de départ. */
export const BRUSH_SIZES_PX = [6, 10, 16, 24, 36, 54, 80, 120, 180];
export const DEFAULT_BRUSH_PX = 36;

/** L'épaisseur suivante (`step` = 1 : plus épais ; -1 : plus fin). */
export function nextBrush(current: number, step: 1 | -1): number {
  const sizes = BRUSH_SIZES_PX;
  if (step > 0) return sizes.find((s) => s > current) ?? sizes[sizes.length - 1];
  return [...sizes].reverse().find((s) => s < current) ?? sizes[0];
}

type Map2 = (u: number, v: number) => Point;

/**
 * Le point `(u, v)` du dessin qui tombe sur `target`, par la méthode de
 * Newton sur `map` (la chaîne qui pose le dessin), en partant de `guess`
 * ou du meilleur point d'une grille. Hors du dessin, le point est
 * prolongé : un geste peut déborder du dessin. `null` si la chaîne ne se
 * laisse pas inverser.
 */
export function invertPoint(map: Map2, target: Point, guess?: [number, number] | null): [number, number] | null {
  let [u, v] = guess ?? coarseGuess(map, target);
  const h = 1e-4;
  for (let k = 0; k < 30; k++) {
    const p = map(u, v);
    const ex = p.x - target.x;
    const ey = p.y - target.y;
    const pu = map(u + h, v);
    const pv = map(u, v + h);
    const a = (pu.x - p.x) / h;
    const b = (pv.x - p.x) / h;
    const c = (pu.y - p.y) / h;
    const d = (pv.y - p.y) / h;
    const det = a * d - b * c;
    if (!Number.isFinite(det) || Math.abs(det) < 1e-12) return null;
    const du = (d * ex - b * ey) / det;
    const dv = (-c * ex + a * ey) / det;
    u -= du;
    v -= dv;
    if (!Number.isFinite(u) || !Number.isFinite(v)) return null;
    if (Math.abs(du) < 1e-9 && Math.abs(dv) < 1e-9) break;
  }
  const end = map(u, v);
  const scale = Math.hypot(map(1, 0).x - map(0, 0).x, map(1, 0).y - map(0, 0).y) || 1;
  return Math.hypot(end.x - target.x, end.y - target.y) < scale * 1e-4 ? [u, v] : null;
}

function coarseGuess(map: Map2, target: Point): [number, number] {
  let best: [number, number] = [0.5, 0.5];
  let distance = Infinity;
  const n = 16;
  for (let j = 0; j <= n; j++)
    for (let i = 0; i <= n; i++) {
      const p = map(i / n, j / n);
      const d = Math.hypot(p.x - target.x, p.y - target.y);
      if (d < distance) {
        distance = d;
        best = [i / n, j / n];
      }
    }
  return best;
}

/**
 * Une chaîne prolongée hors du dessin : `map` ne vaut qu'entre 0 et 1 (la
 * maille d'un dessin dans la diapositive s'arrête au bord) ; au-delà, elle
 * continue en ligne droite, pour qu'un geste puisse déborder.
 */
export function extended(map: Map2): Map2 {
  return (u, v) => {
    const cu = Math.min(Math.max(u, 0), 1);
    const cv = Math.min(Math.max(v, 0), 1);
    const p = map(cu, cv);
    if (cu === u && cv === v) return p;
    const h = 1e-3;
    const su = cu > 0.5 ? -h : h;
    const sv = cv > 0.5 ? -h : h;
    const pu = map(cu + su, cv);
    const pv = map(cu, cv + sv);
    const du = u - cu;
    const dv = v - cv;
    return {
      x: p.x + ((pu.x - p.x) / su) * du + ((pv.x - p.x) / sv) * dv,
      y: p.y + ((pu.y - p.y) / su) * du + ((pv.y - p.y) / sv) * dv,
    };
  };
}

/** Longueur sur l'écran (mm) du plus grand côté du dessin posé. */
export function longestSide(map: Map2): number {
  const o = map(0, 0);
  const a = map(1, 0);
  const b = map(0, 1);
  return Math.max(Math.hypot(a.x - o.x, a.y - o.y), Math.hypot(b.x - o.x, b.y - o.y));
}

/**
 * Le coup de masque d'un geste : les points tracés (mm) ramenés dans le
 * dessin. `widthMm` : épaisseur du pinceau en mm (ignorée pour un lasso).
 * `null` si le geste ne touche pas le dessin.
 */
export function strokeFor(
  map: Map2,
  points: Point[],
  shape: MaskShape,
  reveal: boolean,
  widthMm: number,
): MaskStroke | null {
  const chain = extended(map);
  const out: [number, number][] = [];
  let guess: [number, number] | null = null;
  for (const p of points) {
    const uv = invertPoint(chain, p, guess);
    if (!uv) continue;
    out.push(uv);
    guess = uv;
  }
  const lasso = shape === "lasso";
  if (out.length === 0 || (lasso && out.length < 3)) return null;
  const side = longestSide(map);
  const size = lasso ? 0 : Math.min(1, Math.max(1e-4, widthMm / (side || 1)));
  const round = (x: number) => Math.round(x * 1e6) / 1e6;
  // Un geste qui déborde beaucoup est ramené dans les limites du projet (-1 à 2).
  const clamp = (x: number) => Math.min(2, Math.max(-1, x));
  const pts = out.map(([u, v]) => [round(clamp(u)), round(clamp(v))] as [number, number]);
  const stroke: MaskStroke = { tool: reveal ? "draw" : "erase", size: round(size), points: pts };
  if (lasso) stroke.fill = true;
  if (!touchesDrawing(stroke)) return null;
  return stroke;
}

/** Le coup touche-t-il le carré du dessin ? (Sinon, rien à enregistrer.) */
function touchesDrawing(stroke: MaskStroke): boolean {
  const reach = stroke.fill ? 0 : stroke.size;
  const us = stroke.points.map((p) => p[0]);
  const vs = stroke.points.map((p) => p[1]);
  return (
    Math.max(...us) >= -reach && Math.min(...us) <= 1 + reach && Math.max(...vs) >= -reach && Math.min(...vs) <= 1 + reach
  );
}

function insidePolygon(points: [number, number][], u: number, v: number): boolean {
  let hit = false;
  for (let i = 0, j = points.length - 1; i < points.length; j = i++) {
    const [ax, ay] = points[i];
    const [bx, by] = points[j];
    if (ay > v !== by > v && u < ((bx - ax) * (v - ay)) / (by - ay) + ax) hit = !hit;
  }
  return hit;
}

function nearLine(points: [number, number][], r: number, x: number, y: number, aspect: [number, number]): boolean {
  const pts = points.map(([u, v]) => [u * aspect[0], v * aspect[1]] as const);
  if (pts.length === 1) return Math.hypot(x - pts[0][0], y - pts[0][1]) <= r;
  for (let k = 0; k + 1 < pts.length; k++) {
    const [ax, ay] = pts[k];
    const [bx, by] = pts[k + 1];
    const dx = bx - ax;
    const dy = by - ay;
    const len = dx * dx + dy * dy;
    const t = len > 0 ? Math.min(1, Math.max(0, ((x - ax) * dx + (y - ay) * dy) / len)) : 0;
    if (Math.hypot(x - (ax + t * dx), y - (ay + t * dy)) <= r) return true;
  }
  return false;
}

/** Proportions du dessin, le plus grand côté valant 1 (comme le moteur). */
export function aspectOf(width: number, height: number): [number, number] {
  const side = Math.max(width, height);
  return side > 0 ? [width / side, height / side] : [1, 1];
}

/** Le point `(u, v)` du dessin se voit-il malgré le masque ? */
export function visibleAt(strokes: MaskStroke[] | undefined, aspect: [number, number], u: number, v: number): boolean {
  let visible = true;
  for (const s of strokes ?? []) {
    const covered = s.fill
      ? s.points.length >= 3 && insidePolygon(s.points, u, v)
      : nearLine(s.points, s.size / 2, u * aspect[0], v * aspect[1], aspect);
    if (covered) visible = s.tool === "draw";
  }
  return visible;
}

/** Clé d'un aperçu masqué : le dessin et son masque. */
export function maskKey(drawing: string, strokes: MaskStroke[]): string {
  return `${drawing}|${JSON.stringify(strokes)}`;
}

/**
 * L'aperçu masqué d'un dessin : son image (traits clairs sur fond
 * transparent) effacée là où le masque cache, et sa silhouette en noir,
 * effacée de même (une partie cachée ne cache plus rien derrière). Deux
 * adresses d'images à poser sur la façade à la place de l'aperçu.
 */
export async function renderMasked(
  previewUrl: string,
  silhouette: [number, number][][],
  strokes: MaskStroke[],
): Promise<{ pose: string; shade: string | null }> {
  const response = await fetch(previewUrl);
  const bitmap = await createImageBitmap(await response.blob());
  const { width, height } = bitmap;
  const canvas = (w: number, h: number) => {
    const c = document.createElement("canvas");
    c.width = w;
    c.height = h;
    return c;
  };
  // Visibilité : opaque partout, puis le masque (« erase » efface).
  const visibility = canvas(width, height);
  const vis = visibility.getContext("2d")!;
  vis.fillStyle = "#fff";
  vis.fillRect(0, 0, width, height);
  paintStrokes(vis, strokes, width, height);

  const pose = canvas(width, height);
  const p = pose.getContext("2d")!;
  p.drawImage(bitmap, 0, 0);
  p.globalCompositeOperation = "destination-in";
  p.drawImage(visibility, 0, 0);
  bitmap.close();

  let shade: string | null = null;
  if (silhouette.length > 0) {
    const s = canvas(width, height);
    const c = s.getContext("2d")!;
    c.fillStyle = "#000";
    for (const polygon of silhouette) {
      c.beginPath();
      polygon.forEach(([u, v], i) => (i === 0 ? c.moveTo(u * width, v * height) : c.lineTo(u * width, v * height)));
      c.closePath();
      c.fill("evenodd");
    }
    c.globalCompositeOperation = "destination-in";
    c.drawImage(visibility, 0, 0);
    shade = await blobUrl(s);
  }
  return { pose: await blobUrl(pose), shade };
}

function blobUrl(canvas: HTMLCanvasElement): Promise<string> {
  return new Promise((resolve, reject) =>
    canvas.toBlob((blob) => (blob ? resolve(URL.createObjectURL(blob)) : reject(new Error("aperçu masqué"))), "image/png"),
  );
}

// Écran de redressement : la logique pure, testée sans écran (`npm test`).
// Le calcul qui fait foi (détection, redressement, export) est dans le
// moteur ; ici on prépare la saisie des coins et l'affichage.

import type { Point, Pyramid } from "../viewer";
import { tilePath, tileRect } from "../viewer/geometry";
import type { RectificationView, XY } from "../engine";

/** Les quatre coins de la diapositive, dans l'ordre attendu par le moteur. */
export const CORNER_NAMES = ["haut-gauche", "haut-droit", "bas-droit", "bas-gauche"] as const;

export type Corners = [Point, Point, Point, Point];

export function toXY(p: Point): XY {
  return [p.x, p.y];
}

export function fromXY([x, y]: XY): Point {
  return { x, y };
}

/**
 * Coins de départ quand la détection n'a rien donné : un carré centré qui
 * occupe 60 % du plus petit côté de la photo, à glisser sur les vrais coins.
 */
export function defaultCorners(width: number, height: number): Corners {
  const side = 0.6 * Math.min(width, height);
  const x0 = (width - side) / 2;
  const y0 = (height - side) / 2;
  return [
    { x: x0, y: y0 },
    { x: x0 + side, y: y0 },
    { x: x0 + side, y: y0 + side },
    { x: x0, y: y0 + side },
  ];
}

/**
 * Indice de la poignée la plus proche du point visé, si elle est à moins de
 * `radius` (même unité que les points, en général des pixels d'écran).
 */
export function hitHandle(handles: Point[], target: Point, radius: number): number | null {
  let best: number | null = null;
  let bestDistance = radius;
  handles.forEach((h, i) => {
    const d = Math.hypot(h.x - target.x, h.y - target.y);
    if (d <= bestDistance) {
      best = i;
      bestDistance = d;
    }
  });
  return best;
}

/**
 * Vrai si les quatre coins, pris dans l'ordre, forment un quadrilatère
 * convexe qui ne se croise pas : sinon la mire serait retournée ou les
 * coins mal ordonnés, et la détection partirait de travers.
 */
export function isConvexQuad(corners: Point[]): boolean {
  if (corners.length !== 4) return false;
  let sign = 0;
  for (let i = 0; i < 4; i++) {
    const a = corners[i];
    const b = corners[(i + 1) % 4];
    const c = corners[(i + 2) % 4];
    const cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
    if (Math.abs(cross) < 1e-9) return false;
    const s = Math.sign(cross);
    if (sign === 0) sign = s;
    else if (s !== sign) return false;
  }
  return true;
}

/** Côté le plus court du quadrilatère, pour refuser des coins confondus. */
export function shortestSide(corners: Point[]): number {
  let shortest = Infinity;
  for (let i = 0; i < corners.length; i++) {
    const a = corners[i];
    const b = corners[(i + 1) % corners.length];
    shortest = Math.min(shortest, Math.hypot(b.x - a.x, b.y - a.y));
  }
  return shortest;
}

/** Qualité d'un croisement d'après son écart de validation croisée. */
export type NodeQuality = "bon" | "moyen" | "mauvais";

/** Moins de 0,15 mm : bon (précision attendue d'un croisement) ; moins de 0,3 mm : moyen. */
export function nodeQuality(errorMm: number): NodeQuality {
  if (errorMm < 0.15) return "bon";
  if (errorMm < 0.3) return "moyen";
  return "mauvais";
}

const mm2 = new Intl.NumberFormat("fr-FR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const s1 = new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 1 });
const int = new Intl.NumberFormat("fr-FR");

/** Résumé d'un redressement, en phrases courtes pour la barre du bas. */
export function summarize(view: RectificationView): { text: string; quality: NodeQuality } {
  const how = view.method === "automatic" ? "Mire détectée automatiquement" : "Mire détectée à partir des coins posés à la main";
  const nodes = `${int.format(view.nodes.length)} croisements sur ${int.format(view.measurableNodes)}`;
  const precision = `écart estimé ${mm2.format(view.leaveOneOutMm.mean)} mm en moyenne, ${mm2.format(view.leaveOneOutMm.p95)} mm pour 95 % des croisements`;
  const coverage = `mire couverte à ${Math.round(view.coverage * 100)} %`;
  const time = view.seconds !== null ? ` · calcul ${s1.format(view.seconds)} s` : "";
  // Jugé comme le critère de la tâche 2.4a : 95 % des croisements à moins de
  // 0,3 mm (l'écart de validation croisée est un majorant).
  const p95 = view.leaveOneOutMm.p95;
  const quality: NodeQuality = view.coverage < 0.8 || p95 >= 0.6 ? "mauvais" : p95 < 0.3 ? "bon" : "moyen";
  return { text: `${how} : ${nodes}, ${precision}, ${coverage}${time}`, quality };
}

/** Nom proposé pour l'export : « A-redressee-500dpi.tif ». */
export function exportFileName(label: string, dpi: number, format: "tif" | "png"): string {
  const clean = label.trim().replace(/[/\\:*?"<>|]+/g, "-") || "projecteur";
  return `${clean}-redressee-${Math.round(dpi)}dpi.${format}`;
}

/** Résolutions d'export proposées, en points par pouce. */
export const EXPORT_DPI_CHOICES = [300, 500, 1000, 1400] as const;

/** Taille d'une image de 280 mm à cette résolution, en px (même arrondi que le moteur). */
export function slidePixels(dpi: number): number {
  return Math.round((280 / 25.4) * dpi);
}

/** Libellé d'un nouveau projecteur : la première lettre libre (A, B, C…). */
export function nextProjectorLabel(existing: string[]): string {
  const taken = new Set(existing.map((l) => l.trim().toUpperCase()));
  for (let i = 0; i < 26; i++) {
    const letter = String.fromCharCode(65 + i);
    if (!taken.has(letter)) return letter;
  }
  let n = 1;
  while (taken.has(`P${n}`)) n++;
  return `P${n}`;
}

/** Une tuile pleine résolution à poser dans la loupe, en pixels d'écran. */
export interface LoupeTile {
  path: string;
  left: number;
  top: number;
  width: number;
  height: number;
}

/**
 * Tuiles du niveau 0 (pleine résolution) qui couvrent une loupe carrée de
 * `size` pixels d'écran centrée sur `center` (pixels de photo), grossie
 * `zoom` fois, avec leur place dans la loupe.
 */
export function loupeTiles(pyramid: Pyramid, center: Point, zoom: number, size: number): LoupeTile[] {
  const half = size / (2 * zoom);
  const x0 = center.x - half;
  const y0 = center.y - half;
  const t = pyramid.tileSize;
  const level = pyramid.levels[0];
  const colMin = Math.max(0, Math.floor(x0 / t));
  const rowMin = Math.max(0, Math.floor(y0 / t));
  const colMax = Math.min(level.columns - 1, Math.floor((center.x + half) / t));
  const rowMax = Math.min(level.rows - 1, Math.floor((center.y + half) / t));
  const tiles: LoupeTile[] = [];
  for (let row = rowMin; row <= rowMax; row++) {
    for (let col = colMin; col <= colMax; col++) {
      const r = tileRect(pyramid, 0, col, row);
      tiles.push({
        path: tilePath(pyramid, 0, col, row),
        left: (r.x - x0) * zoom,
        top: (r.y - y0) * zoom,
        width: r.width * zoom,
        height: r.height * zoom,
      });
    }
  }
  return tiles;
}

/** Place d'un point de la photo dans la loupe, en pixels d'écran. */
export function loupePosition(point: Point, center: Point, zoom: number, size: number): Point {
  return { x: (point.x - center.x) * zoom + size / 2, y: (point.y - center.y) * zoom + size / 2 };
}

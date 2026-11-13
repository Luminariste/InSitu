// Gomme et crayon sur un dessin (tâche 4.8) : logique pure, testée sans
// écran (`retouchLogic.test.ts`). Les retouches sont en fractions du dessin,
// comme dans le moteur (`crates/insitu-core/src/library/retouch.rs`).

import type { RetouchStroke } from "../engine";

export type RetouchTool = RetouchStroke["tool"];
export type RetouchShape = "trait" | "zone";

/** Les retouches et celles annulées, qu'on peut rétablir. */
export interface RetouchHistory {
  strokes: RetouchStroke[];
  undone: RetouchStroke[];
}

export function startHistory(strokes: RetouchStroke[]): RetouchHistory {
  return { strokes, undone: [] };
}

export function pushStroke(history: RetouchHistory, stroke: RetouchStroke): RetouchHistory {
  return { strokes: [...history.strokes, stroke], undone: [] };
}

export function undoStroke(history: RetouchHistory): RetouchHistory {
  const last = history.strokes.at(-1);
  if (!last) return history;
  return { strokes: history.strokes.slice(0, -1), undone: [...history.undone, last] };
}

export function redoStroke(history: RetouchHistory): RetouchHistory {
  const next = history.undone.at(-1);
  if (!next) return history;
  return { strokes: [...history.strokes, next], undone: history.undone.slice(0, -1) };
}

/** Tout effacer : revenir au dessin d'origine (annulable). */
export function clearStrokes(history: RetouchHistory): RetouchHistory {
  if (history.strokes.length === 0) return history;
  return { strokes: [], undone: [...history.undone, ...[...history.strokes].reverse()] };
}

/**
 * Ajoute un point au tracé en cours s'il est assez loin du précédent
 * (`minPx` pixels du dessin), pour garder `projet.json` léger.
 */
export function addPoint(
  points: [number, number][],
  point: [number, number],
  size: { width: number; height: number },
  minPx: number,
): [number, number][] {
  const last = points.at(-1);
  if (last) {
    const dx = (point[0] - last[0]) * size.width;
    const dy = (point[1] - last[1]) * size.height;
    if (Math.hypot(dx, dy) < minPx) return points;
  }
  return [...points, [round(point[0]), round(point[1])]];
}

/** Six décimales : un millionième du dessin, bien sous le pixel. */
function round(value: number): number {
  return Math.round(value * 1e6) / 1e6;
}

/**
 * La retouche d'un tracé fini, ou `null` s'il ne fait rien (une zone de
 * moins de trois points). `sizePx` : épaisseur en pixels du dessin.
 */
export function finishStroke(
  tool: RetouchTool,
  shape: RetouchShape,
  sizePx: number,
  points: [number, number][],
  size: { width: number; height: number },
): RetouchStroke | null {
  if (points.length === 0) return null;
  if (shape === "zone") {
    return points.length >= 3 ? { tool, fill: true, size: 0, points } : null;
  }
  const side = Math.max(size.width, size.height);
  const fraction = Math.min(1, Math.max(1e-6, sizePx / side));
  return { tool, size: round(fraction), points };
}

/** Épaisseurs proposées (pixels du dessin), de la plus fine à la plus large. */
export function sizeSteps(width: number, height: number): number[] {
  const side = Math.max(width, height);
  const steps = [1, 2, 3, 5, 8, 12, 20, 30, 50, 80, 120, 200, 300, 500];
  return steps.filter((s) => s <= side / 4);
}

/** Épaisseur suivante (`step` = +1) ou précédente (-1) dans la liste. */
export function nextSize(current: number, step: 1 | -1, width: number, height: number): number {
  const steps = sizeSteps(width, height);
  if (steps.length === 0) return current;
  if (step > 0) return steps.find((s) => s > current) ?? steps[steps.length - 1];
  return [...steps].reverse().find((s) => s < current) ?? steps[0];
}

/** Épaisseur de départ : environ 1 % du plus grand côté. */
export function defaultSize(width: number, height: number): number {
  const target = Math.max(width, height) / 100;
  const steps = sizeSteps(width, height);
  return steps.reduce((best, s) => (Math.abs(s - target) < Math.abs(best - target) ? s : best), steps[0] ?? 1);
}

/** Résumé pour l'inspecteur : « Dessin d'origine », « 3 retouches (2 à la gomme, 1 au crayon) ». */
export function retouchLine(strokes: RetouchStroke[] | undefined): string {
  const list = strokes ?? [];
  if (list.length === 0) return "Dessin d'origine, sans retouche";
  const erased = list.filter((s) => s.tool === "erase").length;
  const drawn = list.length - erased;
  const parts = [erased > 0 ? `${erased} à la gomme` : "", drawn > 0 ? `${drawn} au crayon` : ""].filter(Boolean);
  return `${list.length} retouche${list.length > 1 ? "s" : ""} (${parts.join(", ")})`;
}

/** Deux listes de retouches identiques ? */
export function sameStrokes(a: RetouchStroke[], b: RetouchStroke[]): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * Peint les retouches sur un canevas où le dessin est déjà posé (noir
 * transparent) : la gomme retire le trait, le crayon peint du blanc.
 */
export function paintStrokes(context: CanvasRenderingContext2D, strokes: RetouchStroke[], width: number, height: number): void {
  const side = Math.max(width, height);
  for (const stroke of strokes) {
    context.globalCompositeOperation = stroke.tool === "erase" ? "destination-out" : "source-over";
    context.fillStyle = "#fff";
    context.strokeStyle = "#fff";
    const points = stroke.points.map(([u, v]) => [u * width, v * height] as const);
    if (stroke.fill) {
      context.beginPath();
      points.forEach(([x, y], i) => (i === 0 ? context.moveTo(x, y) : context.lineTo(x, y)));
      context.closePath();
      context.fill("evenodd");
      continue;
    }
    const lineWidth = Math.max(1, stroke.size * side);
    if (points.length === 1) {
      context.beginPath();
      context.arc(points[0][0], points[0][1], lineWidth / 2, 0, 2 * Math.PI);
      context.fill();
      continue;
    }
    context.lineWidth = lineWidth;
    context.lineCap = "round";
    context.lineJoin = "round";
    context.beginPath();
    points.forEach(([x, y], i) => (i === 0 ? context.moveTo(x, y) : context.lineTo(x, y)));
    context.stroke();
  }
  context.globalCompositeOperation = "source-over";
}

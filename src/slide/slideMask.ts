// Masque général de la diapositive (demande de Benjamin du 14 novembre,
// D-3.6-h) : calculs purs, testés dans slideMask.test.ts.
//
// Des formes tracées sur la photo redressée, au lasso (à main levée) ou à
// la plume (point par point), en mm de diapositive. Dedans, ou dehors au
// choix, tout devient noir à l'export, quels que soient les dessins.

import type { SlideMask } from "../engine";
import type { LayerSpec } from "../viewer";
import type { Point } from "../compose/placement";

export type MaskPen = "lasso" | "pen";

/** Le masque vide d'une diapositive. */
export const EMPTY_MASK: SlideMask = { shapes: [] };

/** Distance (points d'écran) sous laquelle un clic de plume ferme la forme sur son premier point. */
export const CLOSE_PX = 8;

/** Le masque avec une forme de plus (trois points au moins, sinon inchangé). */
export function addShape(mask: SlideMask | null | undefined, points: Point[]): SlideMask | null {
  if (points.length < 3) return null;
  const round = (x: number) => Math.round(x * 1000) / 1000;
  const shape = points.map((p) => [round(p.x), round(p.y)] as [number, number]);
  const base = mask ?? EMPTY_MASK;
  return { ...base, shapes: [...base.shapes, shape] };
}

/** Le masque sans sa forme n° `index` ; `null` quand il ne reste rien à garder. */
export function removeShape(mask: SlideMask, index: number): SlideMask | null {
  const shapes = mask.shapes.filter((_, i) => i !== index);
  return shapes.length === 0 && !mask.outside ? null : { ...mask, shapes };
}

/** Le masque à appliquer (comme le moteur) : ni caché, ni sans effet. */
export function maskActive(mask: SlideMask | null | undefined): boolean {
  return !!mask && !mask.hidden && (mask.shapes.length > 0 || !!mask.outside);
}

/** Les points d'une plume : le clic ferme-t-il la forme sur son premier point ? */
export function closesOnFirst(points: Point[], click: Point, toScreen: (p: Point) => Point): boolean {
  if (points.length < 3) return false;
  const a = toScreen(points[0]);
  const b = toScreen(click);
  return Math.hypot(a.x - b.x, a.y - b.y) <= CLOSE_PX;
}

const toPoints = (shape: [number, number][]) => shape.map(([x, y]) => ({ x, y }));

/**
 * Ce que la visionneuse montre du masque : le noir là où l'export sera
 * noir (assombri, pour voir la photo dessous), le contour de chaque forme,
 * et la forme en cours de tracé.
 */
export function maskLayers(mask: SlideMask | null | undefined, square: Point[], drawing: Point[], selected: number | null): LayerSpec[] {
  const out: LayerSpec[] = [];
  if (mask && !mask.hidden) {
    const shapes = mask.shapes.map(toPoints);
    out.push({
      id: "masque-noir",
      kind: "polygons",
      shapes: mask.outside
        ? [{ points: square, holes: shapes, color: 0x000000, fillAlpha: 0.7, strokeAlpha: 0 }]
        : shapes.map((points) => ({ points, color: 0x000000, fillAlpha: 0.7, strokeAlpha: 0 })),
    });
    out.push({
      id: "masque-contours",
      kind: "polygons",
      shapes: shapes.map((points, i) => ({ points, color: i === selected ? 0x0a84ff : 0xff9f0a, widthPx: i === selected ? 2.5 : 1.5 })),
    });
  }
  // Deux points : un trait (le triangle aplati le montre).
  const trace = drawing.length === 2 ? [drawing[0], drawing[1], drawing[1]] : drawing;
  if (trace.length >= 3)
    out.push({ id: "masque-trace", kind: "polygons", shapes: [{ points: trace, color: 0x0a84ff, widthPx: 2, fillAlpha: 0.15 }] });
  return out;
}

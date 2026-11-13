// Retouche d'un dessin dans la vue Diapositive (tâche 4.6) : calculs purs,
// testés dans adjust.test.ts.
//
// Le moteur situe chaque dessin dans la diapositive par une maille de
// points (mm de diapositive, avant retouche). La retouche est une grille de
// déformation tendue sur le rectangle qui contient ces points ; l'écran
// applique la même grille que le moteur (`deformation.ts`, prouvé identique
// par la tâche 4.4), l'aperçu montre donc ce que l'export fera.

import type { SlideDrawing } from "../engine";
import {
  deformPoint,
  frameFractions,
  framePoint,
  identity,
  moveNode,
  type Deformation,
  type DeformationFrame,
} from "../compose/deformation";
import type { Point } from "../compose/placement";

/** Le cadre d'une nouvelle retouche, comme le moteur (`placement::adjust::adjustment_frame`). */
export function restFrame(points: Point[]): DeformationFrame | null {
  if (points.length === 0) return null;
  const xs = points.map((p) => p.x);
  const ys = points.map((p) => p.y);
  const x0 = Math.min(...xs);
  const x1 = Math.max(...xs);
  const y0 = Math.min(...ys);
  const y1 = Math.max(...ys);
  if (!(x1 > x0 && y1 > y0)) return null;
  return { center: { x: (x0 + x1) / 2, y: (y0 + y1) / 2 }, width: x1 - x0, height: y1 - y0, rotation: 0 };
}

/** Une grille de retouche au repos de `columns × rows` cases sur le dessin. */
export function restAdjustment(d: SlideDrawing, columns: number, rows: number): Deformation | null {
  const frame = restFrame(d.points);
  return frame ? identity(frame, columns, Math.max(rows, 1)) : null;
}

/** La maille du dessin dans la diapositive, retouche comprise. */
export function adjustedPoints(d: SlideDrawing, adjustment: Deformation | null): Point[] {
  return adjustment ? d.points.map((p) => deformPoint(adjustment, p)) : d.points;
}

/** Le contour du dessin dans la diapositive (bord de la maille), retouche comprise. */
export function adjustedOutline(d: SlideDrawing, adjustment: Deformation | null): Point[] {
  const pts = adjustedPoints(d, adjustment);
  const n = d.divisions;
  const at = (i: number, j: number) => pts[j * (n + 1) + i];
  const out: Point[] = [];
  for (let i = 0; i < n; i++) out.push(at(i, 0));
  for (let j = 0; j < n; j++) out.push(at(n, j));
  for (let i = n; i > 0; i--) out.push(at(i, n));
  for (let j = n; j > 0; j--) out.push(at(0, j));
  return out;
}

function inside(polygon: Point[], pt: Point): boolean {
  let hit = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const a = polygon[i];
    const b = polygon[j];
    if (a.y > pt.y !== b.y > pt.y && pt.x < ((b.x - a.x) * (pt.y - a.y)) / (b.y - a.y) + a.x) hit = !hit;
  }
  return hit;
}

/** Le dessin sous le point (mm de diapositive) : le plus en avant d'abord. */
export function drawingAt(
  drawings: SlideDrawing[],
  adjustmentOf: (placement: string) => Deformation | null,
  pt: Point,
): SlideDrawing | null {
  for (let k = drawings.length - 1; k >= 0; k--) {
    const d = drawings[k];
    if (inside(adjustedOutline(d, adjustmentOf(d.placement)), pt)) return d;
  }
  return null;
}

/** Une retouche en cours de geste. */
export type AdjustGesture =
  | { kind: "node"; start: Point; before: Deformation; index: number; origin: Point }
  | { kind: "move"; start: Point; before: Deformation };

/**
 * Tous les nœuds décalés de (dx, dy) mm : le dessin glisse dans la
 * diapositive. (Déplacer le cadre ne suffirait pas : une grille au repos
 * reste au repos où qu'on mette son cadre.)
 */
export function shiftNodes(d: Deformation, dx: number, dy: number): Deformation {
  const r = (d.frame.rotation * Math.PI) / 180;
  const ds = (dx * Math.cos(r) + dy * Math.sin(r)) / d.frame.width;
  const dt = (-dx * Math.sin(r) + dy * Math.cos(r)) / d.frame.height;
  return { ...d, nodes: d.nodes.map(([s, t]) => [s + ds, t + dt] as [number, number]) };
}

/** La retouche au point `at` du geste. */
export function applyAdjustGesture(g: AdjustGesture, at: Point): Deformation {
  const dx = at.x - g.start.x;
  const dy = at.y - g.start.y;
  if (g.kind === "move") return shiftNodes(g.before, dx, dy);
  return moveNode(g.before, g.index, { x: g.origin.x + dx, y: g.origin.y + dy });
}

/** Centre du rectangle qui contient le dessin retouché (mm de diapositive). */
export function adjustedCenter(d: SlideDrawing, adjustment: Deformation | null): Point {
  const pts = adjustedPoints(d, adjustment);
  const xs = pts.map((p) => p.x);
  const ys = pts.map((p) => p.y);
  return { x: (Math.min(...xs) + Math.max(...xs)) / 2, y: (Math.min(...ys) + Math.max(...ys)) / 2 };
}

/**
 * Le dessin tourné de `deg` degrés (sens des aiguilles d'une montre, y vers
 * le bas) autour de `pivot`, dans cette diapositive seulement : ce sont les
 * nœuds qui tournent (tourner le cadre seul ne changerait rien, comme pour
 * le décalage). La grille reste une rotation exacte : la perspective des
 * quatre coins et la surface lisse de la grille suivent leurs nœuds.
 */
export function rotateAdjustment(d: Deformation, deg: number, pivot: Point): Deformation {
  const r = (deg * Math.PI) / 180;
  const cos = Math.cos(r);
  const sin = Math.sin(r);
  const nodes = d.nodes.map(([s, t]) => {
    const p = framePoint(d.frame, s, t);
    const dx = p.x - pivot.x;
    const dy = p.y - pivot.y;
    return frameFractions(d.frame, { x: pivot.x + dx * cos - dy * sin, y: pivot.y + dx * sin + dy * cos });
  });
  return { ...d, nodes };
}

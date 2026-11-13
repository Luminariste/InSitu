// Outils de la vue Diapositive, comme ceux de la Façade (demande de
// Benjamin, D-3.6-f) : calculs purs, testés dans tools.test.ts.
//
// Dans une diapositive, un dessin se modifie pour cette diapositive seule,
// par sa retouche (tâche 4.6) : une grille de déformation en mm de
// diapositive, tendue sur le dessin au repos. Déplacer, tourner, étirer,
// aligner, répartir sont donc des transformations des nœuds de cette
// grille ; la façade ne bouge pas.

import { deformPoint, frameFractions, framePoint, identity, moveNode, type Deformation } from "../compose/deformation";
import {
  constrainMove,
  copyOf,
  newId,
  normalizeAngle,
  snapAngle,
  ROTATE_SNAP_DEGREES,
  type Alignment,
  type Placement,
  type Point,
  type Rect,
} from "../compose/placement";
import { frameHandles } from "../compose/gesture";

/** Le plus petit étirement permis : en dessous, le dessin se retournerait (refusé par le moteur). */
export const MIN_STRETCH = 0.01;

/** Chaque nœud de la retouche passé par `f` (mm de diapositive) ; le cadre ne bouge pas. */
export function transformAdjustment(d: Deformation, f: (p: Point) => Point): Deformation {
  return { ...d, nodes: d.nodes.map(([s, t]) => frameFractions(d.frame, f(framePoint(d.frame, s, t)))) };
}

/** Le dessin étiré de `sx`, `sy` depuis `anchor` (mm de diapositive). */
export function scaleAdjustment(d: Deformation, sx: number, sy: number, anchor: Point): Deformation {
  return transformAdjustment(d, (p) => ({ x: anchor.x + (p.x - anchor.x) * sx, y: anchor.y + (p.y - anchor.y) * sy }));
}

/** Le dessin tourné de `deg` degrés (sens des aiguilles d'une montre) autour de `pivot`. */
export function turnAdjustment(d: Deformation, deg: number, pivot: Point): Deformation {
  const r = (deg * Math.PI) / 180;
  const cos = Math.cos(r);
  const sin = Math.sin(r);
  return transformAdjustment(d, (p) => {
    const dx = p.x - pivot.x;
    const dy = p.y - pivot.y;
    return { x: pivot.x + dx * cos - dy * sin, y: pivot.y + dx * sin + dy * cos };
  });
}

/**
 * Une grille de `columns × rows` cases qui garde la retouche actuelle :
 * chaque nouveau nœud est mis là où la retouche envoie son point au repos.
 * (Tendre une grille ne doit pas défaire un déplacement ou une rotation.)
 */
export function regrid(d: Deformation, columns: number, rows: number): Deformation {
  const fresh = identity(d.frame, columns, Math.max(rows, 1));
  return { ...fresh, nodes: fresh.nodes.map(([s, t]) => frameFractions(d.frame, deformPoint(d, framePoint(d.frame, s, t)))) };
}

/** Rectangle droit qui contient les points ; `null` sans point. */
export function boxOf(points: Point[]): Rect | null {
  if (points.length === 0) return null;
  const xs = points.map((p) => p.x);
  const ys = points.map((p) => p.y);
  const x = Math.min(...xs);
  const y = Math.min(...ys);
  return { x, y, width: Math.max(...xs) - x, height: Math.max(...ys) - y };
}

/** Un bloc à aligner : un dessin seul, ou un groupe, qui bouge d'un bloc. */
export interface SlideUnit {
  ids: string[];
  box: Rect;
}

/** Décalage (mm de diapositive) de chaque dessin pour aligner les blocs sur le cadre de la sélection. */
export function alignShifts(units: SlideUnit[], how: Alignment): Map<string, Point> {
  const out = new Map<string, Point>();
  const all = boxOf(units.flatMap((u) => [
    { x: u.box.x, y: u.box.y },
    { x: u.box.x + u.box.width, y: u.box.y + u.box.height },
  ]));
  if (!all || units.length < 2) return out;
  for (const { ids, box: b } of units) {
    let dx = 0;
    let dy = 0;
    if (how === "left") dx = all.x - b.x;
    if (how === "hcenter") dx = all.x + all.width / 2 - (b.x + b.width / 2);
    if (how === "right") dx = all.x + all.width - (b.x + b.width);
    if (how === "top") dy = all.y - b.y;
    if (how === "vcenter") dy = all.y + all.height / 2 - (b.y + b.height / 2);
    if (how === "bottom") dy = all.y + all.height - (b.y + b.height);
    for (const id of ids) out.set(id, { x: dx, y: dy });
  }
  return out;
}

/** Décalages pour un même écart entre blocs voisins, les deux extrêmes en place (trois blocs au moins). */
export function distributeShifts(units: SlideUnit[], axis: "horizontal" | "vertical"): Map<string, Point> {
  const out = new Map<string, Point>();
  if (units.length < 3) return out;
  const h = axis === "horizontal";
  const start = (r: Rect) => (h ? r.x : r.y);
  const size = (r: Rect) => (h ? r.width : r.height);
  const sorted = [...units].sort((a, b) => start(a.box) - start(b.box));
  const first = sorted[0].box;
  const last = sorted[sorted.length - 1].box;
  const total = sorted.reduce((sum, u) => sum + size(u.box), 0);
  const gap = (start(last) + size(last) - start(first) - total) / (sorted.length - 1);
  let cursor = start(first);
  for (const { ids, box } of sorted) {
    const delta = cursor - start(box);
    for (const id of ids) out.set(id, { x: h ? delta : 0, y: h ? 0 : delta });
    cursor += size(box) + gap;
  }
  return out;
}

/** Tous les nœuds décalés de (dx, dy) mm. */
export function shiftAdjustment(d: Deformation, dx: number, dy: number): Deformation {
  return transformAdjustment(d, (p) => ({ x: p.x + dx, y: p.y + dy }));
}

/** Un geste à la souris dans la diapositive ; `before` : retouche de chaque dessin choisi au début du geste. */
export type SlideGesture =
  | { kind: "move"; start: Point; before: Map<string, Deformation> }
  | { kind: "rotate"; start: Point; pivot: Point; before: Map<string, Deformation> }
  | { kind: "scale"; start: Point; frame: Rect; corner: number; before: Map<string, Deformation> }
  | { kind: "node"; start: Point; id: string; index: number; origin: Point; before: Map<string, Deformation> };

const angleOf = (p: Point, pivot: Point) => (Math.atan2(p.y - pivot.y, p.x - pivot.x) * 180) / Math.PI;

/**
 * Le geste au point `at` (mm de diapositive), comme sur la façade : ⇧
 * contraint le déplacement, arrondit la rotation à 5° et garde les
 * proportions sur un coin ; ⌥ étire depuis le centre.
 */
export function applySlideGesture(g: SlideGesture, at: Point, mods: { shift: boolean; alt: boolean }): Map<string, Deformation> {
  const out = new Map<string, Deformation>();
  if (g.kind === "node") {
    const d = g.before.get(g.id);
    if (d) out.set(g.id, moveNode(d, g.index, { x: g.origin.x + at.x - g.start.x, y: g.origin.y + at.y - g.start.y }));
    return out;
  }
  if (g.kind === "move") {
    let d = { x: at.x - g.start.x, y: at.y - g.start.y };
    if (mods.shift) d = constrainMove(d.x, d.y);
    for (const [id, before] of g.before) out.set(id, shiftAdjustment(before, d.x, d.y));
    return out;
  }
  if (g.kind === "rotate") {
    let deg = normalizeAngle(angleOf(at, g.pivot) - angleOf(g.start, g.pivot));
    if (mods.shift) deg = snapAngle(deg, ROTATE_SNAP_DEGREES);
    for (const [id, before] of g.before) out.set(id, turnAdjustment(before, deg, g.pivot));
    return out;
  }
  const handles = frameHandles(g.frame);
  const opposite = g.corner < 4 ? (g.corner + 2) % 4 : 4 + ((g.corner - 4 + 2) % 4);
  const anchor = mods.alt ? { x: g.frame.x + g.frame.width / 2, y: g.frame.y + g.frame.height / 2 } : handles[opposite];
  const ratio = (to: number, from: number) => (Math.abs(from) > 1e-9 ? to / from : 1);
  let sx = g.corner === 4 || g.corner === 6 ? 1 : ratio(at.x - anchor.x, g.start.x - anchor.x);
  let sy = g.corner === 5 || g.corner === 7 ? 1 : ratio(at.y - anchor.y, g.start.y - anchor.y);
  if (g.corner < 4 && mods.shift) {
    const from = Math.hypot(g.start.x - anchor.x, g.start.y - anchor.y);
    const to = Math.hypot(at.x - anchor.x, at.y - anchor.y);
    sx = sy = from > 0 ? to / from : 1;
  }
  sx = Math.max(MIN_STRETCH, sx);
  sy = Math.max(MIN_STRETCH, sy);
  for (const [id, before] of g.before) out.set(id, scaleAdjustment(before, sx, sy, anchor));
  return out;
}

/** Décalage d'une copie faite par ⌘D dans la diapositive, en mm de diapositive. */
export const SLIDE_COPY_OFFSET_MM = 5;

/**
 * Copies des dessins `retouches` (identifiant → retouche de la copie, en mm
 * de diapositive), comme ⌘D ou ⌥ + glisser sur la façade (demande de
 * Benjamin, 13 novembre). La copie garde la place de l'original sur la
 * façade, est affectée à la main à cette diapositive, et porte la retouche
 * donnée : elle n'apparaît donc qu'ici, décalée. Chaque copie est rangée
 * juste devant son original ; un groupe copié devient un nouveau groupe.
 */
export function slideCopies(
  placements: Placement[],
  retouches: Map<string, Deformation>,
  projector: string,
  makeId: () => string = newId,
): { placements: Placement[]; ids: string[] } {
  const sources = placements.filter((p) => retouches.has(p.id));
  const copies = copyOf(sources, { x: 0, y: 0 }, makeId).map((c, k) => ({
    ...c,
    projector,
    adjustment: { projector, deformation: retouches.get(sources[k].id)! },
  }));
  const bySource = new Map(sources.map((p, k) => [p.id, copies[k]]));
  const out = placements.flatMap((p) => (bySource.has(p.id) ? [p, bySource.get(p.id)!] : [p]));
  return { placements: out, ids: copies.map((c) => c.id) };
}

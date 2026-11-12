// Gestes à la souris sur les dessins posés (tâche 4.2), à la manière
// d'Illustrator : glisser un dessin le déplace (⌥ : une copie, ⇧ : à
// l'horizontale, à la verticale ou à 45°), une poignée de coin l'étire en x
// et en y (⇧ : en gardant les proportions), une poignée de côté seulement en
// x ou en y (⌥ : depuis le centre, D-4.2-l), juste au-delà d'un coin on le tourne (⇧ : par pas
// de 45°), glisser dans le vide trace un cadre de sélection, un nœud d'un
// dessin déformé se tire (tâche 4.4). ⌘D répète le dernier geste. Calculs
// purs, testés sans écran (`gesture.test.ts`).

import type { Deformation } from "./deformation";
import {
  bounds,
  constrainMove,
  duplicate,
  inMarquee,
  moveSharedNode,
  rectCenter,
  rotate,
  scale,
  snapAngle,
  stretch,
  translate,
  type Placement,
  type Point,
  type Rect,
} from "./placement";

/** Rayon de saisie d'une poignée de coin, en pixels d'écran. */
export const HANDLE_PX = 7;
/** Au-delà de la poignée et jusqu'à cette distance : zone de rotation. */
export const ROTATE_PX = 24;

/** Cadre de la sélection : coins haut-gauche, haut-droit, bas-droit, bas-gauche. */
export function frameCorners(r: Rect): [Point, Point, Point, Point] {
  return [
    { x: r.x, y: r.y },
    { x: r.x + r.width, y: r.y },
    { x: r.x + r.width, y: r.y + r.height },
    { x: r.x, y: r.y + r.height },
  ];
}

/**
 * Les huit poignées du cadre : les quatre coins (0 à 3, comme
 * `frameCorners`), puis le milieu des côtés haut, droit, bas, gauche (4 à 7).
 */
export function frameHandles(r: Rect): Point[] {
  const [a, b, c, d] = frameCorners(r);
  const mid = (p: Point, q: Point) => ({ x: (p.x + q.x) / 2, y: (p.y + q.y) / 2 });
  return [a, b, c, d, mid(a, b), mid(b, c), mid(c, d), mid(d, a)];
}

/** `corner` : la poignée saisie, de 0 à 7 (voir `frameHandles`) ; la rotation ne se prend qu'aux coins. */
export type Handle = { kind: "scale" | "rotate"; corner: number };

/**
 * Ce que le pointeur saisit sur le cadre (les huit poignées à l'écran, en
 * pixels, ou les quatre coins seuls) : une poignée d'étirement, la zone de
 * rotation autour d'un coin, ou rien. La rotation se prend hors du cadre
 * seulement.
 */
export function handleAt(screenHandles: Point[], pointer: Point): Handle | null {
  const screenCorners = screenHandles.slice(0, 4);
  for (let side = 4; side < screenHandles.length; side++) {
    const c = screenHandles[side];
    if (Math.hypot(pointer.x - c.x, pointer.y - c.y) <= HANDLE_PX) return { kind: "scale", corner: side };
  }
  const xs = screenCorners.map((c) => c.x);
  const ys = screenCorners.map((c) => c.y);
  const inside =
    pointer.x > Math.min(...xs) &&
    pointer.x < Math.max(...xs) &&
    pointer.y > Math.min(...ys) &&
    pointer.y < Math.max(...ys);
  let best: { corner: number; distance: number } | null = null;
  screenCorners.forEach((c, corner) => {
    const distance = Math.hypot(pointer.x - c.x, pointer.y - c.y);
    if (!best || distance < best.distance) best = { corner, distance };
  });
  if (!best) return null;
  const { corner, distance } = best as { corner: number; distance: number };
  if (distance <= HANDLE_PX) return { kind: "scale", corner };
  if (distance <= ROTATE_PX && !inside) return { kind: "rotate", corner };
  return null;
}

/**
 * Le nœud de déformation sous le pointeur (nœuds à l'écran, en pixels), ou
 * aucun ; le plus proche l'emporte (tâche 4.4).
 */
export function nodeAt(screenNodes: Point[], pointer: Point): number | null {
  let best: { index: number; distance: number } | null = null;
  screenNodes.forEach((c, index) => {
    const distance = Math.hypot(pointer.x - c.x, pointer.y - c.y);
    if (distance <= HANDLE_PX && (!best || distance < best.distance)) best = { index, distance };
  });
  return best ? (best as { index: number }).index : null;
}

/** Un geste en cours. `before` est la liste au début du geste. */
export type Gesture =
  | { kind: "move"; start: Point; before: Placement[]; ids: Set<string> }
  | { kind: "scale"; start: Point; before: Placement[]; ids: Set<string>; frame: Rect; corner: number }
  | { kind: "rotate"; start: Point; before: Placement[]; ids: Set<string>; pivot: Point }
  | { kind: "marquee"; start: Point; before: Placement[]; ids: Set<string> }
  | {
      kind: "node";
      start: Point;
      before: Placement[];
      ids: Set<string>;
      deformation: Deformation;
      index: number;
      /** Position du nœud au début du geste. */
      origin: Point;
    };

/** Ce qu'a fait un geste terminé, pour le répéter par ⌘D. */
export type Repeatable =
  | { kind: "move"; dx: number; dy: number; copy: boolean }
  | { kind: "rotate"; deg: number; copy: boolean }
  | { kind: "scale"; factor: number; copy: boolean }
  | { kind: "stretch"; sx: number; sy: number; copy: boolean };

export interface Modifiers {
  shift: boolean;
  alt: boolean;
}

export interface GestureStep {
  placements: Placement[];
  /** Sélection à montrer pendant et après le geste. */
  selection: Set<string>;
  /** Cadre de sélection tracé (geste « marquee »). */
  marquee?: Rect;
  /** Le geste tel qu'on pourra le répéter. */
  repeat?: Repeatable;
}

function angleOf(p: Point, pivot: Point): number {
  return (Math.atan2(p.y - pivot.y, p.x - pivot.x) * 180) / Math.PI;
}

/** Applique le geste au point `at` (mm de façade). */
export function applyGesture(g: Gesture, at: Point, mods: Modifiers, makeId?: () => string): GestureStep {
  if (g.kind === "marquee") {
    const rect = {
      x: Math.min(g.start.x, at.x),
      y: Math.min(g.start.y, at.y),
      width: Math.abs(at.x - g.start.x),
      height: Math.abs(at.y - g.start.y),
    };
    const hit = inMarquee(g.before, rect);
    // ⇧ ajoute à la sélection d'avant.
    const selection = mods.shift ? new Set([...g.ids, ...hit]) : hit;
    return { placements: g.before, selection, marquee: rect };
  }
  if (g.kind === "node") {
    // Le nœud suit le pointeur, sans saut à la prise.
    const to = { x: g.origin.x + at.x - g.start.x, y: g.origin.y + at.y - g.start.y };
    return { placements: moveSharedNode(g.before, g.ids, g.deformation, g.index, to), selection: g.ids };
  }
  if (g.kind === "move") {
    let d = { x: at.x - g.start.x, y: at.y - g.start.y };
    if (mods.shift) d = constrainMove(d.x, d.y);
    if (mods.alt) {
      const { placements, copies } = duplicate(g.before, g.ids, d, makeId);
      return { placements, selection: copies, repeat: { kind: "move", dx: d.x, dy: d.y, copy: true } };
    }
    return {
      placements: translate(g.before, g.ids, d.x, d.y),
      selection: g.ids,
      repeat: { kind: "move", dx: d.x, dy: d.y, copy: false },
    };
  }
  if (g.kind === "rotate") {
    let deg = angleOf(at, g.pivot) - angleOf(g.start, g.pivot);
    if (mods.shift) deg = snapAngle(deg);
    return {
      placements: rotate(g.before, g.ids, deg, g.pivot),
      selection: g.ids,
      repeat: { kind: "rotate", deg, copy: false },
    };
  }
  // Poignées : la poignée opposée reste en place, ou le centre avec ⌥.
  const handles = frameHandles(g.frame);
  const opposite = g.corner < 4 ? (g.corner + 2) % 4 : 4 + ((g.corner - 4 + 2) % 4);
  const anchor = mods.alt ? rectCenter(g.frame) : handles[opposite];
  if (g.corner < 4 && mods.shift) {
    // ⇧ sur un coin : proportions gardées.
    const from = Math.hypot(g.start.x - anchor.x, g.start.y - anchor.y);
    const to = Math.hypot(at.x - anchor.x, at.y - anchor.y);
    const factor = from > 0 ? to / from : 1;
    return {
      placements: scale(g.before, g.ids, factor, anchor),
      selection: g.ids,
      repeat: { kind: "scale", factor, copy: false },
    };
  }
  const ratio = (to: number, from: number) => (Math.abs(from) > 1e-9 ? to / from : 1);
  // Côtés haut et bas (4, 6) : en y seulement ; droit et gauche (5, 7) : en x seulement.
  const sx = g.corner === 4 || g.corner === 6 ? 1 : ratio(at.x - anchor.x, g.start.x - anchor.x);
  const sy = g.corner === 5 || g.corner === 7 ? 1 : ratio(at.y - anchor.y, g.start.y - anchor.y);
  return {
    placements: stretch(g.before, g.ids, sx, sy, anchor),
    selection: g.ids,
    repeat: { kind: "stretch", sx, sy, copy: false },
  };
}

/**
 * Répète un geste sur la sélection (⌘D) : même déplacement, même angle
 * autour du centre de la sélection, même facteur depuis ce centre ; une
 * copie si le geste en faisait une.
 */
export function repeatGesture(
  placements: Placement[],
  selection: Set<string>,
  r: Repeatable,
  makeId?: () => string,
): { placements: Placement[]; selection: Set<string> } {
  const frame = bounds(placements.filter((p) => selection.has(p.id)));
  if (!frame) return { placements, selection };
  let list = placements;
  let ids = selection;
  if (r.copy) {
    const copied = duplicate(list, ids, { x: 0, y: 0 }, makeId);
    list = copied.placements;
    ids = copied.copies;
  }
  const c = rectCenter(frame);
  if (r.kind === "move") list = translate(list, ids, r.dx, r.dy);
  if (r.kind === "rotate") list = rotate(list, ids, r.deg, c);
  if (r.kind === "scale") list = scale(list, ids, r.factor, c);
  if (r.kind === "stretch") list = stretch(list, ids, r.sx, r.sy, c);
  return { placements: list, selection: ids };
}

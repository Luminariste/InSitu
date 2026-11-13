// Gestes à la souris sur les dessins posés (tâche 4.2), à la manière
// d'Illustrator : glisser un dessin le déplace (⌥ : une copie, ⇧ : à
// l'horizontale, à la verticale ou à 45°), une poignée de coin l'étire en x
// et en y (⇧ : en gardant les proportions), une poignée de côté seulement en
// x ou en y (⌥ : depuis le centre, D-4.2-l), juste au-delà d'un coin on le tourne (⇧ : par pas
// de 5°), la poignée ronde au-dessus du cadre le tourne aussi (demande de
// Benjamin, D-4.2-m), glisser dans le vide trace un cadre de sélection, un nœud d'un
// dessin déformé se tire (tâche 4.4). ⌘D répète le dernier geste. Calculs
// purs, testés sans écran (`gesture.test.ts`).

import type { Deformation } from "./deformation";
import {
  bounds,
  constrainMove,
  duplicate,
  inMarquee,
  moveSharedNode,
  normalizeAngle,
  ROTATE_SNAP_DEGREES,
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
/** Distance entre le haut du cadre et la poignée ronde de rotation, en pixels d'écran (D-4.2-m). */
export const KNOB_PX = 28;
/** Numéro de la poignée ronde de rotation dans `Handle.corner`. */
export const KNOB = 8;

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

/**
 * Poignée ronde de rotation, à l'écran : au-dessus du milieu du côté haut
 * (`screenHandles` : les huit poignées à l'écran, voir `frameHandles`).
 */
export function knobOf(screenHandles: Point[]): Point {
  const top = screenHandles[4];
  return { x: top.x, y: top.y - KNOB_PX };
}

/**
 * `corner` : la poignée saisie, de 0 à 7 (voir `frameHandles`), ou `KNOB`
 * pour la poignée ronde ; la rotation se prend aux coins ou à la poignée ronde.
 */
export type Handle = { kind: "scale" | "rotate"; corner: number };

/**
 * Ce que le pointeur saisit sur le cadre (les huit poignées à l'écran, en
 * pixels, ou les quatre coins seuls) : une poignée d'étirement, la zone de
 * rotation autour d'un coin, ou rien. La rotation se prend hors du cadre
 * seulement.
 */
export function handleAt(screenHandles: Point[], pointer: Point): Handle | null {
  const screenCorners = screenHandles.slice(0, 4);
  if (screenHandles.length >= 8) {
    const knob = knobOf(screenHandles);
    if (Math.hypot(pointer.x - knob.x, pointer.y - knob.y) <= HANDLE_PX + 2) return { kind: "rotate", corner: KNOB };
  }
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
  | { kind: "rotate"; start: Point; before: Placement[]; ids: Set<string>; pivot: Point; corner?: number }
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
  /** Angle tourné depuis le début du geste, en degrés (geste « rotate »). */
  turned?: number;
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
    let deg = normalizeAngle(angleOf(at, g.pivot) - angleOf(g.start, g.pivot));
    if (mods.shift) {
      // ⇧ : un dessin seul s'arrête sur un angle rond (multiple de 5°) ;
      // plusieurs dessins tournent par pas de 5° (D-4.2-q).
      const only = g.ids.size === 1 ? g.before.find((p) => g.ids.has(p.id)) : undefined;
      const base = only?.rotation ?? 0;
      deg = normalizeAngle(snapAngle(base + deg, ROTATE_SNAP_DEGREES) - base);
    }
    return {
      placements: rotate(g.before, g.ids, deg, g.pivot),
      selection: g.ids,
      repeat: { kind: "rotate", deg, copy: false },
      turned: deg,
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

/** Direction d'une poignée de rotation vue du centre du cadre, en degrés (0 : vers le haut, sens horaire). */
const HANDLE_BEARING = [-45, 45, 135, -135, 0, 90, 180, -90, 0];

/**
 * Curseur de rotation : une flèche courbe à deux pointes, tournée vers la
 * poignée saisie, et qui suit la rotation pendant le geste (`turned`,
 * arrondi à 15° près). Dessiné en SVG : les curseurs du système n'en ont pas
 * (D-4.2-m).
 */
export function rotateCursor(corner: number, turned = 0): string {
  const angle = Math.round(((HANDLE_BEARING[corner] ?? 0) + turned) / 15) * 15;
  const arc = "M6 14 A8 8 0 0 1 18 14";
  const heads = "M3.4 17 L3.9 12.2 L8.1 15.9 Z M20.6 17 L20.1 12.2 L15.9 15.9 Z";
  const svg =
    `<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24'>` +
    `<g transform='rotate(${angle} 12 12)'>` +
    `<path d='${arc}' fill='none' stroke='white' stroke-width='4' stroke-linecap='round'/>` +
    `<path d='${heads}' fill='black' stroke='white' stroke-width='1.2' stroke-linejoin='round'/>` +
    `<path d='${arc}' fill='none' stroke='black' stroke-width='1.6'/>` +
    `</g></svg>`;
  return `url("data:image/svg+xml,${encodeURIComponent(svg)}") 12 12, alias`;
}

/** Curseur d'une poignée d'étirement : flèche double dans l'axe de la poignée. */
export function scaleCursor(corner: number): string {
  return (
    ["nwse-resize", "nesw-resize", "nwse-resize", "nesw-resize", "ns-resize", "ew-resize", "ns-resize", "ew-resize"][
      corner
    ] ?? "nwse-resize"
  );
}

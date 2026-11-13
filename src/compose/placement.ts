// Dessins posés sur la façade (tâche 4.2) : calculs purs des gestes.
//
// Ces fonctions ne touchent ni l'écran ni le moteur : elles prennent la liste
// des placements et rendent une nouvelle liste. L'écran l'affiche aussitôt et
// l'envoie au moteur à la fin du geste (`savePlacements`), qui la contrôle et
// l'enregistre. Même convention que le moteur (`Placement` dans
// `crates/insitu-core/src/project/model.rs`) : mm réels de façade, y vers le
// bas, rotation en degrés dans le sens des aiguilles d'une montre.

import {
  deformPoint,
  flipDeformation,
  identity,
  moveNode,
  rotateDeformation,
  sameDeformation,
  scaleDeformation,
  stretchDeformation,
  translateDeformation,
  type Deformation,
  type DeformationFrame,
} from "./deformation";

export interface Point {
  x: number;
  y: number;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Placement {
  id: string;
  /** Identifiant du dessin dans la bibliothèque. */
  drawing: string;
  center: Point;
  /** Taille du dessin posé, en mm, avant rotation. */
  width: number;
  height: number;
  rotation: number;
  /** Retourné de gauche à droite. */
  mirrored?: boolean;
  /** Groupe : les dessins d'un groupe se sélectionnent ensemble. */
  group?: string | null;
  /** Calque du dessin posé ; absent : le premier calque. */
  layer?: string | null;
  /** Déformation libre (tâche 4.4) ; absente : le dessin reste droit. */
  deformation?: Deformation | null;
  /** Diapositive choisie à la main (tâche 4.5) ; absente : celle que propose le moteur. */
  projector?: string | null;
  /** Retouche dans une diapositive (tâche 4.6), sans effet sur la façade. */
  adjustment?: SlideAdjustment | null;
  /** Masqué un à un (arborescence des diapositives) : ni affiché, ni exporté (D-4.5-g). */
  hidden?: boolean;
  /** Verrouillé un à un : affiché, exporté, mais plus choisi à la souris (D-4.5-g). */
  locked?: boolean;
}

/** Retouche d'un dessin dans la diapositive d'un projecteur : grille en mm de diapositive. */
export interface SlideAdjustment {
  projector: string;
  deformation: Deformation;
}

/** Calque de composition : l'ordre de la liste est l'ordre d'empilement (le premier dessous). */
export interface Layer {
  id: string;
  name: string;
  /** Faux : calque masqué (absent : visible). */
  visible?: boolean;
  /** Vrai : calque verrouillé, ses dessins ne se sélectionnent plus. */
  locked?: boolean;
}

/** Déplacement par les flèches : 10 mm, ou 100 mm avec Maj. */
export const NUDGE_MM = 10;
export const NUDGE_SHIFT_FACTOR = 10;
/** Décalage d'une copie collée par ⌘V ou dupliquée par ⌘D sans geste à répéter. */
export const PASTE_OFFSET_MM = 100;
/** Pas d'orientation d'un déplacement avec Maj. */
export const SNAP_DEGREES = 45;
/** Pas de la rotation à la souris avec Maj (demande de Benjamin, D-4.2-q). */
export const ROTATE_SNAP_DEGREES = 5;
/** Plus petite taille d'un dessin posé. */
export const MIN_SIZE_MM = 1;

/** Un identifiant neuf (16 caractères hexadécimaux, comme le moteur). */
export function newId(): string {
  let id = "";
  for (let i = 0; i < 16; i++) id += Math.floor(Math.random() * 16).toString(16);
  return id;
}

/** Ramène un angle entre -180 (exclu) et 180 degrés. */
export function normalizeAngle(deg: number): number {
  let a = deg % 360;
  if (a <= -180) a += 360;
  if (a > 180) a -= 360;
  return a === 0 ? 0 : a; // pas de -0
}

/** Point de la façade où tombe le point (u, v) du fichier, en fractions (déformation comprise). */
export function facadePoint(p: Placement, u: number, v: number): Point {
  const q = rigidPoint(p, u, v);
  return p.deformation ? deformPoint(p.deformation, q) : q;
}

/** Comme `facadePoint`, sans la déformation libre. */
export function rigidPoint(p: Placement, u: number, v: number): Point {
  let x = (u - 0.5) * p.width;
  const y = (v - 0.5) * p.height;
  if (p.mirrored) x = -x;
  const r = (p.rotation * Math.PI) / 180;
  const cos = Math.cos(r);
  const sin = Math.sin(r);
  return { x: p.center.x + x * cos - y * sin, y: p.center.y + x * sin + y * cos };
}

/** Coins du fichier sur la façade : haut-gauche, haut-droit, bas-droit, bas-gauche. */
export function corners(p: Placement): [Point, Point, Point, Point] {
  return [facadePoint(p, 0, 0), facadePoint(p, 1, 0), facadePoint(p, 1, 1), facadePoint(p, 0, 1)];
}

/** Points par côté du contour d'un dessin déformé. */
const OUTLINE_STEPS = 12;

/** Contour du dessin sur la façade (ses quatre coins, ou une courbe s'il est déformé). */
export function outline(p: Placement): Point[] {
  if (!p.deformation) return corners(p);
  const out: Point[] = [];
  const edges: [number, number, number, number][] = [
    [0, 0, 1, 0],
    [1, 0, 1, 1],
    [1, 1, 0, 1],
    [0, 1, 0, 0],
  ];
  for (const [u0, v0, u1, v1] of edges)
    for (let k = 0; k < OUTLINE_STEPS; k++) {
      const a = k / OUTLINE_STEPS;
      out.push(facadePoint(p, u0 + (u1 - u0) * a, v0 + (v1 - v0) * a));
    }
  return out;
}

/** Le point est-il dans le polygone ? (règle pair-impair) */
function insidePolygon(polygon: Point[], pt: Point): boolean {
  let inside = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const a = polygon[i];
    const b = polygon[j];
    if (a.y > pt.y !== b.y > pt.y && pt.x < ((b.x - a.x) * (pt.y - a.y)) / (b.y - a.y) + a.x) inside = !inside;
  }
  return inside;
}

/** Rectangle droit qui contient des points. */
export function boundsOfPoints(points: Point[]): Rect | null {
  if (points.length === 0) return null;
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const p of points) {
    x0 = Math.min(x0, p.x);
    y0 = Math.min(y0, p.y);
    x1 = Math.max(x1, p.x);
    y1 = Math.max(y1, p.y);
  }
  return { x: x0, y: y0, width: x1 - x0, height: y1 - y0 };
}

/** Rectangle droit qui contient des dessins posés. */
export function bounds(placements: Placement[]): Rect | null {
  return boundsOfPoints(placements.flatMap((p) => outline(p)));
}

export function rectCenter(r: Rect): Point {
  return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
}

/** Le point tombe-t-il sur le dessin posé (sur son rectangle) ? */
export function contains(p: Placement, pt: Point): boolean {
  if (p.deformation) return insidePolygon(outline(p), pt);
  const r = (-p.rotation * Math.PI) / 180;
  const dx = pt.x - p.center.x;
  const dy = pt.y - p.center.y;
  const x = dx * Math.cos(r) - dy * Math.sin(r);
  const y = dx * Math.sin(r) + dy * Math.cos(r);
  return Math.abs(x) <= p.width / 2 && Math.abs(y) <= p.height / 2;
}

/** Le dessin le plus en avant sous le point, ou null. */
export function hitTest(placements: Placement[], pt: Point): Placement | null {
  for (let i = placements.length - 1; i >= 0; i--) if (contains(placements[i], pt)) return placements[i];
  return null;
}

/** La sélection, étendue aux groupes de ses dessins. */
export function expandToGroups(placements: Placement[], ids: Iterable<string>): Set<string> {
  const wanted = new Set(ids);
  const groups = new Set(placements.filter((p) => wanted.has(p.id) && p.group).map((p) => p.group));
  return new Set(placements.filter((p) => wanted.has(p.id) || (p.group && groups.has(p.group))).map((p) => p.id));
}

/** Dessins qui touchent un rectangle de sélection (tracé à la souris), groupes compris. */
export function inMarquee(placements: Placement[], rect: Rect): Set<string> {
  const hit = placements.filter((p) => {
    const b = bounds([p]);
    return (
      b !== null &&
      b.x <= rect.x + rect.width &&
      b.x + b.width >= rect.x &&
      b.y <= rect.y + rect.height &&
      b.y + b.height >= rect.y
    );
  });
  return expandToGroups(
    placements,
    hit.map((p) => p.id),
  );
}

/** Unités d'alignement : un groupe compte pour un, un dessin seul aussi. */
function units(placements: Placement[], ids: Set<string>): Placement[][] {
  const out = new Map<string, Placement[]>();
  for (const p of placements) {
    if (!ids.has(p.id)) continue;
    const key = p.group ? `g:${p.group}` : `p:${p.id}`;
    const list = out.get(key) ?? [];
    list.push(p);
    out.set(key, list);
  }
  return [...out.values()];
}

function mapSelected(placements: Placement[], ids: Set<string>, f: (p: Placement) => Placement): Placement[] {
  return placements.map((p) => (ids.has(p.id) ? f(p) : p));
}

export function translate(placements: Placement[], ids: Set<string>, dx: number, dy: number): Placement[] {
  return mapSelected(placements, ids, (p) => ({
    ...p,
    center: { x: p.center.x + dx, y: p.center.y + dy },
    deformation: p.deformation && translateDeformation(p.deformation, dx, dy),
  }));
}

/** Tourne la sélection de `deg` degrés autour de `pivot`. */
export function rotate(placements: Placement[], ids: Set<string>, deg: number, pivot: Point): Placement[] {
  const r = (deg * Math.PI) / 180;
  const cos = Math.cos(r);
  const sin = Math.sin(r);
  return mapSelected(placements, ids, (p) => {
    const dx = p.center.x - pivot.x;
    const dy = p.center.y - pivot.y;
    return {
      ...p,
      center: { x: pivot.x + dx * cos - dy * sin, y: pivot.y + dx * sin + dy * cos },
      rotation: normalizeAngle(p.rotation + deg),
      deformation: p.deformation && rotateDeformation(p.deformation, deg, pivot),
    };
  });
}

/**
 * Agrandit ou réduit la sélection du facteur `factor` depuis `anchor`, en
 * gardant les proportions (D-4.2-c). Le facteur est limité pour qu'aucun
 * dessin ne descende sous `MIN_SIZE_MM`.
 */
export function scale(placements: Placement[], ids: Set<string>, factor: number, anchor: Point): Placement[] {
  const chosen = placements.filter((p) => ids.has(p.id));
  if (chosen.length === 0 || !Number.isFinite(factor)) return placements;
  const smallest = Math.min(...chosen.map((p) => Math.min(p.width, p.height)));
  const f = Math.max(factor, MIN_SIZE_MM / smallest);
  return mapSelected(placements, ids, (p) => ({
    ...p,
    center: { x: anchor.x + (p.center.x - anchor.x) * f, y: anchor.y + (p.center.y - anchor.y) * f },
    width: p.width * f,
    height: p.height * f,
    deformation: p.deformation && scaleDeformation(p.deformation, f, anchor),
  }));
}

/**
 * Étire la sélection de `sx` en x et `sy` en y (axes de la façade) depuis
 * `anchor`, comme les poignées du cadre d'Illustrator (D-4.2-l). Un dessin
 * droit (ou tourné d'un quart) change simplement de largeur et de hauteur ;
 * un dessin tourné ou déformé reçoit une déformation qui porte l'étirement
 * exact (sinon il faudrait le cisailler). Les facteurs sont limités pour
 * qu'aucun dessin ne descende sous `MIN_SIZE_MM`.
 */
export function stretch(placements: Placement[], ids: Set<string>, sx: number, sy: number, anchor: Point): Placement[] {
  const chosen = placements.filter((p) => ids.has(p.id));
  if (chosen.length === 0 || !Number.isFinite(sx) || !Number.isFinite(sy)) return placements;
  const box = bounds(chosen);
  if (!box) return placements;
  const fx = Math.max(sx, box.width > 0 ? MIN_SIZE_MM / box.width : 1);
  const fy = Math.max(sy, box.height > 0 ? MIN_SIZE_MM / box.height : 1);
  if (fx === 1 && fy === 1) return placements;
  const k = Math.sqrt(fx * fy);
  const shift = (c: Point, f: number, g: number) => ({
    x: anchor.x + (c.x - anchor.x) * f,
    y: anchor.y + (c.y - anchor.y) * g,
  });
  return mapSelected(placements, ids, (p) => {
    const quarter = Math.round(p.rotation / 90);
    const straight = Math.abs(p.rotation - quarter * 90) < 1e-9;
    if (!p.deformation && straight) {
      const swap = Math.abs(quarter) % 2 === 1;
      return {
        ...p,
        center: shift(p.center, fx, fy),
        width: p.width * (swap ? fy : fx),
        height: p.height * (swap ? fx : fy),
      };
    }
    const own =
      p.deformation ??
      identity({ center: { ...p.center }, width: p.width, height: p.height, rotation: p.rotation }, 1, 1);
    return {
      ...p,
      center: shift(p.center, k, k),
      width: p.width * k,
      height: p.height * k,
      deformation: stretchDeformation(own, fx, fy, k, anchor),
    };
  });
}

export type FlipAxis = "horizontal" | "vertical";

/**
 * Symétrie de la sélection autour du centre de son cadre : « horizontale »
 * échange la gauche et la droite, « verticale » le haut et le bas.
 */
export function flip(placements: Placement[], ids: Set<string>, axis: FlipAxis): Placement[] {
  const box = bounds(placements.filter((p) => ids.has(p.id)));
  if (!box) return placements;
  const c = rectCenter(box);
  return mapSelected(placements, ids, (p) =>
    axis === "horizontal"
      ? {
          ...p,
          center: { x: 2 * c.x - p.center.x, y: p.center.y },
          mirrored: !p.mirrored,
          rotation: normalizeAngle(-p.rotation),
          deformation: p.deformation && flipDeformation(p.deformation, "horizontal", c.x),
        }
      : {
          ...p,
          center: { x: p.center.x, y: 2 * c.y - p.center.y },
          mirrored: !p.mirrored,
          rotation: normalizeAngle(180 - p.rotation),
          deformation: p.deformation && flipDeformation(p.deformation, "vertical", c.y),
        },
  );
}

/**
 * Copie de la sélection, posée au premier plan avec un décalage ; chaque
 * groupe copié devient un nouveau groupe. Rend la liste et les identifiants
 * des copies.
 */
export function duplicate(
  placements: Placement[],
  ids: Set<string>,
  offset: Point,
  makeId: () => string = newId,
): { placements: Placement[]; copies: Set<string> } {
  const copies = copyOf(
    placements.filter((p) => ids.has(p.id)),
    offset,
    makeId,
  );
  return { placements: [...placements, ...copies], copies: new Set(copies.map((p) => p.id)) };
}

/** Copies indépendantes (nouveaux identifiants, nouveaux groupes) de dessins posés. */
export function copyOf(source: Placement[], offset: Point, makeId: () => string = newId): Placement[] {
  const groupIds = new Map<string, string>();
  return source.map((p) => {
    let group = p.group ?? null;
    if (group) {
      if (!groupIds.has(group)) groupIds.set(group, makeId());
      group = groupIds.get(group)!;
    }
    return {
      ...p,
      id: makeId(),
      group,
      center: { x: p.center.x + offset.x, y: p.center.y + offset.y },
      deformation: p.deformation && translateDeformation(p.deformation, offset.x, offset.y),
      // Une copie va où le moteur la propose, pas dans la diapositive choisie pour l'original.
      projector: null,
      // Ni sa retouche de diapositive (tâche 4.6), faite pour l'original à sa place.
      adjustment: null,
    };
  });
}

/** Choisit à la main la diapositive de la sélection ; `null` la rend à la proposition du moteur. */
export function assignTo(placements: Placement[], ids: Set<string>, projector: string | null): Placement[] {
  return mapSelected(placements, ids, (p) => ({ ...p, projector }));
}

export function remove(placements: Placement[], ids: Set<string>): Placement[] {
  return placements.filter((p) => !ids.has(p.id));
}

/** Regroupe la sélection (les groupes qu'elle contient sont fondus dans le nouveau). */
export function group(placements: Placement[], ids: Set<string>, makeId: () => string = newId): Placement[] {
  if (ids.size < 2) return placements;
  const id = makeId();
  return mapSelected(placements, ids, (p) => ({ ...p, group: id }));
}

export function ungroup(placements: Placement[], ids: Set<string>): Placement[] {
  return mapSelected(placements, ids, (p) => ({ ...p, group: null }));
}

export type Arrange = "front" | "forward" | "backward" | "back";

/** Ordre d'empilement : premier plan, en avant, en arrière, arrière-plan. */
export function arrange(placements: Placement[], ids: Set<string>, how: Arrange): Placement[] {
  const selected = placements.filter((p) => ids.has(p.id));
  const others = placements.filter((p) => !ids.has(p.id));
  if (selected.length === 0 || others.length === 0) return placements;
  if (how === "front") return [...others, ...selected];
  if (how === "back") return [...selected, ...others];
  const list = [...placements];
  if (how === "forward") {
    // Chaque dessin choisi passe devant le premier dessin non choisi qui le suit.
    for (let i = list.length - 2; i >= 0; i--) {
      if (ids.has(list[i].id) && !ids.has(list[i + 1].id)) [list[i], list[i + 1]] = [list[i + 1], list[i]];
    }
  } else {
    for (let i = 1; i < list.length; i++) {
      if (ids.has(list[i].id) && !ids.has(list[i - 1].id)) [list[i], list[i - 1]] = [list[i - 1], list[i]];
    }
  }
  return list;
}

export type Alignment = "left" | "hcenter" | "right" | "top" | "vcenter" | "bottom";

/**
 * Aligne les dessins choisis sur le cadre de la sélection, comme le panneau
 * Alignement d'Illustrator ; un groupe bouge d'un bloc.
 */
export function align(placements: Placement[], ids: Set<string>, how: Alignment): Placement[] {
  const blocks = units(placements, ids);
  const box = bounds(blocks.flat());
  if (!box || blocks.length < 2) return placements;
  let out = placements;
  for (const block of blocks) {
    const b = bounds(block)!;
    let dx = 0;
    let dy = 0;
    if (how === "left") dx = box.x - b.x;
    if (how === "hcenter") dx = box.x + box.width / 2 - (b.x + b.width / 2);
    if (how === "right") dx = box.x + box.width - (b.x + b.width);
    if (how === "top") dy = box.y - b.y;
    if (how === "vcenter") dy = box.y + box.height / 2 - (b.y + b.height / 2);
    if (how === "bottom") dy = box.y + box.height - (b.y + b.height);
    out = translate(out, new Set(block.map((p) => p.id)), dx, dy);
  }
  return out;
}

/**
 * Répartit l'espacement : même écart entre les dessins voisins, les deux
 * extrêmes restant en place (au moins trois blocs).
 */
export function distribute(placements: Placement[], ids: Set<string>, axis: "horizontal" | "vertical"): Placement[] {
  const blocks = units(placements, ids).map((block) => ({ block, box: bounds(block)! }));
  if (blocks.length < 3) return placements;
  const h = axis === "horizontal";
  const start = (r: Rect) => (h ? r.x : r.y);
  const size = (r: Rect) => (h ? r.width : r.height);
  blocks.sort((a, b) => start(a.box) - start(b.box));
  const first = blocks[0].box;
  const last = blocks[blocks.length - 1].box;
  const total = blocks.reduce((sum, b) => sum + size(b.box), 0);
  const gap = (start(last) + size(last) - start(first) - total) / (blocks.length - 1);
  let out = placements;
  let cursor = start(first);
  for (const { block, box } of blocks) {
    const delta = cursor - start(box);
    out = translate(out, new Set(block.map((p) => p.id)), h ? delta : 0, h ? 0 : delta);
    cursor += size(box) + gap;
  }
  return out;
}

/** Arrondit un angle au multiple de `step` le plus proche (Maj enfoncée), 45° par défaut. */
export function snapAngle(deg: number, step = SNAP_DEGREES): number {
  return Math.round(deg / step) * step;
}

/** Déplacement contraint (Maj) : horizontal, vertical ou à 45°. */
export function constrainMove(dx: number, dy: number): Point {
  const length = Math.hypot(dx, dy);
  if (length === 0) return { x: 0, y: 0 };
  const angle = (snapAngle((Math.atan2(dy, dx) * 180) / Math.PI) * Math.PI) / 180;
  const along = dx * Math.cos(angle) + dy * Math.sin(angle);
  return { x: clean(along * Math.cos(angle)), y: clean(along * Math.sin(angle)) };
}

function clean(v: number): number {
  return Math.abs(v) < 1e-9 ? 0 : v;
}

/**
 * Un dessin lâché sur la façade : centré au point de chute, large d'un quart
 * de la largeur visible, proportions du fichier (D-4.2-d).
 */
export function dropped(
  drawing: { id: string; width: number; height: number },
  at: Point,
  visibleWidthMm: number,
  makeId: () => string = newId,
): Placement {
  const width = Math.max(MIN_SIZE_MM, visibleWidthMm / 4);
  const aspect = drawing.width > 0 && drawing.height > 0 ? drawing.height / drawing.width : 1;
  return {
    id: makeId(),
    drawing: drawing.id,
    center: { ...at },
    width,
    height: width * aspect,
    rotation: 0,
    mirrored: false,
    group: null,
  };
}

// --- Déformation libre (tâche 4.4) --------------------------------------------

/**
 * Le cadre d'une nouvelle déformation, comme le moteur
 * (`placement::deformation_frame`) : le dessin lui-même s'il est seul,
 * sinon le rectangle droit qui contient tous les dessins (sans leur
 * déformation précédente).
 */
export function deformationFrameOf(chosen: Placement[]): DeformationFrame | null {
  if (chosen.length === 0) return null;
  if (chosen.length === 1) {
    const p = chosen[0];
    return { center: { ...p.center }, width: p.width, height: p.height, rotation: p.rotation };
  }
  const box = boundsOfPoints(
    chosen.flatMap((p) => [rigidPoint(p, 0, 0), rigidPoint(p, 1, 0), rigidPoint(p, 1, 1), rigidPoint(p, 0, 1)]),
  );
  if (!box) return null;
  return { center: rectCenter(box), width: box.width, height: box.height, rotation: 0 };
}

/**
 * Tend une grille au repos de `columns × rows` cases (1 × 1 : quatre coins)
 * sur la sélection, d'un seul bloc ; `columns` à 0 retire la déformation.
 */
export function deform(placements: Placement[], ids: Set<string>, columns: number, rows: number): Placement[] {
  const chosen = placements.filter((p) => ids.has(p.id));
  if (chosen.length === 0) return placements;
  if (columns === 0) return mapSelected(placements, ids, (p) => ({ ...p, deformation: null }));
  const frame = deformationFrameOf(chosen);
  if (!frame) return placements;
  const d = identity(frame, columns, Math.max(rows, 1));
  return mapSelected(placements, ids, (p) => ({ ...p, deformation: d }));
}

/**
 * La déformation que la sélection partage (un dessin déformé, ou un groupe
 * déformé d'un bloc) : ses nœuds se tirent à la souris. Aucune si les
 * dessins choisis n'ont pas tous la même.
 */
export function sharedDeformation(selected: Placement[]): Deformation | null {
  const first = selected[0]?.deformation;
  if (!first) return null;
  return selected.every((p) => sameDeformation(p.deformation, first)) ? first : null;
}

/** Déplace un nœud de la déformation `d` pour tous les dessins choisis qui la partagent. */
export function moveSharedNode(
  placements: Placement[],
  ids: Set<string>,
  d: Deformation,
  index: number,
  to: Point,
): Placement[] {
  const moved = moveNode(d, index, to);
  return mapSelected(placements, ids, (p) => (sameDeformation(p.deformation, d) ? { ...p, deformation: moved } : p));
}

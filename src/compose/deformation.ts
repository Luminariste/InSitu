// Déformation libre d'un dessin posé (tâche 4.4) : le même calcul que le
// moteur (`crates/insitu-core/src/placement/mesh.rs`), pour l'aperçu sur la
// façade et les poignées. Les cas de `deformation-cases.json`, calculés par
// le moteur, prouvent que l'aperçu tombe exactement où l'export tombera.
//
// Une grille de `columns × rows` cases est tendue sur un cadre (rectangle
// tourné de la façade) ; chaque nœud est donné en fractions du cadre. Une
// seule case : quatre coins en perspective. Plus : surface lisse de
// Catmull-Rom qui passe par chaque nœud.

import type { Point } from "./placement";

export interface DeformationFrame {
  center: Point;
  width: number;
  height: number;
  rotation: number;
}

export interface Deformation {
  frame: DeformationFrame;
  columns: number;
  rows: number;
  /** Nœuds en fractions du cadre, ligne par ligne. */
  nodes: [number, number][];
}

/** Grilles proposées dans l'inspecteur : quatre coins, puis grilles. */
export const MAX_DIVISIONS = 16;

export function framePoint(f: DeformationFrame, s: number, t: number): Point {
  const x = (s - 0.5) * f.width;
  const y = (t - 0.5) * f.height;
  const r = (f.rotation * Math.PI) / 180;
  const cos = Math.cos(r);
  const sin = Math.sin(r);
  return { x: f.center.x + x * cos - y * sin, y: f.center.y + x * sin + y * cos };
}

export function frameFractions(f: DeformationFrame, p: Point): [number, number] {
  const dx = p.x - f.center.x;
  const dy = p.y - f.center.y;
  const r = (f.rotation * Math.PI) / 180;
  const cos = Math.cos(r);
  const sin = Math.sin(r);
  return [(dx * cos + dy * sin) / f.width + 0.5, (-dx * sin + dy * cos) / f.height + 0.5];
}

export function identity(frame: DeformationFrame, columns: number, rows: number): Deformation {
  const nodes: [number, number][] = [];
  for (let j = 0; j <= rows; j++) for (let i = 0; i <= columns; i++) nodes.push([i / columns, j / rows]);
  return { frame, columns, rows, nodes };
}

/** Homographie du carré unité vers les quatre nœuds (ordre des lignes). */
function squareToQuad(n: [number, number][]): (s: number, t: number) => [number, number] {
  // Coins dans l'ordre du tour : (0,0), (1,0), (1,1), (0,1).
  const [x0, y0] = n[0];
  const [x1, y1] = n[1];
  const [x2, y2] = n[3];
  const [x3, y3] = n[2];
  const dx3 = x0 - x1 + x2 - x3;
  const dy3 = y0 - y1 + y2 - y3;
  let a: number, b: number, d: number, e: number, g: number, h: number;
  if (dx3 === 0 && dy3 === 0) {
    a = x1 - x0;
    b = x2 - x1;
    d = y1 - y0;
    e = y2 - y1;
    g = 0;
    h = 0;
  } else {
    const dx1 = x1 - x2;
    const dx2 = x3 - x2;
    const dy1 = y1 - y2;
    const dy2 = y3 - y2;
    const den = dx1 * dy2 - dx2 * dy1;
    g = (dx3 * dy2 - dx2 * dy3) / den;
    h = (dx1 * dy3 - dx3 * dy1) / den;
    a = x1 - x0 + g * x1;
    b = x3 - x0 + h * x3;
    d = y1 - y0 + g * y1;
    e = y3 - y0 + h * y3;
  }
  return (s, t) => {
    const w = g * s + h * t + 1;
    return [(a * s + b * t + x0) / w, (d * s + e * t + y0) / w];
  };
}

function weights(a: number): [number, number, number, number] {
  const a2 = a * a;
  const a3 = a2 * a;
  return [(-a3 + 2 * a2 - a) / 2, (3 * a3 - 5 * a2 + 2) / 2, (-3 * a3 + 4 * a2 + a) / 2, (a3 - a2) / 2];
}

/** La grille en fractions du cadre : (s, t) au repos → (s, t) déformé. */
export function meshPoint(d: Deformation, s: number, t: number): [number, number] {
  const c = d.columns;
  const r = d.rows;
  if (c === 1 && r === 1) return squareToQuad(d.nodes)(s, t);
  const node = (i: number, j: number): [number, number] => {
    const extend = (n: number, a: [number, number], b: [number, number], e: [number, number]): [number, number] =>
      n >= 2 ? [3 * a[0] - 3 * b[0] + e[0], 3 * a[1] - 3 * b[1] + e[1]] : [2 * a[0] - b[0], 2 * a[1] - b[1]];
    if (i < 0) return extend(c, node(0, j), node(1, j), node(Math.min(2, c), j));
    if (i > c) return extend(c, node(c, j), node(c - 1, j), node(Math.max(c - 2, 0), j));
    if (j < 0) return extend(r, node(i, 0), node(i, 1), node(i, Math.min(2, r)));
    if (j > r) return extend(r, node(i, r), node(i, r - 1), node(i, Math.max(r - 2, 0)));
    return d.nodes[j * (c + 1) + i];
  };
  const x = s * c;
  const y = t * r;
  const i = Math.min(Math.max(Math.floor(x), 0), c - 1);
  const j = Math.min(Math.max(Math.floor(y), 0), r - 1);
  const wx = weights(x - i);
  const wy = weights(y - j);
  let qx = 0;
  let qy = 0;
  for (let b = 0; b < 4; b++)
    for (let a = 0; a < 4; a++) {
      const n = node(i - 1 + a, j - 1 + b);
      const w = wx[a] * wy[b];
      qx += n[0] * w;
      qy += n[1] * w;
    }
  return [qx, qy];
}

/** Image par la déformation d'un point de la façade (avant déformation). */
export function deformPoint(d: Deformation, p: Point): Point {
  const [s, t] = frameFractions(d.frame, p);
  const [a, b] = meshPoint(d, s, t);
  return framePoint(d.frame, a, b);
}

/** Position des nœuds sur la façade, ligne par ligne. */
export function nodePoints(d: Deformation): Point[] {
  return d.nodes.map(([s, t]) => framePoint(d.frame, s, t));
}

/** Déplace le nœud `index` au point `to` de la façade. */
export function moveNode(d: Deformation, index: number, to: Point): Deformation {
  const nodes = d.nodes.slice();
  nodes[index] = frameFractions(d.frame, to);
  return { ...d, nodes };
}

// --- La déformation suit les gestes du dessin --------------------------------

export function translateDeformation(d: Deformation, dx: number, dy: number): Deformation {
  return { ...d, frame: { ...d.frame, center: { x: d.frame.center.x + dx, y: d.frame.center.y + dy } } };
}

export function rotateDeformation(d: Deformation, deg: number, pivot: Point): Deformation {
  const r = (deg * Math.PI) / 180;
  const cos = Math.cos(r);
  const sin = Math.sin(r);
  const dx = d.frame.center.x - pivot.x;
  const dy = d.frame.center.y - pivot.y;
  return {
    ...d,
    frame: {
      ...d.frame,
      center: { x: pivot.x + dx * cos - dy * sin, y: pivot.y + dx * sin + dy * cos },
      rotation: d.frame.rotation + deg,
    },
  };
}

/**
 * Étire la déformation de `sx` en x et `sy` en y depuis `anchor` (axes de la
 * façade). Le cadre est agrandi de `k` comme le dessin (même agrandissement
 * proportionnel des deux) ; ce qui n'est pas proportionnel passe dans les
 * nœuds, exactement : l'interpolation de la grille suit une transformation
 * affine.
 */
export function stretchDeformation(d: Deformation, sx: number, sy: number, k: number, anchor: Point): Deformation {
  const frame = scaleDeformation(d, k, anchor).frame;
  const nodes = d.nodes.map(([s, t]) => {
    const q = framePoint(d.frame, s, t);
    return frameFractions(frame, { x: anchor.x + (q.x - anchor.x) * sx, y: anchor.y + (q.y - anchor.y) * sy });
  });
  return { ...d, frame, nodes };
}

export function scaleDeformation(d: Deformation, factor: number, anchor: Point): Deformation {
  const f = d.frame;
  return {
    ...d,
    frame: {
      ...f,
      center: { x: anchor.x + (f.center.x - anchor.x) * factor, y: anchor.y + (f.center.y - anchor.y) * factor },
      width: f.width * factor,
      height: f.height * factor,
    },
  };
}

/**
 * Symétrie de la déformation autour de la verticale `x = axis` (horizontale)
 * ou de l'horizontale `y = axis` (verticale) : le cadre se retourne, et la
 * grille aussi, nœud pour nœud.
 */
export function flipDeformation(d: Deformation, axis: "horizontal" | "vertical", at: number): Deformation {
  const f = d.frame;
  const c = d.columns;
  const r = d.rows;
  const nodes = d.nodes.map((_, k) => {
    const i = k % (c + 1);
    const j = Math.floor(k / (c + 1));
    if (axis === "horizontal") {
      const [s, t] = d.nodes[j * (c + 1) + (c - i)];
      return [1 - s, t] as [number, number];
    }
    const [s, t] = d.nodes[(r - j) * (c + 1) + i];
    return [s, 1 - t] as [number, number];
  });
  const center =
    axis === "horizontal" ? { x: 2 * at - f.center.x, y: f.center.y } : { x: f.center.x, y: 2 * at - f.center.y };
  return { ...d, nodes, frame: { ...f, center, rotation: -f.rotation } };
}

/** Deux déformations identiques (même cadre, même grille) : un groupe déformé d'un bloc. */
export function sameDeformation(a: Deformation | null | undefined, b: Deformation | null | undefined): boolean {
  return !!a && !!b && JSON.stringify(a) === JSON.stringify(b);
}

/** Points par case sur les lignes de la grille dessinées à l'écran. */
const LINE_STEPS = 8;

/** Lignes de la grille déformée sur la façade (rangées puis colonnes), pour l'affichage. */
export function meshLines(d: Deformation): Point[][] {
  const at = (s: number, t: number) => {
    const [a, b] = meshPoint(d, s, t);
    return framePoint(d.frame, a, b);
  };
  const along = (n: number, f: (x: number) => Point) =>
    Array.from({ length: n * LINE_STEPS + 1 }, (_, k) => f(k / (n * LINE_STEPS)));
  const lines: Point[][] = [];
  for (let j = 0; j <= d.rows; j++) lines.push(along(d.columns, (s) => at(s, j / d.rows)));
  for (let i = 0; i <= d.columns; i++) lines.push(along(d.rows, (t) => at(i / d.columns, t)));
  return lines;
}

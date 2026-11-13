// Logique pure de la retouche de silhouette (tâche 4.7) : trouver un point
// ou un bord sous le pointeur, déplacer, ajouter, retirer un point. Les
// polygones sont en fractions du dessin ; les distances se mesurent en
// pixels du dessin (`width` × `height`), pour qu'un dessin allongé se
// retouche comme un carré. Testée sans écran (`silhouetteLogic.test.ts`).

import type { SilhouettePolygon } from "../engine";

/** Un point d'un contour : contour `polygon`, point `index`. */
export interface VertexRef {
  polygon: number;
  index: number;
}

/** Taille du dessin, pour mesurer les distances. */
export interface DrawingSize {
  width: number;
  height: number;
}

function distance(size: DrawingSize, a: [number, number], b: [number, number]): number {
  return Math.hypot((a[0] - b[0]) * size.width, (a[1] - b[1]) * size.height);
}

/** Le point le plus proche de `at`, à moins de `radius` pixels du dessin. */
export function vertexAt(
  polygons: SilhouettePolygon[],
  at: [number, number],
  size: DrawingSize,
  radius: number,
): VertexRef | null {
  let best: VertexRef | null = null;
  let bestDistance = radius;
  polygons.forEach((polygon, p) =>
    polygon.forEach((point, index) => {
      const d = distance(size, point, at);
      if (d <= bestDistance) {
        bestDistance = d;
        best = { polygon: p, index };
      }
    }),
  );
  return best;
}

/** Le bord le plus proche de `at`, à moins de `radius` pixels : le point s'insère après `index`. */
export function edgeAt(
  polygons: SilhouettePolygon[],
  at: [number, number],
  size: DrawingSize,
  radius: number,
): (VertexRef & { point: [number, number] }) | null {
  let best: (VertexRef & { point: [number, number] }) | null = null;
  let bestDistance = radius;
  polygons.forEach((polygon, p) =>
    polygon.forEach((a, index) => {
      const b = polygon[(index + 1) % polygon.length];
      const ax = a[0] * size.width;
      const ay = a[1] * size.height;
      const dx = b[0] * size.width - ax;
      const dy = b[1] * size.height - ay;
      const len2 = dx * dx + dy * dy;
      const px = at[0] * size.width;
      const py = at[1] * size.height;
      const t = len2 === 0 ? 0 : Math.min(1, Math.max(0, ((px - ax) * dx + (py - ay) * dy) / len2));
      const q: [number, number] = [(ax + t * dx) / size.width, (ay + t * dy) / size.height];
      const d = distance(size, q, at);
      if (d <= bestDistance) {
        bestDistance = d;
        best = { polygon: p, index, point: q };
      }
    }),
  );
  return best;
}

/** Les contours avec un point déplacé en `to` (gardé dans le dessin). */
export function moveVertex(polygons: SilhouettePolygon[], ref: VertexRef, to: [number, number]): SilhouettePolygon[] {
  const clamped: [number, number] = [Math.min(1, Math.max(0, to[0])), Math.min(1, Math.max(0, to[1]))];
  return polygons.map((polygon, p) =>
    p === ref.polygon ? polygon.map((point, i) => (i === ref.index ? clamped : point)) : polygon,
  );
}

/**
 * Le point décalé de `screen` points d'écran à cette échelle (points d'écran
 * par unité du dessin) : flèches du clavier, D15-p.
 */
export function nudgeVertex(
  polygons: SilhouettePolygon[],
  ref: VertexRef,
  screen: { x: number; y: number },
  scale: number,
  size: { width: number; height: number },
): SilhouettePolygon[] {
  const point = polygons[ref.polygon]?.[ref.index];
  if (!point || scale <= 0) return polygons;
  return moveVertex(polygons, ref, [point[0] + screen.x / (scale * size.width), point[1] + screen.y / (scale * size.height)]);
}

/** Les contours avec un point ajouté après `ref.index`. */
export function insertVertex(
  polygons: SilhouettePolygon[],
  ref: VertexRef,
  point: [number, number],
): { polygons: SilhouettePolygon[]; added: VertexRef } {
  return {
    polygons: polygons.map((polygon, p) =>
      p === ref.polygon ? [...polygon.slice(0, ref.index + 1), point, ...polygon.slice(ref.index + 1)] : polygon,
    ),
    added: { polygon: ref.polygon, index: ref.index + 1 },
  };
}

/** Les contours sans ce point ; un contour réduit à moins de trois points disparaît. */
export function removeVertex(polygons: SilhouettePolygon[], ref: VertexRef): SilhouettePolygon[] {
  return polygons.flatMap((polygon, p) => {
    if (p !== ref.polygon) return [polygon];
    const rest = polygon.filter((_, i) => i !== ref.index);
    return rest.length >= 3 ? [rest] : [];
  });
}

/** Un contour carré neuf, centré en `at` (côté : un cinquième du dessin). */
export function squareAt(at: [number, number]): SilhouettePolygon {
  const h = 0.1;
  const c = (v: number) => Math.min(1 - h, Math.max(h, v));
  const [x, y] = [c(at[0]), c(at[1])];
  return [
    [x - h, y - h],
    [x + h, y - h],
    [x + h, y + h],
    [x - h, y + h],
  ];
}

/** Ligne courte : « Détectée · 3 contours », « Retouchée à la main · 1 contour », « Aucune ». */
export function silhouetteLine(polygons: SilhouettePolygon[], manual: boolean): string {
  if (polygons.length === 0) return manual ? "Aucune : ce dessin ne cache rien" : "Aucune détectée (dessin vide ?)";
  const count = `${polygons.length} contour${polygons.length > 1 ? "s" : ""}`;
  return `${manual ? "Retouchée à la main" : "Détectée automatiquement"} · ${count}`;
}

/** Même contours, à l'arrondi près. */
export function samePolygons(a: SilhouettePolygon[], b: SilhouettePolygon[]): boolean {
  return (
    a.length === b.length &&
    a.every(
      (polygon, p) =>
        polygon.length === b[p].length &&
        polygon.every((point, i) => point[0] === b[p][i][0] && point[1] === b[p][i][1]),
    )
  );
}

/** Tracé SVG des contours, dans un cadre `width` × `height`. */
export function polygonsPath(polygons: SilhouettePolygon[], size: DrawingSize): string {
  return polygons
    .map(
      (polygon) =>
        polygon
          .map(([u, v], i) => `${i === 0 ? "M" : "L"}${(u * size.width).toFixed(2)} ${(v * size.height).toFixed(2)}`)
          .join("") + "Z",
    )
    .join("");
}

// Ce que l'écran de recalage dessine sur chacune des deux vues : les points
// communs numérotés (couleur selon leur écart), le point en attente de son
// double, les lignes communes (L1, L2…) et le trait en attente de son double,
// et, sur la façade, l'emprise du projecteur.
// Tailles en pixels d'écran, quel que soit le zoom.

import { Container, Graphics, Text } from "pixi.js";
import type { FrameContext, Point, Rect, SceneNode } from "../viewer";
import type { PairStatus } from "./logic";

export const STATUS_COLORS: Record<PairStatus, number> = {
  bon: 0x3ccf6e,
  moyen: 0xe8b04a,
  aberrant: 0xff4d4d,
  inconnu: 0x4fb3ff,
};
const PENDING = 0xffe14d;
const SELECTED = 0xffffff;
const FOOTPRINT = 0x4fb3ff;
const HANDLE = 0xffffff;
const LINE = 0xff7ad9;

export interface MarkersContent {
  points: { x: number; y: number; status: PairStatus }[];
  /** Indice du point sélectionné (à déplacer), ou rien. */
  selected: number | null;
  /** Point posé de ce côté, en attente de l'autre. */
  pending: Point | null;
  /** Contour de l'emprise du projecteur (façade seulement). */
  footprint: Point[] | null;
  /** Poignée qui déplace toute la photo, au centre de la diapositive (façade seulement). */
  handle?: Point | null;
  /** Lignes communes de ce côté (D-3.2-k), chacune par ses deux bouts. */
  lines?: [Point, Point][];
  /** Trait en cours de tracé, ou tracé de ce côté et en attente de l'autre. */
  draft?: [Point, Point] | null;
}

export const EMPTY_MARKERS: MarkersContent = { points: [], selected: null, pending: null, footprint: null, handle: null, lines: [], draft: null };

export class PairMarkersNode implements SceneNode {
  readonly container = new Container();
  private readonly graphics = new Graphics();
  private readonly labels: Text[] = [];
  private content: MarkersContent = EMPTY_MARKERS;
  private drawnZoom = 0;

  constructor(name: string) {
    this.container.label = `recalage:${name}`;
    this.container.addChild(this.graphics);
  }

  set(content: MarkersContent): void {
    this.content = content;
    this.drawnZoom = 0; // redessiner à la prochaine image
  }

  bounds(): Rect | null {
    return null; // les repères ne comptent pas dans « Tout voir »
  }

  finestDetail(): number | null {
    return null;
  }

  private label(i: number): Text {
    while (this.labels.length <= i) {
      const text = new Text({ text: "" });
      text.style = { fill: 0xffffff, fontSize: 13, fontWeight: "700", fontFamily: "-apple-system, sans-serif" };
      this.labels.push(text);
      this.container.addChild(text);
    }
    return this.labels[i];
  }

  update(frame: FrameContext): void {
    const zoom = frame.view.zoom;
    if (zoom === this.drawnZoom) return;
    this.drawnZoom = zoom;
    const { points, selected, pending, footprint, handle, lines = [], draft = null } = this.content;
    const px = 1 / zoom; // un pixel d'écran, en unités de la scène
    const g = this.graphics;
    g.clear();

    if (footprint && footprint.length > 1) {
      g.moveTo(footprint[0].x, footprint[0].y);
      for (const p of footprint.slice(1)) g.lineTo(p.x, p.y);
      g.stroke({ width: 1.5 * px, color: FOOTPRINT, alpha: 0.9 });
    }

    points.forEach((p, i) => {
      const color = STATUS_COLORS[p.status];
      const active = i === selected;
      const r = (active ? 10 : 8) * px;
      // Croix fine pour viser juste, cercle pour la voir de loin.
      g.moveTo(p.x - 1.8 * r, p.y).lineTo(p.x + 1.8 * r, p.y).stroke({ width: 1 * px, color });
      g.moveTo(p.x, p.y - 1.8 * r).lineTo(p.x, p.y + 1.8 * r).stroke({ width: 1 * px, color });
      g.circle(p.x, p.y, r).stroke({ width: (active ? 3 : 2) * px, color: active ? SELECTED : color });
      const label = this.label(i);
      label.text = String(i + 1);
      label.style.fill = active ? SELECTED : color;
      label.visible = true;
      label.scale.set(px);
      label.position.set(p.x + r + 3 * px, p.y + r - 2 * px);
    });

    lines.forEach(([a, b], j) => {
      g.moveTo(a.x, a.y).lineTo(b.x, b.y).stroke({ width: 2 * px, color: LINE });
      for (const end of [a, b]) g.circle(end.x, end.y, 5 * px).stroke({ width: 2 * px, color: LINE });
      const label = this.label(points.length + j);
      label.text = `L${j + 1}`;
      label.style.fill = LINE;
      label.visible = true;
      label.scale.set(px);
      label.position.set((a.x + b.x) / 2 + 6 * px, (a.y + b.y) / 2 + 4 * px);
    });
    for (let i = points.length + lines.length; i < this.labels.length; i++) this.labels[i].visible = false;

    if (draft) {
      const [a, b] = draft;
      g.moveTo(a.x, a.y).lineTo(b.x, b.y).stroke({ width: 2 * px, color: PENDING });
      for (const end of [a, b]) g.circle(end.x, end.y, 5 * px).stroke({ width: 2 * px, color: PENDING });
    }

    if (handle) {
      // Poignée de déplacement : un disque et quatre flèches.
      const r = 12 * px;
      g.circle(handle.x, handle.y, r).fill({ color: 0x000000, alpha: 0.45 }).stroke({ width: 2 * px, color: HANDLE });
      const a = 7 * px;
      const h = 3 * px;
      for (const [dx, dy] of [
        [1, 0],
        [-1, 0],
        [0, 1],
        [0, -1],
      ]) {
        const tip = { x: handle.x + dx * a, y: handle.y + dy * a };
        g.moveTo(handle.x, handle.y).lineTo(tip.x, tip.y).stroke({ width: 1.5 * px, color: HANDLE });
        g.moveTo(tip.x - dy * h - dx * h, tip.y - dx * h - dy * h)
          .lineTo(tip.x, tip.y)
          .lineTo(tip.x + dy * h - dx * h, tip.y + dx * h - dy * h)
          .stroke({ width: 1.5 * px, color: HANDLE });
      }
    }

    if (pending) {
      const r = 9 * px;
      g.moveTo(pending.x - 2 * r, pending.y).lineTo(pending.x + 2 * r, pending.y).stroke({ width: 1 * px, color: PENDING });
      g.moveTo(pending.x, pending.y - 2 * r).lineTo(pending.x, pending.y + 2 * r).stroke({ width: 1 * px, color: PENDING });
      g.circle(pending.x, pending.y, r).stroke({ width: 2 * px, color: PENDING });
    }
  }

  destroy(): void {
    this.container.destroy({ children: true });
  }
}

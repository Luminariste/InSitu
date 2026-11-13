// Repères du calage d'un plan par deux points (D-PL-e) : chaque point pris
// sur le plan à caler (cyan, « 1 », « 2 ») et l'endroit où il doit venir
// (rouge, « 1′ », « 2′ »), reliés par une flèche. Même taille à l'écran à
// tout zoom.

import { Container, Graphics, Text } from "pixi.js";
import type { FrameContext, Point, Rect, SceneNode } from "../../viewer";

const CYAN = 0x2ec5e8;
const RED = 0xed1c24;
const NAMES = ["1", "1′", "2", "2′"];

export class AlignMarkersNode implements SceneNode {
  readonly container = new Container();
  private readonly lines = new Graphics();
  private readonly labels = NAMES.map((text) => new Text({ text }));
  /** Dans l'ordre du calage : 1, 1′, 2, 2′ (mm). */
  private points: Point[] = [];
  private drawnZoom = 0;

  constructor() {
    this.container.label = "facade:align-markers";
    this.container.addChild(this.lines);
    this.labels.forEach((label, i) => {
      label.style = {
        fill: i % 2 === 0 ? CYAN : RED,
        fontSize: 14,
        fontWeight: "600",
        fontFamily: "-apple-system, sans-serif",
      };
      label.visible = false;
      this.container.addChild(label);
    });
  }

  setPoints(points: Point[]): void {
    this.points = points.slice(0, 4);
    this.drawnZoom = 0;
  }

  bounds(): Rect | null {
    return null;
  }

  finestDetail(): number | null {
    return null;
  }

  update(frame: FrameContext): void {
    const zoom = frame.view.zoom;
    if (zoom === this.drawnZoom) return;
    this.drawnZoom = zoom;
    const px = 1 / zoom;
    const g = this.lines;
    g.clear();
    for (let i = 0; i + 1 < this.points.length; i += 2) {
      const [a, b] = [this.points[i], this.points[i + 1]];
      g.moveTo(a.x, a.y).lineTo(b.x, b.y).stroke({ width: 1.5 * px, color: RED, alpha: 0.8 });
    }
    this.points.forEach((p, i) => {
      const color = i % 2 === 0 ? CYAN : RED;
      g.moveTo(p.x - 12 * px, p.y).lineTo(p.x + 12 * px, p.y).stroke({ width: 1 * px, color });
      g.moveTo(p.x, p.y - 12 * px).lineTo(p.x, p.y + 12 * px).stroke({ width: 1 * px, color });
      g.circle(p.x, p.y, 7 * px).stroke({ width: 2 * px, color });
      const label = this.labels[i];
      label.visible = true;
      label.scale.set(px);
      label.position.set(p.x + 9 * px, p.y - 24 * px);
    });
    for (let i = this.points.length; i < this.labels.length; i++) this.labels[i].visible = false;
  }

  destroy(): void {
    this.container.destroy({ children: true });
  }
}

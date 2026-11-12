// Repères de mise à l'échelle posés sur la visionneuse : les points A et B
// et le trait qui les joint. Ils gardent la même taille à l'écran quel que
// soit le zoom.

import { Container, Graphics, Text } from "pixi.js";
import type { FrameContext, Point, Rect, SceneNode } from "../viewer";

const RED = 0xed1c24; // le rouge du cercle de la mire

export class ScaleMarkersNode implements SceneNode {
  readonly container = new Container();
  private readonly lines = new Graphics();
  private readonly labels = [new Text({ text: "A" }), new Text({ text: "B" })];
  private points: Point[] = [];
  private drawnZoom = 0;

  constructor() {
    this.container.label = "facade:scale-markers";
    this.container.addChild(this.lines);
    for (const label of this.labels) {
      label.style = { fill: RED, fontSize: 14, fontWeight: "600", fontFamily: "-apple-system, sans-serif" };
      label.visible = false;
      this.container.addChild(label);
    }
  }

  /** Points en coordonnées de la visionneuse (mm, ou unité du plan avant l'échelle). */
  setPoints(points: Point[]): void {
    this.points = points.slice(0, 2);
    this.drawnZoom = 0; // redessiner à la prochaine image
  }

  bounds(): Rect | null {
    return null; // les repères ne comptent pas dans « Tout voir »
  }

  finestDetail(): number | null {
    return null;
  }

  update(frame: FrameContext): void {
    const zoom = frame.view.zoom;
    if (zoom === this.drawnZoom) return;
    this.drawnZoom = zoom;
    const px = 1 / zoom; // un pixel d'écran, en unités de la scène
    const g = this.lines;
    g.clear();
    if (this.points.length === 2) {
      const [a, b] = this.points;
      g.moveTo(a.x, a.y).lineTo(b.x, b.y).stroke({ width: 1.5 * px, color: RED });
    }
    this.points.forEach((p, i) => {
      // Croix fine pour viser juste, cercle pour la voir de loin.
      g.moveTo(p.x - 12 * px, p.y).lineTo(p.x + 12 * px, p.y).stroke({ width: 1 * px, color: RED });
      g.moveTo(p.x, p.y - 12 * px).lineTo(p.x, p.y + 12 * px).stroke({ width: 1 * px, color: RED });
      g.circle(p.x, p.y, 7 * px).stroke({ width: 2 * px, color: RED });
      const label = this.labels[i];
      label.visible = true;
      label.scale.set(px);
      label.position.set(p.x + 9 * px, p.y - 24 * px);
    });
    for (let i = this.points.length; i < 2; i++) this.labels[i].visible = false;
  }

  destroy(): void {
    this.container.destroy({ children: true });
  }
}

// Ce que l'écran de redressement dessine par-dessus la photo : les
// croisements retenus (couleur selon leur écart), le contour de la mire,
// et les quatre poignées de coins.
// Tailles en pixels d'écran, quel que soit le zoom.

import { Container, Graphics } from "pixi.js";
import type { FrameContext, Point, Rect, SceneNode } from "../viewer";
import { orderCorners, type NodeQuality } from "./logic";

const COLORS: Record<NodeQuality, number> = { bon: 0x3ccf6e, moyen: 0xe8b04a, mauvais: 0xff4d4d };
const RED = 0xed1c24; // le rouge du cercle de la mire
const OUTLINE = 0x4fb3ff;

export interface OverlayContent {
  /** Croisements, en coordonnées de la scène. */
  nodes: { x: number; y: number; quality: NodeQuality }[];
  /** Contour de la mire (fermé), ou rien. */
  outline: Point[] | null;
  /** Poignées des quatre coins, ou rien. */
  handles: Point[] | null;
  /** Poignée en cours de déplacement. */
  activeHandle: number | null;
}

const EMPTY: OverlayContent = { nodes: [], outline: null, handles: null, activeHandle: null };

export class RectificationOverlay implements SceneNode {
  readonly container = new Container();
  private readonly graphics = new Graphics();
  private content: OverlayContent = EMPTY;
  private drawnZoom = 0;

  constructor() {
    this.container.label = "redressement:repères";
    this.container.addChild(this.graphics);
  }

  set(content: OverlayContent): void {
    this.content = content;
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
    const { nodes, outline, handles, activeHandle } = this.content;
    const px = 1 / zoom; // un pixel d'écran, en unités de la scène
    const g = this.graphics;
    g.clear();

    if (outline && outline.length > 1) {
      g.moveTo(outline[0].x, outline[0].y);
      for (const p of outline.slice(1)) g.lineTo(p.x, p.y);
      g.stroke({ width: 1.5 * px, color: OUTLINE, alpha: 0.9 });
    }

    // Les croisements, regroupés par couleur : trois tracés seulement.
    const radius = Math.max(1.6, Math.min(4, 0.25 * zoom)) * px;
    for (const quality of ["bon", "moyen", "mauvais"] as NodeQuality[]) {
      let any = false;
      for (const n of nodes) {
        if (n.quality !== quality) continue;
        g.circle(n.x, n.y, radius);
        any = true;
      }
      if (any) g.fill({ color: COLORS[quality], alpha: 0.85 });
    }

    if (handles) {
      // Les poignées se posent dans n'importe quel ordre : le contour les relie
      // dans le sens où le moteur les rangera.
      const quad = handles.length === 4 ? orderCorners(handles) : handles;
      g.moveTo(quad[0].x, quad[0].y);
      for (const p of [...quad.slice(1), quad[0]]) g.lineTo(p.x, p.y);
      g.stroke({ width: 1 * px, color: RED, alpha: 0.7 });
      handles.forEach((p, i) => {
        const active = i === activeHandle;
        const r = (active ? 11 : 9) * px;
        // Croix fine pour viser juste, cercle pour la saisir.
        g.moveTo(p.x - 2 * r, p.y).lineTo(p.x + 2 * r, p.y).stroke({ width: 1 * px, color: RED });
        g.moveTo(p.x, p.y - 2 * r).lineTo(p.x, p.y + 2 * r).stroke({ width: 1 * px, color: RED });
        g.circle(p.x, p.y, r).stroke({ width: (active ? 3 : 2) * px, color: RED });
      });
    }
  }

  destroy(): void {
    this.container.destroy({ children: true });
  }
}

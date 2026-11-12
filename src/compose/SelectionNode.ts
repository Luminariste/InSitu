// Sélection des dessins posés, dessinée sur la visionneuse : le contour de
// chaque dessin choisi, le cadre de la sélection avec ses quatre poignées,
// et le cadre de sélection tracé à la souris. Mêmes épaisseurs à l'écran
// quel que soit le zoom.

import { Container, Graphics } from "pixi.js";
import type { FrameContext, Rect, SceneNode } from "../viewer";
import { HANDLE_PX, frameCorners } from "./gesture";
import type { Point } from "./placement";

/** Bleu de sélection du Mac (`--ui-accent`, thème sombre). */
const ACCENT = 0x0a84ff;

export interface SelectionDrawing {
  /** Contours des dessins choisis (quatre coins chacun). */
  outlines: Point[][];
  /** Cadre de la sélection, avec poignées ; absent si rien n'est choisi. */
  frame: Rect | null;
  /** Cadre tracé à la souris. */
  marquee: Rect | null;
}

export class SelectionNode implements SceneNode {
  readonly container = new Container();
  private readonly graphics = new Graphics();
  private drawing: SelectionDrawing = { outlines: [], frame: null, marquee: null };
  private drawnZoom = 0;

  constructor() {
    this.container.label = "composition:selection";
    this.container.addChild(this.graphics);
  }

  set(drawing: SelectionDrawing): void {
    this.drawing = drawing;
    this.drawnZoom = 0;
  }

  bounds(): Rect | null {
    return null; // ne compte pas dans « Tout voir »
  }

  finestDetail(): number | null {
    return null;
  }

  update(frame: FrameContext): void {
    const zoom = frame.view.zoom;
    if (zoom === this.drawnZoom) return;
    this.drawnZoom = zoom;
    const px = 1 / zoom;
    const g = this.graphics;
    g.clear();
    const { outlines, frame: box, marquee } = this.drawing;
    for (const outline of outlines) {
      g.poly(
        outline.flatMap((p) => [p.x, p.y]),
        true,
      ).stroke({ width: 1 * px, color: ACCENT });
    }
    if (box) {
      g.rect(box.x, box.y, box.width, box.height).stroke({ width: 1 * px, color: ACCENT, alpha: 0.8 });
      const h = HANDLE_PX * 0.8 * px;
      for (const c of frameCorners(box)) {
        g.rect(c.x - h, c.y - h, 2 * h, 2 * h)
          .fill({ color: 0xffffff })
          .stroke({ width: 1 * px, color: ACCENT });
      }
    }
    if (marquee) {
      g.rect(marquee.x, marquee.y, marquee.width, marquee.height)
        .fill({ color: ACCENT, alpha: 0.08 })
        .stroke({ width: 1 * px, color: ACCENT });
    }
  }

  destroy(): void {
    this.container.destroy({ children: true });
  }
}

// Sélection des dessins posés, dessinée sur la visionneuse : le contour de
// chaque dessin dans la couleur de sa diapositive (tâche 4.5), celui de
// chaque dessin choisi, le cadre de la sélection avec ses huit poignées et
// sa poignée ronde de rotation (D-4.2-m),
// la grille de déformation et ses nœuds (tâche 4.4), et le cadre de
// sélection tracé à la souris. En rouge par-dessus, ce que les
// diapositives ne projetteront pas (D-4.5-h). Mêmes épaisseurs à l'écran
// quel que soit le zoom.

import { Container, Graphics } from "pixi.js";
import type { FrameContext, Rect, SceneNode } from "../viewer";
import { HANDLE_PX, KNOB_PX, frameHandles } from "./gesture";
import type { Point } from "./placement";

/** Bleu de sélection du Mac (`--ui-accent`, thème sombre). */
const ACCENT = 0x0a84ff;

/** Rouge d'alerte du Mac : dessin à cheval, part non projetée. */
const WARNING = 0xff453a;

/** Contour d'un dessin posé dans la couleur de sa diapositive (tâche 4.5). */
export interface SlideOutline {
  outline: Point[];
  /** Couleur du projecteur ; absente : le dessin ne tombe dans aucune emprise. */
  color: number | null;
  /** Faux : le dessin dépasse de sa diapositive. */
  whole: boolean;
}

export interface SelectionDrawing {
  /** Tous les dessins visibles, chacun dans la couleur de sa diapositive. */
  slides: SlideOutline[];
  /**
   * Parts de dessins qu'aucune diapositive ne projettera (D-4.5-h) : pour
   * chaque morceau, le contour extérieur puis ses trous, en mm de façade.
   */
  lost: Point[][][];
  /** Contours des dessins choisis (quatre coins chacun). */
  outlines: Point[][];
  /** Cadre de la sélection, avec poignées ; absent si rien n'est choisi. */
  frame: Rect | null;
  /** Cadre tracé à la souris. */
  marquee: Rect | null;
  /** Grille de la déformation partagée par la sélection, avec ses nœuds (tâche 4.4). */
  mesh: { lines: Point[][]; nodes: Point[]; active?: number | null } | null;
}

export class SelectionNode implements SceneNode {
  readonly container = new Container();
  private readonly graphics = new Graphics();
  private drawing: SelectionDrawing = { slides: [], lost: [], outlines: [], frame: null, marquee: null, mesh: null };
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
    const { slides, lost, outlines, frame: box, marquee, mesh } = this.drawing;
    for (const [outer, ...holes] of lost) {
      if (!outer || outer.length < 3) continue;
      g.poly(outer.flatMap((p) => [p.x, p.y]), true);
      g.fill({ color: WARNING, alpha: 0.45 });
      for (const hole of holes) {
        if (hole.length < 3) continue;
        g.poly(hole.flatMap((p) => [p.x, p.y]), true);
        g.cut();
      }
      g.poly(outer.flatMap((p) => [p.x, p.y]), true).stroke({ width: 1.5 * px, color: WARNING, alpha: 0.95 });
    }
    for (const s of slides) {
      g.poly(
        s.outline.flatMap((p) => [p.x, p.y]),
        true,
      ).stroke({
        width: (s.whole ? 1.5 : 2.5) * px,
        color: s.whole ? (s.color ?? 0x8e8e93) : WARNING,
        alpha: 0.9,
      });
    }
    for (const outline of outlines) {
      g.poly(
        outline.flatMap((p) => [p.x, p.y]),
        true,
      ).stroke({ width: 1 * px, color: ACCENT });
    }
    if (box) {
      g.rect(box.x, box.y, box.width, box.height).stroke({ width: 1 * px, color: ACCENT, alpha: 0.8 });
      // Poignée ronde de rotation, reliée au milieu du côté haut.
      const top = { x: box.x + box.width / 2, y: box.y };
      const knob = { x: top.x, y: top.y - KNOB_PX * px };
      g.moveTo(top.x, top.y).lineTo(knob.x, knob.y).stroke({ width: 1 * px, color: ACCENT, alpha: 0.8 });
      g.circle(knob.x, knob.y, HANDLE_PX * 0.75 * px)
        .fill({ color: 0xffffff })
        .stroke({ width: 1.5 * px, color: ACCENT });
      const h = HANDLE_PX * 0.8 * px;
      for (const c of frameHandles(box)) {
        g.rect(c.x - h, c.y - h, 2 * h, 2 * h)
          .fill({ color: 0xffffff })
          .stroke({ width: 1 * px, color: ACCENT });
      }
    }
    if (mesh) {
      // Grille en trait fin, nœuds ronds (les poignées du cadre sont carrées).
      for (const line of mesh.lines)
        g.poly(
          line.flatMap((p) => [p.x, p.y]),
          false,
        ).stroke({ width: 1 * px, color: ACCENT, alpha: 0.6 });
      const r = HANDLE_PX * 0.7 * px;
      mesh.nodes.forEach((n, i) => {
        // Le point choisi (que les flèches déplacent) : plus gros, cœur blanc.
        if (i === mesh.active)
          g.circle(n.x, n.y, r * 1.6)
            .fill({ color: 0xffffff })
            .stroke({ width: 3 * px, color: ACCENT });
        else
          g.circle(n.x, n.y, r)
            .fill({ color: ACCENT })
            .stroke({ width: 1 * px, color: 0xffffff });
      });
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

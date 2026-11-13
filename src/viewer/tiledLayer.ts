// Calque d'une grande image affichée par tuiles.

import { ColorMatrixFilter, Container, Rectangle, Sprite, Texture } from "pixi.js";
import {
  pickLevel,
  placedBounds,
  tileAncestor,
  tilePath,
  tileRect,
  visibleTileRange,
  type Rect,
} from "./geometry";
import type { FrameContext, SceneNode, TiledLayerSpec } from "./nodes";

interface Slot {
  sprite: Sprite;
  /** Adresse de la tuile actuellement montrée dans cet emplacement. */
  shown: string | null;
  /** Découpe d'une tuile moins fine, créée pour cet emplacement, à libérer. */
  cropped: Texture | null;
}

export class TiledLayerNode implements SceneNode {
  /** Gradué en pixels de l'image d'origine ; placé et mis à l'échelle en mm. */
  readonly container = new Container();
  private spec: TiledLayerSpec;
  /** Un emplacement par tuile visible du niveau courant. */
  private readonly slots = new Map<string, Slot>();

  constructor(spec: TiledLayerSpec) {
    this.spec = spec;
    this.container.label = `tiles:${spec.id}`;
    this.apply(spec);
  }

  /** Vrai si `spec` décrit la même image : le calque peut être gardé. */
  accepts(spec: TiledLayerSpec): boolean {
    return (
      spec.source.baseUrl === this.spec.source.baseUrl &&
      spec.source.pyramid.width === this.spec.source.pyramid.width &&
      spec.source.pyramid.height === this.spec.source.pyramid.height
    );
  }

  apply(spec: TiledLayerSpec): void {
    this.spec = spec;
    const { x, y, mmPerPixel } = spec.placement;
    this.container.position.set(x, y);
    this.container.scale.set(mmPerPixel);
    this.container.alpha = spec.opacity ?? 1;
    this.container.visible = spec.visible ?? true;
    this.container.blendMode = spec.blend ?? "normal";
    this.container.filters = spec.invert ? [this.negativeFilter()] : [];
  }

  private negative: ColorMatrixFilter | null = null;

  /** Filtre du négatif, créé au premier besoin, à la finesse de l'écran. */
  private negativeFilter(): ColorMatrixFilter {
    if (!this.negative) {
      this.negative = new ColorMatrixFilter({ resolution: "inherit" });
      this.negative.negative(false);
    }
    return this.negative;
  }

  bounds(): Rect | null {
    if (!this.container.visible) return null;
    return placedBounds(this.spec.source.pyramid, this.spec.placement);
  }

  finestDetail(): number | null {
    return this.container.visible ? this.spec.placement.mmPerPixel : null;
  }

  update(frame: FrameContext): void {
    if (!this.container.visible) {
      this.clearSlots();
      return;
    }
    const { pyramid, baseUrl } = this.spec.source;
    const placement = this.spec.placement;
    const url = (level: number, column: number, row: number) =>
      `${baseUrl}/${tilePath(pyramid, level, column, row)}`;

    // La tuile unique du dernier niveau reste toujours demandée : elle sert de
    // fond en attendant les autres, où que l'on soit.
    const coarsest = pyramid.levels.length - 1;
    frame.wantTile(url(coarsest, 0, 0), -1);

    const level = pickLevel(pyramid.levels.length, placement.mmPerPixel, frame.view.zoom, frame.devicePixelRatio);
    const range = visibleTileRange(pyramid, level, placement, frame.view, frame.viewport);
    if (!range) {
      this.clearSlots();
      return;
    }

    const centerColumn = (range.columnMin + range.columnMax) / 2;
    const centerRow = (range.rowMin + range.rowMax) / 2;
    const levelFactor = 2 ** level;
    const used = new Set<string>();

    for (let row = range.rowMin; row <= range.rowMax; row++) {
      for (let column = range.columnMin; column <= range.columnMax; column++) {
        const wanted = url(level, column, row);
        // Les tuiles du centre de l'écran d'abord.
        frame.wantTile(wanted, Math.hypot(column - centerColumn, row - centerRow));
        used.add(wanted);
        const slot = this.slot(wanted);
        const rect = tileRect(pyramid, level, column, row);
        slot.sprite.position.set(rect.x * levelFactor, rect.y * levelFactor);

        // La tuile voulue si elle est là ; sinon la partie correspondante de
        // la tuile moins fine la plus proche. Un emplacement ne montre jamais
        // deux images superposées : l'opacité du calque reste juste.
        let found = false;
        for (let up = 0; level + up <= coarsest; up++) {
          const ancestor = tileAncestor(pyramid, level, column, row, up);
          const key = url(ancestor.level, ancestor.column, ancestor.row);
          const texture = frame.tiles.get(key);
          if (!texture) continue;
          this.show(slot, key, texture, up === 0 ? null : ancestor.frame);
          slot.sprite.scale.set(2 ** ancestor.level);
          found = true;
          break;
        }
        slot.sprite.visible = found;
        if (!found) this.release(slot);
      }
    }

    for (const [key, slot] of this.slots) {
      if (used.has(key)) continue;
      this.release(slot);
      slot.sprite.destroy();
      this.slots.delete(key);
    }
  }

  destroy(): void {
    this.clearSlots();
    this.container.destroy({ children: true });
    this.negative?.destroy();
  }

  private slot(key: string): Slot {
    let slot = this.slots.get(key);
    if (!slot) {
      slot = { sprite: new Sprite(), shown: null, cropped: null };
      this.container.addChild(slot.sprite);
      this.slots.set(key, slot);
    }
    return slot;
  }

  private show(slot: Slot, key: string, texture: Texture, frame: Rect | null): void {
    if (slot.shown === key) return;
    this.release(slot);
    if (frame) {
      slot.cropped = new Texture({
        source: texture.source,
        frame: new Rectangle(frame.x, frame.y, frame.width, frame.height),
      });
      slot.sprite.texture = slot.cropped;
    } else {
      slot.sprite.texture = texture;
    }
    slot.shown = key;
  }

  private release(slot: Slot): void {
    if (slot.shown === null) return;
    slot.sprite.texture = Texture.EMPTY;
    // La découpe est propre à l'emplacement ; la tuile elle-même appartient à
    // la réserve, qui la libère quand elle ne sert plus.
    slot.cropped?.destroy(false);
    slot.cropped = null;
    slot.shown = null;
  }

  private clearSlots(): void {
    for (const slot of this.slots.values()) {
      this.release(slot);
      slot.sprite.destroy();
    }
    this.slots.clear();
  }
}

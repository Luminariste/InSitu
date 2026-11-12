// Deux calques de plus pour la vue façade (tâche 3.3) :
// - une image tendue sur un maillage (photo redressée posée sur la façade) ;
// - des polygones (emprises, recouvrements), au trait d'épaisseur constante
//   à l'écran.

import { Container, Graphics, ImageSource, MeshSimple, Texture } from "pixi.js";
import type { Rect } from "./geometry";
import type { FrameContext, MeshImageLayerSpec, PolygonsLayerSpec, SceneNode } from "./nodes";

/** Indices des triangles d'une grille de (n + 1)² points, ligne par ligne. */
export function gridTriangles(divisions: number): Uint32Array {
  const n = divisions;
  const out = new Uint32Array(n * n * 6);
  let k = 0;
  for (let j = 0; j < n; j++) {
    for (let i = 0; i < n; i++) {
      const a = j * (n + 1) + i;
      const b = a + 1;
      const c = a + n + 1;
      const d = c + 1;
      out.set([a, b, c, b, d, c], k);
      k += 6;
    }
  }
  return out;
}

/** Coordonnées de texture d'une grille régulière (0 à 1), ligne par ligne. */
export function gridUvs(divisions: number): Float32Array {
  const n = divisions;
  const out = new Float32Array((n + 1) * (n + 1) * 2);
  for (let j = 0; j <= n; j++) {
    for (let i = 0; i <= n; i++) {
      const k = (j * (n + 1) + i) * 2;
      out[k] = i / n;
      out[k + 1] = j / n;
    }
  }
  return out;
}

function pointsBounds(points: { x: number; y: number }[]): Rect | null {
  if (points.length === 0) return null;
  let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
  for (const p of points) {
    x0 = Math.min(x0, p.x);
    y0 = Math.min(y0, p.y);
    x1 = Math.max(x1, p.x);
    y1 = Math.max(y1, p.y);
  }
  return { x: x0, y: y0, width: x1 - x0, height: y1 - y0 };
}

export class MeshImageNode implements SceneNode {
  readonly container = new Container();
  private spec: MeshImageLayerSpec;
  private mesh: MeshSimple | null = null;
  private texture: Texture | null = null;
  private readonly abort = new AbortController();

  constructor(spec: MeshImageLayerSpec, onLoaded: () => void) {
    this.spec = spec;
    this.container.label = `mesh:${spec.id}`;
    this.apply(spec);
    void this.load(onLoaded);
  }

  accepts(spec: MeshImageLayerSpec): boolean {
    return spec.url === this.spec.url && spec.divisions === this.spec.divisions;
  }

  apply(spec: MeshImageLayerSpec): void {
    this.spec = spec;
    this.container.alpha = spec.opacity ?? 1;
    this.container.visible = spec.visible ?? true;
    this.container.blendMode = spec.blend === "screen" ? "screen" : "normal";
    if (this.mesh) {
      this.mesh.tint = spec.tint ?? 0xffffff;
      this.setVertices(this.mesh);
    }
  }

  private setVertices(mesh: MeshSimple): void {
    const vertices = mesh.vertices;
    this.spec.points.forEach((p, k) => {
      vertices[2 * k] = p.x;
      vertices[2 * k + 1] = p.y;
    });
    mesh.vertices = vertices;
  }

  private async load(onLoaded: () => void): Promise<void> {
    try {
      const response = await fetch(this.spec.url, {
        signal: this.abort.signal,
      });
      if (!response.ok) return;
      const bitmap = await createImageBitmap(await response.blob());
      if (this.abort.signal.aborted) {
        bitmap.close();
        return;
      }
      this.texture = new Texture({
        source: new ImageSource({ resource: bitmap, scaleMode: "linear" }),
      });
      const vertices = new Float32Array(this.spec.points.length * 2);
      this.mesh = new MeshSimple({
        texture: this.texture,
        vertices,
        uvs: gridUvs(this.spec.divisions),
        indices: gridTriangles(this.spec.divisions),
      });
      this.mesh.tint = this.spec.tint ?? 0xffffff;
      this.setVertices(this.mesh);
      this.container.addChild(this.mesh);
      onLoaded();
    } catch {
      // Image absente ou chargement annulé : le calque reste vide.
    }
  }

  bounds(): Rect | null {
    return this.container.visible ? pointsBounds(this.spec.points) : null;
  }

  finestDetail(): number | null {
    return null;
  }

  update(): void {}

  destroy(): void {
    this.abort.abort();
    this.container.destroy({ children: true });
    const resource = this.texture?.source.resource as ImageBitmap | undefined;
    this.texture?.destroy(true);
    resource?.close?.();
  }
}

export class PolygonsNode implements SceneNode {
  readonly container = new Container();
  private readonly graphics = new Graphics();
  private spec: PolygonsLayerSpec;
  private drawnZoom = 0;

  constructor(spec: PolygonsLayerSpec) {
    this.spec = spec;
    this.container.label = `polygones:${spec.id}`;
    this.container.addChild(this.graphics);
    this.apply(spec);
  }

  apply(spec: PolygonsLayerSpec): void {
    this.spec = spec;
    this.container.alpha = spec.opacity ?? 1;
    this.container.visible = spec.visible ?? true;
    this.drawnZoom = 0;
  }

  bounds(): Rect | null {
    return this.container.visible ? pointsBounds(this.spec.shapes.flatMap((s) => s.points)) : null;
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
    for (const shape of this.spec.shapes) {
      if (shape.points.length < 3) continue;
      g.poly(
        shape.points.flatMap((p) => [p.x, p.y]),
        true,
      );
      if (shape.fillAlpha && shape.fillAlpha > 0) g.fill({ color: shape.color, alpha: shape.fillAlpha });
      g.stroke({
        width: (shape.widthPx ?? 1.5) * px,
        color: shape.color,
        alpha: shape.strokeAlpha ?? 1,
      });
    }
  }

  destroy(): void {
    this.container.destroy({ children: true });
  }
}

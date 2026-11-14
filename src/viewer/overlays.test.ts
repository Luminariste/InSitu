import { afterEach, describe, expect, it, vi } from "vitest";
import { MeshImageNode } from "./overlays";
import type { MeshImageLayerSpec } from "./nodes";

const SQUARE = [
  { x: 0, y: 0 },
  { x: 10, y: 0 },
  { x: 0, y: 10 },
  { x: 10, y: 10 },
];
const spec = (url: string): MeshImageLayerSpec => ({ id: "apercu", kind: "mesh-image", url, divisions: 1, points: SQUARE });

/** Une image chargée à la demande : `release(url)` termine son chargement. */
function slowImages() {
  const waiting = new Map<string, () => void>();
  vi.stubGlobal("fetch", (url: string) => new Promise((resolve) => waiting.set(url, () => resolve({ ok: true, blob: async () => new Blob() }))));
  vi.stubGlobal("createImageBitmap", async () => ({ width: 4, height: 4, close() {} }));
  return async (url: string) => {
    waiting.get(url)?.();
    for (let k = 0; k < 10; k++) await Promise.resolve();
  };
}

afterEach(() => vi.unstubAllGlobals());

describe("image posée sur une grille", () => {
  it("garde l'ancienne image jusqu'à ce que la nouvelle soit prête (pas d'écran noir)", async () => {
    const release = slowImages();
    const loaded = vi.fn();
    const node = new MeshImageNode(spec("a.png"), loaded);
    await release("a.png");
    expect(node.container.children.length).toBe(1);
    const shown = node.container.children[0];

    // Un nouvel aperçu (diapositive recalculée) : même grille, autre image.
    expect(node.accepts(spec("b.png"))).toBe(true);
    node.apply(spec("b.png"));
    expect(node.container.children).toEqual([shown]);
    await release("b.png");
    expect(node.container.children).toEqual([shown]);
    expect(loaded).toHaveBeenCalledTimes(2);
  });

  it("une image demandée puis remplacée avant d'arriver est abandonnée", async () => {
    const release = slowImages();
    const node = new MeshImageNode(spec("a.png"), () => {});
    node.apply(spec("b.png"));
    await release("a.png");
    expect(node.container.children.length).toBe(0);
    await release("b.png");
    expect(node.container.children.length).toBe(1);
  });
});

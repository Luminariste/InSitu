import { describe, expect, it } from "vitest";
import { applyGesture, frameCorners, handleAt, repeatGesture, type Gesture } from "./gesture";
import { bounds, type Placement } from "./placement";

const placed = (id: string, x: number, y: number): Placement => ({
  id,
  drawing: "merle",
  center: { x, y },
  width: 200,
  height: 100,
  rotation: 0,
});
const none = { shift: false, alt: false };
let n = 0;
const makeId = () => `c${++n}`;

describe("gestes sur les dessins posés", () => {
  it("distingue poignée, rotation et intérieur du cadre", () => {
    const corners = frameCorners({ x: 100, y: 100, width: 200, height: 100 });
    expect(handleAt(corners, { x: 103, y: 98 })).toEqual({ kind: "scale", corner: 0 });
    expect(handleAt(corners, { x: 315, y: 215 })).toEqual({ kind: "rotate", corner: 2 });
    // Juste à l'intérieur, près d'un coin : c'est le dessin qu'on prend, pas la rotation.
    expect(handleAt(corners, { x: 115, y: 112 })).toBeNull();
    expect(handleAt(corners, { x: 200, y: 150 })).toBeNull();
  });

  it("déplace, copie avec ⌥ et contraint avec ⇧", () => {
    const before = [placed("a", 0, 0)];
    const g: Gesture = { kind: "move", start: { x: 0, y: 0 }, before, ids: new Set(["a"]) };
    const moved = applyGesture(g, { x: 30, y: 4 }, { shift: true, alt: false });
    expect(moved.placements[0].center).toEqual({ x: 30, y: 0 });
    const copied = applyGesture(g, { x: 30, y: 4 }, { shift: false, alt: true }, makeId);
    expect(copied.placements).toHaveLength(2);
    expect(copied.placements[0]).toBe(before[0]);
    expect([...copied.selection]).toEqual([copied.placements[1].id]);
    expect(copied.repeat).toEqual({ kind: "move", dx: 30, dy: 4, copy: true });
  });

  it("agrandit depuis le coin opposé, ou depuis le centre avec ⌥", () => {
    const before = [placed("a", 100, 50)];
    const frame = bounds(before)!; // 0,0 → 200,100
    const g: Gesture = { kind: "scale", start: { x: 200, y: 100 }, before, ids: new Set(["a"]), frame, corner: 2 };
    const step = applyGesture(g, { x: 400, y: 200 }, none);
    expect(step.placements[0]).toMatchObject({ width: 400, height: 200, center: { x: 200, y: 100 } });
    const centered = applyGesture(g, { x: 300, y: 150 }, { shift: false, alt: true });
    expect(centered.placements[0]).toMatchObject({ width: 400, center: { x: 100, y: 50 } });
  });

  it("tourne autour du centre, par pas de 45° avec ⇧", () => {
    const before = [placed("a", 0, 0)];
    const g: Gesture = { kind: "rotate", start: { x: 100, y: 0 }, before, ids: new Set(["a"]), pivot: { x: 0, y: 0 } };
    expect(applyGesture(g, { x: 0, y: 100 }, none).placements[0].rotation).toBeCloseTo(90);
    expect(applyGesture(g, { x: 100, y: 30 }, { shift: true, alt: false }).placements[0].rotation).toBe(0);
  });

  it("le cadre de sélection prend ce qu'il touche, ⇧ ajoute", () => {
    const before = [placed("a", 0, 0), placed("b", 1000, 0)];
    const g: Gesture = { kind: "marquee", start: { x: 900, y: -200 }, before, ids: new Set(["a"]) };
    expect([...applyGesture(g, { x: 950, y: 200 }, none).selection]).toEqual(["b"]);
    expect([...applyGesture(g, { x: 950, y: 200 }, { shift: true, alt: false }).selection].sort()).toEqual(["a", "b"]);
  });

  it("⌘D répète une copie déplacée : une série régulière", () => {
    let list = [placed("a", 0, 0)];
    let selection = new Set(["a"]);
    for (let i = 0; i < 3; i++)
      ({ placements: list, selection } = repeatGesture(
        list,
        selection,
        { kind: "move", dx: 250, dy: 0, copy: true },
        makeId,
      ));
    expect(list.map((p) => p.center.x)).toEqual([0, 250, 500, 750]);
    expect(selection.size).toBe(1);
    const turned = repeatGesture(list, selection, { kind: "rotate", deg: 15, copy: false });
    expect(turned.placements[3].rotation).toBe(15);
    expect(turned.placements[3].center.x).toBeCloseTo(750);
  });
});

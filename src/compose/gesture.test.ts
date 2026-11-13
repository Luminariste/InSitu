import { describe, expect, it } from "vitest";
import {
  KNOB,
  KNOB_PX,
  applyGesture,
  frameCorners,
  frameHandles,
  handleAt,
  knobOf,
  repeatGesture,
  rotateCursor,
  scaleCursor,
  type Gesture,
} from "./gesture";
import { bounds, facadePoint, type Placement } from "./placement";

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

  it("poignées du cadre comme Illustrator : coin en x et y, côté en x ou en y (D-4.2-l)", () => {
    const before = [placed("a", 100, 50)];
    const frame = bounds(before)!; // 0,0 → 200,100
    const handles = frameHandles(frame);
    expect(handles[5]).toEqual({ x: 200, y: 50 });
    expect(handleAt(handles, { x: 202, y: 52 })).toEqual({ kind: "scale", corner: 5 });
    expect(handleAt(handles, { x: 100, y: -3 })).toEqual({ kind: "scale", corner: 4 });
    // Coin bas-droit tiré en (400, 150) : 2 fois en x, 1,5 fois en y.
    const corner: Gesture = { kind: "scale", start: { x: 200, y: 100 }, before, ids: new Set(["a"]), frame, corner: 2 };
    expect(applyGesture(corner, { x: 400, y: 150 }, none).placements[0]).toMatchObject({
      width: 400,
      height: 150,
      center: { x: 200, y: 75 },
    });
    // ⇧ : proportions gardées.
    expect(applyGesture(corner, { x: 400, y: 200 }, { shift: true, alt: false }).placements[0]).toMatchObject({
      width: 400,
      height: 200,
    });
    // Côté droit : en x seulement, même si le pointeur monte.
    const right: Gesture = { kind: "scale", start: { x: 200, y: 50 }, before, ids: new Set(["a"]), frame, corner: 5 };
    const wide = applyGesture(right, { x: 300, y: 10 }, none);
    expect(wide.placements[0]).toMatchObject({ width: 300, height: 100, center: { x: 150, y: 50 } });
    expect(wide.repeat).toEqual({ kind: "stretch", sx: 1.5, sy: 1, copy: false });
    // Côté haut avec ⌥ : depuis le centre, en y seulement.
    const top: Gesture = { kind: "scale", start: { x: 100, y: 0 }, before, ids: new Set(["a"]), frame, corner: 4 };
    expect(applyGesture(top, { x: 100, y: -50 }, { shift: false, alt: true }).placements[0]).toMatchObject({
      width: 200,
      height: 200,
      center: { x: 100, y: 50 },
    });
  });

  it("étire exactement un dessin tourné : il reçoit une déformation", () => {
    const tilted = [{ ...placed("a", 0, 0), rotation: 30 }];
    const frame = bounds(tilted)!;
    const right: Gesture = {
      kind: "scale",
      start: { x: frame.x + frame.width, y: 0 },
      before: tilted,
      ids: new Set(["a"]),
      frame,
      corner: 5,
    };
    const step = applyGesture(right, { x: frame.x + 2 * frame.width, y: 0 }, none);
    const p = step.placements[0];
    expect(p.deformation).toBeTruthy();
    // Chaque coin du dessin : x doublé depuis le bord gauche, y inchangé.
    const left = frame.x;
    for (const [u, v] of [
      [0, 0],
      [1, 0],
      [1, 1],
      [0, 1],
      [0.3, 0.7],
    ]) {
      const was = facadePoint(tilted[0], u, v);
      const now = facadePoint(p, u, v);
      expect(now.x).toBeCloseTo(left + (was.x - left) * 2, 6);
      expect(now.y).toBeCloseTo(was.y, 6);
    }
    // Un dessin droit tourné d'un quart : largeur et hauteur s'échangent, sans déformation.
    const quarter = [{ ...placed("b", 0, 0), rotation: 90 }];
    const qf = bounds(quarter)!; // 100 de large, 200 de haut
    const qg: Gesture = {
      kind: "scale",
      start: { x: qf.x + qf.width, y: 0 },
      before: quarter,
      ids: new Set(["b"]),
      frame: qf,
      corner: 5,
    };
    const q = applyGesture(qg, { x: qf.x + 3 * qf.width, y: 0 }, none).placements[0];
    expect(q.deformation).toBeUndefined();
    expect(q).toMatchObject({ width: 200, height: 300 });
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

  it("poignée ronde au-dessus du cadre : elle tourne la sélection (D-4.2-m)", () => {
    const handles = frameHandles({ x: 100, y: 100, width: 200, height: 100 });
    const knob = knobOf(handles);
    expect(knob).toEqual({ x: 200, y: 100 - KNOB_PX });
    expect(handleAt(handles, { x: 203, y: 100 - KNOB_PX + 2 })).toEqual({ kind: "rotate", corner: KNOB });
    // Le milieu du côté haut reste une poignée d'étirement.
    expect(handleAt(handles, { x: 200, y: 101 })).toEqual({ kind: "scale", corner: 4 });
    // Tirer la poignée ronde d'un quart de tour vers la droite tourne le dessin de 90°.
    const before = [placed("a", 200, 150)];
    const g: Gesture = {
      kind: "rotate",
      start: { x: 200, y: 100 - KNOB_PX },
      before,
      ids: new Set(["a"]),
      pivot: { x: 200, y: 150 },
      corner: KNOB,
    };
    const step = applyGesture(g, { x: 300, y: 150 }, none);
    expect(step.turned).toBeCloseTo(90);
    expect(step.placements[0].rotation).toBeCloseTo(90);
    expect(step.placements[0].center).toEqual({ x: 200, y: 150 });
    // Un tour presque complet se lit comme un petit angle de l'autre côté.
    const back = applyGesture(g, { x: 190, y: 0 }, none);
    expect(Math.abs(back.turned!)).toBeLessThan(10);
    // ⇧ : par pas de 45°.
    expect(applyGesture(g, { x: 300, y: 120 }, { shift: true, alt: false }).placements[0].rotation).toBeCloseTo(90);
  });

  it("curseurs : flèche double sur les poignées, flèche courbe pour tourner (D-4.2-m)", () => {
    expect(scaleCursor(0)).toBe("nwse-resize");
    expect(scaleCursor(1)).toBe("nesw-resize");
    expect(scaleCursor(4)).toBe("ns-resize");
    expect(scaleCursor(5)).toBe("ew-resize");
    const knob = rotateCursor(KNOB);
    expect(knob).toMatch(/^url\("data:image\/svg\+xml,.*"\) 12 12, alias$/);
    expect(decodeURIComponent(knob)).toContain("rotate(0 12 12)");
    expect(decodeURIComponent(rotateCursor(1))).toContain("rotate(45 12 12)");
    // Pendant le geste, la flèche suit l'angle tourné (arrondi à 15°).
    expect(decodeURIComponent(rotateCursor(KNOB, 92))).toContain("rotate(90 12 12)");
  });
});

import { describe, expect, it } from "vitest";
import { deformPoint, identity, nodePoints, type Deformation } from "../compose/deformation";
import {
  alignShifts,
  applySlideGesture,
  boxOf,
  distributeShifts,
  regrid,
  scaleAdjustment,
  shiftAdjustment,
  slideCopies,
  turnAdjustment,
} from "./tools";
import type { Placement } from "../compose/placement";

const rest = (): Deformation => identity({ center: { x: 100, y: 50 }, width: 40, height: 20, rotation: 0 }, 1, 1);
const near = (p: { x: number; y: number }, x: number, y: number) => {
  expect(p.x).toBeCloseTo(x, 6);
  expect(p.y).toBeCloseTo(y, 6);
};

describe("outils de la diapositive", () => {
  it("décale, étire et tourne les nœuds de la retouche", () => {
    const moved = nodePoints(shiftAdjustment(rest(), 5, -2));
    near(moved[0], 85, 38);
    const scaled = nodePoints(scaleAdjustment(rest(), 2, 0.5, { x: 80, y: 40 }));
    near(scaled[0], 80, 40);
    near(scaled[3], 160, 50);
    const turned = nodePoints(turnAdjustment(rest(), 90, { x: 100, y: 50 }));
    // Coin haut-gauche (80, 40) tourné d'un quart de tour : (110, 30).
    near(turned[0], 110, 30);
  });

  it("une grille garde la retouche déjà faite", () => {
    const before = shiftAdjustment(turnAdjustment(rest(), 30, { x: 100, y: 50 }), 7, 3);
    const grid = regrid(before, 3, 2);
    expect(grid.columns).toBe(3);
    expect(grid.rows).toBe(2);
    for (const p of [
      { x: 81, y: 41 },
      { x: 100, y: 50 },
      { x: 117, y: 58 },
    ]) {
      const a = deformPoint(before, p);
      const b = deformPoint(grid, p);
      near(b, a.x, a.y);
    }
  });

  it("aligne et répartit des blocs, un groupe d'un bloc", () => {
    const units = [
      { ids: ["a"], box: { x: 0, y: 0, width: 10, height: 10 } },
      { ids: ["b", "c"], box: { x: 30, y: 5, width: 20, height: 10 } },
      { ids: ["d"], box: { x: 100, y: 20, width: 10, height: 10 } },
    ];
    const left = alignShifts(units, "left");
    expect(left.get("a")).toEqual({ x: 0, y: 0 });
    expect(left.get("b")).toEqual({ x: -30, y: 0 });
    expect(left.get("c")).toEqual({ x: -30, y: 0 });
    expect(alignShifts(units, "bottom").get("a")).toEqual({ x: 0, y: 20 });
    expect(alignShifts(units.slice(0, 1), "left").size).toBe(0);
    // Écart de (110 − 40) / 2 = 35 : le bloc du milieu passe de x = 30 à 45.
    const spread = distributeShifts(units, "horizontal");
    expect(spread.get("a")).toEqual({ x: 0, y: 0 });
    expect(spread.get("b")!.x).toBeCloseTo(15);
    expect(spread.get("d")!.x).toBeCloseTo(0);
    expect(distributeShifts(units.slice(0, 2), "vertical").size).toBe(0);
  });

  it("les gestes à la souris suivent ceux de la façade", () => {
    const before = new Map([["a", rest()]]);
    const frame = boxOf(nodePoints(rest()))!;
    // Déplacer, avec ⇧ : seulement à l'horizontale.
    const moved = applySlideGesture({ kind: "move", start: { x: 0, y: 0 }, before }, { x: 10, y: 2 }, { shift: true, alt: false });
    near(nodePoints(moved.get("a")!)[0], 90, 40);
    // Coin bas-droit tiré de (120, 60) à (140, 70), coin haut-gauche fixe.
    const scaled = applySlideGesture(
      { kind: "scale", start: { x: 120, y: 60 }, frame, corner: 2, before },
      { x: 140, y: 70 },
      { shift: false, alt: false },
    );
    near(nodePoints(scaled.get("a")!)[3], 140, 70);
    near(nodePoints(scaled.get("a")!)[0], 80, 40);
    // Jamais retourné : l'étirement s'arrête avant zéro.
    const flipped = applySlideGesture(
      { kind: "scale", start: { x: 120, y: 60 }, frame, corner: 2, before },
      { x: 20, y: 70 },
      { shift: false, alt: false },
    );
    expect(nodePoints(flipped.get("a")!)[3].x).toBeGreaterThan(80);
    // Rotation : ⇧ arrondit à 5°.
    const turned = applySlideGesture(
      { kind: "rotate", start: { x: 140, y: 50 }, pivot: { x: 100, y: 50 }, before },
      { x: 100 + 40 * Math.cos(0.2), y: 50 + 40 * Math.sin(0.2) },
      { shift: true, alt: false },
    );
    const expected = nodePoints(turnAdjustment(rest(), 10, { x: 100, y: 50 }))[0];
    near(nodePoints(turned.get("a")!)[0], expected.x, expected.y);
  });
});

describe("copier dans la diapositive (⌘D, ⌥ + glisser)", () => {
  it("copie à la même place sur la façade, affectée à cette diapositive, juste devant l'original", () => {
    const frame = { center: { x: 50, y: 50 }, width: 20, height: 10, rotation: 0 };
    const moved = shiftAdjustment(identity(frame, 1, 1), 5, 5);
    const base = { drawing: "d", center: { x: 1, y: 2 }, width: 3, height: 4, rotation: 0, mirrored: false, layer: null, deformation: null, hidden: false, locked: false };
    const placements = [
      { ...base, id: "a", group: "g", projector: null, adjustment: null },
      { ...base, id: "b", group: "g", projector: null, adjustment: null },
      { ...base, id: "c", group: null, projector: null, adjustment: null },
    ] as unknown as Placement[];
    let n = 0;
    const r = slideCopies(placements, new Map([["a", moved], ["b", moved]]), "A", () => `n${++n}`);
    expect(r.placements.map((p) => p.id)).toEqual(["a", r.ids[0], "b", r.ids[1], "c"]);
    const copy = r.placements[1];
    expect(copy.center).toEqual({ x: 1, y: 2 });
    expect(copy.projector).toBe("A");
    expect(copy.adjustment).toEqual({ projector: "A", deformation: moved });
    // Le groupe copié est un nouveau groupe, commun aux deux copies.
    expect(copy.group).not.toBe("g");
    expect(r.placements[3].group).toBe(copy.group);
  });
});

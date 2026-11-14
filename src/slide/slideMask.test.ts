import { describe, expect, it } from "vitest";
import { addShape, closesOnFirst, maskActive, maskLayers, removeShape } from "./slideMask";

const tri = [
  { x: 10, y: 10 },
  { x: 100, y: 10 },
  { x: 50, y: 90 },
];
const square = [
  { x: 0, y: 0 },
  { x: 280, y: 0 },
  { x: 280, y: 280 },
  { x: 0, y: 280 },
];

describe("masque de la diapositive", () => {
  it("ajoute une forme d'au moins trois points, et la retire", () => {
    expect(addShape(null, tri.slice(0, 2))).toBeNull();
    const m = addShape(null, tri)!;
    expect(m.shapes).toEqual([
      [
        [10, 10],
        [100, 10],
        [50, 90],
      ],
    ]);
    const two = addShape(m, tri)!;
    expect(two.shapes).toHaveLength(2);
    expect(removeShape(two, 0)!.shapes).toHaveLength(1);
    expect(removeShape(m, 0)).toBeNull();
    // « Noir dehors » sans forme reste : tout est noir.
    expect(removeShape({ ...m, outside: true }, 0)).toEqual({ shapes: [], outside: true });
  });

  it("n'agit pas caché, ni vide", () => {
    expect(maskActive(null)).toBe(false);
    expect(maskActive({ shapes: [] })).toBe(false);
    expect(maskActive({ shapes: [[[0, 0], [1, 0], [0, 1]]] })).toBe(true);
    expect(maskActive({ shapes: [[[0, 0], [1, 0], [0, 1]]], hidden: true })).toBe(false);
  });

  it("ferme la plume sur le premier point", () => {
    const id = (p: { x: number; y: number }) => p;
    expect(closesOnFirst(tri, { x: 14, y: 12 }, id)).toBe(true);
    expect(closesOnFirst(tri, { x: 30, y: 30 }, id)).toBe(false);
    expect(closesOnFirst(tri.slice(0, 2), { x: 10, y: 10 }, id)).toBe(false);
  });

  it("noircit dehors par un carré troué, dedans par les formes", () => {
    const shape: [number, number][] = [
      [10, 10],
      [100, 10],
      [50, 90],
    ];
    const outside = maskLayers({ shapes: [shape], outside: true }, square, [], null);
    const fill = outside[0];
    expect(fill.kind === "polygons" && fill.shapes[0].holes).toHaveLength(1);
    const inside = maskLayers({ shapes: [shape] }, square, [], null);
    expect(inside[0].kind === "polygons" && inside[0].shapes[0].points).toHaveLength(3);
    expect(maskLayers({ shapes: [shape], hidden: true }, square, [], null)).toHaveLength(0);
  });
});

import { describe, expect, it } from "vitest";
import { aspectOf, invertPoint, nextBrush, strokeFor, visibleAt, type MaskStroke } from "./mask";
import { facadePoint, touches, type Placement } from "./placement";
import { slidePoint } from "../slide/adjust";
import type { SlideDrawing } from "../engine";

const placed = (extra: Partial<Placement> = {}): Placement => ({
  id: "p",
  drawing: "d",
  center: { x: 1000, y: 500 },
  width: 400,
  height: 200,
  rotation: 30,
  mirrored: true,
  ...extra,
});

describe("masque d'un dessin posé", () => {
  it("retrouve le point du dessin sous un point de la façade, tourné et retourné", () => {
    const p = placed();
    const map = (u: number, v: number) => facadePoint(p, u, v);
    const uv = invertPoint(map, facadePoint(p, 0.2, 0.7));
    expect(uv![0]).toBeCloseTo(0.2, 6);
    expect(uv![1]).toBeCloseTo(0.7, 6);
  });

  it("ramène un coup de pinceau dans le dessin, épaisseur en fraction du grand côté", () => {
    const p = placed({ rotation: 0, mirrored: false });
    const map = (u: number, v: number) => facadePoint(p, u, v);
    const s = strokeFor(map, [facadePoint(p, 0.1, 0.5), facadePoint(p, 0.4, 0.5)], "brush", false, 40)!;
    expect(s.tool).toBe("erase");
    expect(s.fill).toBeUndefined();
    expect(s.size).toBeCloseTo(0.1, 6); // 40 mm sur 400 mm
    expect(s.points[0][0]).toBeCloseTo(0.1, 5);
    expect(s.points[1][0]).toBeCloseTo(0.4, 5);
  });

  it("⌥ fait réapparaître, le lasso fait une zone", () => {
    const p = placed();
    const map = (u: number, v: number) => facadePoint(p, u, v);
    const pts = [facadePoint(p, 0.1, 0.1), facadePoint(p, 0.9, 0.1), facadePoint(p, 0.5, 0.9)];
    const s = strokeFor(map, pts, "lasso", true, 10)!;
    expect(s.tool).toBe("draw");
    expect(s.fill).toBe(true);
    expect(s.points).toHaveLength(3);
  });

  it("n'enregistre rien d'un geste loin du dessin", () => {
    const p = placed({ rotation: 0, mirrored: false });
    const map = (u: number, v: number) => facadePoint(p, u, v);
    expect(strokeFor(map, [{ x: 5000, y: 5000 }], "brush", false, 10)).toBeNull();
  });

  it("suit le dessin dans la diapositive, au-delà du bord de sa maille", () => {
    const d = {
      placement: "p",
      drawing: "d",
      divisions: 1,
      points: [
        { x: 100, y: 100 },
        { x: 200, y: 110 },
        { x: 95, y: 180 },
        { x: 205, y: 190 },
      ],
    } as unknown as SlideDrawing;
    const map = (u: number, v: number) => slidePoint(d, null, u, v);
    const s = strokeFor(map, [map(0.5, 0.5), { x: 90, y: 140 }], "brush", false, 11)!;
    expect(s.points[0][0]).toBeCloseTo(0.5, 5);
    expect(s.points[1][0]).toBeLessThan(0);
  });

  it("dit ce qui reste visible, coup après coup", () => {
    const strokes: MaskStroke[] = [
      { tool: "erase", fill: true, size: 0, points: [[0, 0], [0.5, 0], [0.5, 1], [0, 1]] },
      { tool: "draw", size: 0.2, points: [[0.25, 0.5]] },
    ];
    const aspect = aspectOf(200, 100);
    expect(visibleAt(strokes, aspect, 0.25, 0.1)).toBe(false);
    expect(visibleAt(strokes, aspect, 0.25, 0.5)).toBe(true);
    expect(visibleAt(strokes, aspect, 0.75, 0.5)).toBe(true);
  });

  it("laisse passer le clic là où le dessin est caché", () => {
    const p = placed({
      rotation: 0,
      mirrored: false,
      mask: [{ tool: "erase", fill: true, size: 0, points: [[0, 0], [0.5, 0], [0.5, 1], [0, 1]] }],
    });
    expect(touches(p, facadePoint(p, 0.25, 0.5))).toBe(false);
    expect(touches(p, facadePoint(p, 0.75, 0.5))).toBe(true);
  });

  it("change l'épaisseur du pinceau par crans", () => {
    expect(nextBrush(36, 1)).toBe(54);
    expect(nextBrush(36, -1)).toBe(24);
    expect(nextBrush(6, -1)).toBe(6);
  });
});

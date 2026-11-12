import { describe, expect, it } from "vitest";
import type { SlideDrawing } from "../engine";
import { nodePoints } from "../compose/deformation";
import { adjustedOutline, adjustedPoints, applyAdjustGesture, drawingAt, restAdjustment, restFrame } from "./adjust";

function drawing(placement: string, x: number, y: number, w: number, h: number): SlideDrawing {
  const n = 4;
  const points = [];
  for (let j = 0; j <= n; j++) for (let i = 0; i <= n; i++) points.push({ x: x + (w * i) / n, y: y + (h * j) / n });
  return { placement, drawing: "merle", divisions: n, points };
}

const close = (a: { x: number; y: number }, b: { x: number; y: number }) =>
  expect(Math.hypot(a.x - b.x, a.y - b.y)).toBeLessThan(1e-9);

describe("retouche d'un dessin dans la diapositive (tâche 4.6)", () => {
  it("une retouche neuve est au repos sur le rectangle du dessin", () => {
    const d = drawing("a", 10, 20, 100, 50);
    expect(restFrame(d.points)).toEqual({ center: { x: 60, y: 45 }, width: 100, height: 50, rotation: 0 });
    const rest = restAdjustment(d, 3, 2)!;
    adjustedPoints(d, rest).forEach((p, k) => close(p, d.points[k]));
  });

  it("glisser le dessin le décale, tirer un point le déforme", () => {
    const d = drawing("a", 10, 20, 100, 50);
    const rest = restAdjustment(d, 1, 1)!;
    const moved = applyAdjustGesture({ kind: "move", start: { x: 50, y: 40 }, before: rest }, { x: 62, y: 35 });
    adjustedPoints(d, moved).forEach((p, k) => close(p, { x: d.points[k].x + 12, y: d.points[k].y - 5 }));
    const origin = nodePoints(rest)[3];
    const pulled = applyAdjustGesture(
      { kind: "node", start: { x: 111, y: 71 }, before: rest, index: 3, origin },
      { x: 121, y: 81 },
    );
    close(nodePoints(pulled)[3], { x: 120, y: 80 });
    // Le coin haut-gauche ne bouge pas.
    close(adjustedPoints(d, pulled)[0], d.points[0]);
  });

  it("un clic choisit le dessin de devant, à sa place retouchée", () => {
    const a = drawing("a", 0, 0, 100, 100);
    const b = drawing("b", 50, 50, 100, 100);
    const none = () => null;
    expect(drawingAt([a, b], none, { x: 75, y: 75 })?.placement).toBe("b");
    expect(drawingAt([a, b], none, { x: 25, y: 25 })?.placement).toBe("a");
    expect(drawingAt([a, b], none, { x: 200, y: 200 })).toBeNull();
    // A décalé de 150 mm : on le trouve à sa nouvelle place.
    const shifted = applyAdjustGesture(
      { kind: "move", start: { x: 0, y: 0 }, before: restAdjustment(a, 1, 1)! },
      { x: 150, y: 0 },
    );
    const of = (id: string) => (id === "a" ? shifted : null);
    expect(drawingAt([a, b], of, { x: 25, y: 25 })).toBeNull();
    expect(drawingAt([a, b], of, { x: 200, y: 25 })?.placement).toBe("a");
    expect(adjustedOutline(a, shifted).length).toBe(16);
  });
});

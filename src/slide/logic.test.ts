import { describe, expect, it } from "vitest";
import type { SlideView } from "../engine";
import { drawingsLayer, nextProjector, viewProblem } from "./logic";

const view = (id: string, photo: boolean, preview: boolean, problem: string | null = null): SlideView => ({
  projectorId: id,
  label: id.toUpperCase(),
  rectifiedPhoto: photo ? `photos-redressees/${id}.png` : null,
  preview: preview ? `diapositives/${id}.png` : null,
  problem,
  drawings: [],
});

describe("vue diapositive", () => {
  it("passe d'un projecteur à l'autre, en boucle", () => {
    const views = [view("a", true, true), view("b", true, true), view("c", true, false)];
    expect(nextProjector(views, "a", 1)).toBe("b");
    expect(nextProjector(views, "c", 1)).toBe("a");
    expect(nextProjector(views, "a", -1)).toBe("c");
    expect(nextProjector(views, null, 1)).toBe("a");
    expect(nextProjector([], "a", 1)).toBeNull();
  });

  it("pose les dessins sur le carré de 280 mm", () => {
    const [layer] = drawingsLayer("tuiles://x.png", true, 0.9, 0x00e5ff);
    expect(layer.kind === "mesh-image" && layer.points[3]).toEqual({ x: 280, y: 280 });
    expect(drawingsLayer(null, true, 1, 0)).toEqual([]);
  });

  it("dit ce qui manque", () => {
    expect(viewProblem(view("a", true, true))).toBeNull();
    expect(viewProblem(view("a", false, true))).toContain("pas de photo redressée");
    expect(viewProblem(view("a", true, false, "aucune fresque importée"))).toBe("A : aucune fresque importée.");
  });
});

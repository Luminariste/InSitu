import { describe, expect, it } from "vitest";
import type { SlideView } from "../engine";
import {
  BLACK,
  DEFAULT_LOOK,
  LIGHTBURN_LOOK,
  WHITE,
  backdropLayer,
  drawingsLayer,
  isLightBurnLook,
  nextProjector,
  parseLook,
  viewProblem,
  withBackdrop,
} from "./logic";

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

describe("aspect de la diapositive", () => {
  it("montre la photo par défaut, et sans photo ce qui part dans LightBurn", () => {
    expect(DEFAULT_LOOK.photo).toBe(true);
    expect(isLightBurnLook(DEFAULT_LOOK)).toBe(false);
    expect(LIGHTBURN_LOOK).toMatchObject({ photo: false, backdrop: "noir", tint: WHITE, opacity: 1 });
    expect(isLightBurnLook(LIGHTBURN_LOOK)).toBe(true);
  });

  it("remplit le carré de 280 mm de la couleur du fond", () => {
    const layer = backdropLayer("pierre");
    expect(layer.kind).toBe("polygons");
    if (layer.kind !== "polygons") return;
    expect(layer.shapes[0].color).toBe(0x8a7764);
    expect(layer.shapes[0].points).toContainEqual({ x: 280, y: 280 });
    expect(layer.shapes[0].fillAlpha).toBe(1);
  });

  it("ne laisse jamais des traits blancs sur fond blanc", () => {
    const white = withBackdrop(LIGHTBURN_LOOK, "blanc");
    expect(white.tint).toBe(BLACK);
    expect(withBackdrop(white, "noir").tint).toBe(WHITE);
    expect(withBackdrop(white, "pierre").tint).toBe(WHITE);
    // Une couleur choisie (cyan) est gardée.
    expect(withBackdrop(DEFAULT_LOOK, "blanc").tint).toBe(DEFAULT_LOOK.tint);
  });

  it("relit le réglage enregistré, et complète un réglage abîmé", () => {
    expect(parseLook(JSON.stringify(LIGHTBURN_LOOK))).toEqual(LIGHTBURN_LOOK);
    expect(parseLook(null)).toEqual(DEFAULT_LOOK);
    expect(parseLook("pas du json")).toEqual(DEFAULT_LOOK);
    expect(parseLook(JSON.stringify({ backdrop: "vert", opacity: 4, photo: false }))).toEqual({ ...DEFAULT_LOOK, photo: false });
  });
});

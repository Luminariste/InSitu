import { describe, expect, it } from "vitest";
import type { FacadeView, PlanGroup, PlanInfo } from "../../engine";
import {
  choicesFor,
  dragOrigin,
  groupRect,
  nextAlignStep,
  parseMeters,
  planGroupLayers,
  scaleForWidth,
  viewKey,
} from "./planGroups";

const group = (over: Partial<PlanGroup> = {}): PlanGroup => ({
  id: "g",
  name: "Écuries",
  source: "facade/plans/ecuries.ai",
  origin: { x: 100, y: 200 },
  mmPerUnit: 10,
  plans: [
    { id: "a", name: "Détaillé", sourceWidth: 400, sourceHeight: 200, layers: ["Détaillé"] },
    { id: "b", name: "Guides", sourceWidth: 400, sourceHeight: 200, layers: ["Guides"], hidden: true },
  ],
  ...over,
});

const view = (): FacadeView => ({
  kind: "pdf",
  source: { pyramid: { width: 4000, height: 2000, tileSize: 256, levels: [] } as never, baseUrl: "tuiles://localhost/plan-x" },
  sourceWidth: 400,
  sourceHeight: 200,
  sourceUnitsPerPixel: 0.1,
});

describe("plans ajoutés", () => {
  it("montre les vues visibles, en produit sur une feuille blanche", () => {
    const g = group();
    const views = new Map([
      [viewKey(g, g.plans[0]), view()],
      [viewKey(g, g.plans[1]), view()],
    ]);
    const { sheets, plans } = planGroupLayers([g], views, { negative: false });
    // La vue cachée n'est pas montrée.
    expect(plans).toHaveLength(1);
    expect(plans[0]).toMatchObject({ kind: "tiles", blend: "multiply", invert: false, placement: { x: 100, y: 200, mmPerPixel: 1 } });
    expect(sheets).toHaveLength(1);
    expect(sheets[0].kind).toBe("polygons");
  });

  it("en négatif : superposition claire, sans feuille", () => {
    const g = group();
    const views = new Map([[viewKey(g, g.plans[0]), view()]]);
    const { sheets, plans } = planGroupLayers([g], views, { negative: true });
    expect(sheets).toHaveLength(0);
    expect(plans[0]).toMatchObject({ blend: "screen", invert: true });
  });

  it("un groupe caché ne montre rien ; une place provisoire suit le geste", () => {
    const g = group();
    const views = new Map([[viewKey(g, g.plans[0]), view()]]);
    expect(planGroupLayers([{ ...g, hidden: true }], views, { negative: false }).plans).toHaveLength(0);
    const moved = planGroupLayers([g], views, { negative: false, override: { group: "g", origin: { x: 0, y: 5 }, mmPerUnit: 20 } });
    expect(moved.plans[0]).toMatchObject({ placement: { x: 0, y: 5, mmPerPixel: 2 } });
  });

  it("un quart de tour demande un autre affichage", () => {
    const g = group();
    expect(viewKey(g, g.plans[0])).not.toBe(viewKey({ ...g, rotation: 1 }, g.plans[0]));
  });

  it("déplace, mesure et met à la largeur", () => {
    expect(dragOrigin({ x: 10, y: 10 }, { x: 0, y: 0 }, { x: 5, y: -3 })).toEqual({ x: 15, y: 7 });
    expect(groupRect(group())).toEqual({ x: 100, y: 200, width: 4000, height: 2000 });
    expect(scaleForWidth(group(), 8000)).toBe(20);
    expect(scaleForWidth(group(), 0)).toBeNull();
    expect(parseMeters("12,5 m")).toBe(12500);
    expect(parseMeters("-3")).toBe(-3000);
    expect(parseMeters("abc")).toBeNull();
  });

  it("enchaîne les quatre points du calage", () => {
    expect(nextAlignStep("from1")).toBe("to1");
    expect(nextAlignStep("to1")).toBe("from2");
    expect(nextAlignStep("to2")).toBeNull();
  });

  it("une vue par calque coché, sinon par page cochée", () => {
    const ai: PlanInfo = { kind: "pdf", width: 1, height: 1, unit: "pt", pageCount: 1, page: 1, layers: ["Plan", "Guides", "Fresque"] };
    expect(choicesFor(ai, [1], ["Plan", "Guides"])).toEqual([
      { page: 1, layers: ["Plan"], name: "Plan" },
      { page: 1, layers: ["Guides"], name: "Guides" },
    ]);
    const pdf: PlanInfo = { kind: "pdf", width: 1, height: 1, unit: "pt", pageCount: 3, page: 1 };
    expect(choicesFor(pdf, [1, 3], [])).toEqual([{ page: 1 }, { page: 3 }]);
    const png: PlanInfo = { kind: "image", width: 1, height: 1, unit: "px" };
    expect(choicesFor(png, [], [])).toEqual([{}]);
  });
});

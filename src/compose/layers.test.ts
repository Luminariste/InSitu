import { describe, expect, it } from "vitest";
import {
  FIRST_LAYER,
  addLayer,
  effectiveLayers,
  layerOf,
  moveLayer,
  moveToLayer,
  nextLayerName,
  pickable,
  removeLayer,
  shown,
  stack,
  updateLayer,
} from "./layers";
import type { Layer, Placement } from "./placement";

function placed(id: string, layer?: string): Placement {
  return { id, drawing: "merle", center: { x: 0, y: 0 }, width: 100, height: 100, rotation: 0, layer };
}
const ids = (list: Placement[]) => list.map((p) => p.id);
const fond: Layer = { id: "fond", name: "Fond" };
const devant: Layer = { id: "devant", name: "Devant" };

describe("calques", () => {
  it("un projet sans calque montre un seul calque implicite", () => {
    expect(effectiveLayers(undefined)).toEqual([FIRST_LAYER]);
    expect(layerOf([], placed("a")).id).toBe(FIRST_LAYER.id);
  });

  it("empile par calque, puis dans l'ordre de la liste ; sans calque ou inconnu = le premier", () => {
    const list = [placed("a", "devant"), placed("b"), placed("c", "fond"), placed("d", "perdu"), placed("e", "devant")];
    expect(ids(stack(list, [fond, devant]))).toEqual(["b", "c", "d", "a", "e"]);
    expect(ids(stack(list, [devant, fond]))).toEqual(["a", "b", "d", "e", "c"]);
  });

  it("un calque masqué ne s'affiche pas ; masqué ou verrouillé, il ne se choisit pas", () => {
    const list = [placed("a", "fond"), placed("b", "devant")];
    expect(ids(shown(list, [{ ...fond, visible: false }, devant]))).toEqual(["b"]);
    expect(ids(pickable(list, [fond, { ...devant, locked: true }]))).toEqual(["a"]);
    expect(ids(pickable(list, [{ ...fond, visible: false }, devant]))).toEqual(["b"]);
  });

  it("ajouter un calque à un projet sans calque écrit les deux, et range les dessins dans le premier", () => {
    const r = addLayer({ layers: [], placements: [placed("a")] }, undefined, "neuf");
    expect(r.layers.map((l) => l.id)).toEqual([FIRST_LAYER.id, "neuf"]);
    expect(r.added.name).toBe("Calque 2");
    expect(r.placements[0].layer).toBe(FIRST_LAYER.id);
  });

  it("ajoute juste au-dessus du calque demandé", () => {
    const r = addLayer({ layers: [fond, devant], placements: [] }, "fond", "milieu");
    expect(r.layers.map((l) => l.id)).toEqual(["fond", "milieu", "devant"]);
  });

  it("nomme le calque suivant sans doublon", () => {
    expect(nextLayerName([{ id: "x", name: "Calque 4" }, fond])).toBe("Calque 5");
    expect(nextLayerName([fond, devant])).toBe("Calque 3");
  });

  it("monter un calque ne change pas le calque des dessins qui n'en avaient pas", () => {
    const r = moveLayer({ layers: [fond, devant], placements: [placed("a"), placed("b", "devant")] }, "fond", 1);
    expect(r.layers.map((l) => l.id)).toEqual(["devant", "fond"]);
    expect(r.placements.map((p) => p.layer)).toEqual(["fond", "devant"]);
    expect(ids(stack(r.placements, r.layers))).toEqual(["b", "a"]);
    // Au bord, rien ne bouge.
    expect(moveLayer(r, "fond", 1)).toBe(r);
  });

  it("supprimer un calque retire ses dessins", () => {
    const r = removeLayer({ layers: [fond, devant], placements: [placed("a"), placed("b", "devant")] }, "fond");
    expect(r.layers).toEqual([devant]);
    expect(ids(r.placements)).toEqual(["b"]);
  });

  it("masquer puis réafficher n'écrit pas la valeur par défaut", () => {
    const hidden = updateLayer({ layers: [fond], placements: [] }, "fond", { visible: false, locked: true });
    expect(hidden.layers[0]).toEqual({ ...fond, visible: false, locked: true });
    const back = updateLayer(hidden, "fond", { visible: true, locked: false });
    expect(back.layers[0]).toEqual(fond);
  });

  it("déplace des dessins vers un calque, devant ceux qui y sont", () => {
    const c = { layers: [fond, devant], placements: [placed("a", "devant"), placed("b"), placed("c")] };
    const r = moveToLayer(c, new Set(["b"]), "devant");
    expect(r.placements.map((p) => [p.id, p.layer])).toEqual([
      ["a", "devant"],
      ["c", "fond"],
      ["b", "devant"],
    ]);
    expect(ids(stack(r.placements, r.layers))).toEqual(["c", "a", "b"]);
    expect(moveToLayer(c, new Set(["b"]), "perdu")).toBe(c);
  });
});

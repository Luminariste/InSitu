import { describe, expect, it } from "vitest";
import type { Assignment } from "../engine";
import type { Placement } from "./placement";
import { dropInTree, slideTree } from "./slideTree";

const placed = (id: string, layer?: string): Placement => ({
  id,
  drawing: "d",
  center: { x: 0, y: 0 },
  width: 10,
  height: 10,
  rotation: 0,
  ...(layer ? { layer } : {}),
});

const assigned = (entries: [string, string | null, boolean?][]) =>
  new Map<string, Assignment>(
    entries.map(([placement, projector, manual]) => [
      placement,
      { placement, projector, manual: manual ?? false, whole: projector !== null, candidates: [] },
    ]),
  );

const projectors = [
  { id: "a", label: "A", registered: true },
  { id: "b", label: "B", registered: true },
  { id: "c", label: "C", registered: false },
];

describe("arborescence des diapositives", () => {
  const placements = [placed("p1"), placed("p2"), placed("p3"), placed("p4")];
  const assignments = assigned([
    ["p1", "a"],
    ["p2", "b", true],
    ["p3", "a"],
    ["p4", null],
  ]);

  it("range les dessins par diapositive, celui de devant en haut", () => {
    const groups = slideTree(placements, [], projectors, assignments);
    expect(groups.map((g) => g.label)).toEqual(["A", "B", "Hors de toute diapositive"]);
    expect(groups[0].rows.map((r) => r.placement.id)).toEqual(["p3", "p1"]);
    expect(groups[1].rows[0].manual).toBe(true);
    expect(groups[0].colorIndex).toBe(0);
    expect(groups[2].projector).toBeNull();
    expect(groups[2].rows[0].whole).toBe(false);
  });

  it("garde un groupe vide pour un projecteur recalé, pas pour un autre", () => {
    const groups = slideTree([placed("p1")], [], projectors, assigned([["p1", "a"]]));
    expect(groups.map((g) => g.projector)).toEqual(["a", "b"]);
    expect(groups[1].rows).toEqual([]);
  });

  it("suit l'ordre des calques : un calque au-dessus passe devant", () => {
    const layers = [
      { id: "fond", name: "Fond" },
      { id: "haut", name: "Haut" },
    ];
    const withLayers = [placed("p1", "haut"), placed("p3", "fond")];
    const groups = slideTree(withLayers, layers, projectors, assignments);
    expect(groups[0].rows.map((r) => r.placement.id)).toEqual(["p1", "p3"]);
  });

  it("lâché sur une autre diapositive : affecté à la main, à la même place", () => {
    const next = dropInTree(placements, assignments, "p1", { projector: "b" });
    expect(next.map((p) => p.id)).toEqual(["p1", "p2", "p3", "p4"]);
    expect(next[0].projector).toBe("b");
    // Déjà dans ce groupe, sans dessin visé : rien ne change.
    expect(dropInTree(placements, assignments, "p1", { projector: "a" })).toBe(placements);
  });

  it("lâché sur un dessin : passe juste devant lui, sur son calque", () => {
    // p1 (fond) passe devant p3, dans la même diapositive : ordre seul.
    let next = dropInTree(placements, assignments, "p1", { projector: "a", before: "p3" });
    expect(next.map((p) => p.id)).toEqual(["p2", "p3", "p1", "p4"]);
    expect(next[2].projector).toBeUndefined();
    // p3 passe devant p2, dans B : affecté à B, et sur le calque de p2.
    const layered = [placed("p1"), placed("p2", "haut"), placed("p3"), placed("p4")];
    next = dropInTree(layered, assignments, "p3", { projector: "b", before: "p2" });
    expect(next.map((p) => p.id)).toEqual(["p1", "p2", "p3", "p4"]);
    expect(next[2].projector).toBe("b");
    expect(next[2].layer).toBe("haut");
    // Sur lui-même, ou sur un dessin inconnu : rien.
    expect(dropInTree(placements, assignments, "p1", { projector: "a", before: "p1" })).toBe(placements);
    expect(dropInTree(placements, assignments, "p1", { projector: "a", before: "zz" })).toBe(placements);
  });
});

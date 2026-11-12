import { describe, expect, it } from "vitest";
import type { FacadeOverview, ProjectorOverview, Simulation, SimulationAlert } from "../engine";
import { DEFAULT_DISPLAY, projectorColor } from "./overview";
import { alertTarget, simulationLayers, simulationSummary } from "./simulation";

const square = (x: number) => [
  { x, y: 0 },
  { x: x + 100, y: 0 },
  { x: x + 100, y: 100 },
  { x, y: 100 },
];

function projector(id: string, x: number): ProjectorOverview {
  return {
    projectorId: id,
    label: id.toUpperCase(),
    pairs: 4,
    footprint: [...square(x), square(x)[0]],
    areaM2: 0.01,
    mesh: { divisions: 1, facade: square(x) },
    photoPreview: null,
    problem: null,
  };
}

const overview: FacadeOverview = { projectors: [projector("a", 0), projector("b", 50)], overlaps: [] };

const cut: SimulationAlert = {
  kind: "cut",
  placement: "p1",
  projectors: ["a"],
  message: "« étoile » est coupé.",
  bounds: { x: 40, y: 10, width: 400, height: 300 },
};
const doubled: SimulationAlert = {
  kind: "doubled",
  placement: null,
  projectors: ["a", "b"],
  message: "Lumière projetée deux fois.",
  bounds: { x: 2400, y: 500, width: 4, height: 1000 },
};

const simulation: Simulation = {
  slides: [
    { projectorId: "a", preview: "diapositives/a.png", problem: null },
    { projectorId: "b", preview: null, problem: "recalage incomplet" },
  ],
  alerts: [cut, doubled],
};

describe("simulation", () => {
  it("chaque alerte mène au dessin concerné, ou à sa zone", () => {
    const t = alertTarget(cut);
    expect(t.selection).toEqual(["p1"]);
    // Le rectangle montré contient le dessin, avec de la marge.
    expect(t.rect.x).toBeLessThan(40);
    expect(t.rect.x + t.rect.width).toBeGreaterThan(440);
    expect(t.rect.y + t.rect.height).toBeGreaterThan(310);
    // Un trait fin : un rectangle assez large pour le voir, centré dessus.
    const d = alertTarget(doubled);
    expect(d.selection).toEqual([]);
    expect(d.rect.width).toBe(200);
    expect(d.rect.x + d.rect.width / 2).toBeCloseTo(2402);
    expect(d.rect.height).toBe(1500);
  });

  it("résume les alertes par sorte", () => {
    expect(simulationSummary(simulation)).toBe("2 alertes : 1 dessin coupé, 1 projeté deux fois.");
    expect(simulationSummary({ slides: [], alerts: [] })).toMatch(/^Aucune alerte/);
  });

  it("tend chaque diapositive calculée sur son maillage, teintée de la couleur du projecteur", () => {
    const layers = simulationLayers(overview, simulation, DEFAULT_DISPLAY, true, (r) => `tuiles://${r}`);
    const slides = layers.filter((l) => l.kind === "mesh-image");
    expect(slides).toHaveLength(1);
    const a = slides[0];
    expect(a.kind === "mesh-image" && a.url).toBe("tuiles://diapositives/a.png");
    expect(a.kind === "mesh-image" && a.tint).toBe(projectorColor(0));
    expect(a.visible).toBe(true);
    const frames = layers.find((l) => l.id === "alertes");
    expect(frames?.kind === "polygons" && frames.shapes.length).toBe(2);
    // Diapositives masquées : elles restent dans la scène, cachées.
    const hidden = simulationLayers(overview, simulation, DEFAULT_DISPLAY, false, (r) => r);
    expect(hidden.find((l) => l.kind === "mesh-image")?.visible).toBe(false);
    // Projecteur masqué dans la liste : sa diapositive aussi.
    const display = { ...DEFAULT_DISPLAY, hidden: new Set(["a"]) };
    expect(simulationLayers(overview, simulation, display, true, (r) => r)[0].visible).toBe(false);
    expect(simulationLayers(overview, null, DEFAULT_DISPLAY, true, (r) => r)).toEqual([]);
  });
});

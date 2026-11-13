import { describe, expect, it } from "vitest";
import type { FacadeOverview, ProjectorOverview } from "../engine";
import {
  DEFAULT_DISPLAY,
  changedDisplay,
  nearestOpacity,
  formatArea,
  overlapLabel,
  overviewLayers,
  overviewSummary,
  projectorColor,
  visibleLayers,
  PROJECTOR_COLORS,
} from "./overview";

const square = (x: number) => [
  { x, y: 0 },
  { x: x + 100, y: 0 },
  { x: x + 100, y: 100 },
  { x, y: 100 },
  { x, y: 0 },
];

function projector(id: string, x: number, placed = true): ProjectorOverview {
  return {
    projectorId: id,
    label: id.toUpperCase(),
    pairs: placed ? 6 : 1,
    footprint: placed ? square(x) : [],
    areaM2: placed ? 0.01 : null,
    mesh: placed ? { divisions: 1, facade: square(x).slice(0, 4) } : null,
    photoPreview: placed ? `apercus/${id}.jpg` : null,
    problem: placed ? null : "Recalage incomplet : 1 paire de points sur 4.",
  };
}

const overview: FacadeOverview = {
  projectors: [projector("a", 0), projector("b", 50), projector("c", 0, false)],
  overlaps: [{ projectors: ["a", "b"], areaM2: 0.005, outlines: [square(50)] }],
};

describe("vue façade", () => {
  it("pose les photos, puis les recouvrements, puis les emprises", () => {
    const layers = overviewLayers(overview, DEFAULT_DISPLAY, (r) => `tuiles://${r}`);
    expect(layers.map((l) => l.id)).toEqual(["photo:a", "photo:b", "recouvrements", "emprises"]);
    const photo = layers[0];
    expect(photo.kind === "mesh-image" && photo.url).toBe("tuiles://apercus/a.jpg");
    const footprints = layers[3];
    expect(footprints.kind === "polygons" && footprints.shapes.length).toBe(2);
  });

  it("un projecteur masqué emporte sa photo, son emprise et ses recouvrements", () => {
    const layers = visibleLayers(overviewLayers(overview, { ...DEFAULT_DISPLAY, hidden: new Set(["b"]) }, (r) => r));
    expect(layers.map((l) => l.id)).toEqual(["photo:a", "recouvrements", "emprises"]);
    for (const l of layers) if (l.kind === "polygons") expect(l.shapes.length).toBe(l.id === "emprises" ? 1 : 0);
  });

  it("les interrupteurs retirent chaque famille de calques", () => {
    const display = { ...DEFAULT_DISPLAY, photos: false, overlaps: false };
    const layers = overviewLayers(overview, display, (r) => r);
    expect(visibleLayers(layers).map((l) => l.id)).toEqual(["emprises"]);
    // Les photos masquées restent chargées, prêtes à revenir.
    expect(layers.filter((l) => l.kind === "mesh-image").length).toBe(2);
    expect(overviewLayers(null, DEFAULT_DISPLAY, (r) => r)).toEqual([]);
  });

  it("donne une couleur stable à chaque projecteur", () => {
    expect(projectorColor(0)).toBe(PROJECTOR_COLORS[0]);
    expect(projectorColor(PROJECTOR_COLORS.length)).toBe(PROJECTOR_COLORS[0]);
  });

  it("écrit les surfaces et les résumés en français", () => {
    expect(formatArea(7.84)).toMatch(/^7,84\s?m²$/);
    expect(formatArea(null)).toBe("—");
    expect(overlapLabel(overview.overlaps[0], overview.projectors)).toBe("A et B");
    expect(overviewSummary(overview)).toBe("2 recalés sur 3 · 1 recouvrement");
  });
});

describe("vue des projecteurs dans le menu Affichage", () => {
  it("coche l'opacité proposée la plus proche", () => {
    expect(nearestOpacity(0.8)).toBe(0.8);
    expect(nearestOpacity(0.3)).toBe(0.25);
    expect(nearestOpacity(0.95)).toBe(1);
  });
  it("compte ce qui a changé, pas les projecteurs masqués", () => {
    expect(changedDisplay(DEFAULT_DISPLAY)).toBe(0);
    expect(changedDisplay({ ...DEFAULT_DISPLAY, photos: false, photoOpacity: 0.5, hidden: new Set(["a"]) })).toBe(2);
  });
});

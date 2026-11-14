import { describe, expect, it } from "vitest";
import type { FacadeOverview, ProjectorOverview } from "../engine";
import {
  DEFAULT_DISPLAY,
  parseDisplay,
  readDisplay,
  saveDisplay,
  serializeDisplay,
  withHidden,
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

describe("réglages retenus de la vue des projecteurs", () => {
  it("relit photos, emprises, recouvrements et opacité, et les projecteurs masqués du projet", () => {
    const changed = { ...DEFAULT_DISPLAY, photos: false, overlaps: false, photoOpacity: 0.25, hidden: new Set(["K", "A"]) };
    const hidden = withHidden(null, "/projets/epinal", changed.hidden);
    const back = parseDisplay(serializeDisplay(changed), hidden, "/projets/epinal");
    expect(back.photos).toBe(false);
    expect(back.footprints).toBe(true);
    expect(back.overlaps).toBe(false);
    expect(back.photoOpacity).toBe(0.25);
    expect([...back.hidden].sort()).toEqual(["A", "K"]);
  });

  it("les projecteurs masqués d'un projet ne valent pas pour un autre", () => {
    const table = withHidden(null, "/projets/epinal", new Set(["K"]));
    const other = withHidden(table, "/projets/trevarez", new Set(["B"]));
    expect([...parseDisplay(null, other, "/projets/epinal").hidden]).toEqual(["K"]);
    expect([...parseDisplay(null, other, "/projets/trevarez").hidden]).toEqual(["B"]);
    expect(parseDisplay(null, other, "/projets/autre").hidden.size).toBe(0);
    // Tout recoché : le projet disparaît de la table.
    expect(JSON.parse(withHidden(other, "/projets/epinal", new Set()))).toEqual({ "/projets/trevarez": ["B"] });
  });

  it("rien d'enregistré ou un texte abîmé : réglage d'ouverture", () => {
    const blank = parseDisplay(null, null, null);
    expect({ ...blank, hidden: [...blank.hidden] }).toEqual({ ...DEFAULT_DISPLAY, hidden: [] });
    const broken = parseDisplay("{pas du json", "[1,2]", "/p");
    expect(broken.photos).toBe(DEFAULT_DISPLAY.photos);
    expect(broken.hidden.size).toBe(0);
    expect(parseDisplay(JSON.stringify({ photoOpacity: 7 }), null, null).photoOpacity).toBe(DEFAULT_DISPLAY.photoOpacity);
  });

  it("survit à la fermeture de l'écran : relu tel qu'enregistré", () => {
    const store = new Map<string, string>();
    const fake = {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
    };
    const previous = (globalThis as { localStorage?: unknown }).localStorage;
    (globalThis as { localStorage?: unknown }).localStorage = fake;
    try {
      saveDisplay("/projets/epinal", { ...DEFAULT_DISPLAY, footprints: false, photoOpacity: 0.5, hidden: new Set(["C"]) });
      const back = readDisplay("/projets/epinal");
      expect(back.footprints).toBe(false);
      expect(back.photoOpacity).toBe(0.5);
      expect([...back.hidden]).toEqual(["C"]);
    } finally {
      (globalThis as { localStorage?: unknown }).localStorage = previous;
    }
  });
});

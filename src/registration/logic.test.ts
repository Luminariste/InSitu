import { describe, expect, it } from "vitest";
import type { FacadeOverview, RegistrationReport } from "../engine";
import {
  NO_PENDING,
  SLIDE_MM,
  click,
  facadePhotoLayer,
  formatFacadeCm,
  formatSlideMm,
  hint,
  movePoint,
  onSlide,
  pairStatus,
  removePair,
  summary,
  cornerName,
  cornerPairs,
  demoTrapezeMesh,
  facadeGrab,
  initialSide,
  locateOnSlide,
  meshCenter,
  pairTitle,
  translateFacade,
} from "./logic";

const p = (x: number, y: number) => ({ x, y });

describe("pose des points communs", () => {
  it("ajoute une paire quand les deux côtés sont posés, dans n'importe quel ordre", () => {
    let state = click([], NO_PENDING, null, "slide", p(10, 20));
    expect(state.changed).toBe(false);
    expect(state.pending.slide).toEqual(p(10, 20));
    state = click(state.pairs, state.pending, null, "facade", p(1000, 2000));
    expect(state.changed).toBe(true);
    expect(state.pairs).toEqual([{ slide: p(10, 20), facade: p(1000, 2000) }]);
    expect(state.pending).toEqual(NO_PENDING);

    const other = click(state.pairs, NO_PENDING, null, "facade", p(5, 5));
    const done = click(other.pairs, other.pending, null, "slide", p(1, 1));
    expect(done.pairs[1]).toEqual({ slide: p(1, 1), facade: p(5, 5) });
  });

  it("un second clic du même côté remplace le point en attente", () => {
    const a = click([], NO_PENDING, null, "slide", p(1, 1));
    const b = click(a.pairs, a.pending, null, "slide", p(2, 2));
    expect(b.pending.slide).toEqual(p(2, 2));
    expect(b.pairs).toHaveLength(0);
  });

  it("déplace le point du côté cliqué de la paire sélectionnée", () => {
    const pairs = [
      { slide: p(1, 1), facade: p(10, 10) },
      { slide: p(2, 2), facade: p(20, 20) },
    ];
    const moved = click(pairs, NO_PENDING, 1, "facade", p(25, 21));
    expect(moved.changed).toBe(true);
    expect(moved.pairs[1]).toEqual({ slide: p(2, 2), facade: p(25, 21) });
    expect(moved.pairs[0]).toBe(pairs[0]);
    expect(pairs[1].facade).toEqual(p(20, 20)); // pas de modification en place
  });

  it("retire une paire", () => {
    const pairs = [
      { slide: p(1, 1), facade: p(10, 10) },
      { slide: p(2, 2), facade: p(20, 20) },
    ];
    expect(removePair(pairs, 0)).toEqual([pairs[1]]);
  });

  it("refuse un point hors du carré de 280 mm", () => {
    expect(onSlide(p(0, 280))).toBe(true);
    expect(onSlide(p(-0.1, 10))).toBe(false);
    expect(onSlide(p(10, 280.5))).toBe(false);
  });
});

describe("textes", () => {
  it("écarts en mm sur la diapositive et en cm sur la façade", () => {
    expect(formatSlideMm(0.345)).toBe("0,35 mm");
    expect(formatFacadeCm(24)).toBe("2,4 cm");
    expect(formatSlideMm(null)).toBe("—");
    expect(formatFacadeCm(undefined)).toBe("—");
  });

  it("état d'un point", () => {
    expect(pairStatus(undefined)).toBe("inconnu");
    expect(pairStatus({ slideResidualMm: null, facadeResidualMm: null, outlier: false })).toBe("inconnu");
    expect(pairStatus({ slideResidualMm: 0.4, facadeResidualMm: 12, outlier: false })).toBe("bon");
    expect(pairStatus({ slideResidualMm: 1.6, facadeResidualMm: 50, outlier: false })).toBe("moyen");
    expect(pairStatus({ slideResidualMm: 6, facadeResidualMm: 200, outlier: true })).toBe("aberrant");
  });

  it("résumé selon l'avancement", () => {
    expect(summary(null, 1)).toMatch(/^Encore 3 points/);
    expect(summary(null, 3)).toMatch(/^Encore 1 point pour/);
    const base: RegistrationReport = {
      projectorId: "x",
      stage: "trapeze",
      pairs: [],
      rmsSlideMm: null,
      rmsFacadeMm: null,
      trapezeRmsSlideMm: 0,
      footprint: [],
    };
    expect(summary(base, 4)).toMatch(/^Trapèze calculé \(4 points/);
    const corrected: RegistrationReport = {
      ...base,
      stage: "corrected",
      rmsSlideMm: 0.31,
      rmsFacadeMm: 9,
      pairs: [
        { slideResidualMm: 0.3, facadeResidualMm: 9, outlier: false },
        { slideResidualMm: 7, facadeResidualMm: 210, outlier: true },
      ],
    };
    expect(summary(corrected, 8)).toBe(
      "Relief corrigé sur 8 points : écart moyen 0,31 mm sur la diapositive, 0,9 cm sur la façade ; 1 point douteux à vérifier (en rouge).",
    );
  });

  it("consigne de pose", () => {
    expect(hint(NO_PENDING, null)).toBeNull();
    expect(hint({ slide: p(1, 1), facade: null }, null)).toMatch(/sur la façade/);
    expect(hint({ slide: null, facade: p(1, 1) }, null)).toMatch(/sur la photo/);
    expect(hint(NO_PENDING, 2)).toMatch(/^Point 3/);
  });
});

describe("glisser un point", () => {
  const pairs = [
    { slide: { x: 10, y: 10 }, facade: { x: 100, y: 100 } },
    { slide: { x: 20, y: 20 }, facade: { x: 200, y: 200 } },
  ];

  it("déplace le seul point voulu, du seul côté voulu", () => {
    const moved = movePoint(pairs, 1, "facade", { x: 250, y: 260 });
    expect(moved[1]).toEqual({ slide: { x: 20, y: 20 }, facade: { x: 250, y: 260 } });
    expect(moved[0]).toBe(pairs[0]);
    expect(pairs[1].facade).toEqual({ x: 200, y: 200 });
  });

  it("garde un point de diapositive dans le carré", () => {
    expect(movePoint(pairs, 0, "slide", { x: -5, y: 300 })[0].slide).toEqual({ x: 0, y: SLIDE_MM });
  });
});

describe("photo recalée sur la façade", () => {
  const square = [p(0, 0), p(100, 0), p(0, 100), p(100, 100)];
  const overview = {
    projectors: [
      { projectorId: "a", mesh: { divisions: 1, facade: square }, photoPreview: "apercus/a.png" },
      { projectorId: "b", mesh: null, photoPreview: null },
    ],
    overlaps: [],
  } as unknown as FacadeOverview;

  it("pose la photo du projecteur choisi sur son maillage, à l'opacité choisie", () => {
    const layer = facadePhotoLayer(overview, "a", true, 0.6, (r) => `cache://${r}`);
    expect(layer?.kind).toBe("mesh-image");
    expect(layer?.kind === "mesh-image" && layer.url).toBe("cache://apercus/a.png");
    expect(layer?.kind === "mesh-image" && layer.points).toEqual(square);
    expect(layer?.opacity).toBe(0.6);
    expect(layer?.visible).toBe(true);
    expect(facadePhotoLayer(overview, "a", false, 0.6, (r) => r)?.visible).toBe(false);
  });

  it("rien tant que le projecteur n'est pas recalé", () => {
    expect(facadePhotoLayer(overview, "b", true, 0.6, (r) => r)).toBeNull();
    expect(facadePhotoLayer(overview, null, true, 0.6, (r) => r)).toBeNull();
    expect(facadePhotoLayer(null, "a", true, 0.6, (r) => r)).toBeNull();
  });
});

describe("pose directe sur la façade (D-3.2-h)", () => {
  // Diapositive posée agrandie 10 fois, coin haut-gauche en (1000, 500), légèrement en trapèze.
  const pairs = cornerPairs(p(2400, 1900), 2800);
  const skewed = pairs.map((q, i) => (i === 2 ? { ...q, facade: p(q.facade.x + 300, q.facade.y + 200) } : q));

  it("pose les quatre coins de la diapositive en carré autour d'un centre", () => {
    expect(pairs.map((q) => q.slide)).toEqual([p(0, 0), p(280, 0), p(280, 280), p(0, 280)]);
    expect(pairs.map((q) => q.facade)).toEqual([p(1000, 500), p(3800, 500), p(3800, 3300), p(1000, 3300)]);
    expect(pairs.map((_, i) => pairTitle(pairs, i))).toEqual([
      "Coin haut-gauche",
      "Coin haut-droit",
      "Coin bas-droit",
      "Coin bas-gauche",
    ]);
    expect(cornerName({ slide: p(140, 0), facade: p(0, 0) })).toBeNull();
    expect(pairTitle([...pairs, { slide: p(10, 10), facade: p(0, 0) }], 4)).toBe("Point 5");
  });

  it("prend la taille des emprises déjà recalées, sinon 40 % de la vue", () => {
    expect(initialSide({ x: 0, y: 0, width: 10000, height: 5000 }, [])).toBe(2000);
    const square = [p(0, 0), p(3000, 0), p(3000, 3000), p(0, 3000), p(0, 0)];
    expect(initialSide({ x: 0, y: 0, width: 10000, height: 5000 }, [square])).toBeCloseTo(3000, 9);
  });

  it("déplace toute la photo sans toucher à la diapositive", () => {
    const moved = translateFacade(pairs, 100, -50);
    expect(moved[0]).toEqual({ slide: p(0, 0), facade: p(1100, 450) });
  });

  it("retrouve sur la diapositive le détail attrapé sur la façade, comme il est affiché", () => {
    const mesh = demoTrapezeMesh(skewed)!;
    expect(mesh.facade).toHaveLength(17 * 17);
    // Un nœud du maillage : retrouvé exactement.
    const k = 5 * 17 + 3;
    const back = locateOnSlide(mesh, mesh.facade[k])!;
    expect(back.x).toBeCloseTo(3 * 17.5, 9);
    expect(back.y).toBeCloseTo(5 * 17.5, 9);
    // Le carré non déformé : un point quelconque, à l'échelle 10.
    const flat = demoTrapezeMesh(pairs)!;
    const inside = locateOnSlide(flat, p(1000 + 1234, 500 + 567))!;
    expect(inside.x).toBeCloseTo(123.4, 6);
    expect(inside.y).toBeCloseTo(56.7, 6);
    expect(locateOnSlide(flat, p(900, 400))).toBeNull();
    expect(meshCenter(flat)).toEqual(p(2400, 1900));
  });

  it("choisit entre point, poignée du centre, épingle et déplacement de la vue", () => {
    const mesh = demoTrapezeMesh(pairs)!;
    const base = { pointHit: null, centerDistancePx: 100, mesh, at: p(1500, 1000), photoShown: true, spaceHeld: false };
    expect(facadeGrab({ ...base, pointHit: 2 })).toEqual({ kind: "point", index: 2 });
    expect(facadeGrab({ ...base, centerDistancePx: 5 })).toEqual({ kind: "move" });
    const pin = facadeGrab(base);
    expect(pin.kind).toBe("pin");
    if (pin.kind === "pin") {
      expect(pin.slide.x).toBeCloseTo(50, 6);
      expect(pin.slide.y).toBeCloseTo(50, 6);
    }
    expect(facadeGrab({ ...base, at: p(0, 0) })).toEqual({ kind: "none" });
    expect(facadeGrab({ ...base, photoShown: false })).toEqual({ kind: "none" });
    expect(facadeGrab({ ...base, pointHit: 2, spaceHeld: true })).toEqual({ kind: "none" });
  });
});

import { describe, expect, it } from "vitest";
import type { RegistrationReport } from "../engine";
import {
  NO_PENDING,
  SLIDE_MM,
  click,
  formatFacadeCm,
  formatSlideMm,
  hint,
  movePoint,
  onSlide,
  pairStatus,
  removePair,
  summary,
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

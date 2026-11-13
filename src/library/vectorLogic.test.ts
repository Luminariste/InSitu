import { describe, expect, it } from "vitest";
import { DEFAULT_VECTOR_SETTINGS, type VectorSummary } from "../engine";
import { deviationVerdict, draftOf, fitView, isDefault, parseDraft, scaleLabel, sliderText, vectorLine, zoomAt } from "./vectorLogic";

function summary(maxDeviationPx: number): VectorSummary {
  return {
    settings: DEFAULT_VECTOR_SETTINGS,
    defaultSettings: true,
    width: 4096,
    height: 2048,
    stats: { contours: 1234, removedContours: 0, rawPoints: 99, lines: 4000, cubics: 580, maxFitErrorPx: 0.3 },
    comparison: {
      maxDeviationPx,
      rawToVectorPx: 0,
      vectorToRawPx: 0,
      sourcePixels: 0,
      vectorPixels: 0,
      mismatchedPixels: 0,
      maxMismatchDistancePx: 0,
    },
    seconds: 0.4,
    filledSvg: "vecteurs/a-trait.svg",
    outlineSvg: "vecteurs/a-contour.svg",
    image: "vignettes/a.png",
  };
}

describe("textes", () => {
  it("résume les vecteurs à la française", () => {
    expect(vectorLine(summary(0.284))).toBe("1 234 contours · 4 580 morceaux · écart 0,28 px");
  });
  it("juge l'écart au demi-pixel", () => {
    expect(deviationVerdict(summary(0.49)).good).toBe(true);
    expect(deviationVerdict(summary(0.5)).good).toBe(false);
  });
});

describe("réglages saisis", () => {
  it("relit les réglages par défaut tels qu'affichés", () => {
    const draft = draftOf(DEFAULT_VECTOR_SETTINGS);
    expect(draft.tolerancePx).toBe("0,3");
    const parsed = parseDraft(draft);
    expect("settings" in parsed && isDefault(parsed.settings)).toBe(true);
  });
  it("accepte la virgule et le point", () => {
    const parsed = parseDraft({ threshold: "200", tolerancePx: "1.5", cornerAngleDeg: "45", minAreaPx: "2,5", smoothingPx: "3,5" });
    expect(parsed).toEqual({ settings: { threshold: 200, tolerancePx: 1.5, cornerAngleDeg: 45, minAreaPx: 2.5, smoothingPx: 3.5 } });
  });
  it("un lissage vide vaut 0, et 0 reste le réglage par défaut", () => {
    const parsed = parseDraft({ ...draftOf(DEFAULT_VECTOR_SETTINGS), smoothingPx: "" });
    expect("settings" in parsed && parsed.settings.smoothingPx).toBe(0);
    expect("settings" in parsed && isDefault(parsed.settings)).toBe(true);
    // Un dessin enregistré sans lissage (champ absent) est au défaut.
    const { smoothingPx: _unused, ...withoutSmoothing } = DEFAULT_VECTOR_SETTINGS;
    expect(isDefault(withoutSmoothing)).toBe(true);
  });
  it("refuse ce que le moteur refuserait", () => {
    const base = draftOf(DEFAULT_VECTOR_SETTINGS);
    for (const wrong of [
      { ...base, threshold: "0" },
      { ...base, threshold: "12,5" },
      { ...base, tolerancePx: "0" },
      { ...base, tolerancePx: "" },
      { ...base, cornerAngleDeg: "180" },
      { ...base, minAreaPx: "-1" },
      { ...base, smoothingPx: "-1" },
      { ...base, smoothingPx: "21" },
    ]) {
      expect("error" in parseDraft(wrong)).toBe(true);
    }
  });
});

describe("curseurs", () => {
  it("écrit la valeur sans zéros inutiles", () => {
    expect(sliderText(0.35)).toBe("0,35");
    expect(sliderText(0.3)).toBe("0,3");
    expect(sliderText(2)).toBe("2");
    expect(sliderText(10)).toBe("10");
    expect(sliderText(128)).toBe("128");
    expect(sliderText(0)).toBe("0");
  });
});

describe("zoom", () => {
  it("ajuste le dessin entier au centre", () => {
    const view = fitView(4096, 2048, 1000, 600, 0);
    expect(view.scale).toBeCloseTo(1000 / 4096);
    expect(view.x).toBeCloseTo(0);
    expect(view.y).toBeCloseTo((600 - 2048 * view.scale) / 2);
  });
  it("garde le point visé sous le pointeur", () => {
    const view = { scale: 0.5, x: 10, y: 20 };
    const zoomed = zoomAt(view, 4, 300, 200, 0.01);
    const before = { x: (300 - view.x) / view.scale, y: (200 - view.y) / view.scale };
    const after = { x: (300 - zoomed.x) / zoomed.scale, y: (200 - zoomed.y) / zoomed.scale };
    expect(zoomed.scale).toBe(2);
    expect(after.x).toBeCloseTo(before.x);
    expect(after.y).toBeCloseTo(before.y);
  });
  it("affiche l'échelle en pour cent", () => {
    expect(scaleLabel(1)).toBe("100 %");
    expect(scaleLabel(0.05)).toBe("5,0 %");
  });
});

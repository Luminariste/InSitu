import { describe, expect, it } from "vitest";
import { DEFAULT_VIEW_OPTIONS, changedViewOptions, parseViewOptions, toggleViewOption } from "./viewOptions";

describe("options d'affichage de la façade", () => {
  it("montre tout, sans négatif, par défaut", () => {
    expect(DEFAULT_VIEW_OPTIONS).toEqual({ plan: true, planNegative: false, fresco: true, drawings: true, negative: false });
    expect(changedViewOptions(DEFAULT_VIEW_OPTIONS)).toBe(0);
  });

  it("cache et remontre le plan", () => {
    const hidden = toggleViewOption(DEFAULT_VIEW_OPTIONS, "plan");
    expect(hidden.plan).toBe(false);
    expect(hidden.fresco).toBe(true);
    expect(changedViewOptions(hidden)).toBe(1);
    expect(toggleViewOption(hidden, "plan")).toEqual(DEFAULT_VIEW_OPTIONS);
  });

  it("relit ce qui a été enregistré", () => {
    const saved = JSON.stringify({ ...DEFAULT_VIEW_OPTIONS, fresco: false, negative: true });
    expect(parseViewOptions(saved)).toEqual({ ...DEFAULT_VIEW_OPTIONS, fresco: false, negative: true });
  });

  it("complète un enregistrement ancien ou abîmé", () => {
    expect(parseViewOptions(null)).toEqual(DEFAULT_VIEW_OPTIONS);
    expect(parseViewOptions("pas du json")).toEqual(DEFAULT_VIEW_OPTIONS);
    expect(parseViewOptions(JSON.stringify({ plan: false, drawings: "oui" }))).toEqual({ ...DEFAULT_VIEW_OPTIONS, plan: false });
  });
});

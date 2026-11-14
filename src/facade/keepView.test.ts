import { describe, expect, it } from "vitest";
import { parseKeptView, viewToRestore } from "./keepView";

describe("vue gardée après ⌘Z (D15-l)", () => {
  const view = { centerX: 1200, centerY: 800, zoom: 0.4 };

  it("remet le même zoom au même endroit", () => {
    expect(viewToRestore({ view, k: 2.5 }, 2.5)).toEqual(view);
  });

  it("recadre si l'échelle du plan a changé, ou sans vue gardée", () => {
    expect(viewToRestore({ view, k: 2.5 }, 3)).toBeNull();
    expect(viewToRestore({ view, k: null }, 2.5)).toBeNull();
    expect(viewToRestore(null, 2.5)).toBeNull();
  });
});

describe("vue retenue d'un lancement à l'autre (D15-v)", () => {
  it("relit une vue enregistrée et refuse une vue abîmée", () => {
    const kept = { view: { centerX: 1200, centerY: 340.5, zoom: 0.8 }, k: 2.5 };
    expect(parseKeptView(JSON.parse(JSON.stringify(kept)))).toEqual(kept);
    expect(parseKeptView({ view: { centerX: 1, centerY: 2, zoom: 1 }, k: null })?.k).toBeNull();
    expect(parseKeptView({ view: { centerX: 1, centerY: 2, zoom: 0 }, k: 1 })).toBeUndefined();
    expect(parseKeptView({ view: { centerX: "a", centerY: 2, zoom: 1 }, k: 1 })).toBeUndefined();
    expect(parseKeptView(null)).toBeUndefined();
  });
});

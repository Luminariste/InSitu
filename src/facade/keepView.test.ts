import { describe, expect, it } from "vitest";
import { viewToRestore } from "./keepView";

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

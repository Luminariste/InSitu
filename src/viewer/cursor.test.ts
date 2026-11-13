import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { setViewerCursor } from "./cursor";

function fakeElement() {
  const props = new Map<string, string>();
  const style = {
    cursor: "",
    setProperty: (name: string, value: string) => props.set(name, value),
    removeProperty: (name: string) => props.delete(name),
  };
  return { element: { style } as unknown as HTMLElement, props, style };
}

describe("curseur de la visionneuse", () => {
  it("la zone de la visionneuse lit le curseur posé par l'écran, la main sinon", () => {
    // Retour d'essai de la 0.1.39 : « cursor: grab » écrasait le curseur des poignées.
    const css = readFileSync(new URL("./viewer.css", import.meta.url), "utf8");
    const block = css.slice(css.indexOf(".viewer-canvas {"), css.indexOf("}", css.indexOf(".viewer-canvas {")));
    expect(block).toContain("cursor: var(--viewer-cursor, grab)");
  });

  it("pose puis retire le curseur", () => {
    const { element, props, style } = fakeElement();
    setViewerCursor(element, "nwse-resize");
    expect(props.get("--viewer-cursor")).toBe("nwse-resize");
    expect(style.cursor).toBe("nwse-resize");
    setViewerCursor(element, "");
    expect(props.has("--viewer-cursor")).toBe(false);
    expect(style.cursor).toBe("");
    setViewerCursor(null, "move");
  });
});

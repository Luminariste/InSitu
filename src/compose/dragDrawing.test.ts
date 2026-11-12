import { describe, expect, it } from "vitest";
import { DRAWING_DRAG_TYPE, droppedDrawingId } from "./dragDrawing";

const data = (values: Record<string, string>) => ({ getData: (type: string) => values[type] ?? "" });

describe("dessin lâché sur la façade", () => {
  it("se lit dans les données du glisser, type maison ou texte", () => {
    expect(droppedDrawingId(data({ [DRAWING_DRAG_TYPE]: "d1" }), null)).toBe("d1");
    expect(droppedDrawingId(data({ "text/plain": "insitu-dessin:d2" }), null)).toBe("d2");
  });

  it("à défaut, c'est le dessin dont le glisser est en cours", () => {
    expect(droppedDrawingId(data({}), "d3")).toBe("d3");
    expect(droppedDrawingId(data({ "text/plain": "autre chose" }), "d3")).toBe("d3");
    expect(droppedDrawingId(data({}), null)).toBeNull();
  });
});

import { describe, expect, it } from "vitest";
import {
  addPoint,
  clearStrokes,
  defaultSize,
  finishStroke,
  nextSize,
  pushStroke,
  redoStroke,
  retouchLine,
  sameStrokes,
  startHistory,
  undoStroke,
} from "./retouchLogic";

const size = { width: 2000, height: 1000 };

describe("gomme et crayon", () => {
  it("garde les points espacés d'au moins quelques pixels", () => {
    let points: [number, number][] = [];
    points = addPoint(points, [0.1, 0.1], size, 2);
    points = addPoint(points, [0.1005, 0.1], size, 2); // 1 px : ignoré
    points = addPoint(points, [0.102, 0.1], size, 2); // 4 px : gardé
    expect(points).toEqual([
      [0.1, 0.1],
      [0.102, 0.1],
    ]);
  });

  it("donne l'épaisseur en fraction du plus grand côté", () => {
    const stroke = finishStroke("erase", "trait", 20, [[0.5, 0.5]], size);
    expect(stroke).toEqual({ tool: "erase", size: 0.01, points: [[0.5, 0.5]] });
    expect(finishStroke("draw", "trait", 20, [], size)).toBeNull();
  });

  it("refuse une zone de moins de trois points", () => {
    expect(finishStroke("draw", "zone", 20, [[0, 0], [1, 1]], size)).toBeNull();
    expect(finishStroke("draw", "zone", 20, [[0, 0], [1, 0], [1, 1]], size)).toEqual({
      tool: "draw",
      fill: true,
      size: 0,
      points: [[0, 0], [1, 0], [1, 1]],
    });
  });

  it("annule et rétablit les retouches une à une", () => {
    const a = finishStroke("erase", "trait", 10, [[0.1, 0.1]], size)!;
    const b = finishStroke("draw", "trait", 10, [[0.2, 0.2]], size)!;
    let history = pushStroke(pushStroke(startHistory([]), a), b);
    history = undoStroke(history);
    expect(history.strokes).toEqual([a]);
    history = redoStroke(history);
    expect(history.strokes).toEqual([a, b]);
    // Un nouveau trait efface ce qu'on pouvait rétablir.
    history = pushStroke(undoStroke(history), a);
    expect(redoStroke(history).strokes).toEqual([a, a]);
    // Revenir au dessin d'origine s'annule aussi, dans l'ordre.
    const cleared = clearStrokes(pushStroke(startHistory([a]), b));
    expect(cleared.strokes).toEqual([]);
    expect(redoStroke(cleared).strokes).toEqual([a]);
    expect(redoStroke(redoStroke(cleared)).strokes).toEqual([a, b]);
    expect(sameStrokes(redoStroke(redoStroke(cleared)).strokes, [a, b])).toBe(true);
  });

  it("propose des épaisseurs à la taille du dessin", () => {
    expect(defaultSize(4096, 4096)).toBe(50);
    expect(defaultSize(400, 300)).toBe(3);
    expect(nextSize(5, 1, 4096, 4096)).toBe(8);
    expect(nextSize(5, -1, 4096, 4096)).toBe(3);
    expect(nextSize(1, -1, 4096, 4096)).toBe(1);
  });

  it("résume les retouches", () => {
    expect(retouchLine(undefined)).toBe("Dessin d'origine, sans retouche");
    const a = finishStroke("erase", "trait", 10, [[0.1, 0.1]], size)!;
    const b = finishStroke("draw", "zone", 10, [[0, 0], [1, 0], [1, 1]], size)!;
    expect(retouchLine([a, a, b])).toBe("3 retouches (2 à la gomme, 1 au crayon)");
    expect(retouchLine([a])).toBe("1 retouche (1 à la gomme)");
  });
});

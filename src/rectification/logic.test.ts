import { describe, expect, it } from "vitest";
import type { RectificationView } from "../engine";
import {
  defaultCorners,
  exportFileName,
  hitHandle,
  isConvexQuad,
  loupePosition,
  loupeTiles,
  nextProjectorLabel,
  nodeQuality,
  shortestSide,
  slidePixels,
  summarize,
} from "./logic";

describe("coins posés à la main", () => {
  it("part d'un carré centré, dans l'ordre haut-gauche, haut-droit, bas-droit, bas-gauche", () => {
    const [a, b, c, d] = defaultCorners(4032, 3024);
    const side = 0.6 * 3024;
    expect(a).toEqual({ x: (4032 - side) / 2, y: (3024 - side) / 2 });
    expect(b.x - a.x).toBeCloseTo(side);
    expect(c.y - b.y).toBeCloseTo(side);
    expect(d.x).toBeCloseTo(a.x);
    expect(isConvexQuad([a, b, c, d])).toBe(true);
  });

  it("saisit la poignée la plus proche, et seulement à portée", () => {
    const handles = [
      { x: 0, y: 0 },
      { x: 100, y: 0 },
      { x: 100, y: 100 },
      { x: 0, y: 100 },
    ];
    expect(hitHandle(handles, { x: 5, y: 3 }, 16)).toBe(0);
    expect(hitHandle(handles, { x: 92, y: 95 }, 16)).toBe(2);
    expect(hitHandle(handles, { x: 50, y: 50 }, 16)).toBeNull();
    // Deux poignées à portée : la plus proche gagne.
    expect(hitHandle([{ x: 0, y: 0 }, { x: 10, y: 0 }], { x: 7, y: 0 }, 16)).toBe(1);
  });

  it("refuse des coins croisés ou alignés", () => {
    const square = [
      { x: 0, y: 0 },
      { x: 10, y: 0 },
      { x: 10, y: 10 },
      { x: 0, y: 10 },
    ];
    expect(isConvexQuad(square)).toBe(true);
    // Mire vue de biais : trapèze, toujours convexe.
    expect(isConvexQuad([{ x: 2, y: 0 }, { x: 8, y: 1 }, { x: 10, y: 10 }, { x: 0, y: 9 }])).toBe(true);
    // Coins 2 et 3 inversés : le quadrilatère se croise.
    expect(isConvexQuad([square[0], square[2], square[1], square[3]])).toBe(false);
    expect(isConvexQuad([{ x: 0, y: 0 }, { x: 5, y: 0 }, { x: 10, y: 0 }, { x: 0, y: 10 }])).toBe(false);
    expect(isConvexQuad(square.slice(0, 3))).toBe(false);
    expect(shortestSide(square)).toBe(10);
  });
});

describe("affichage du résultat", () => {
  it("classe les croisements selon leur écart", () => {
    expect(nodeQuality(0.05)).toBe("bon");
    expect(nodeQuality(0.15)).toBe("moyen");
    expect(nodeQuality(0.29)).toBe("moyen");
    expect(nodeQuality(0.3)).toBe("mauvais");
  });

  it("résume la détection en français", () => {
    const view = {
      method: "automatic",
      nodes: new Array(746).fill({ photo: [0, 0], slide: [0, 0], errorMm: 0.05 }),
      measurableNodes: 804,
      leaveOneOutMm: { mean: 0.058, p95: 0.161, max: 0.3 },
      coverage: 0.97,
      seconds: 9.04,
    } as unknown as RectificationView;
    const { text, quality } = summarize(view);
    expect(text).toContain("détectée automatiquement");
    expect(text).toContain("746 croisements sur 804");
    expect(text).toContain("0,06 mm en moyenne");
    expect(text).toContain("0,16 mm pour 95 %");
    expect(text).toContain("couverte à 97 %");
    expect(text).toContain("calcul 9 s");
    expect(quality).toBe("bon");
    const p95 = (v: number) => ({ ...view, leaveOneOutMm: { mean: 0.1, p95: v, max: 1 } });
    expect(summarize(p95(0.35)).quality).toBe("moyen");
    expect(summarize(p95(0.7)).quality).toBe("mauvais");
    expect(summarize({ ...view, coverage: 0.5 }).quality).toBe("mauvais");
    expect(summarize({ ...view, method: "manual", seconds: null }).text).toContain("coins posés à la main");
  });
});

describe("export et projecteurs", () => {
  it("propose un nom de fichier lisible", () => {
    expect(exportFileName("A", 500, "tif")).toBe("A-redressee-500dpi.tif");
    expect(exportFileName(" Est/2 ", 1400, "png")).toBe("Est-2-redressee-1400dpi.png");
    expect(exportFileName("", 300, "tif")).toBe("projecteur-redressee-300dpi.tif");
  });

  it("donne la taille en pixels comme le moteur", () => {
    expect(slidePixels(500)).toBe(5512);
    expect(slidePixels(1400)).toBe(15433);
    expect(slidePixels(300)).toBe(3307);
  });

  it("nomme le projecteur suivant par la première lettre libre", () => {
    expect(nextProjectorLabel([])).toBe("A");
    expect(nextProjectorLabel(["A", "b", "D"])).toBe("C");
    const all = Array.from({ length: 26 }, (_, i) => String.fromCharCode(65 + i));
    expect(nextProjectorLabel(all)).toBe("P1");
  });
});

describe("loupe", () => {
  const pyramid = {
    version: 1,
    width: 1200,
    height: 700,
    tileSize: 512,
    format: "jpeg" as const,
    channels: 3,
    levels: [{ level: 0, width: 1200, height: 700, columns: 3, rows: 2 }],
  };

  it("ne charge que les tuiles pleine résolution sous la loupe", () => {
    // Centre sur le coin commun des quatre premières tuiles.
    const tiles = loupeTiles(pyramid, { x: 512, y: 512 }, 4, 240);
    expect(tiles.map((t) => t.path)).toEqual(["0/0_0.jpg", "0/1_0.jpg", "0/0_1.jpg", "0/1_1.jpg"]);
    // La tuile 1_1 commence au centre de la loupe.
    expect(tiles[3]).toMatchObject({ left: 120, top: 120, width: 512 * 4, height: 188 * 4 });
    // Loin des bords : une seule tuile ; hors de l'image : rien en trop.
    expect(loupeTiles(pyramid, { x: 200, y: 200 }, 4, 240)).toHaveLength(1);
    expect(loupeTiles(pyramid, { x: 1195, y: 695 }, 4, 240).map((t) => t.path)).toEqual(["0/2_1.jpg"]);
  });

  it("place un point de la photo dans la loupe", () => {
    expect(loupePosition({ x: 10, y: 20 }, { x: 10, y: 20 }, 4, 240)).toEqual({ x: 120, y: 120 });
    expect(loupePosition({ x: 12, y: 19 }, { x: 10, y: 20 }, 4, 240)).toEqual({ x: 128, y: 116 });
  });
});

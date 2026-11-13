import { describe, expect, it } from "vitest";
import type { SilhouettePolygon } from "../engine";
import {
  edgeAt,
  insertVertex,
  moveVertex,
  nudgeVertex,
  polygonsPath,
  removeVertex,
  samePolygons,
  silhouetteLine,
  squareAt,
  vertexAt,
} from "./silhouetteLogic";

const square: SilhouettePolygon = [
  [0.25, 0.25],
  [0.75, 0.25],
  [0.75, 0.75],
  [0.25, 0.75],
];
// Dessin allongé : 2 000 × 1 000 px.
const size = { width: 2000, height: 1000 };

describe("retouche de silhouette", () => {
  it("trouve le point le plus proche, en pixels du dessin", () => {
    expect(vertexAt([square], [0.752, 0.25], size, 8)).toEqual({ polygon: 0, index: 1 });
    // 0,01 en largeur = 20 px : trop loin.
    expect(vertexAt([square], [0.76, 0.25], size, 8)).toBeNull();
    // 0,005 en hauteur = 5 px : assez près.
    expect(vertexAt([square], [0.75, 0.755], size, 8)).toEqual({ polygon: 0, index: 2 });
  });

  it("trouve un bord et y ajoute un point", () => {
    const edge = edgeAt([square], [0.5, 0.252], size, 4);
    expect(edge).toMatchObject({ polygon: 0, index: 0 });
    expect(edge?.point[0]).toBeCloseTo(0.5);
    expect(edge?.point[1]).toBeCloseTo(0.25);
    // Le bord qui ferme le contour (du dernier point au premier).
    expect(edgeAt([square], [0.25, 0.5], size, 4)).toMatchObject({ index: 3 });
    expect(edgeAt([square], [0.5, 0.5], size, 4)).toBeNull();
    const { polygons, added } = insertVertex([square], { polygon: 0, index: 0 }, [0.5, 0.2]);
    expect(polygons[0]).toHaveLength(5);
    expect(polygons[0][1]).toEqual([0.5, 0.2]);
    expect(added).toEqual({ polygon: 0, index: 1 });
  });

  it("déplace un point sans sortir du dessin", () => {
    const moved = moveVertex([square], { polygon: 0, index: 0 }, [-0.2, 0.1]);
    expect(moved[0][0]).toEqual([0, 0.1]);
    expect(moved[0][1]).toBe(square[1]);
    expect(square[0]).toEqual([0.25, 0.25]);
  });

  it("décale un point aux flèches d'un point d'écran, selon le zoom (D15-p)", () => {
    // Dessin de 200 × 100 affiché à 2 points d'écran par unité : 10 points d'écran = 5 unités.
    const moved = nudgeVertex([square], { polygon: 0, index: 0 }, { x: 10, y: -10 }, 2, { width: 200, height: 100 });
    expect(moved[0][0][0]).toBeCloseTo(0.25 + 5 / 200);
    expect(moved[0][0][1]).toBeCloseTo(0.25 - 5 / 100);
    expect(nudgeVertex([square], { polygon: 3, index: 0 }, { x: 1, y: 0 }, 2, { width: 1, height: 1 })).toEqual([square]);
  });

  it("retire un point, et le contour entier sous trois points", () => {
    const five = insertVertex([square], { polygon: 0, index: 0 }, [0.5, 0.2]).polygons;
    expect(removeVertex(five, { polygon: 0, index: 1 })).toEqual([square]);
    const triangle: SilhouettePolygon = square.slice(0, 3);
    expect(removeVertex([triangle, square], { polygon: 0, index: 2 })).toEqual([square]);
  });

  it("carré neuf, textes, comparaison et tracé", () => {
    const fresh = squareAt([0.98, 0.5]);
    expect(Math.max(...fresh.map((p) => p[0]))).toBeCloseTo(1);
    expect(silhouetteLine([square], false)).toBe("Détectée automatiquement · 1 contour");
    expect(silhouetteLine([square, square], true)).toBe("Retouchée à la main · 2 contours");
    expect(silhouetteLine([], true)).toBe("Aucune : ce dessin ne cache rien");
    expect(samePolygons([square], [square.map((p) => [...p] as [number, number])])).toBe(true);
    expect(samePolygons([square], [fresh])).toBe(false);
    expect(polygonsPath([square], size)).toBe("M500.00 250.00L1500.00 250.00L1500.00 750.00L500.00 750.00Z");
  });
});

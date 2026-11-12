import { describe, expect, it } from "vitest";
import {
  facadePlacement,
  formatLength,
  isClick,
  mmPerSourceUnit,
  parseDistanceMm,
  projectNameFromPath,
  sourceToView,
  viewToSource,
  withProjectExtension,
} from "./scale";

describe("parseDistanceMm", () => {
  it("lit les millimètres par défaut", () => {
    expect(parseDistanceMm("12500")).toBe(12500);
    expect(parseDistanceMm(" 350 mm ")).toBe(350);
  });
  it("lit les mètres et centimètres, virgule ou point", () => {
    expect(parseDistanceMm("12,5 m")).toBe(12500);
    expect(parseDistanceMm("12.5m")).toBe(12500);
    expect(parseDistanceMm("1250 cm")).toBe(12500);
    expect(parseDistanceMm("35,28 M")).toBeCloseTo(35280, 6);
  });
  it("accepte les espaces entre milliers", () => {
    expect(parseDistanceMm("12 500")).toBe(12500);
    expect(parseDistanceMm("12 500 mm")).toBe(12500);
  });
  it("refuse ce qui n'est pas une longueur positive", () => {
    for (const text of ["", "abc", "0", "0 m", "-3 m", "12 km", "1,2,3"]) {
      expect(parseDistanceMm(text)).toBeNull();
    }
  });
});

describe("mise à l'échelle", () => {
  // Cote de 1 000 pt sur un plan au 1:100 : 352,78 mm × 100.
  const scale = { pointA: { x: 200, y: 1484 }, pointB: { x: 1200, y: 1484 }, distanceMm: 35277.78 };

  it("donne les millimètres par unité du fichier", () => {
    expect(mmPerSourceUnit(scale)).toBeCloseTo(35.27778, 5);
    expect(mmPerSourceUnit(null)).toBeNull();
    expect(mmPerSourceUnit({ ...scale, pointB: scale.pointA })).toBeNull();
  });

  it("place le plan en mm réels une fois à l'échelle", () => {
    // Un PDF A1 de 2 384 pt rendu sur 65 536 px.
    const unitsPerPixel = 2384 / 65536;
    const placed = facadePlacement(unitsPerPixel, mmPerSourceUnit(scale));
    expect(65536 * placed.mmPerPixel).toBeCloseTo(2384 * 35.27778, 0);
    // Avant mise à l'échelle : l'unité du fichier.
    expect(65536 * facadePlacement(unitsPerPixel, null).mmPerPixel).toBeCloseTo(2384, 9);
  });

  it("passe de la visionneuse au fichier et retour", () => {
    const k = mmPerSourceUnit(scale);
    const point = { x: 1000.3, y: 842 };
    const back = viewToSource(sourceToView(point, k), k);
    expect(back.x).toBeCloseTo(point.x, 9);
    expect(back.y).toBeCloseTo(point.y, 9);
    expect(viewToSource(point, null)).toEqual(point);
  });
});

describe("présentation", () => {
  it("écrit les longueurs en m ou en mm", () => {
    expect(formatLength(84102.2)).toMatch(/^84,10\s?m$/);
    expect(formatLength(350)).toMatch(/^350\s?mm$/);
  });
  it("distingue un clic d'un glisser", () => {
    expect(isClick({ x: 10, y: 10, t: 0 }, { x: 12, y: 11, t: 150 })).toBe(true);
    expect(isClick({ x: 10, y: 10, t: 0 }, { x: 30, y: 10, t: 150 })).toBe(false);
    expect(isClick({ x: 10, y: 10, t: 0 }, { x: 10, y: 10, t: 2000 })).toBe(false);
  });
  it("nomme le dossier de projet", () => {
    expect(withProjectExtension("/Users/b/Épinal")).toBe("/Users/b/Épinal.insitu");
    expect(withProjectExtension("/Users/b/Épinal.insitu/")).toBe("/Users/b/Épinal.insitu");
    expect(projectNameFromPath("/Users/b/Épinal.insitu")).toBe("Épinal");
  });
});

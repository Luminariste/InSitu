import { describe, expect, it } from "vitest";
import {
  actualSizeZoom,
  classifyWheel,
  fitView,
  keepInBounds,
  mmToScreen,
  panBy,
  pickLevel,
  placedBounds,
  placementForWidth,
  screenToMm,
  tileAncestor,
  tileCount,
  tilePath,
  tileRect,
  unionRects,
  visibleTileRange,
  wheelZoomFactor,
  zoomAt,
  zoomPercent,
  type Pyramid,
  type ViewState,
} from "./geometry";

/** Même calcul que le moteur (`level_layout` dans tiles.rs). */
function pyramidOf(width: number, height: number, tileSize = 512, format: Pyramid["format"] = "png"): Pyramid {
  const levels = [];
  let w = width;
  let h = height;
  for (;;) {
    levels.push({
      level: levels.length,
      width: w,
      height: h,
      columns: Math.ceil(w / tileSize),
      rows: Math.ceil(h / tileSize),
    });
    if (w <= tileSize && h <= tileSize) break;
    w = Math.ceil(w / 2);
    h = Math.ceil(h / 2);
  }
  return { version: 1, width, height, tileSize, format, channels: 1, levels };
}

const SLIDE_MM = 280;
const engraving = pyramidOf(15_433, 15_433);
const photo = pyramidOf(5_512, 5_512, 512, "jpeg");
const engravingPlacement = placementForWidth(engraving.width, SLIDE_MM);
const photoPlacement = placementForWidth(photo.width, SLIDE_MM);
const viewport = { width: 1200, height: 800 };

describe("conversions mm ↔ écran", () => {
  const view: ViewState = { centerX: 140, centerY: 140, zoom: 2.5 };

  it("place le centre de la vue au centre de l'écran", () => {
    expect(mmToScreen(view, viewport, { x: 140, y: 140 })).toEqual({ x: 600, y: 400 });
    expect(mmToScreen(view, viewport, { x: 150, y: 130 })).toEqual({ x: 625, y: 375 });
  });

  it("fait l'aller-retour sans perte", () => {
    for (const point of [
      { x: 0, y: 0 },
      { x: 279.99, y: 0.01 },
      { x: -35.2, y: 412.7 },
    ]) {
      const back = screenToMm(view, viewport, mmToScreen(view, viewport, point));
      expect(back.x).toBeCloseTo(point.x, 9);
      expect(back.y).toBeCloseTo(point.y, 9);
    }
  });

  it("superpose exactement deux images de tailles différentes étalées sur 280 mm", () => {
    const a = placedBounds(engraving, engravingPlacement);
    const b = placedBounds(photo, photoPlacement);
    expect(a.width).toBeCloseTo(280, 9);
    expect(b.width).toBeCloseTo(280, 9);
    expect(a).toEqual({ x: 0, y: 0, width: a.width, height: a.height });
    // Le même point de la diapositive, vu dans chaque image.
    const mm = { x: 70, y: 210 };
    expect(mm.x / engravingPlacement.mmPerPixel).toBeCloseTo(15_433 / 4, 6);
    expect(mm.x / photoPlacement.mmPerPixel).toBeCloseTo(5_512 / 4, 6);
  });

  it("réunit des rectangles", () => {
    expect(unionRects([])).toBeNull();
    expect(
      unionRects([
        { x: 0, y: 0, width: 280, height: 280 },
        { x: 250, y: -20, width: 280, height: 100 },
      ]),
    ).toEqual({ x: 0, y: -20, width: 530, height: 300 });
  });
});

describe("zoom et déplacement", () => {
  it("garde fixe le point sous le curseur pendant un zoom", () => {
    const view: ViewState = { centerX: 100, centerY: 80, zoom: 3 };
    const cursor = { x: 937, y: 121 };
    const before = screenToMm(view, viewport, cursor);
    for (const factor of [1.07, 0.5, 12, 0.01]) {
      const zoomed = zoomAt(view, viewport, cursor, factor);
      expect(zoomed.zoom).toBeCloseTo(3 * factor, 9);
      const after = screenToMm(zoomed, viewport, cursor);
      expect(after.x).toBeCloseTo(before.x, 9);
      expect(after.y).toBeCloseTo(before.y, 9);
    }
  });

  it("respecte les limites de zoom, toujours autour du curseur", () => {
    const view: ViewState = { centerX: 100, centerY: 80, zoom: 3 };
    const cursor = { x: 10, y: 790 };
    const before = screenToMm(view, viewport, cursor);
    const zoomed = zoomAt(view, viewport, cursor, 1000, { min: 0.5, max: 40 });
    expect(zoomed.zoom).toBe(40);
    expect(screenToMm(zoomed, viewport, cursor).x).toBeCloseTo(before.x, 9);
    expect(zoomAt(view, viewport, cursor, 0.0001, { min: 0.5, max: 40 }).zoom).toBe(0.5);
  });

  it("déplace le contenu avec le doigt", () => {
    const view: ViewState = { centerX: 100, centerY: 80, zoom: 4 };
    const grabbed = screenToMm(view, viewport, { x: 300, y: 300 });
    const moved = panBy(view, 40, -20);
    expect(mmToScreen(moved, viewport, grabbed)).toEqual({ x: 340, y: 280 });
  });

  it("ne laisse pas le centre de l'écran quitter le contenu", () => {
    const bounds = { x: 0, y: 0, width: 280, height: 280 };
    const inside: ViewState = { centerX: 100, centerY: 279, zoom: 4 };
    expect(keepInBounds(inside, bounds)).toBe(inside);
    expect(keepInBounds({ centerX: -500, centerY: 900, zoom: 4 }, bounds)).toEqual({
      centerX: 0,
      centerY: 280,
      zoom: 4,
    });
  });

  it("montre tout, centré, avec une marge", () => {
    const view = fitView({ x: 0, y: 0, width: 280, height: 280 }, viewport, 0.05);
    expect(view.centerX).toBe(140);
    expect(view.centerY).toBe(140);
    // La hauteur est la contrainte : 280 mm sur 90 % de 800 px.
    expect(view.zoom).toBeCloseTo(720 / 280, 9);
    const corner = mmToScreen(view, viewport, { x: 280, y: 280 });
    expect(corner.y).toBeCloseTo(760, 9);
  });

  it("définit 100 % comme un pixel d'image par pixel physique d'écran", () => {
    for (const ratio of [1, 2]) {
      const zoom = actualSizeZoom(engravingPlacement.mmPerPixel, ratio);
      expect(zoomPercent(zoom, engravingPlacement.mmPerPixel, ratio)).toBeCloseTo(100, 9);
      // 15 433 pixels d'image occupent 15 433 pixels physiques.
      expect(SLIDE_MM * zoom * ratio).toBeCloseTo(15_433, 6);
    }
  });
});

describe("molette et trackpad", () => {
  const base = { deltaX: 0, deltaY: 0, deltaMode: 0, ctrlKey: false, metaKey: false };

  it("distingue pincement, molette et défilement à deux doigts", () => {
    expect(classifyWheel({ ...base, deltaY: -3.2, ctrlKey: true })).toBe("zoom-pinch");
    expect(classifyWheel({ ...base, deltaY: 12, metaKey: true })).toBe("zoom-wheel");
    expect(classifyWheel({ ...base, deltaY: 100, wheelDeltaY: -120 })).toBe("zoom-wheel");
    expect(classifyWheel({ ...base, deltaY: 3, deltaMode: 1 })).toBe("zoom-wheel");
    expect(classifyWheel({ ...base, deltaY: 7, wheelDeltaY: -21 })).toBe("pan");
    expect(classifyWheel({ ...base, deltaX: 4, deltaY: 40, wheelDeltaY: -120 })).toBe("pan");
    expect(classifyWheel({ ...base, deltaX: -9 })).toBe("pan");
  });

  it("agrandit quand la molette monte, et borne les à-coups", () => {
    const up = { ...base, deltaY: -100 };
    const down = { ...base, deltaY: 100 };
    expect(wheelZoomFactor(up, "zoom-wheel")).toBeGreaterThan(1);
    expect(wheelZoomFactor(up, "zoom-wheel") * wheelZoomFactor(down, "zoom-wheel")).toBeCloseTo(1, 12);
    expect(wheelZoomFactor({ ...base, deltaY: -1e6 }, "zoom-wheel")).toBe(
      wheelZoomFactor({ ...base, deltaY: -300 }, "zoom-wheel"),
    );
    // Un pincement est plus sensible qu'une molette.
    expect(wheelZoomFactor({ ...base, deltaY: -5 }, "zoom-pinch")).toBeGreaterThan(
      wheelZoomFactor({ ...base, deltaY: -5 }, "zoom-wheel"),
    );
  });
});

describe("choix du niveau", () => {
  const levels = engraving.levels.length; // 6 : 15 433, 7 717, 3 859, 1 930, 965, 483
  const mmPerPixel = engravingPlacement.mmPerPixel;
  const at = (percent: number, ratio = 1) =>
    pickLevel(levels, mmPerPixel, (actualSizeZoom(mmPerPixel, ratio) * percent) / 100, ratio);

  it("prend la pleine résolution à 100 % et au-delà", () => {
    expect(at(100)).toBe(0);
    expect(at(800)).toBe(0);
  });

  it("ne réduit jamais une tuile de plus de la moitié, et ne l'agrandit jamais", () => {
    expect(at(99)).toBe(0);
    expect(at(50.1)).toBe(0);
    expect(at(50)).toBe(1);
    expect(at(25.1)).toBe(1);
    expect(at(25)).toBe(2);
    expect(at(12.5)).toBe(3);
    expect(at(6.25)).toBe(4);
    expect(at(3.125)).toBe(5);
  });

  it("s'arrête au dernier niveau", () => {
    expect(at(0.5)).toBe(5);
    expect(at(0.0001)).toBe(5);
  });

  it("tient compte des écrans Retina", () => {
    // Même zoom en pixels CSS : l'écran Retina a besoin d'un niveau plus fin.
    const zoom = actualSizeZoom(mmPerPixel, 1) / 4;
    expect(pickLevel(levels, mmPerPixel, zoom, 1)).toBe(2);
    expect(pickLevel(levels, mmPerPixel, zoom, 2)).toBe(1);
  });

  it("choisit le niveau de chaque calque d'après sa propre résolution", () => {
    // Vue d'ensemble sur 720 px : le Verre descend à 1/16, la photo à 1/4.
    const zoom = 720 / 280;
    expect(pickLevel(levels, mmPerPixel, zoom, 1)).toBe(4);
    expect(pickLevel(photo.levels.length, photoPlacement.mmPerPixel, zoom, 1)).toBe(2);
  });
});

describe("tuiles visibles", () => {
  it("couvre toute l'image en vue d'ensemble avec très peu de tuiles", () => {
    const view = fitView(placedBounds(engraving, engravingPlacement), viewport);
    const level = pickLevel(engraving.levels.length, engravingPlacement.mmPerPixel, view.zoom, 1);
    expect(level).toBe(4);
    const range = visibleTileRange(engraving, level, engravingPlacement, view, viewport)!;
    expect(range).toEqual({ level: 4, columnMin: 0, columnMax: 1, rowMin: 0, rowMax: 1 });
    expect(tileCount(range)).toBe(4);
  });

  it("ne demande à 100 % que les tuiles de l'écran", () => {
    const zoom = actualSizeZoom(engravingPlacement.mmPerPixel, 1);
    // Centre de l'écran sur le pixel (6000 ; 9000) du Verre.
    const view: ViewState = {
      centerX: 6000 * engravingPlacement.mmPerPixel,
      centerY: 9000 * engravingPlacement.mmPerPixel,
      zoom,
    };
    const range = visibleTileRange(engraving, 0, engravingPlacement, view, viewport)!;
    // Écran de 1200 × 800 : pixels 5400 à 6600 et 8600 à 9400.
    expect(range).toEqual({ level: 0, columnMin: 10, columnMax: 12, rowMin: 16, rowMax: 18 });
    expect(tileCount(range)).toBe(9);
  });

  it("s'arrête aux bords de l'image", () => {
    const zoom = actualSizeZoom(engravingPlacement.mmPerPixel, 1);
    const corner: ViewState = { centerX: 280, centerY: 280, zoom };
    expect(visibleTileRange(engraving, 0, engravingPlacement, corner, viewport)).toEqual({
      level: 0,
      columnMin: 28,
      columnMax: 30,
      rowMin: 29,
      rowMax: 30,
    });
    const origin: ViewState = { centerX: 0, centerY: 0, zoom };
    expect(visibleTileRange(engraving, 0, engravingPlacement, origin, viewport)).toEqual({
      level: 0,
      columnMin: 0,
      columnMax: 1,
      rowMin: 0,
      rowMax: 0,
    });
  });

  it("ne demande rien quand l'image est hors champ", () => {
    const away: ViewState = { centerX: 2000, centerY: 140, zoom: 2 };
    expect(visibleTileRange(engraving, 4, engravingPlacement, away, viewport)).toBeNull();
    // Bord à bord, sans recouvrement : toujours rien.
    const touching: ViewState = { centerX: 280 + 300, centerY: 140, zoom: 2 };
    expect(visibleTileRange(engraving, 4, engravingPlacement, touching, viewport)).toBeNull();
  });

  it("suit un calque décalé dans le repère commun", () => {
    const shifted = placementForWidth(photo.width, SLIDE_MM, 1000, 500);
    const view = fitView(placedBounds(photo, shifted), viewport);
    const level = pickLevel(photo.levels.length, shifted.mmPerPixel, view.zoom, 1);
    const range = visibleTileRange(photo, level, shifted, view, viewport)!;
    expect(range.columnMin).toBe(0);
    expect(range.columnMax).toBe(photo.levels[level].columns - 1);
    expect(range.rowMax).toBe(photo.levels[level].rows - 1);
  });
});

describe("tuiles et ancêtres", () => {
  it("donne des tuiles plus petites au bord", () => {
    expect(tileRect(engraving, 0, 0, 0)).toEqual({ x: 0, y: 0, width: 512, height: 512 });
    // 15 433 = 30 × 512 + 73.
    expect(tileRect(engraving, 0, 30, 30)).toEqual({ x: 15_360, y: 15_360, width: 73, height: 73 });
    expect(tileRect(engraving, 5, 0, 0)).toEqual({ x: 0, y: 0, width: 483, height: 483 });
  });

  it("nomme les fichiers comme le moteur", () => {
    expect(tilePath(engraving, 2, 7, 3)).toBe("2/7_3.png");
    expect(tilePath(photo, 0, 10, 0)).toBe("0/10_0.jpg");
  });

  it("retrouve la partie d'une tuile moins fine qui recouvre une tuile", () => {
    // Tuile (5 ; 2) du niveau 1 : pixels 2560 à 3072 et 1024 à 1536.
    expect(tileAncestor(engraving, 1, 5, 2, 0)).toEqual({
      level: 1,
      column: 5,
      row: 2,
      frame: { x: 0, y: 0, width: 512, height: 512 },
    });
    // Un niveau plus haut : pixels 1280 à 1536, dans la tuile (2 ; 1).
    expect(tileAncestor(engraving, 1, 5, 2, 1)).toEqual({
      level: 2,
      column: 2,
      row: 1,
      frame: { x: 256, y: 0, width: 256, height: 256 },
    });
    // Jusqu'à la tuile unique du dernier niveau.
    expect(tileAncestor(engraving, 1, 5, 2, 4)).toEqual({
      level: 5,
      column: 0,
      row: 0,
      frame: { x: 160, y: 64, width: 32, height: 32 },
    });
  });

  it("ne dépasse jamais de la tuile ancêtre, même au bord", () => {
    for (const pyramid of [engraving, photo, pyramidOf(5_145, 3_508), pyramidOf(513, 700)]) {
      const last = pyramid.levels.length - 1;
      for (let level = 0; level < last; level++) {
        const info = pyramid.levels[level];
        for (const [column, row] of [
          [0, 0],
          [info.columns - 1, info.rows - 1],
          [info.columns - 1, 0],
        ]) {
          for (let up = 1; level + up <= last; up++) {
            const ancestor = tileAncestor(pyramid, level, column, row, up);
            const rect = tileRect(pyramid, ancestor.level, ancestor.column, ancestor.row);
            expect(ancestor.frame.x).toBeGreaterThanOrEqual(0);
            expect(ancestor.frame.width).toBeGreaterThan(0);
            expect(ancestor.frame.height).toBeGreaterThan(0);
            expect(ancestor.frame.x + ancestor.frame.width).toBeLessThanOrEqual(rect.width);
            expect(ancestor.frame.y + ancestor.frame.height).toBeLessThanOrEqual(rect.height);
          }
        }
      }
    }
  });
});

import { describe, expect, it } from "vitest";
import type { ExportReport } from "../engine";
import {
  DEFAULT_EXPORT,
  exportSettingsOf,
  exportSizeLabel,
  exportSummary,
  frescoPlacement,
  validDpi,
} from "./exportLogic";

describe("export des diapositives", () => {
  it("donne la taille du PNG, avec ou sans marge", () => {
    expect(exportSizeLabel(DEFAULT_EXPORT)).toMatch(/^16\s535 px · plaque de 300 mm$/);
    expect(exportSizeLabel({ ...DEFAULT_EXPORT, margin: false })).toMatch(/^15\s433 px · image de 280 mm$/);
  });

  it("complète les réglages enregistrés", () => {
    expect(exportSettingsOf({ dpi: 1000, mirror: true })).toEqual({ ...DEFAULT_EXPORT, dpi: 1000, mirror: true });
    expect(exportSettingsOf(undefined)).toEqual(DEFAULT_EXPORT);
    expect(validDpi(1400) && !validDpi(10) && !validDpi(Number.NaN)).toBe(true);
  });

  it("pose la fresque sur son rectangle de façade", () => {
    const fresco = { source: "fresque/f.png", frame: { x: 10, y: 20, width: 12_000, height: 6_000 } };
    expect(frescoPlacement(fresco, 24_000)).toEqual({ x: 10, y: 20, mmPerPixel: 0.5 });
  });

  it("résume l'export", () => {
    const report: ExportReport = {
      slides: [
        { projectorId: "a", label: "A", path: "/x/E-A.png", sizePx: 16535, sizeMm: 300, bytes: 1, seconds: 12.4 },
        { projectorId: "b", label: "B", path: "/x/E-B.png", sizePx: 16535, sizeMm: 300, bytes: 1, seconds: 11.2 },
      ],
      skipped: [{ projectorId: "c", label: "C", reason: "recalage incomplet" }],
      settings: DEFAULT_EXPORT,
    };
    expect(exportSummary(report, "/Users/b/Gravure")).toBe(
      "2 diapositives écrites dans « Gravure » en 24 s. Non exporté (recalage incomplet) : C.",
    );
  });
});

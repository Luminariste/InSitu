import { describe, expect, it } from "vitest";
import type { ExportReport } from "../engine";
import {
  DEFAULT_EXPORT,
  exportSettingsOf,
  exportSizeLabel,
  exportSummary,
  frescoPlacement,
  sheetFileName,
  sheetSummary,
  validDpi, ALL_SLIDES, exportButtonLabel, exportTargets, keptTarget,
  controlPlanButtonLabel,
  controlPlanSummary,
} from "./exportLogic";

describe("export des diapositives", () => {
  it("donne la taille du PNG, avec ou sans marge", () => {
    expect(exportSizeLabel(DEFAULT_EXPORT)).toMatch(/^15\s433 px · image de 280 mm$/);
    expect(exportSizeLabel({ ...DEFAULT_EXPORT, margin: true })).toMatch(/^16\s535 px · plaque de 300 mm$/);
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
    const withSvg = { ...report, slides: report.slides.map((s) => ({ ...s, svg: s.path.replace(".png", ".svg") })) };
    expect(exportSummary(withSvg, "/Users/b/Gravure")).toMatch(/^2 diapositives écrites \(PNG et SVG\) dans/);
  });

  it("nomme et résume la fiche de calepinage", () => {
    expect(sheetFileName("Épinal")).toBe("Épinal - calepinage.pdf");
    expect(sheetFileName("a/b")).toBe("a-b - calepinage.pdf");
    expect(sheetFileName(undefined)).toBe("Projet - calepinage.pdf");
    const report = { pages: 3, slides: 7, alerts: 2, smallestTextMm: 3 };
    expect(sheetSummary(report, "/x/Épinal - calepinage.pdf")).toBe(
      "Fiche de calepinage écrite dans « Épinal - calepinage.pdf » : 3 pages A3, 7 diapositives, 2 alertes.",
    );
    expect(sheetSummary({ ...report, alerts: 0 }, "f.pdf")).toMatch(/aucune alerte\.$/);
  });
});

describe("une diapositive ou toutes", () => {
  const pair = { slide: { x: 0, y: 0 }, facade: { x: 0, y: 0 } };
  const projectors = [
    { id: "p1", label: "A", registration: [pair, pair, pair, pair] },
    { id: "p2", label: "B", registration: [pair, pair] },
    { id: "p3", label: "C", registration: [pair, pair, pair, pair, pair] },
  ];

  it("ne propose seuls que les projecteurs recalés", () => {
    expect(exportTargets(projectors)).toEqual([
      { id: "p1", label: "A" },
      { id: "p3", label: "C" },
    ]);
  });

  it("dit ce que le bouton va exporter", () => {
    const targets = exportTargets(projectors);
    expect(exportButtonLabel(ALL_SLIDES, targets)).toBe("Exporter les diapositives…");
    expect(exportButtonLabel("p3", targets)).toBe("Exporter la diapositive C…");
  });

  it("revient à toutes quand le projecteur choisi n'est plus recalé", () => {
    const targets = exportTargets(projectors);
    expect(keptTarget("p1", targets)).toBe("p1");
    expect(keptTarget("p2", targets)).toBe(ALL_SLIDES);
  });
});

describe("plan de contrôle", () => {
  const targets = [
    { id: "p1", label: "A" },
    { id: "p2", label: "B" },
  ];
  it("le bouton dit ce qu'il écrit", () => {
    expect(controlPlanButtonLabel("", targets)).toBe("Plans de contrôle…");
    expect(controlPlanButtonLabel("p2", targets)).toBe("Plan de contrôle B…");
  });
  it("le compte rendu donne la raison des plans manquants", () => {
    const slide = { projectorId: "p1", label: "A", path: "/x/A.png", sizePx: 10, sizeMm: 280, bytes: 1, svg: null, seconds: 1 };
    const settings = { dpi: 1400, margin: false, mirror: true, negative: true, oneBit: true, threshold: 128, svg: false };
    expect(controlPlanSummary({ slides: [slide], skipped: [], settings }, "/Users/b/Gravure")).toContain(
      "1 plan de contrôle écrit dans « Gravure »",
    );
    const partial = controlPlanSummary(
      { slides: [slide, slide], skipped: [{ projectorId: "p3", label: "C", reason: "recalage incomplet" }], settings },
      "/Users/b/Gravure",
    );
    expect(partial).toBe("2 plans de contrôle écrits dans « Gravure ». Sans plan : C (recalage incomplet).");
  });
});

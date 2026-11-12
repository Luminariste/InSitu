import { describe, expect, it } from "vitest";
import type { CompositionReport } from "../engine";
import { compositionSummary, compositionWarnings, linkName } from "./illustratorLogic";

const report: CompositionReport = {
  found: 245,
  placed: 38,
  drawings: 6,
  folder: "bibliotheque/ECURIES-COMPOSITION",
  layers: ["Premier Plan", "Deuxième plan"],
  missing: [
    { link: "/Users/marc/Dropbox/Algues/Algue-H01.png", count: 3, reason: "fichier introuvable" },
    { link: "/Users/marc/Dropbox/Poissons/Betta.png", count: 1, reason: "fichier introuvable" },
  ],
  embedded: 10,
  skewed: 0,
};

describe("compte rendu d'import Illustrator", () => {
  it("dit ce qui a été posé", () => {
    expect(compositionSummary(report)).toBe(
      "38 dessins posés sur 245 images liées, dans 2 calques ; 6 fichiers ajoutés à la bibliothèque (ECURIES-COMPOSITION).",
    );
  });
  it("dit ce qui manque, par nom de fichier", () => {
    const lines = compositionWarnings(report);
    expect(lines).toHaveLength(2);
    expect(lines[0]).toContain("2 liens introuvables (4 images non posées) : Algue-H01.png, Betta.png.");
    expect(lines[1]).toContain("10 images incorporées");
    expect(compositionWarnings({ ...report, missing: [], embedded: 0 })).toEqual([]);
  });
  it("garde le seul nom du fichier d'un lien", () => {
    expect(linkName("/a/b/Murène.png")).toBe("Murène.png");
  });
});

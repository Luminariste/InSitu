import { describe, expect, it } from "vitest";
import { countLabel, filterEntries, folderTree, formatSize, importSummary, matches, normalize } from "./logic";

const entry = (name: string, folder: string) => ({ name, folder });

describe("recherche", () => {
  it("ignore accents et majuscules", () => {
    expect(normalize("  Héron CENDRÉ ")).toBe("heron cendre");
    expect(matches(entry("héron", "Oiseaux"), "HERON")).toBe(true);
  });

  it("demande chaque mot, dans le nom ou le dossier", () => {
    const e = entry("goéland", "Oiseaux/Mer");
    expect(matches(e, "oiseau mer")).toBe(true);
    expect(matches(e, "goe")).toBe(true);
    expect(matches(e, "oiseau montagne")).toBe(false);
    expect(matches(e, "   ")).toBe(true);
  });

  it("filtre par dossier, sous-dossiers compris", () => {
    const all = [entry("a", "Oiseaux"), entry("b", "Oiseaux/Mer"), entry("c", "Oiseaux-bis"), entry("d", "")];
    expect(filterEntries(all, "", "Oiseaux").map((e) => e.name)).toEqual(["a", "b"]);
    expect(filterEntries(all, "", null).length).toBe(4);
    expect(filterEntries(all, "b", "Oiseaux").map((e) => e.name)).toEqual(["b"]);
  });
});

describe("dossiers", () => {
  it("donne l'arbre à plat avec les comptes", () => {
    const rows = folderTree([
      entry("1", "Oiseaux/Mer"),
      entry("2", "Oiseaux"),
      entry("3", "Arbres"),
      entry("4", "Oiseaux/Mer"),
      entry("5", ""),
      entry("6", "Écureuils"),
    ]);
    expect(rows.map((r) => [r.path, r.depth, r.count])).toEqual([
      ["Arbres", 0, 1],
      ["Écureuils", 0, 1],
      ["Oiseaux", 0, 3],
      ["Oiseaux/Mer", 1, 2],
    ]);
    expect(rows[3].label).toBe("Mer");
  });

  it("range les numéros dans l'ordre naturel", () => {
    expect(folderTree([entry("", "Série 10"), entry("", "Série 2")]).map((r) => r.path)).toEqual(["Série 2", "Série 10"]);
  });
});

describe("textes", () => {
  it("écrit tailles et nombres à la française", () => {
    expect(formatSize({ width: 4096, height: 2048, source: "bibliotheque/a.png" })).toBe("4 096 × 2 048 px");
    expect(formatSize({ width: 400, height: 200, source: "bibliotheque/a.SVG" })).toBe("400 × 200 (vectoriel)");
    expect(countLabel(1)).toBe("1 dessin");
    expect(countLabel(200)).toBe("200 dessins");
  });

  it("résume un import", () => {
    expect(importSummary({ folder: "bibliotheque/X", added: ["a", "b"], alreadyPresent: 1, failures: [] })).toBe(
      "2 dessins ajoutés, 1 dessin déjà présent.",
    );
    expect(
      importSummary({ folder: "bibliotheque/X", added: [], alreadyPresent: 0, failures: [{ file: "casse.png", reason: "?" }] }),
    ).toBe("Aucun nouveau dessin, 1 fichier illisible (casse.png).");
  });
});

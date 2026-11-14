import { describe, expect, it } from "vitest";
import {
  childFolder,
  countLabel,
  draggedDrawings,
  fileStem,
  filterEntries,
  folderTree,
  formatSize,
  importSummary,
  matches,
  moveSummary,
  newFolderName,
  normalize,
  parentFolder,
  searchFolders,
  searchSummary,
  sharedBadge,
  sharedSummary,
} from "./logic";

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

  it("retrouve un dessin renommé par le nom de son fichier", () => {
    expect(fileStem("bibliotheque/Oiseaux/Merle noir.png")).toBe("Merle noir");
    expect(fileStem("bibliotheque\\a\\b.v2.svg")).toBe("b.v2");
    const renamed = { name: "Oiseau du toit", folder: "Oiseaux", source: "bibliotheque/Oiseaux/merle.png" };
    expect(matches(renamed, "merle")).toBe(true);
    expect(matches(renamed, "toit merle")).toBe(true);
    expect(matches(renamed, "png")).toBe(false);
  });

  it("montre les dossiers trouvés par leur nom et ceux qui contiennent des dessins trouvés", () => {
    const all = [
      entry("goéland", "Oiseaux/Mer"),
      entry("sapin", "Arbres"),
      entry("chêne", "Arbres/Feuillus"),
      entry("étoile de mer", "Décor"),
    ];
    const rows = folderTree(all, ["Vide/Mer calme"]);
    const found = searchFolders(rows, all, "mer");
    expect(found.map((r) => [r.path, r.hit, r.count])).toEqual([
      ["Décor", false, 1],
      ["Oiseaux", false, 1],
      ["Oiseaux/Mer", true, 1],
      ["Vide", false, 0],
      ["Vide/Mer calme", true, 0],
    ]);
    // Sans recherche : l'arbre entier, rien de mis en avant.
    expect(searchFolders(rows, all, " ").length).toBe(rows.length);
    expect(searchFolders(rows, all, "").some((r) => r.hit)).toBe(false);
    // Un mot qui ne répond nulle part : aucun dossier.
    expect(searchFolders(rows, all, "baleine")).toEqual([]);
  });

  it("résume ce qui est trouvé", () => {
    expect(searchSummary(0, 0)).toBe("aucun dessin");
    expect(searchSummary(1, 0)).toBe("1 dessin");
    expect(searchSummary(3, 2)).toBe("3 dessins, 2 dossiers");
    expect(searchSummary(2, 1)).toBe("2 dessins, 1 dossier");
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

describe("ranger la bibliothèque", () => {
  it("montre les dossiers créés à la main, même vides", () => {
    const rows = folderTree([entry("a", "Oiseaux")], ["Oiseaux/Mer", "Poissons"]);
    expect(rows.map((r) => [r.path, r.count, r.depth])).toEqual([
      ["Oiseaux", 1, 0],
      ["Oiseaux/Mer", 0, 1],
      ["Poissons", 0, 0],
    ]);
  });

  it("trouve un nom libre pour un nouveau dossier", () => {
    expect(newFolderName(["Oiseaux"], null)).toBe("Nouveau dossier");
    expect(newFolderName(["Nouveau dossier", "Nouveau dossier 2"], null)).toBe("Nouveau dossier 3");
    expect(newFolderName(["Nouveau dossier"], "Oiseaux")).toBe("Nouveau dossier");
    expect(childFolder("Oiseaux", "Mer")).toBe("Oiseaux/Mer");
    expect(childFolder(null, "Mer")).toBe("Mer");
    expect(parentFolder("Oiseaux/Mer")).toBe("Oiseaux");
    expect(parentFolder("Oiseaux")).toBe("");
  });

  it("glisse toute la sélection, ou le seul dessin pris", () => {
    const selection = new Set(["a", "b"]);
    expect(draggedDrawings("a", selection).sort()).toEqual(["a", "b"]);
    expect(draggedDrawings("c", selection)).toEqual(["c"]);
  });

  it("dit où les dessins sont rangés", () => {
    expect(moveSummary(["merle"], "Oiseaux")).toBe("« merle » rangé dans « Oiseaux ».");
    expect(moveSummary(["a", "b"], "")).toBe("2 dessins rangés hors de tout dossier.");
  });
});

describe("bibliothèque commune", () => {
  it("étiquette les dessins selon la vue", () => {
    expect(sharedBadge(null, false)).toBeNull();
    expect(sharedBadge("upToDate", false)).toBe("Commune");
    expect(sharedBadge("newer", false)).toBe("Nouvelle version");
    expect(sharedBadge("missing", false)).toBe("Retiré de la commune");
    expect(sharedBadge("upToDate", true)).toBe("Dans le projet");
    expect(sharedBadge("newer", true)).toBe("Projet à reprendre");
  });

  it("résume une copie ou un envoi", () => {
    const report = { added: ["a", "b"], alreadyPresent: 1, updated: ["c"], failures: [] };
    expect(sharedSummary(report, "copier")).toBe(
      "2 dessins copiés dans le projet, 1 dessin repris dans sa nouvelle version, 1 dessin déjà à jour dans le projet.",
    );
    expect(sharedSummary({ added: ["a"], alreadyPresent: 0, updated: [], failures: [] }, "envoyer")).toBe(
      "1 dessin envoyé dans la bibliothèque commune.",
    );
    expect(sharedSummary({ added: [], alreadyPresent: 0, updated: [], failures: [] }, "copier")).toBe("Rien à faire.");
  });
});

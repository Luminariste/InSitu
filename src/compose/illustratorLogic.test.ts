import { describe, expect, it } from "vitest";
import type { CompositionPreview, CompositionReport, LinkStatus, PreviewImage } from "../engine";
import {
  checkState,
  choiceSummary,
  choiceText,
  compositionSummary,
  compositionWarnings,
  defaultChoice,
  emptyLayers,
  layerGroups,
  linkName,
  toggleImages,
} from "./illustratorLogic";

const report: CompositionReport = {
  found: 245,
  placed: 38,
  drawings: 6,
  reused: 0,
  folder: "bibliotheque/ECURIES-COMPOSITION",
  layers: ["Premier Plan", "Deuxième plan"],
  missing: [
    { link: "/Users/marc/Dropbox/Algues/Algue-H01.png", count: 3, reason: "fichier introuvable" },
    { link: "/Users/marc/Dropbox/Poissons/Betta.png", count: 1, reason: "fichier introuvable" },
  ],
  embedded: 10,
  extracted: 0,
  skewed: 0,
  skipped: 0,
};

describe("compte rendu d'import Illustrator", () => {
  it("dit ce qui a été posé", () => {
    expect(compositionSummary(report)).toBe(
      "38 dessins posés sur 245 images, dans 2 calques ; 6 fichiers ajoutés à la bibliothèque (ECURIES-COMPOSITION).",
    );
  });
  it("dit ce qui manque, par nom de fichier", () => {
    const lines = compositionWarnings(report);
    expect(lines).toHaveLength(2);
    expect(lines[0]).toContain("2 liens introuvables (4 images non posées) : Algue-H01.png, Betta.png.");
    expect(lines[1]).toContain("10 images incorporées dans le .ai non reprises");
    expect(compositionWarnings({ ...report, missing: [], embedded: 0 })).toEqual([]);
  });
  it("garde le seul nom du fichier d'un lien", () => {
    expect(linkName("/a/b/Murène.png")).toBe("Murène.png");
  });
});

const image = (layer: number, link: number): PreviewImage => ({
  layer,
  link,
  visible: true,
  center: { x: 0, y: 0 },
  width: 1,
  height: 1,
  rotation: 0,
  mirrored: false,
  skewed: false,
});

const layer = (name: string, images: number, embedded = 0) => ({
  name,
  visible: true,
  locked: false,
  color: null,
  images,
  embedded,
  existing: false,
});

const link = (name: string, status: LinkStatus, count: number) => ({
  link: `/Users/marc/${name}`,
  name,
  status,
  file: null,
  drawing: null,
  drawingName: null,
  count,
});

// Fond : merle ×2 (bibliothèque), murène (nouvelle) ; Devant : merle, absent (introuvable).
const preview: CompositionPreview = {
  name: "Composition",
  folder: "bibliotheque/Composition",
  frame: { x: 0, y: 0, width: 100, height: 50 },
  layers: [layer("Fond", 3), layer("Tracés", 0), layer("Devant", 2, 1)],
  links: [link("merle.png", "library", 3), link("Murène.png", "new", 1), link("absent.png", "missing", 1)],
  images: [image(0, 0), image(0, 1), image(0, 0), image(2, 0), image(2, 2)],
  embedded: 1,
};

describe("fenêtre d'aperçu de l'import Illustrator", () => {
  it("choisit d'abord tout ce qui peut être importé", () => {
    expect([...defaultChoice(preview)]).toEqual([0, 1, 2, 3]);
  });
  it("range les fichiers par calque, l'avant en haut, sans les calques vides", () => {
    const groups = layerGroups(preview);
    expect(groups.map((g) => preview.layers[g.layer].name)).toEqual(["Devant", "Fond"]);
    expect(groups[1].rows).toEqual([
      { link: 0, images: [0, 2] },
      { link: 1, images: [1] },
    ]);
    expect(emptyLayers(preview)).toEqual(["Tracés"]);
  });
  it("coche et décoche un calque ou un fichier d'un coup", () => {
    const all = defaultChoice(preview);
    // « Devant » : seul le merle compte, l'image introuvable ne se coche pas.
    expect(checkState(preview, all, [3, 4])).toBe("all");
    const without = toggleImages(preview, all, [3, 4]);
    expect([...without]).toEqual([0, 1, 2]);
    expect(checkState(preview, without, [0, 1, 2, 3, 4])).toBe("some");
    expect(checkState(preview, without, [3, 4])).toBe("none");
    expect([...toggleImages(preview, without, [3, 4])].sort()).toEqual([0, 1, 2, 3]);
  });
  it("dit ce que l'import fera", () => {
    const summary = choiceSummary(preview, defaultChoice(preview));
    expect(summary).toEqual({ images: 4, added: 1, reused: 1, layers: 2, missing: 1 });
    expect(choiceText(summary)).toBe(
      "4 dessins seront posés dans 2 calques ; 1 dessin repris de la bibliothèque ; 1 fichier ajouté à la bibliothèque.",
    );
    expect(choiceText(choiceSummary(preview, new Set()))).toBe("Aucune image choisie.");
  });
  it("compte les images incorporées extraites", () => {
    expect(compositionSummary({ ...report, placed: 6, drawings: 5, extracted: 6, embedded: 0, found: 251 })).toBe(
      "6 dessins posés, dont 6 images incorporées extraites, sur 251 images, dans 2 calques ; 5 fichiers ajoutés à la bibliothèque (ECURIES-COMPOSITION).",
    );
    const withEmbedded: CompositionPreview = {
      ...preview,
      links: [...preview.links, link("Incorporée 01 - Fond.png", "embedded", 1)],
      images: [...preview.images, image(0, 3)],
    };
    expect(choiceSummary(withEmbedded, defaultChoice(withEmbedded))).toEqual({ images: 5, added: 2, reused: 1, layers: 2, missing: 1 });
  });
  it("compte les dessins repris et les images laissées de côté", () => {
    expect(compositionSummary({ ...report, reused: 2, skipped: 45 })).toBe(
      "38 dessins posés sur 200 choisies, dans 2 calques ; 4 fichiers ajoutés à la bibliothèque (ECURIES-COMPOSITION), 2 dessins repris de la bibliothèque.",
    );
  });
});

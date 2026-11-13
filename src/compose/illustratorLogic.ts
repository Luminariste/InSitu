// Import d'une composition Illustrator (tâche 6.3) : compte rendu en
// français, et choix de ce qu'on importe dans la fenêtre d'aperçu (demande
// de Benjamin, 13 novembre 2026). Fonctions pures, testées sans écran.

import type { CompositionPreview, CompositionReport, LinkStatus } from "../engine";

const plural = (n: number, one: string, many: string) => `${n.toLocaleString("fr-FR")} ${n > 1 ? many : one}`;

/** Le nom du fichier d'un lien, sans son dossier. */
export function linkName(link: string): string {
  return link.split(/[/\\:]/).filter(Boolean).pop() ?? link;
}

/** Phrase principale : ce qui a été posé. */
export function compositionSummary(report: CompositionReport): string {
  const posed = plural(report.placed, "dessin posé", "dessins posés");
  const layers = plural(report.layers.length, "calque", "calques");
  const added = report.drawings - report.reused;
  const files = plural(added, "fichier ajouté", "fichiers ajoutés");
  const reused = report.reused > 0 ? `, ${plural(report.reused, "dessin repris", "dessins repris")} de la bibliothèque` : "";
  const of = report.skipped > 0 ? `${report.found - report.skipped} choisies` : `${report.found} image${report.found > 1 ? "s" : ""} liée${report.found > 1 ? "s" : ""}`;
  return `${posed} sur ${of}, dans ${layers} ; ${files} à la bibliothèque (${report.folder.replace(/^bibliotheque\//, "")})${reused}.`;
}

/** Ce qui manque, une ligne par cas ; vide si tout a été posé. */
export function compositionWarnings(report: CompositionReport): string[] {
  const lines: string[] = [];
  if (report.missing.length > 0) {
    const images = report.missing.reduce((sum, m) => sum + m.count, 0);
    const names = report.missing.slice(0, 5).map((m) => linkName(m.link));
    const more = report.missing.length > 5 ? ` et ${report.missing.length - 5} autres` : "";
    lines.push(
      `${plural(report.missing.length, "lien introuvable", "liens introuvables")} (${plural(images, "image non posée", "images non posées")}) : ${names.join(", ")}${more}. Les fichiers liés doivent être à côté du .ai, comme dans Illustrator, ou déjà dans la bibliothèque.`,
    );
  }
  if (report.embedded > 0) {
    lines.push(`${plural(report.embedded, "image incorporée", "images incorporées")} dans le .ai, non importée${report.embedded > 1 ? "s" : ""} : les lier dans Illustrator pour les retrouver.`);
  }
  if (report.skewed > 0) {
    lines.push(`${plural(report.skewed, "image déformée", "images déformées")} en biais dans Illustrator, posée${report.skewed > 1 ? "s" : ""} sans le biais.`);
  }
  return lines;
}

// --- Fenêtre d'aperçu -------------------------------------------------------

/** Ce que dit l'aperçu de chaque fichier lié. */
export const STATUS_LABEL: Record<LinkStatus, string> = {
  library: "Bibliothèque",
  libraryByName: "Bibliothèque (même nom)",
  new: "Nouveau",
  missing: "Introuvable",
};

export const STATUS_HINT: Record<LinkStatus, string> = {
  library: "Le même fichier est déjà dans la bibliothèque : ce dessin est repris, avec ses retouches.",
  libraryByName:
    "Le fichier lié n'est pas sur ce Mac, mais un dessin du même nom est dans la bibliothèque : il est repris.",
  new: "Retrouvé à côté du .ai : il entrera dans la bibliothèque.",
  missing: "Introuvable sur ce Mac et absent de la bibliothèque : ses images ne seront pas posées.",
};

/** L'image `i` peut être importée : son fichier est sur ce Mac ou dans la bibliothèque. */
export function importable(preview: CompositionPreview, i: number): boolean {
  return preview.links[preview.images[i].link].status !== "missing";
}

/** Le choix de départ : tout ce qui peut être importé. */
export function defaultChoice(preview: CompositionPreview): Set<number> {
  return new Set(preview.images.map((_, i) => i).filter((i) => importable(preview, i)));
}

/** Un fichier lié dans un calque, et les images qui le placent. */
export interface LinkRow {
  link: number;
  images: number[];
}

/** Un calque d'Illustrator dans la fenêtre d'aperçu. */
export interface LayerGroup {
  layer: number;
  /** Toutes ses images liées. */
  images: number[];
  /** Ses fichiers liés, dans l'ordre du nom. */
  rows: LinkRow[];
}

/**
 * Les calques qui ont des images, l'avant en haut comme dans le panneau
 * Calques d'Illustrator.
 */
export function layerGroups(preview: CompositionPreview): LayerGroup[] {
  const groups = preview.layers.map((_, layer) => ({ layer, images: [] as number[], rows: new Map<number, number[]>() }));
  preview.images.forEach((image, i) => {
    const g = groups[image.layer];
    g.images.push(i);
    const row = g.rows.get(image.link) ?? [];
    row.push(i);
    g.rows.set(image.link, row);
  });
  return groups
    .filter((g) => g.images.length > 0 || preview.layers[g.layer].embedded > 0)
    .reverse()
    .map((g) => ({
      layer: g.layer,
      images: g.images,
      rows: [...g.rows.entries()]
        .map(([link, images]) => ({ link, images }))
        .sort((a, b) => preview.links[a.link].name.localeCompare(preview.links[b.link].name, "fr")),
    }));
}

/** Les calques sans aucune image (tracés seulement) : seulement nommés. */
export function emptyLayers(preview: CompositionPreview): string[] {
  return preview.layers.filter((l) => l.images === 0 && l.embedded === 0).map((l) => l.name);
}

export type CheckState = "all" | "some" | "none";

/** État de la case d'un ensemble d'images (seules comptent celles qu'on peut importer). */
export function checkState(preview: CompositionPreview, chosen: Set<number>, images: number[]): CheckState {
  const usable = images.filter((i) => importable(preview, i));
  const n = usable.filter((i) => chosen.has(i)).length;
  if (n === 0) return "none";
  return n === usable.length ? "all" : "some";
}

/** Coche ou décoche un ensemble d'images d'un coup : tout coché → tout décoché, sinon tout coché. */
export function toggleImages(preview: CompositionPreview, chosen: Set<number>, images: number[]): Set<number> {
  const next = new Set(chosen);
  const usable = images.filter((i) => importable(preview, i));
  if (checkState(preview, chosen, images) === "all") usable.forEach((i) => next.delete(i));
  else usable.forEach((i) => next.add(i));
  return next;
}

/** Ce que l'import fera avec ce choix. */
export interface ChoiceSummary {
  images: number;
  /** Fichiers qui entreront dans la bibliothèque. */
  added: number;
  /** Dessins repris de la bibliothèque. */
  reused: number;
  layers: number;
  /** Images qu'on ne peut pas importer (lien introuvable). */
  missing: number;
}

export function choiceSummary(preview: CompositionPreview, chosen: Set<number>): ChoiceSummary {
  const links = new Set<number>();
  const layers = new Set<number>();
  for (const i of chosen) {
    links.add(preview.images[i].link);
    layers.add(preview.images[i].layer);
  }
  let added = 0;
  let reused = 0;
  for (const k of links) {
    const status = preview.links[k].status;
    if (status === "new") added += 1;
    else if (status !== "missing") reused += 1;
  }
  const missing = preview.images.filter((_, i) => !importable(preview, i)).length;
  return { images: chosen.size, added, reused, layers: layers.size, missing };
}

/** La phrase du bas de la fenêtre. */
export function choiceText(s: ChoiceSummary): string {
  if (s.images === 0) return "Aucune image choisie.";
  const parts = [
    `${plural(s.images, "dessin sera posé", "dessins seront posés")} dans ${plural(s.layers, "calque", "calques")}`,
  ];
  if (s.reused > 0) parts.push(`${plural(s.reused, "dessin repris", "dessins repris")} de la bibliothèque`);
  if (s.added > 0) parts.push(`${plural(s.added, "fichier ajouté", "fichiers ajoutés")} à la bibliothèque`);
  return `${parts.join(" ; ")}.`;
}

/** Couleur CSS d'un calque d'Illustrator ; `null` : celle de l'accent. */
export function layerColor(color: [number, number, number] | null): string | null {
  return color ? `rgb(${color[0]}, ${color[1]}, ${color[2]})` : null;
}

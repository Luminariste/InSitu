// Compte rendu en français de l'import d'une composition Illustrator (tâche 6.3).

import type { CompositionReport } from "../engine";

const plural = (n: number, one: string, many: string) => `${n.toLocaleString("fr-FR")} ${n > 1 ? many : one}`;

/** Le nom du fichier d'un lien, sans son dossier. */
export function linkName(link: string): string {
  return link.split(/[/\\:]/).filter(Boolean).pop() ?? link;
}

/** Phrase principale : ce qui a été posé. */
export function compositionSummary(report: CompositionReport): string {
  const posed = plural(report.placed, "dessin posé", "dessins posés");
  const layers = plural(report.layers.length, "calque", "calques");
  const files = plural(report.drawings, "fichier", "fichiers");
  return `${posed} sur ${report.found} image${report.found > 1 ? "s" : ""} liée${report.found > 1 ? "s" : ""}, dans ${layers} ; ${files} ajouté${report.drawings > 1 ? "s" : ""} à la bibliothèque (${report.folder.replace(/^bibliotheque\//, "")}).`;
}

/** Ce qui manque, une ligne par cas ; vide si tout a été posé. */
export function compositionWarnings(report: CompositionReport): string[] {
  const lines: string[] = [];
  if (report.missing.length > 0) {
    const images = report.missing.reduce((sum, m) => sum + m.count, 0);
    const names = report.missing.slice(0, 5).map((m) => linkName(m.link));
    const more = report.missing.length > 5 ? ` et ${report.missing.length - 5} autres` : "";
    lines.push(
      `${plural(report.missing.length, "lien introuvable", "liens introuvables")} (${plural(images, "image non posée", "images non posées")}) : ${names.join(", ")}${more}. Les fichiers liés doivent être à côté du .ai, comme dans Illustrator.`,
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

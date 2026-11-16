// Fresque et export des diapositives (tâche 3.5) : logique pure de l'écran
// Façade, testée dans exportLogic.test.ts.

import type { ExportReport, ExportSettings, FrescoImage, SheetReport } from "../engine";
import type { Placement } from "../viewer/geometry";

/** Réglages par défaut, les mêmes que ceux du moteur (`units.rs`). */
export const DEFAULT_EXPORT: ExportSettings = {
  dpi: 1400,
  mirror: false,
  negative: false,
  margin: false,
  oneBit: false,
  threshold: 128,
  svg: false,
  fresco: true,
};

const SLIDE_MM = 280;
const PLATE_MM = 300;
const MM_PER_INCH = 25.4;

/** Réglages du projet complétés par les valeurs par défaut. */
export function exportSettingsOf(saved: Partial<ExportSettings> | undefined): ExportSettings {
  return { ...DEFAULT_EXPORT, ...(saved ?? {}) };
}

/** Pose l'image de la fresque (`width` px de large) sur son rectangle de façade. */
export function frescoPlacement(fresco: FrescoImage, width: number): Placement {
  return { x: fresco.frame.x, y: fresco.frame.y, mmPerPixel: fresco.frame.width / width };
}

/** Taille du PNG : « 15 433 px · image de 280 mm ». */
export function exportSizeLabel(settings: ExportSettings): string {
  const slide = Math.round((SLIDE_MM / MM_PER_INCH) * settings.dpi);
  const margin = Math.round(((PLATE_MM - SLIDE_MM) / 2 / MM_PER_INCH) * settings.dpi);
  const px = settings.margin ? slide + 2 * margin : slide;
  const what = settings.margin ? "plaque de 300 mm" : "image de 280 mm";
  return `${px.toLocaleString("fr-FR")} px · ${what}`;
}

/** Une résolution utilisable (le moteur accepte de 50 à 3 000 dpi). */
/** La résolution tapée : « 1400 », « 1 400 » ou « 1400,5 ». */
export function parseDpi(text: string): number {
  const cleaned = text.replace(/[\s\u00a0\u202f]/g, "").replace(",", ".");
  return cleaned === "" ? Number.NaN : Number(cleaned);
}

export function validDpi(dpi: number): boolean {
  return Number.isFinite(dpi) && dpi >= 50 && dpi <= 3000;
}

/** Dernier élément d'un chemin. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** Ce qu'on dit après un export. */
export function exportSummary(report: ExportReport, destination: string): string {
  const n = report.slides.length;
  const formats = report.slides.some((s) => s.svg) ? " (PNG et SVG)" : "";
  const written = `${n} diapositive${n > 1 ? "s" : ""} écrite${n > 1 ? "s" : ""}${formats} dans « ${fileName(destination)} »`;
  const seconds = report.slides.reduce((sum, s) => sum + s.seconds, 0);
  const time = ` en ${seconds.toLocaleString("fr-FR", { maximumFractionDigits: 0 })} s`;
  if (report.skipped.length === 0) return `${written}${time}.`;
  const skipped = report.skipped.map((s) => s.label).join(", ");
  return `${written}${time}. Non exporté${report.skipped.length > 1 ? "s" : ""} (recalage incomplet) : ${skipped}.`;
}

/**
 * Ce qu'on dit après l'export des plans de contrôle (D-3.5-m) : combien de
 * plans, où, et ceux qui manquent avec leur raison.
 */
export function controlPlanSummary(report: ExportReport, destination: string): string {
  const n = report.slides.length;
  const written = `${n} plan${n > 1 ? "s" : ""} de contrôle écrit${n > 1 ? "s" : ""} dans « ${fileName(destination)} »`;
  if (report.skipped.length === 0) return `${written}. À graver comme une diapositive : projeté, le plan doit tomber sur la façade.`;
  const skipped = report.skipped.map((s) => `${s.label} (${s.reason})`).join(", ");
  return `${written}. Sans plan : ${skipped}.`;
}

/** Libellé du bouton du plan de contrôle : un seul projecteur, ou tous. */
export function controlPlanButtonLabel(choice: string, targets: ExportTarget[]): string {
  const one = targets.find((t) => t.id === choice);
  return one ? `Plan de contrôle ${one.label}…` : "Plans de contrôle…";
}

/** Nom proposé pour la fiche de calepinage. */
export function sheetFileName(projectName: string | undefined): string {
  const name = (projectName ?? "").replace(/[/\\:*?"<>|]/g, "-").trim();
  return `${name || "Projet"} - calepinage.pdf`;
}

/** Ce qu'on dit après l'écriture de la fiche. */
export function sheetSummary(report: SheetReport, path: string): string {
  const alerts = report.alerts === 0 ? "aucune alerte" : `${report.alerts} alerte${report.alerts > 1 ? "s" : ""}`;
  return `Fiche de calepinage écrite dans « ${fileName(path)} » : ${report.pages} pages A3, ${report.slides} diapositive${report.slides > 1 ? "s" : ""}, ${alerts}.`;
}

/** Un projecteur qu'on peut exporter seul : recalé (4 paires de points au moins). */
export interface ExportTarget {
  id: string;
  label: string;
}

/** Les projecteurs recalés, dans l'ordre du projet : ceux qu'on peut exporter. */
export function exportTargets(projectors: { id: string; label: string; registration?: unknown[] }[]): ExportTarget[] {
  return projectors.filter((p) => (p.registration?.length ?? 0) >= 4).map((p) => ({ id: p.id, label: p.label }));
}

/** « Toutes les diapositives » (valeur vide) ou l'identifiant d'un projecteur. */
export const ALL_SLIDES = "";

/** Le choix gardé s'il existe encore, sinon toutes les diapositives. */
export function keptTarget(choice: string, targets: ExportTarget[]): string {
  return targets.some((t) => t.id === choice) ? choice : ALL_SLIDES;
}

/** Libellé du bouton d'export selon le choix. */
export function exportButtonLabel(choice: string, targets: ExportTarget[]): string {
  const one = targets.find((t) => t.id === choice);
  return one ? `Exporter la diapositive ${one.label}…` : "Exporter les diapositives…";
}

/**
 * Dessins à graver (D-3.5-q) : tous (par défaut, comme avant), ou seulement
 * les dessins choisis, pour compléter une plaque déjà gravée.
 */
export type DrawingsChoice = "all" | "chosen";

/** Le choix gardé tant qu'il y a des dessins choisis, sinon tous. */
export function keptDrawingsChoice(choice: DrawingsChoice, chosen: number): DrawingsChoice {
  return choice === "chosen" && chosen > 0 ? "chosen" : "all";
}

/** « le dessin choisi » ou « les 3 dessins choisis ». */
export function chosenDrawingsLabel(n: number): string {
  return n === 1 ? "le dessin choisi" : `les ${n} dessins choisis`;
}

/** Libellé de l'option « dessins choisis » du menu. */
export function chosenDrawingsOption(n: number): string {
  if (n === 0) return "Les dessins choisis seulement (aucun choisi)";
  const text = chosenDrawingsLabel(n);
  return `${text[0].toUpperCase()}${text.slice(1)} seulement`;
}

/** Libellé du bouton d'export quand seuls des dessins choisis partent. */
export function chosenExportButtonLabel(n: number): string {
  return `Exporter ${chosenDrawingsLabel(n)}…`;
}

/** Les identifiants des dessins choisis, sans doublon, dans l'ordre. */
export function chosenIds(ids: string[]): string[] {
  return [...new Set(ids)];
}

/** Ce qu'on dit après l'export des seuls dessins choisis. */
export function chosenSummary(report: ExportReport, destination: string, n: number): string {
  const count = report.slides.length;
  const labels = report.slides.map((s) => s.label).join(", ");
  const formats = report.slides.some((s) => s.svg) ? " (PNG et SVG)" : "";
  const written =
    count === 1
      ? `Diapositive ${labels} écrite${formats} avec ${chosenDrawingsLabel(n)} seulement`
      : `${count} diapositives (${labels}) écrites${formats} avec ${chosenDrawingsLabel(n)} seulement`;
  return `${written}, dans « ${fileName(destination)} », sous le nom « … - DESSINS CHOISIS ». Même cadrage que la diapositive complète : à poser au même endroit dans LightBurn.`;
}

// Logique pure de l'écran Recalage (tâche 3.2) : paires de points, pose en
// deux clics, textes affichés. Testée sans écran (`logic.test.ts`).

import type { PairReport, RegistrationPair, RegistrationReport } from "../engine";

export interface XYPoint {
  x: number;
  y: number;
}

export const SLIDE_MM = 280;

/** Côté d'un point en cours de pose. */
export type Side = "slide" | "facade";

/** Un point posé d'un seul côté, en attente de l'autre. */
export interface Pending {
  slide: XYPoint | null;
  facade: XYPoint | null;
}

export const NO_PENDING: Pending = { slide: null, facade: null };

/** Vrai si le point est sur la diapositive (0 à 280 mm). */
export function onSlide(p: XYPoint): boolean {
  return p.x >= 0 && p.x <= SLIDE_MM && p.y >= 0 && p.y <= SLIDE_MM;
}

/**
 * Un clic sur un côté. Sans paire sélectionnée, il complète le point en
 * attente ; quand les deux côtés sont posés, la paire est ajoutée. Avec une
 * paire sélectionnée, il déplace le point de ce côté.
 */
export function click(
  pairs: RegistrationPair[],
  pending: Pending,
  selected: number | null,
  side: Side,
  at: XYPoint,
): { pairs: RegistrationPair[]; pending: Pending; changed: boolean } {
  const point = { x: at.x, y: at.y };
  if (selected !== null && selected >= 0 && selected < pairs.length) {
    const next = pairs.map((pair, i) => (i === selected ? { ...pair, [side]: point } : pair));
    return { pairs: next, pending, changed: true };
  }
  const placed: Pending = { ...pending, [side]: point };
  if (placed.slide && placed.facade) {
    return { pairs: [...pairs, { slide: placed.slide, facade: placed.facade }], pending: NO_PENDING, changed: true };
  }
  return { pairs, pending: placed, changed: false };
}

/** Déplace le point d'une paire, d'un côté ; sur la diapositive, il reste dans le carré. */
export function movePoint(pairs: RegistrationPair[], index: number, side: Side, at: XYPoint): RegistrationPair[] {
  const point =
    side === "slide"
      ? { x: Math.min(Math.max(at.x, 0), SLIDE_MM), y: Math.min(Math.max(at.y, 0), SLIDE_MM) }
      : { x: at.x, y: at.y };
  return pairs.map((pair, i) => (i === index ? { ...pair, [side]: point } : pair));
}

/** Rayon, en pixels d'écran, dans lequel on saisit un point pour le glisser. */
export const GRAB_RADIUS_PX = 14;

export function removePair(pairs: RegistrationPair[], index: number): RegistrationPair[] {
  return pairs.filter((_, i) => i !== index);
}

/** État d'un point pour l'affichage. */
export type PairStatus = "aberrant" | "bon" | "moyen" | "inconnu";

/** Écart sur la diapositive au-delà duquel un point est signalé en orange. */
export const WARN_SLIDE_MM = 1;

export function pairStatus(report: PairReport | undefined): PairStatus {
  if (!report) return "inconnu";
  if (report.outlier) return "aberrant";
  if (report.slideResidualMm === null) return "inconnu";
  return report.slideResidualMm <= WARN_SLIDE_MM ? "bon" : "moyen";
}

const mm1 = new Intl.NumberFormat("fr-FR", { minimumFractionDigits: 1, maximumFractionDigits: 1 });
const mm2 = new Intl.NumberFormat("fr-FR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

/** Écart sur la diapositive : « 0,35 mm ». */
export function formatSlideMm(value: number | null | undefined): string {
  return value === null || value === undefined || !Number.isFinite(value) ? "—" : `${mm2.format(value)} mm`;
}

/** Écart sur la façade, en centimètres : « 2,4 cm ». */
export function formatFacadeCm(value: number | null | undefined): string {
  return value === null || value === undefined || !Number.isFinite(value) ? "—" : `${mm1.format(value / 10)} cm`;
}

/** Phrase de résumé du recalage, sous la vue. */
export function summary(report: RegistrationReport | null, count: number): string {
  if (count < 4) {
    const missing = 4 - count;
    return `Encore ${missing} point${missing > 1 ? "s" : ""} pour le trapèze. Cliquer un point sur la photo, puis le même sur la façade.`;
  }
  if (!report) return `${count} points.`;
  if (report.stage === "trapeze") {
    return count === 4
      ? "Trapèze calculé (4 points, sans écart mesurable). Ajouter des points pour suivre le relief : l'écart de chacun s'affiche à partir du 5e."
      : "Trapèze calculé sur 5 points. Au 6e, le relief est corrigé et les points douteux sont signalés.";
  }
  const outliers = report.pairs.filter((p) => p.outlier).length;
  const mean = `écart moyen ${formatSlideMm(report.rmsSlideMm)} sur la diapositive, ${formatFacadeCm(report.rmsFacadeMm)} sur la façade`;
  const flagged =
    outliers === 0
      ? "aucun point douteux"
      : `${outliers} point${outliers > 1 ? "s" : ""} douteux à vérifier (en rouge)`;
  return `Relief corrigé sur ${count} points : ${mean} ; ${flagged}.`;
}

/** Consigne selon ce qui reste à poser. */
export function hint(pending: Pending, selected: number | null): string | null {
  if (selected !== null)
    return `Point ${selected + 1} : le glisser, ou cliquer sur la photo ou sur la façade pour l'y amener.`;
  if (pending.slide && !pending.facade) return "Cliquer maintenant le même point sur la façade.";
  if (pending.facade && !pending.slide) return "Cliquer maintenant le même point sur la photo.";
  return null;
}

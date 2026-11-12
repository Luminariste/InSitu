// Logique pure de l'assemblage de photos de façade (tâche 6.4) : pose des
// points communs en deux clics, états et textes affichés. Testée sans écran
// (`logic.test.ts`). Les points sont en pixels de chaque photo.

import type { AssemblyPair, AssemblyPairReport, AssemblyPhoto, AssemblyPhotoReport, AssemblyReport, AutoReport } from "../engine";

export interface XYPoint {
  x: number;
  y: number;
}

/** Côté d'un point : la photo choisie (`here`) ou sa photo d'appui (`there`). */
export type Side = "here" | "there";

/** Un point posé d'un seul côté, en attente de l'autre. */
export interface Pending {
  here: XYPoint | null;
  there: XYPoint | null;
}

export const NO_PENDING: Pending = { here: null, there: null };

/** Points communs qu'il faut au moins pour placer une photo. */
export const MIN_PAIRS = 4;

/** Rayon, en pixels d'écran, dans lequel on saisit un point pour le glisser. */
export const GRAB_RADIUS_PX = 14;

/** Écart au-delà duquel un point est signalé en orange, en pixels. */
export const WARN_RESIDUAL_PX = 3;

/** Vrai si le point est dans la photo de `width` × `height` pixels. */
export function inside(p: XYPoint, width: number, height: number): boolean {
  return p.x >= 0 && p.x <= width && p.y >= 0 && p.y <= height;
}

function clamp(p: XYPoint, width: number, height: number): XYPoint {
  return { x: Math.min(Math.max(p.x, 0), width), y: Math.min(Math.max(p.y, 0), height) };
}

/**
 * Un clic sur un côté. Sans paire sélectionnée, il complète le point en
 * attente ; quand les deux côtés sont posés, la paire est ajoutée. Avec une
 * paire sélectionnée, il déplace le point de ce côté.
 */
export function click(
  pairs: AssemblyPair[],
  pending: Pending,
  selected: number | null,
  side: Side,
  at: XYPoint,
): { pairs: AssemblyPair[]; pending: Pending; changed: boolean } {
  const point = { x: at.x, y: at.y };
  if (selected !== null && selected >= 0 && selected < pairs.length) {
    return { pairs: pairs.map((pair, i) => (i === selected ? { ...pair, [side]: point } : pair)), pending, changed: true };
  }
  const placed: Pending = { ...pending, [side]: point };
  if (placed.here && placed.there) {
    return { pairs: [...pairs, { here: placed.here, there: placed.there }], pending: NO_PENDING, changed: true };
  }
  return { pairs, pending: placed, changed: false };
}

/** Déplace le point d'une paire, d'un côté ; il reste dans sa photo. */
export function movePoint(
  pairs: AssemblyPair[],
  index: number,
  side: Side,
  at: XYPoint,
  size: { width: number; height: number },
): AssemblyPair[] {
  const point = clamp(at, size.width, size.height);
  return pairs.map((pair, i) => (i === index ? { ...pair, [side]: point } : pair));
}

export function removePair(pairs: AssemblyPair[], index: number): AssemblyPair[] {
  return pairs.filter((_, i) => i !== index);
}

/** État d'un point pour l'affichage (mêmes noms que l'écran Recalage). */
export type PairStatus = "aberrant" | "bon" | "moyen" | "inconnu";

export function pairStatus(report: AssemblyPairReport | undefined): PairStatus {
  if (!report) return "inconnu";
  if (report.outlier) return "aberrant";
  if (report.residualPx === null || !Number.isFinite(report.residualPx)) return "inconnu";
  return report.residualPx <= WARN_RESIDUAL_PX ? "bon" : "moyen";
}

const px1 = new Intl.NumberFormat("fr-FR", { minimumFractionDigits: 1, maximumFractionDigits: 1 });
const int = new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 0 });

/** Un écart en pixels : « 1,4 px ». */
export function formatPx(value: number | null | undefined): string {
  return value === null || value === undefined || !Number.isFinite(value) ? "—" : `${px1.format(value)} px`;
}

/** Taille d'une image : « 4 032 × 3 024 px ». */
export function formatSize(width: number, height: number): string {
  return `${int.format(width)} × ${int.format(height)} px`;
}

/** État d'une photo dans la liste : couleur et ligne de détail. */
export function photoLine(
  index: number,
  photo: AssemblyPhotoReport | undefined,
  names: Map<string, string>,
): { tone: "success" | "warning" | "danger" | undefined; detail: string } {
  if (index === 0) return { tone: "success", detail: "référence" };
  if (!photo) return { tone: undefined, detail: "…" };
  const anchor = photo.anchor ? (names.get(photo.anchor) ?? "?") : "?";
  const count = photo.pairs.length;
  const outliers = photo.pairs.filter((p) => p.outlier).length;
  if (!photo.placed) {
    const missing = Math.max(0, MIN_PAIRS - count);
    const detail =
      missing > 0
        ? `sur ${anchor} · encore ${missing} point${missing > 1 ? "s" : ""}`
        : (photo.problem ?? `sur ${anchor} · pas encore placée`);
    return { tone: count > 0 ? "warning" : undefined, detail };
  }
  const residual = photo.meanResidualPx !== null ? ` · ${formatPx(photo.meanResidualPx)}` : "";
  return {
    tone: outliers > 0 ? "danger" : "success",
    detail: `sur ${anchor} · ${count} points${residual}${outliers > 0 ? ` · ${outliers} douteux` : ""}`,
  };
}

/** Photos sur lesquelles `id` peut s'appuyer : toutes, sauf elle-même et celles qui s'appuient sur elle. */
export function possibleAnchors(photos: AssemblyPhoto[], id: string): AssemblyPhoto[] {
  const dependsOn = (candidate: AssemblyPhoto): boolean => {
    let cursor: string | null | undefined = candidate.id;
    for (let steps = 0; cursor && steps <= photos.length; steps++) {
      if (cursor === id) return true;
      cursor = photos.find((p) => p.id === cursor)?.anchor;
    }
    return false;
  };
  return photos.filter((p) => p.id !== id && !dependsOn(p));
}

/** Phrase de la barre d'état. */
export function summary(report: AssemblyReport | null, photoCount: number): string {
  if (photoCount === 0) return "Ajouter des photos de la façade : la première sert de référence.";
  if (photoCount === 1) return "Une seule photo : en ajouter une autre qui la recouvre en partie.";
  if (!report) return `${photoCount} photos.`;
  const unplaced = report.photos.filter((p) => !p.placed).length;
  const outliers = report.photos.reduce((n, p) => n + p.pairs.filter((q) => q.outlier).length, 0);
  if (unplaced > 0) {
    return `${unplaced} photo${unplaced > 1 ? "s" : ""} à placer : ${MIN_PAIRS} points communs au moins avec sa photo d'appui.`;
  }
  const flagged = outliers === 0 ? "" : ` ; ${outliers} point${outliers > 1 ? "s" : ""} douteux à vérifier (en rouge)`;
  return `Toutes les photos sont placées${flagged}. « Calculer l'assemblage » en fait la façade du projet.`;
}

/** Consigne selon ce qui reste à poser. */
export function hint(pending: Pending, selected: number | null): string | null {
  if (selected !== null) return `Point ${selected + 1} : le glisser, ou cliquer sur l'une des photos pour l'y amener.`;
  if (pending.here && !pending.there) return "Cliquer maintenant le même point sur la photo d'appui (à droite).";
  if (pending.there && !pending.here) return "Cliquer maintenant le même point sur la photo à placer (à gauche).";
  return null;
}

/** Vrai si une photo autre que la référence a déjà des points communs (que le raccord automatique remplacerait). */
export function hasManualPoints(photos: AssemblyPhoto[]): boolean {
  return photos.some((p, i) => i > 0 && (p.pairs?.length ?? 0) > 0);
}

const seconds = new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 0 });

/** Message après le raccord automatique. */
export function autoMessage(report: AutoReport): { tone: "success" | "warning"; text: string } {
  const linked = report.photos.filter((p, i) => i > 0 && p.linked).length;
  const time = `${seconds.format(Math.max(1, report.seconds))} s`;
  const outliers = report.assembly.photos.reduce((n, p) => n + p.pairs.filter((q) => q.outlier).length, 0);
  const plural = (n: number, word: string) => `${n} ${word}${n > 1 ? "s" : ""}`;
  if (report.unlinked.length > 0) {
    const names = report.unlinked.map((n) => `« ${n} »`).join(", ");
    const lead = linked > 0 ? `${plural(linked, "photo")} raccordée${linked > 1 ? "s" : ""} automatiquement (${time}). ` : "";
    return {
      tone: "warning",
      text: `${lead}Aucun détail commun trouvé pour ${names} : poser ses points à la main, ou la retirer.`,
    };
  }
  const check = outliers > 0 ? ` ${plural(outliers, "point")} douteux, en rouge, à vérifier.` : " Vérifier les points, puis « Calculer l'assemblage ».";
  return { tone: "success", text: `${plural(linked, "photo")} raccordée${linked > 1 ? "s" : ""} automatiquement (${time}).${check}` };
}

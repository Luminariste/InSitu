// Mise à l'échelle de la façade : la logique pure de l'écran Façade, testée
// sans écran (`npm test`). Le calcul qui fait foi, et l'enregistrement, sont
// dans le moteur (`facade::set_scale`) ; ici on ne fait que préparer la
// saisie et l'affichage.

import type { Placement, Point } from "../viewer";

/** Mise à l'échelle telle qu'enregistrée dans le projet. */
export interface ScaleSpec {
  pointA: Point;
  pointB: Point;
  distanceMm: number;
}

const UNITS_MM: Record<string, number> = { mm: 1, cm: 10, m: 1000 };

/**
 * Lit une cote saisie à la main et la rend en millimètres. Accepte la
 * virgule ou le point décimal, les espaces entre milliers, et une unité
 * facultative (mm, cm, m) ; sans unité, la valeur est en millimètres.
 * Renvoie `null` si la saisie n'est pas une longueur positive.
 */
export function parseDistanceMm(text: string): number | null {
  const match = /^\s*([0-9][0-9\s  ]*(?:[.,][0-9]+)?)\s*(mm|cm|m)?\s*$/i.exec(text);
  if (!match) return null;
  const value = Number(match[1].replace(/[\s  ]/g, "").replace(",", "."));
  const unit = (match[2] ?? "mm").toLowerCase();
  const mm = value * UNITS_MM[unit];
  return Number.isFinite(mm) && mm > 0 ? mm : null;
}

/** Millimètres réels par unité du fichier, ou `null` si l'échelle est inutilisable. */
export function mmPerSourceUnit(scale: ScaleSpec | null | undefined): number | null {
  if (!scale) return null;
  const length = Math.hypot(scale.pointB.x - scale.pointA.x, scale.pointB.y - scale.pointA.y);
  const factor = scale.distanceMm / length;
  return length > 0 && Number.isFinite(factor) && factor > 0 ? factor : null;
}

/**
 * Place le plan dans la visionneuse. Une fois à l'échelle, le repère de la
 * visionneuse est en millimètres réels ; avant, il est dans l'unité du
 * fichier (un « mm » affiché vaut alors une unité du plan).
 */
export function facadePlacement(sourceUnitsPerPixel: number, mmPerUnit: number | null): Placement {
  return { x: 0, y: 0, mmPerPixel: sourceUnitsPerPixel * (mmPerUnit ?? 1) };
}

/** Passe d'un point de la visionneuse à un point du fichier. */
export function viewToSource(point: Point, mmPerUnit: number | null): Point {
  const k = mmPerUnit ?? 1;
  return { x: point.x / k, y: point.y / k };
}

/** Passe d'un point du fichier à un point de la visionneuse. */
export function sourceToView(point: Point, mmPerUnit: number | null): Point {
  const k = mmPerUnit ?? 1;
  return { x: point.x * k, y: point.y * k };
}

/** Distance entre deux points du fichier, dans son unité. */
export function sourceDistance(a: Point, b: Point): number {
  return Math.hypot(b.x - a.x, b.y - a.y);
}

const metres = new Intl.NumberFormat("fr-FR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const millimetres = new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 1 });

/** Longueur lisible : « 84,10 m », ou « 350 mm » sous le mètre. */
export function formatLength(mm: number): string {
  return Math.abs(mm) >= 1000 ? `${metres.format(mm / 1000)} m` : `${millimetres.format(mm)} mm`;
}

/**
 * Vrai si un appui suivi d'un relâchement est un clic (et non un
 * glisser-déplacer de la visionneuse) : moins de 4 px de déplacement et
 * moins de 0,6 s.
 */
export function isClick(down: { x: number; y: number; t: number }, up: { x: number; y: number; t: number }): boolean {
  return Math.hypot(up.x - down.x, up.y - down.y) < 4 && up.t - down.t < 600;
}

/** Ajoute `.insitu` au nom de dossier choisi s'il n'y est pas. */
export function withProjectExtension(path: string): string {
  return /\.insitu\/?$/i.test(path) ? path.replace(/\/$/, "") : `${path.replace(/\/$/, "")}.insitu`;
}

/** Nom du projet d'après son dossier : « …/Épinal.insitu » donne « Épinal ». */
export function projectNameFromPath(path: string): string {
  const last = path.replace(/\/$/, "").split("/").pop() ?? "";
  return last.replace(/\.insitu$/i, "");
}

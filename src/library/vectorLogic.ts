// Logique pure de la comparaison image / vecteurs (tâche 5.2) : textes,
// lecture des réglages saisis, zoom. Testée sans écran (`vectorLogic.test.ts`).

import { DEFAULT_VECTOR_SETTINGS, type VectorSettings, type VectorSummary } from "../engine";

/** Nombre à la française : « 1 234 », « 0,28 ». */
export function frNumber(value: number, decimals = 0): string {
  return value.toLocaleString("fr-FR", { minimumFractionDigits: decimals, maximumFractionDigits: decimals }).replace(/ /g, " ");
}

/** Écart maximal sous lequel le vecteur suit l'image (critère de 5.1). */
export const GOOD_DEVIATION_PX = 0.5;

/** Ligne courte sous l'aperçu : « 312 contours · 4 580 morceaux · écart 0,28 px ». */
export function vectorLine(summary: VectorSummary): string {
  const pieces = summary.stats.lines + summary.stats.cubics;
  return [
    `${frNumber(summary.stats.contours)} contour${summary.stats.contours > 1 ? "s" : ""}`,
    `${frNumber(pieces)} morceau${pieces > 1 ? "x" : ""}`,
    `écart ${frNumber(summary.comparison.maxDeviationPx, 2)} px`,
  ].join(" · ");
}

/** Verdict sur l'écart, pour la couleur et la phrase d'aide. */
export function deviationVerdict(summary: VectorSummary): { good: boolean; text: string } {
  const good = summary.comparison.maxDeviationPx < GOOD_DEVIATION_PX;
  return {
    good,
    text: good
      ? "Les courbes suivent l'image à moins d'un demi-pixel."
      : "Les courbes s'écartent de l'image de plus d'un demi-pixel : baisser la tolérance.",
  };
}

/** Réglages tels que saisis : du texte, pour accepter « 0,3 ». */
export type SettingsDraft = Record<keyof VectorSettings, string>;

export function draftOf(settings: VectorSettings): SettingsDraft {
  return {
    threshold: String(settings.threshold),
    tolerancePx: frNumber(settings.tolerancePx, decimalsOf(settings.tolerancePx)),
    cornerAngleDeg: frNumber(settings.cornerAngleDeg, decimalsOf(settings.cornerAngleDeg)),
    minAreaPx: frNumber(settings.minAreaPx, decimalsOf(settings.minAreaPx)),
  };
}

function decimalsOf(value: number): number {
  const text = String(value);
  const dot = text.indexOf(".");
  return dot < 0 ? 0 : Math.min(3, text.length - dot - 1);
}

function parseNumber(text: string): number {
  const cleaned = text.trim().replace(/\s/g, "").replace(",", ".");
  return cleaned === "" ? NaN : Number(cleaned);
}

/**
 * Lit les réglages saisis. Renvoie les réglages, ou le premier problème en
 * français. Les bornes sont celles du moteur (`vectorize::validate`).
 */
export function parseDraft(draft: SettingsDraft): { settings: VectorSettings } | { error: string } {
  const threshold = parseNumber(draft.threshold);
  if (!Number.isInteger(threshold) || threshold < 1 || threshold > 255) {
    return { error: "Le seuil est un nombre entier de 1 à 255." };
  }
  const tolerancePx = parseNumber(draft.tolerancePx);
  if (!(tolerancePx >= 0.01 && tolerancePx <= 10)) {
    return { error: "La tolérance va de 0,01 à 10 pixels." };
  }
  const cornerAngleDeg = parseNumber(draft.cornerAngleDeg);
  if (!(cornerAngleDeg >= 1 && cornerAngleDeg <= 179)) {
    return { error: "L'angle des coins va de 1 à 179 degrés." };
  }
  const minAreaPx = parseNumber(draft.minAreaPx);
  if (!(minAreaPx >= 0 && Number.isFinite(minAreaPx))) {
    return { error: "L'aire minimale est un nombre positif ou nul." };
  }
  return { settings: { threshold, tolerancePx, cornerAngleDeg, minAreaPx } };
}

export function sameSettings(a: VectorSettings, b: VectorSettings): boolean {
  return (
    a.threshold === b.threshold &&
    a.tolerancePx === b.tolerancePx &&
    a.cornerAngleDeg === b.cornerAngleDeg &&
    a.minAreaPx === b.minAreaPx
  );
}

export function isDefault(settings: VectorSettings): boolean {
  return sameSettings(settings, DEFAULT_VECTOR_SETTINGS);
}

// --- Zoom ---------------------------------------------------------------------

/** Vue : échelle (pixels d'écran par pixel du dessin) et position du coin haut-gauche du dessin. */
export interface View {
  scale: number;
  x: number;
  y: number;
}

export const MAX_SCALE = 64;

/** Le dessin entier, centré, avec une marge. */
export function fitView(width: number, height: number, stageWidth: number, stageHeight: number, margin = 16): View {
  const scale = Math.max(1e-3, Math.min((stageWidth - 2 * margin) / width, (stageHeight - 2 * margin) / height));
  return { scale, x: (stageWidth - width * scale) / 2, y: (stageHeight - height * scale) / 2 };
}

/** Zoom de `factor` autour du point d'écran (`cx`, `cy`), qui reste sous le pointeur. */
export function zoomAt(view: View, factor: number, cx: number, cy: number, minScale: number): View {
  const scale = Math.min(MAX_SCALE, Math.max(minScale, view.scale * factor));
  const k = scale / view.scale;
  return { scale, x: cx - (cx - view.x) * k, y: cy - (cy - view.y) * k };
}

/** Échelle affichée en pour cent : 100 % = un pixel du dessin par pixel d'écran. */
export function scaleLabel(scale: number): string {
  return `${frNumber(scale * 100, scale < 0.1 ? 1 : 0)} %`;
}

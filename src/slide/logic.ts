// Vue diapositive (tâche 3.6) : logique pure, testée dans logic.test.ts.

import type { SlideView } from "../engine";
import type { LayerSpec } from "../viewer";

export const SLIDE_MM = 280;

/** Couleurs proposées pour les dessins posés sur la photo. */
export const TINTS: { value: number; label: string }[] = [
  { value: 0x00e5ff, label: "Cyan" },
  { value: 0xffe600, label: "Jaune" },
  { value: 0xff3df2, label: "Magenta" },
  { value: 0xffffff, label: "Blanc" },
];

/** Le carré de la diapositive, coin haut-gauche puis bas-droit (maillage 1 × 1). */
export const SLIDE_SQUARE = [
  { x: 0, y: 0 },
  { x: SLIDE_MM, y: 0 },
  { x: 0, y: SLIDE_MM },
  { x: SLIDE_MM, y: SLIDE_MM },
];

/** Calque des dessins calculés, posés sur la diapositive (mm). */
export function drawingsLayer(url: string | null, visible: boolean, opacity: number, tint: number): LayerSpec[] {
  if (!url) return [];
  return [
    {
      id: "diapositive-calculee",
      kind: "mesh-image",
      url,
      divisions: 1,
      points: SLIDE_SQUARE,
      opacity,
      visible,
      tint,
    },
  ];
}

/** Projecteur suivant (`step` = 1) ou précédent (-1), en boucle. */
export function nextProjector(views: SlideView[], current: string | null, step: number): string | null {
  if (views.length === 0) return null;
  const index = views.findIndex((v) => v.projectorId === current);
  if (index < 0) return views[0].projectorId;
  return views[(index + step + views.length) % views.length].projectorId;
}

/** Ce qu'on dit d'un projecteur quand rien ne peut être montré. */
export function viewProblem(view: SlideView | null): string | null {
  if (!view) return null;
  if (!view.rectifiedPhoto && !view.preview) {
    return `${view.label} : pas de photo redressée${view.problem ? ` ; ${view.problem}` : ""}.`;
  }
  if (!view.rectifiedPhoto) return `${view.label} : pas de photo redressée (onglet Redressement).`;
  if (!view.preview) return `${view.label} : ${view.problem ?? "diapositive non calculée"}.`;
  return null;
}

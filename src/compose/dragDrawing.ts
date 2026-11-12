// Glisser d'un dessin de la liste vers la façade (tâche 4.2). Logique pure,
// testée sans écran (`dragDrawing.test.ts`).

import type { DragEvent } from "react";

/** Type des données glissées depuis la liste des dessins. */
export const DRAWING_DRAG_TYPE = "application/x-insitu-drawing";

/**
 * Le dessin en cours de glisser, gardé aussi hors du presse-papiers du
 * glisser : la vue web du Mac ne rend pas toujours les données d'un type
 * maison pendant le survol ou au lâcher (retour d'essai de la 0.1.28).
 */
let current: string | null = null;

/** Le dessin en cours de glisser, ou rien. */
export function draggedDrawing(): string | null {
  return current;
}

export function startDrawingDrag(event: DragEvent<HTMLElement>, id: string): void {
  current = id;
  event.dataTransfer.setData(DRAWING_DRAG_TYPE, id);
  // Un type courant en plus : certaines vues web n'annoncent que lui.
  event.dataTransfer.setData("text/plain", `insitu-dessin:${id}`);
  event.dataTransfer.effectAllowed = "copy";
}

export function endDrawingDrag(): void {
  current = null;
}

/** Le dessin lâché, d'après les données du glisser ou, à défaut, le glisser en cours. */
export function droppedDrawingId(data: { getData(type: string): string }, current: string | null): string | null {
  const own = data.getData(DRAWING_DRAG_TYPE);
  if (own) return own;
  const text = data.getData("text/plain");
  if (text.startsWith("insitu-dessin:")) return text.slice("insitu-dessin:".length);
  return current;
}

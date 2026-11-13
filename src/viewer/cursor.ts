// Curseur posé par un écran sur sa visionneuse (poignées, dessins, nœuds).
//
// La zone de la visionneuse (`.viewer-canvas`) montre la main « grab » par
// défaut ; un curseur posé sur l'élément parent ne l'emportait donc jamais,
// et le curseur ne changeait pas sur les poignées (retour d'essai de
// Benjamin, 0.1.39). Le curseur passe désormais par la variable CSS
// `--viewer-cursor`, que `.viewer-canvas` lit, et par l'élément lui-même
// (pendant un geste, le pointeur est capturé par lui). Chaîne vide : retour
// à la main.

export function setViewerCursor(element: HTMLElement | null | undefined, cursor: string): void {
  if (!element) return;
  if (cursor) element.style.setProperty("--viewer-cursor", cursor);
  else element.style.removeProperty("--viewer-cursor");
  element.style.cursor = cursor;
}

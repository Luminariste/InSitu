// Orientation du plan de façade (D15-k) : quarts de tour appliqués au
// fichier, dans le sens des aiguilles d'une montre.

/** Ramène un nombre de quarts de tour entre 0 et 3. */
export function normalizeTurns(turns: number | undefined): 0 | 1 | 2 | 3 {
  return (((Math.round(turns ?? 0) % 4) + 4) % 4) as 0 | 1 | 2 | 3;
}

/** L'orientation du plan, en mots, pour l'onglet Plan. */
export function rotationLabel(turns: number | undefined): string {
  switch (normalizeTurns(turns)) {
    case 1:
      return "Tourné d'un quart de tour à droite";
    case 2:
      return "Tourné d'un demi-tour";
    case 3:
      return "Tourné d'un quart de tour à gauche";
    default:
      return "Comme dans le fichier";
  }
}

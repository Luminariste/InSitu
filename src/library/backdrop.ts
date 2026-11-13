// Fond derrière les dessins (Bibliothèque, gomme et crayon, silhouette,
// comparaison des vecteurs) : le noir des dessins est rendu transparent, on
// choisit ce qu'on voit derrière. Un seul choix pour toute la bibliothèque et
// ses éditeurs, retenu d'un lancement à l'autre. Logique pure, testée sans
// écran (`backdrop.test.ts`).

export type Backdrop = "damier" | "noir" | "facade";

export const BACKDROPS: { value: Backdrop; label: string; title: string }[] = [
  { value: "noir", label: "Noir", title: "Fond noir, comme sur la plaque et à la projection" },
  { value: "damier", label: "Damier", title: "Damier : montre ce qui est transparent" },
  { value: "facade", label: "Pierre", title: "Couleur de pierre, comme sur la façade" },
];

const STORAGE_KEY = "insitu.fondDessins";

/** Le fond lu dans un texte enregistré ; damier si le texte est inconnu. */
export function parseBackdrop(saved: string | null): Backdrop {
  return BACKDROPS.some((b) => b.value === saved) ? (saved as Backdrop) : "damier";
}

export function readBackdrop(): Backdrop {
  try {
    return parseBackdrop(localStorage.getItem(STORAGE_KEY));
  } catch {
    return "damier";
  }
}

export function saveBackdrop(backdrop: Backdrop) {
  try {
    localStorage.setItem(STORAGE_KEY, backdrop);
  } catch {
    // Sans conséquence : le damier reviendra au prochain lancement.
  }
}

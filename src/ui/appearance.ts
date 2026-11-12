// Apparence de l'app : automatique (suit macOS), sombre ou clair.
// Le choix est gardé dans le navigateur de l'app (simple confort) et posé
// sur <html data-theme>, que lisent les jetons (`tokens.css`).

export type Appearance = "auto" | "dark" | "light";

const KEY = "insitu.apparence";

export const APPEARANCES: { value: Appearance; label: string }[] = [
  { value: "auto", label: "Automatique (comme macOS)" },
  { value: "dark", label: "Sombre" },
  { value: "light", label: "Clair" },
];

export function readAppearance(): Appearance {
  try {
    const stored = localStorage.getItem(KEY);
    if (stored === "auto" || stored === "dark" || stored === "light") return stored;
  } catch {
    // Stockage indisponible : réglage par défaut.
  }
  return "dark";
}

export function applyAppearance(appearance: Appearance): void {
  document.documentElement.dataset.theme = appearance;
  try {
    localStorage.setItem(KEY, appearance);
  } catch {
    // On s'en passe.
  }
}

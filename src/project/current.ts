// Le projet ouvert, partagé par les écrans Façade et Redressement : chaque
// écran rouvre le dernier projet utilisé (mémorisé dans le navigateur de
// l'app, simple confort ; rien d'autre n'y est gardé).

const LAST_PROJECT_KEY = "insitu.dernierProjet";

export function rememberProject(folder: string | null): void {
  try {
    if (folder) localStorage.setItem(LAST_PROJECT_KEY, folder);
    else localStorage.removeItem(LAST_PROJECT_KEY);
  } catch {
    // Stockage indisponible : on s'en passe.
  }
}

export function recalledProject(): string | null {
  try {
    return localStorage.getItem(LAST_PROJECT_KEY);
  } catch {
    return null;
  }
}

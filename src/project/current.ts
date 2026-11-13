// Le projet ouvert, partagé par tous les écrans : chaque
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

/** Le projet montré par l'écran : son nom et son dossier. */
export interface OpenProject {
  name: string | null;
  folder: string | null;
}

const PROJECT_EVENT = "insitu:projet";

/** L'écran dit quel projet il montre (pour la barre de l'app). */
export function announceProject(name: string | null, folder: string | null): void {
  window.dispatchEvent(new CustomEvent<OpenProject>(PROJECT_EVENT, { detail: { name, folder } }));
}

/** Écoute le projet montré ; renvoie de quoi arrêter d'écouter. */
export function listenProject(listener: (project: OpenProject) => void): () => void {
  const handler = (event: Event) => listener((event as CustomEvent<OpenProject>).detail);
  window.addEventListener(PROJECT_EVENT, handler);
  return () => window.removeEventListener(PROJECT_EVENT, handler);
}

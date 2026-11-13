// Mise à jour automatique (D16) : textes du dialogue, sans écran ni moteur.
import type { UpdateProgress } from "../engine";

const MEGA = 1024 * 1024;

function megabytes(bytes: number): string {
  return `${(bytes / MEGA).toLocaleString("fr-FR", { minimumFractionDigits: 1, maximumFractionDigits: 1 })} Mo`;
}

/** « 12,3 Mo sur 45,6 Mo », ou « 12,3 Mo » si la taille totale est inconnue. */
export function progressLabel({ done, total }: UpdateProgress): string {
  if (total && total > 0) return `${megabytes(Math.min(done, total))} sur ${megabytes(total)}`;
  return megabytes(done);
}

/** Part téléchargée, entre 0 et 1 ; `null` si la taille totale est inconnue. */
export function progressShare({ done, total }: UpdateProgress): number | null {
  if (!total || total <= 0) return null;
  return Math.max(0, Math.min(1, done / total));
}

/** Dernière étape : le téléchargement est fini, l'app est remplacée. */
export function isDownloaded(progress: UpdateProgress): boolean {
  const share = progressShare(progress);
  return share !== null && share >= 1;
}

/**
 * Message d'erreur lisible. Le cas le plus fréquent (pas de réseau, GitHub
 * injoignable) est dit simplement ; le détail technique suit.
 */
export function failureMessage(error: unknown): string {
  const detail = error instanceof Error ? error.message : String(error);
  const network = /network|connect|dns|timed? ?out|resolve|request/i.test(detail);
  const head = network
    ? "La mise à jour n'a pas pu être téléchargée (connexion à GitHub impossible)."
    : "La mise à jour n'a pas pu être installée.";
  return `${head} L'app actuelle reste en place. Détail : ${detail}`;
}

/**
 * Pourquoi une recherche demandée à la main n'a pas abouti. Le cas le plus
 * fréquent (pas de réseau, GitHub injoignable) est dit simplement.
 */
export function checkFailureMessage(error: unknown): string {
  const detail = error instanceof Error ? error.message : String(error);
  const network = /network|connect|dns|timed? ?out|resolve|request/i.test(detail);
  const head = network
    ? "Impossible de joindre GitHub pour savoir s'il existe une nouvelle version : vérifier la connexion à Internet."
    : "La recherche de mise à jour n'a pas abouti.";
  return `${head} Détail : ${detail}`;
}

/** Ce qu'on dit quand l'app est déjà la dernière version publiée. */
export function upToDateMessage(version: string | null): string {
  return version ? `InSitu ${version} est la dernière version publiée.` : "InSitu est la dernière version publiée.";
}

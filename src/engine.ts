// Pont vers le moteur Rust. Toute commande du moteur passe par ce fichier,
// pour que l'interface n'appelle jamais `invoke` directement.
import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Pyramid, TileSource } from "./viewer";

export interface EngineInfo {
  name: string;
  version: string;
  slideSizeMm: number;
  plateSizeMm: number;
  defaultExportDpi: number;
  defaultExportPx: number;
}

/** Vrai dans l'application, faux dans un simple navigateur (`npm run dev`). */
export function hasEngine(): boolean {
  return isTauri();
}

/**
 * Carte d'identité du moteur. Renvoie `null` quand l'interface tourne dans un
 * simple navigateur (`npm run dev` sans Tauri) : le moteur n'y existe pas.
 */
export async function fetchEngineInfo(): Promise<EngineInfo | null> {
  if (!isTauri()) return null;
  return invoke<EngineInfo>("engine_info");
}

/**
 * Schéma d'adresse sous lequel la coque sert le cache de tuiles (voir
 * `src-tauri/src/lib.rs`). Forme propre à macOS, seule cible de l'app.
 */
const TILE_SCHEME_ROOT = "tuiles://localhost";

/**
 * Prépare une grande image pour la visionneuse : le moteur fabrique sa
 * pyramide de tuiles dans le cache de l'app (une seule fois par image, tant
 * qu'elle ne change pas) et l'interface reçoit de quoi les afficher.
 */
export async function openTiledImage(path: string): Promise<TileSource> {
  const cached = await invoke<{ key: string; pyramid: Pyramid }>("open_tiled_image", { path });
  const baseUrl = `${TILE_SCHEME_ROOT}/${cached.key}`;
  // Vérifie tout de suite que l'interface arrive à lire le cache : mieux vaut
  // une erreur claire ici que des tuiles qui restent noires.
  const probe = await fetch(`${baseUrl}/pyramid.json`).catch((error: unknown) => {
    throw new Error(`le cache de tuiles est illisible depuis l'interface (${String(error)})`);
  });
  if (!probe.ok) throw new Error(`le cache de tuiles a répondu ${probe.status}`);
  return { pyramid: cached.pyramid, baseUrl };
}

/**
 * Ouvre une pyramide déjà fabriquée par `insitu tiles` et servie par HTTP
 * (dossier `tuiles-demo/` en développement). Renvoie `null` si elle n'y est pas.
 */
export async function openTileFolder(baseUrl: string): Promise<TileSource | null> {
  const response = await fetch(`${baseUrl}/pyramid.json`).catch(() => null);
  if (!response || !response.ok) return null;
  const pyramid = (await response.json()) as Pyramid;
  return { pyramid, baseUrl };
}

/**
 * Dossier du jeu d'essai sur cette machine, ou `null` s'il est introuvable
 * (app installée sans le dépôt) ou si l'interface tourne sans moteur.
 */
export async function fetchTestSetDir(): Promise<string | null> {
  if (!isTauri()) return null;
  return invoke<string | null>("test_set_dir");
}

// Pont vers le moteur Rust. Toute commande du moteur passe par ce fichier,
// pour que l'interface n'appelle jamais `invoke` directement.
import { invoke, isTauri } from "@tauri-apps/api/core";

export interface EngineInfo {
  name: string;
  version: string;
  slideSizeMm: number;
  plateSizeMm: number;
  defaultExportDpi: number;
  defaultExportPx: number;
}

/**
 * Carte d'identité du moteur. Renvoie `null` quand l'interface tourne dans un
 * simple navigateur (`npm run dev` sans Tauri) : le moteur n'y existe pas.
 */
export async function fetchEngineInfo(): Promise<EngineInfo | null> {
  if (!isTauri()) return null;
  return invoke<EngineInfo>("engine_info");
}

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

// --- Projet et façade (tâche 3.1) ------------------------------------------

/** Un point repéré dans un fichier importé, dans l'unité de ce fichier. */
export interface SourcePoint {
  x: number;
  y: number;
}

export interface FacadeScale {
  pointA: SourcePoint;
  pointB: SourcePoint;
  distanceMm: number;
}

/** La façade d'un projet, telle qu'enregistrée dans `projet.json`. */
export interface Facade {
  /** Chemin relatif au dossier du projet, par exemple `facade/plan.pdf`. */
  source: string;
  sourceWidth: number;
  sourceHeight: number;
  scale: FacadeScale | null;
  page?: number;
  origin?: SourcePoint;
}

/** Ce que l'interface lit d'un projet ; les autres champs sont ignorés ici. */
export interface Project {
  formatVersion: number;
  id: string;
  name: string;
  facade: Facade | null;
}

export type PlanKind = "image" | "pdf" | "svg" | "dxf";

/** Ce que le moteur dit d'un fichier de plan avant import. */
export interface PlanInfo {
  kind: PlanKind;
  width: number;
  height: number;
  unit: string;
  pageCount?: number;
  page?: number;
  origin?: [number, number];
  declaredMmPerUnit?: number;
}

/** Plan prêt à afficher : ses tuiles, et l'unité du fichier par pixel. */
export interface FacadeView {
  kind: PlanKind;
  source: TileSource;
  sourceWidth: number;
  sourceHeight: number;
  sourceUnitsPerPixel: number;
}

function requireEngine(): void {
  if (!isTauri()) throw new Error("cette action demande l'application (le moteur n'est pas joignable dans un simple navigateur)");
}

export async function createProject(folder: string, name: string): Promise<Project> {
  requireEngine();
  return invoke<Project>("project_create", { folder, name });
}

export async function openProject(folder: string): Promise<Project> {
  requireEngine();
  return invoke<Project>("project_open", { folder });
}

export async function inspectPlan(path: string, page?: number): Promise<PlanInfo> {
  requireEngine();
  return invoke<PlanInfo>("facade_inspect", { path, page: page ?? null });
}

export async function importFacade(folder: string, path: string, page?: number): Promise<Project> {
  requireEngine();
  return invoke<Project>("facade_import", { folder, path, page: page ?? null });
}

export async function setFacadeScale(
  folder: string,
  a: SourcePoint,
  b: SourcePoint,
  distanceMm: number,
): Promise<Project> {
  requireEngine();
  return invoke<Project>("facade_set_scale", { folder, a, b, distanceMm });
}

/**
 * Prépare l'affichage de la façade du projet. Pour un plan vectoriel, les
 * tuiles fines sont rendues par le moteur au moment où la visionneuse les
 * demande : le plan reste net à tout zoom.
 */
export async function openFacadeView(folder: string): Promise<FacadeView> {
  requireEngine();
  const shown = await invoke<{
    key: string;
    pyramid: Pyramid;
    kind: PlanKind;
    sourceWidth: number;
    sourceHeight: number;
    sourceUnitsPerPixel: number;
  }>("facade_display", { folder });
  return {
    kind: shown.kind,
    source: { pyramid: shown.pyramid, baseUrl: `${TILE_SCHEME_ROOT}/${shown.key}` },
    sourceWidth: shown.sourceWidth,
    sourceHeight: shown.sourceHeight,
    sourceUnitsPerPixel: shown.sourceUnitsPerPixel,
  };
}

// --- Fenêtres de choix de fichiers (macOS) ----------------------------------

/** Extensions de plan acceptées pour la façade. */
export const PLAN_EXTENSIONS = ["pdf", "png", "jpg", "jpeg", "tif", "tiff", "svg", "dxf"];

/** Demande un dossier de projet existant ; `null` si l'utilisateur renonce. */
export async function chooseProjectFolder(): Promise<string | null> {
  requireEngine();
  const { open } = await import("@tauri-apps/plugin-dialog");
  const chosen = await open({ directory: true, multiple: false, title: "Ouvrir un projet InSitu (dossier .insitu)" });
  return typeof chosen === "string" ? chosen : null;
}

/** Demande où créer un nouveau projet ; `null` si l'utilisateur renonce. */
export async function chooseNewProjectPath(): Promise<string | null> {
  requireEngine();
  const { save } = await import("@tauri-apps/plugin-dialog");
  const chosen = await save({ title: "Nouveau projet InSitu", defaultPath: "Nouveau projet.insitu" });
  return chosen ?? null;
}

/** Demande un fichier de plan ; `null` si l'utilisateur renonce. */
export async function choosePlanFile(): Promise<string | null> {
  requireEngine();
  const { open } = await import("@tauri-apps/plugin-dialog");
  const chosen = await open({
    multiple: false,
    directory: false,
    title: "Importer le plan de façade",
    filters: [{ name: "Plan de façade", extensions: PLAN_EXTENSIONS }],
  });
  return typeof chosen === "string" ? chosen : null;
}

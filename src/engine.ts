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

/** Comment une photo brute a été redressée (résumé ; les points restent dans le moteur). */
export interface RectificationInfo {
  method: "automatic" | "manual";
  residualMm: number | null;
}

/** Un rétroprojecteur du projet. */
export interface Projector {
  id: string;
  label: string;
  /** Chemins relatifs au dossier du projet. */
  rawPhoto: string | null;
  rectifiedPhoto: string | null;
  rectification: RectificationInfo | null;
  /** Recalage sur la façade : points communs (tâche 3.2). */
  registration?: RegistrationPair[];
}

/** Ce que l'interface lit d'un projet ; les autres champs sont ignorés ici. */
export interface Project {
  formatVersion: number;
  id: string;
  name: string;
  facade: Facade | null;
  projectors: Projector[];
  /** La fresque importée en une image (tâche 3.5). */
  fresco?: FrescoImage | null;
  /** Réglages d'export des diapositives (tâche 3.5). */
  export?: ExportSettings;
}

/** La fresque en une image et le rectangle de façade qu'elle couvre (mm). */
export interface FrescoImage {
  source: string;
  frame: { x: number; y: number; width: number; height: number };
}

/** Réglages d'export des diapositives. */
export interface ExportSettings {
  dpi: number;
  mirror: boolean;
  negative: boolean;
  /** Plaque de 300 mm avec nom et repères dans la marge ; sinon 280 mm. */
  margin: boolean;
  /** Noir et blanc pur (PNG 1 bit) ; sinon niveaux de gris. */
  oneBit: boolean;
  threshold: number;
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

// --- Redressement des photos de mire (tâche 2.5) ------------------------------

/** Un point `[x, y]`, comme le moteur l'écrit. */
export type XY = [number, number];

export interface ErrorSummary {
  mean: number;
  p95: number;
  max: number;
}

/** Un croisement de la grille retenu par la détection. */
export interface RectificationNode {
  /** Dans la photo brute, en px (centre du premier pixel en 0,5). */
  photo: XY;
  /** Sur la diapositive, en mm. */
  slide: XY;
  /** Écart de validation croisée, en mm. */
  errorMm: number;
}

/** Redressement d'un projecteur, tel que l'écran l'affiche. */
export interface RectificationView {
  projectorId: string;
  method: "automatic" | "manual";
  photoWidth: number;
  photoHeight: number;
  nodes: RectificationNode[];
  measurableNodes: number;
  outliersRemoved: number;
  /** Coins de la diapositive dans la photo : haut-gauche, haut-droit, bas-droit, bas-gauche. */
  corners: [XY, XY, XY, XY];
  /** Contour du carré de 280 mm dans la photo, fermé. */
  outline: XY[];
  leaveOneOutMm: ErrorSummary;
  estimatedErrorMm: ErrorSummary;
  coverage: number;
  pxPerMm: number;
  seconds: number | null;
  /** Miroir appliqué à la diapositive (gauche-droite, haut-bas). */
  mirror: Mirror;
}

export interface Mirror {
  horizontal: boolean;
  vertical: boolean;
}

export interface ExportedImage {
  path: string;
  format: "png" | "tiff";
  dpi: number;
  sizePx: number;
  bytes: number;
  seconds: number;
}

/** Chemin complet d'un fichier du projet. */
export function projectFile(folder: string, relative: string): string {
  return `${folder.replace(/\/$/, "")}/${relative}`;
}

export async function addProjector(folder: string, label: string): Promise<{ project: Project; id: string }> {
  requireEngine();
  const [project, id] = await invoke<[Project, string]>("projector_add", { folder, label });
  return { project, id };
}

export async function importProjectorPhoto(folder: string, projector: string, path: string): Promise<Project> {
  requireEngine();
  return invoke<Project>("projector_import_photo", { folder, projector, path });
}

/**
 * Détecte la mire et enregistre la photo redressée. Avec `corners` (coins
 * haut-gauche, haut-droit, bas-droit, bas-gauche dans la photo, en px), la
 * recherche automatique est remplacée par ces coins ; la grille affine.
 */
/**
 * Retourne la diapositive d'un projecteur : `change` dit quel axe basculer.
 * La photo redressée est recalculée ; les points du recalage suivent.
 */
export async function mirrorProjector(
  folder: string,
  projector: string,
  change: Mirror,
): Promise<{ project: Project; view: RectificationView }> {
  requireEngine();
  const [project, view] = await invoke<[Project, RectificationView]>("projector_mirror", {
    folder,
    projector,
    change,
  });
  return { project, view };
}

export async function rectifyProjector(
  folder: string,
  projector: string,
  corners: [XY, XY, XY, XY] | null,
): Promise<{ project: Project; view: RectificationView }> {
  requireEngine();
  const [project, view] = await invoke<[Project, RectificationView]>("projector_rectify", {
    folder,
    projector,
    corners,
  });
  return { project, view };
}

/** Redressement enregistré d'un projecteur, ou `null` s'il n'est pas encore fait. */
export async function fetchProjectorView(folder: string, projector: string): Promise<RectificationView | null> {
  requireEngine();
  return invoke<RectificationView | null>("projector_view", { folder, projector });
}

export async function exportRectified(
  folder: string,
  projector: string,
  destination: string,
  dpi: number,
): Promise<ExportedImage> {
  requireEngine();
  return invoke<ExportedImage>("projector_export", { folder, projector, destination, dpi });
}

// --- Fenêtres de choix de fichiers (macOS) ----------------------------------

/** Extensions de plan acceptées pour la façade. */
export const PLAN_EXTENSIONS = ["pdf", "png", "jpg", "jpeg", "tif", "tiff", "heic", "heif", "svg", "dxf"];

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

/** Extensions de photo de mire acceptées (le HEIC des iPhone compris). */
export const PHOTO_EXTENSIONS = ["heic", "heif", "jpg", "jpeg", "png", "tif", "tiff"];

/** Demande une photo de mire ; `null` si l'utilisateur renonce. */
export async function choosePhotoFile(): Promise<string | null> {
  requireEngine();
  const { open } = await import("@tauri-apps/plugin-dialog");
  const chosen = await open({
    multiple: false,
    directory: false,
    title: "Importer la photo de la mire",
    filters: [{ name: "Photo (HEIC, JPEG, PNG, TIFF)", extensions: PHOTO_EXTENSIONS }],
  });
  return typeof chosen === "string" ? chosen : null;
}

/** Demande où exporter la photo redressée ; `null` si l'utilisateur renonce. */
export async function chooseExportPath(defaultName: string): Promise<string | null> {
  requireEngine();
  const { save } = await import("@tauri-apps/plugin-dialog");
  const chosen = await save({
    title: "Exporter la photo redressée",
    defaultPath: defaultName,
    filters: [
      { name: "TIFF", extensions: ["tif", "tiff"] },
      { name: "PNG", extensions: ["png"] },
    ],
  });
  return chosen ?? null;
}

// --- Recalage sur la façade (tâche 3.2) ----------------------------------------

/** Un point commun : sur la diapositive (mm, 0 à 280) et sur la façade (mm réels). */
export interface RegistrationPair {
  slide: { x: number; y: number };
  facade: { x: number; y: number };
}

/** Ce que le moteur dit d'une paire. */
export interface PairReport {
  /** Écart sur la diapositive, en mm ; `null` tant qu'il n'est pas mesurable. */
  slideResidualMm: number | null;
  /** Le même sur la façade, en mm. */
  facadeResidualMm: number | null;
  /** Point jugé aberrant : à vérifier, déplacer ou retirer. */
  outlier: boolean;
}

export interface RegistrationReport {
  projectorId: string;
  stage: "incomplete" | "trapeze" | "corrected";
  pairs: PairReport[];
  rmsSlideMm: number | null;
  rmsFacadeMm: number | null;
  trapezeRmsSlideMm: number | null;
  /** Contour de la diapositive posé sur la façade (mm), fermé. */
  footprint: { x: number; y: number }[];
}

/** Compte rendu du recalage enregistré d'un projecteur. */
export async function fetchRegistration(folder: string, projector: string): Promise<RegistrationReport> {
  requireEngine();
  return invoke<RegistrationReport>("registration_show", { folder, projector });
}

/** Remplace les points communs d'un projecteur ; le projet est enregistré. */
export async function saveRegistration(
  folder: string,
  projector: string,
  pairs: RegistrationPair[],
): Promise<{ project: Project; report: RegistrationReport }> {
  requireEngine();
  const [project, report] = await invoke<[Project, RegistrationReport]>("registration_set", {
    folder,
    projector,
    pairs,
  });
  return { project, report };
}

// --- Bibliothèque de dessins (tâche 4.1) ---------------------------------------

/** Un dessin de la bibliothèque, tel que le moteur le décrit. */
export interface LibraryEntry {
  id: string;
  name: string;
  /** Dossier dans la bibliothèque (`Oiseaux/Grands`), ou vide. */
  folder: string;
  /** Fichier, relatif au dossier projet (`bibliotheque/Oiseaux/merle.png`). */
  source: string;
  /** Taille du fichier : pixels (pixels CSS pour un SVG). */
  width: number;
  height: number;
  /** Vignette dans le cache, à passer à `cacheFileUrl`. */
  thumbnail: string | null;
  /** Ce qui empêche de montrer le dessin (fichier absent, illisible). */
  problem: string | null;
}

export interface ImportProgress {
  done: number;
  total: number;
}

export interface ImportReport {
  /** Dossier qui a reçu les dessins, par exemple `bibliotheque/Oiseaux`. */
  folder: string;
  /** Identifiants des dessins ajoutés. */
  added: string[];
  alreadyPresent: number;
  failures: { file: string; reason: string }[];
}

/** Adresse d'un fichier du cache de l'app (vignette, aperçu). */
export function cacheFileUrl(request: string): string {
  return `${TILE_SCHEME_ROOT}/${request}`;
}

/** Importe un dossier de dessins ; `onProgress` suit l'avancement. */
export async function importDrawingFolder(
  folder: string,
  source: string,
  onProgress: (progress: ImportProgress) => void,
): Promise<{ project: Project; report: ImportReport }> {
  requireEngine();
  const { listen } = await import("@tauri-apps/api/event");
  const stop = await listen<ImportProgress>("bibliotheque-progression", (event) => onProgress(event.payload));
  try {
    const [project, report] = await invoke<[Project, ImportReport]>("library_import", { folder, source });
    return { project, report };
  } finally {
    stop();
  }
}

/** La bibliothèque du projet ; les vignettes manquantes sont refaites. */
export async function fetchLibrary(folder: string): Promise<LibraryEntry[]> {
  requireEngine();
  return invoke<LibraryEntry[]>("library_list", { folder });
}

/** Adresse de l'aperçu (1 024 px, fond transparent) d'un dessin. */
export async function fetchDrawingPreview(folder: string, drawing: string): Promise<string> {
  requireEngine();
  return cacheFileUrl(await invoke<string>("library_preview", { folder, drawing }));
}

/** Demande un dossier de dessins à importer ; `null` si l'utilisateur renonce. */
export async function chooseDrawingFolder(): Promise<string | null> {
  requireEngine();
  const { open } = await import("@tauri-apps/plugin-dialog");
  const chosen = await open({ directory: true, multiple: false, title: "Importer un dossier de dessins" });
  return typeof chosen === "string" ? chosen : null;
}

// --- Vue façade (tâche 3.3) ---------------------------------------------------

/** Photo redressée posée sur la façade : grille de `(divisions + 1)²` points (mm). */
export interface SlideMesh {
  divisions: number;
  facade: SourcePoint[];
}

export interface ProjectorOverview {
  projectorId: string;
  label: string;
  pairs: number;
  /** Emprise sur la façade (mm), fermée ; vide si pas recalé. */
  footprint: SourcePoint[];
  areaM2: number | null;
  mesh: SlideMesh | null;
  /** Aperçu de la photo redressée, dans le cache de l'app. */
  photoPreview: string | null;
  problem: string | null;
}

export interface Overlap {
  projectors: [string, string];
  areaM2: number;
  outlines: SourcePoint[][];
}

export interface FacadeOverview {
  projectors: ProjectorOverview[];
  overlaps: Overlap[];
}

/** Tous les projecteurs du projet posés sur la façade, et leurs recouvrements. */
export async function fetchFacadeOverview(folder: string): Promise<FacadeOverview> {
  requireEngine();
  return invoke<FacadeOverview>("facade_overview", { folder });
}

// --- Fresque et export des diapositives (tâche 3.5) -----------------------------

export interface ExportProgress {
  done: number;
  total: number;
  current: string | null;
}

export interface ExportedSlide {
  projectorId: string;
  label: string;
  path: string;
  sizePx: number;
  sizeMm: number;
  bytes: number;
  seconds: number;
}

export interface ExportReport {
  slides: ExportedSlide[];
  skipped: { projectorId: string; label: string; reason: string }[];
  settings: ExportSettings;
}

/** Extensions acceptées pour la fresque. */
export const FRESCO_EXTENSIONS = ["png", "tif", "tiff", "jpg", "jpeg", "bmp"];

/** Importe la fresque en une image, posée sur toute la largeur de la façade. */
export async function importFresco(folder: string, path: string): Promise<Project> {
  requireEngine();
  return invoke<Project>("fresco_import", { folder, path });
}

/** Écrit un PNG par projecteur recalé dans `destination`. */
export async function exportSlides(
  folder: string,
  destination: string,
  settings: ExportSettings,
  onProgress: (progress: ExportProgress) => void,
): Promise<ExportReport> {
  requireEngine();
  const { listen } = await import("@tauri-apps/api/event");
  const stop = await listen<ExportProgress>("export-progression", (event) => onProgress(event.payload));
  try {
    return await invoke<ExportReport>("slides_export", { folder, destination, settings });
  } finally {
    stop();
  }
}

/** Demande l'image de la fresque ; `null` si l'utilisateur renonce. */
export async function chooseFrescoFile(): Promise<string | null> {
  requireEngine();
  const { open } = await import("@tauri-apps/plugin-dialog");
  const chosen = await open({
    multiple: false,
    directory: false,
    title: "Importer la fresque (une image de toute la façade)",
    filters: [{ name: "Image", extensions: FRESCO_EXTENSIONS }],
  });
  return typeof chosen === "string" ? chosen : null;
}

/** Demande le dossier où écrire les diapositives ; `null` si l'utilisateur renonce. */
export async function chooseExportFolder(): Promise<string | null> {
  requireEngine();
  const { open } = await import("@tauri-apps/plugin-dialog");
  const chosen = await open({ directory: true, multiple: false, title: "Dossier des diapositives à graver" });
  return typeof chosen === "string" ? chosen : null;
}

// --- Vue diapositive (tâche 3.6) -------------------------------------------------

/** Un projecteur dans la vue diapositive. */
export interface SlideView {
  projectorId: string;
  label: string;
  /** Photo redressée, chemin relatif au projet (mm de diapositive). */
  rectifiedPhoto: string | null;
  /** Aperçu de la diapositive calculée, dans le cache de l'app. */
  preview: string | null;
  problem: string | null;
}

/** Photo redressée et diapositive calculée de chaque projecteur. */
export async function fetchSlideViews(folder: string): Promise<SlideView[]> {
  requireEngine();
  return invoke<SlideView[]>("slide_views", { folder });
}

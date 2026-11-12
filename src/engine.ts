// Pont vers le moteur Rust. Toute commande du moteur passe par ce fichier,
// pour que l'interface n'appelle jamais `invoke` directement.
import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Pyramid, TileSource } from "./viewer";
import type { Layer, Placement, SlideAdjustment } from "./compose/placement";
export type { Layer, Placement, SlideAdjustment };

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
/**
 * Écoute la barre de menus du Mac : `handler` reçoit le raccourci de
 * l'entrée choisie (« mod+n »…). Renvoie de quoi arrêter d'écouter. Sans
 * l'app (navigateur seul), ne fait rien.
 */
export function listenMenu(handler: (keys: string) => void): () => void {
  if (!hasEngine()) return () => {};
  let stop: (() => void) | null = null;
  let cancelled = false;
  void import("@tauri-apps/api/event").then(({ listen }) =>
    listen<string>("menu", (event) => handler(event.payload)).then((unlisten) => {
      if (cancelled) unlisten();
      else stop = unlisten;
    }),
  );
  return () => {
    cancelled = true;
    stop?.();
  };
}

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
  /** Quarts de tour (sens des aiguilles d'une montre) appliqués au fichier ; absent : 0 (D15-k). */
  rotation?: number;
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
  /** Dessins posés sur la façade, du fond vers l'avant (tâche 4.2). */
  placements?: Placement[];
  /** Calques de la composition, du fond vers l'avant (tâche 4.3). */
  layers?: Layer[];
  /** Assemblage de photos de façade (tâche 6.4). */
  facadeAssembly?: FacadeAssembly | null;
  /** Dessins de la bibliothèque (seuls les champs lus par l'interface). */
  library?: ProjectDrawing[];
}

/** Un dessin de la bibliothèque, tel qu'enregistré dans `projet.json`. */
export interface ProjectDrawing {
  id: string;
  /** Silhouette retouchée à la main (tâche 4.7) ; absente : détectée. */
  silhouette?: SilhouettePolygon[] | null;
}

/** Un contour de silhouette : points `[u, v]` en fractions du dessin. */
export type SilhouettePolygon = [number, number][];

/** La silhouette d'un dessin (tâche 4.7). */
export interface DrawingSilhouette {
  drawing: string;
  polygons: SilhouettePolygon[];
  /** Vrai si elle a été retouchée à la main. */
  manual: boolean;
}

/** Assemblage de photos de façade, tel qu'enregistré dans `projet.json`. */
export interface FacadeAssembly {
  photos: AssemblyPhoto[];
  output?: { source: string; origin: SourcePoint; scale: number } | null;
}

/** Une photo de l'assemblage ; la première est la référence. */
export interface AssemblyPhoto {
  id: string;
  name: string;
  /** Chemin relatif au projet. */
  source: string;
  width: number;
  height: number;
  /** La photo sur laquelle elle s'appuie (aucune pour la référence). */
  anchor?: string | null;
  pairs?: AssemblyPair[];
}

/** La fresque et le rectangle de façade qu'elle couvre (mm). */
export interface FrescoImage {
  /** L'image affichée ; pour un PDF, un .ai ou un SVG, son aperçu en niveaux de gris. */
  source: string;
  frame: { x: number; y: number; width: number; height: number };
  /** Le fichier vectoriel d'origine (PDF, .ai, SVG), relu à l'export pour chaque diapositive. */
  vector?: { source: string; page?: number } | null;
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
  /** Un SVG en mm à côté de chaque PNG : les courbes des dessins posés (tâche 5.3). */
  svg: boolean;
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

/** Retire le plan de la façade ; le fichier reste dans le projet. */
export async function removeFacade(folder: string): Promise<Project> {
  requireEngine();
  return invoke<Project>("facade_remove", { folder });
}

/**
 * Tourne le plan de la façade de quarts de tour (1 : sens des aiguilles
 * d'une montre, -1 : sens inverse). L'échelle, les dessins posés et les
 * points de recalage tournent avec lui (D15-k).
 */
export async function rotateFacade(folder: string, quarterTurns: number): Promise<Project> {
  requireEngine();
  return invoke<Project>("facade_rotate", { folder, quarterTurns });
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
  /** Pourquoi la grille n'a pas été lue, quand le redressement ne tient qu'aux quatre coins. */
  gridUnread: string | null;
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

/** Renomme un projecteur ; son travail (photos, redressement, recalage) est gardé. */
export async function renameProjector(folder: string, projector: string, label: string): Promise<Project> {
  requireEngine();
  return invoke<Project>("projector_rename", { folder, projector, label });
}

/** Retire un projecteur du projet ; ses photos restent dans le dossier. */
export async function removeProjector(folder: string, projector: string): Promise<Project> {
  requireEngine();
  return invoke<Project>("projector_remove", { folder, projector });
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

/** Tourne la diapositive d'un quart de tour (sens des aiguilles d'une montre). */
export async function rotateProjector(folder: string, projector: string): Promise<{ project: Project; view: RectificationView }> {
  requireEngine();
  const [project, view] = await invoke<[Project, RectificationView]>("projector_rotate", { folder, projector });
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

/** Pose provisoire de la diapositive sur la façade (rien n'est enregistré). */
export interface RegistrationPreview {
  /** Emprise (mm), fermée ; vide sous 4 paires. */
  footprint: SourcePoint[];
  mesh: SlideMesh | null;
}

/** Emprise et maillage pour des paires en cours de glisser (D-3.2-h). */
export async function previewRegistration(pairs: RegistrationPair[]): Promise<RegistrationPreview> {
  requireEngine();
  return invoke<RegistrationPreview>("registration_preview", { pairs });
}

// --- Dessins posés sur la façade (tâche 4.2) ------------------------------------

/** Remplace tous les dessins posés ; le moteur les contrôle et les enregistre. */
export async function savePlacements(folder: string, placements: Placement[]): Promise<Project> {
  requireEngine();
  return invoke<Project>("placements_set", { folder, placements });
}

/** Remplace calques et dessins posés ensemble (tâche 4.3). */
export async function saveComposition(folder: string, layers: Layer[], placements: Placement[]): Promise<Project> {
  requireEngine();
  return invoke<Project>("composition_set", { folder, layers, placements });
}

/** La diapositive d'un dessin posé (tâche 4.5). */
export interface Assignment {
  placement: string;
  /** Projecteur dont la diapositive porte le dessin ; aucun hors de toute emprise. */
  projector: string | null;
  /** Choisi à la main. */
  manual: boolean;
  /** Le dessin tient en entier dans cette diapositive. */
  whole: boolean;
  /** Projecteurs dont la diapositive contient le dessin en entier. */
  candidates: string[];
}

/** La diapositive de chaque dessin posé, proposée par le moteur ou choisie à la main. */
export async function fetchAssignments(folder: string): Promise<Assignment[]> {
  requireEngine();
  return invoke<Assignment[]>("placements_assignment", { folder });
}

// --- Annulation (tâche 4.3) -----------------------------------------------------

/** Annule la dernière action (ou la rétablit) ; rend le projet et ce qui a changé. */
export async function historyStep(folder: string, redo: boolean): Promise<[Project, string]> {
  requireEngine();
  return invoke<[Project, string]>("history_step", { folder, redo });
}

/** Ce que ⌘Z et ⇧⌘Z feraient. */
export async function historyState(folder: string): Promise<{ undo: string | null; redo: string | null; depth: number }> {
  requireEngine();
  return invoke("history_state", { folder });
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
  /** Réglages de vectorisation propres au dessin ; `null` : par défaut. */
  vector: VectorSettings | null;
}

/** Réglages de vectorisation d'un dessin (tâche 5.2). */
export interface VectorSettings {
  /** Seuil, de 1 à 255 : un pixel au moins aussi clair est dans le trait. */
  threshold: number;
  /** Écart toléré entre les courbes et le contour de l'image, en pixels. */
  tolerancePx: number;
  /** Au-delà de cet angle (degrés), un sommet est un angle vif. */
  cornerAngleDeg: number;
  /** Taches et trous plus petits (pixels carrés) ignorés. */
  minAreaPx: number;
}

/** Réglages par défaut, les mêmes que ceux du moteur. */
export const DEFAULT_VECTOR_SETTINGS: VectorSettings = {
  threshold: 128,
  tolerancePx: 0.3,
  cornerAngleDeg: 70,
  minAreaPx: 0,
};

/** Vecteurs d'un dessin, tels que le moteur les décrit (tâche 5.2). */
export interface VectorSummary {
  settings: VectorSettings;
  defaultSettings: boolean;
  /** Cadre des SVG : taille de l'image vectorisée, en pixels. */
  width: number;
  height: number;
  stats: {
    contours: number;
    removedContours: number;
    rawPoints: number;
    lines: number;
    cubics: number;
    maxFitErrorPx: number;
  };
  comparison: {
    maxDeviationPx: number;
    rawToVectorPx: number;
    vectorToRawPx: number;
    sourcePixels: number;
    vectorPixels: number;
    mismatchedPixels: number;
    maxMismatchDistancePx: number;
  };
  seconds: number;
  /** Fichiers du cache, à passer à `cacheFileUrl`. */
  filledSvg: string;
  outlineSvg: string;
  image: string;
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

/** Un lien d'un fichier Illustrator que l'import n'a pas pu poser. */
export interface MissingLink {
  link: string;
  count: number;
  reason: string;
}

/** Compte rendu de l'import d'une composition Illustrator (tâche 6.3). */
export interface CompositionReport {
  found: number;
  placed: number;
  drawings: number;
  folder: string;
  layers: string[];
  missing: MissingLink[];
  embedded: number;
  skewed: number;
}

/** Demande un fichier Illustrator ; `null` si l'utilisateur renonce. */
export async function chooseIllustratorFile(): Promise<string | null> {
  requireEngine();
  const { open } = await import("@tauri-apps/plugin-dialog");
  const chosen = await open({
    multiple: false,
    directory: false,
    title: "Importer une composition Illustrator (dessins liés)",
    filters: [{ name: "Illustrator", extensions: ["ai"] }],
  });
  return typeof chosen === "string" ? chosen : null;
}

/** Recrée la composition d'un .ai : ses images liées deviennent des dessins posés. */
export async function importIllustrator(
  folder: string,
  path: string,
  onProgress: (progress: ImportProgress) => void,
): Promise<{ project: Project; report: CompositionReport }> {
  requireEngine();
  const { listen } = await import("@tauri-apps/api/event");
  const stop = await listen<ImportProgress>("bibliotheque-progression", (event) => onProgress(event.payload));
  try {
    const [project, report] = await invoke<[Project, CompositionReport]>("illustrator_import", { folder, path });
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

/** Retire un dessin de la bibliothèque, et ses placements ; rend le projet et le nombre de placements retirés. */
export async function removeDrawing(folder: string, drawing: string): Promise<[Project, number]> {
  requireEngine();
  return invoke<[Project, number]>("library_remove", { folder, drawing });
}

/** Adresse de l'aperçu (1 024 px, fond transparent) d'un dessin. */
export async function fetchDrawingPreview(folder: string, drawing: string): Promise<string> {
  requireEngine();
  return cacheFileUrl(await invoke<string>("library_preview", { folder, drawing }));
}

export interface VectorizeReport {
  done: number;
  failures: { file: string; reason: string }[];
  seconds: number;
}

/** Vectorise les dessins donnés (tous si la liste est vide) qui ne le sont pas encore. */
export async function vectorizeDrawings(
  folder: string,
  drawings: string[],
  onProgress: (progress: ImportProgress) => void,
): Promise<VectorizeReport> {
  requireEngine();
  const { listen } = await import("@tauri-apps/api/event");
  const stop = await listen<ImportProgress>("vecteurs-progression", (event) => onProgress(event.payload));
  try {
    return await invoke<VectorizeReport>("library_vectorize", { folder, drawings });
  } finally {
    stop();
  }
}

/** Vecteurs d'un dessin, calculés par le moteur s'ils ne sont pas en cache. */
export async function fetchDrawingVectors(folder: string, drawing: string): Promise<VectorSummary> {
  requireEngine();
  return invoke<VectorSummary>("library_vectors", { folder, drawing });
}

/** Change les réglages de vectorisation d'un dessin (`null` : par défaut) ; le projet est enregistré. */
export async function saveVectorSettings(
  folder: string,
  drawing: string,
  settings: VectorSettings | null,
): Promise<{ project: Project; vectors: VectorSummary }> {
  requireEngine();
  const [project, vectors] = await invoke<[Project, VectorSummary]>("library_set_vector_settings", {
    folder,
    drawing,
    settings,
  });
  return { project, vectors };
}

/** Silhouettes des dessins donnés (tous si la liste est vide) : retouchées, sinon détectées. */
export async function fetchSilhouettes(folder: string, drawings: string[]): Promise<DrawingSilhouette[]> {
  requireEngine();
  return invoke<DrawingSilhouette[]>("library_silhouettes", { folder, drawings });
}

/** Retouche la silhouette d'un dessin (`null` : détection automatique) ; le projet est enregistré. */
export async function saveSilhouette(
  folder: string,
  drawing: string,
  polygons: SilhouettePolygon[] | null,
): Promise<{ project: Project; silhouette: DrawingSilhouette }> {
  requireEngine();
  const [project, silhouette] = await invoke<[Project, DrawingSilhouette]>("library_set_silhouette", {
    folder,
    drawing,
    polygons,
  });
  return { project, silhouette };
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
  /** Le SVG écrit à côté, si demandé. */
  svg?: string | null;
  seconds: number;
}

export interface ExportReport {
  slides: ExportedSlide[];
  skipped: { projectorId: string; label: string; reason: string }[];
  settings: ExportSettings;
}

/** Extensions acceptées pour la fresque : images, PDF, Illustrator (compatible PDF), SVG. */
export const FRESCO_EXTENSIONS = ["png", "tif", "tiff", "jpg", "jpeg", "bmp", "pdf", "ai", "svg"];

/** Importe la fresque (image, PDF, .ai ou SVG), posée sur toute la largeur de la façade. */
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
    title: "Importer la fresque (toute la façade : image, PDF ou Illustrator)",
    filters: [{ name: "Fresque", extensions: FRESCO_EXTENSIONS }],
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
  /** Dessins posés que porte cette diapositive, avant leur retouche (tâche 4.6). */
  drawings: SlideDrawing[];
}

/** Un dessin posé situé dans une diapositive, avant sa retouche (tâche 4.6). */
export interface SlideDrawing {
  placement: string;
  drawing: string;
  /** Cases par côté : `(divisions + 1)²` points. */
  divisions: number;
  /** Position (mm de diapositive) du point `(i, j) / divisions` du dessin, ligne par ligne. */
  points: { x: number; y: number }[];
}

/** Enregistre la retouche d'un dessin dans une diapositive (tâche 4.6) ; `null` la retire. */
export async function saveAdjustment(
  folder: string,
  placement: string,
  adjustment: SlideAdjustment | null,
): Promise<Project> {
  requireEngine();
  return invoke<Project>("placement_adjust", { folder, placement, adjustment });
}

/** Photo redressée et diapositive calculée de chaque projecteur. */
export async function fetchSlideViews(folder: string): Promise<SlideView[]> {
  requireEngine();
  return invoke<SlideView[]>("slide_views", { folder });
}

// --- Simulation et alertes (tâche 6.1) ------------------------------------------

/** « cut » : dessin coupé ; « outside » : hors emprise ; « doubled » : projeté deux fois. */
export type AlertKind = "cut" | "outside" | "doubled";

export interface SimulationAlert {
  kind: AlertKind;
  /** Le dessin posé concerné (identifiant du placement), s'il y en a un. */
  placement: string | null;
  /** Les projecteurs en cause. */
  projectors: string[];
  message: string;
  /** Où regarder sur la façade (mm). */
  bounds: { x: number; y: number; width: number; height: number };
}

export interface SimulatedSlide {
  projectorId: string;
  /** Aperçu de la diapositive dans le cache de l'app, à tendre sur le maillage de la vue façade. */
  preview: string | null;
  problem: string | null;
}

export interface Simulation {
  slides: SimulatedSlide[];
  alerts: SimulationAlert[];
}

/** Simulation de tous les projecteurs sur la façade, et ses alertes. */
export async function fetchSimulation(folder: string): Promise<Simulation> {
  requireEngine();
  return invoke<Simulation>("facade_simulation", { folder });
}

// --- Fiche de calepinage (tâche 6.2) --------------------------------------------

export interface SheetReport {
  pages: number;
  slides: number;
  alerts: number;
  /** Plus petit corps de texte de la fiche, en mm. */
  smallestTextMm: number;
}

/** Demande où écrire la fiche de calepinage ; `null` si l'utilisateur renonce. */
export async function chooseSheetPath(defaultName: string): Promise<string | null> {
  requireEngine();
  const { save } = await import("@tauri-apps/plugin-dialog");
  const chosen = await save({
    title: "Fiche de calepinage",
    defaultPath: defaultName,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  return chosen ?? null;
}

/** Écrit la fiche de calepinage (PDF A3), avec les réglages d'export donnés. */
export async function writeLayoutSheet(folder: string, path: string, settings: ExportSettings): Promise<SheetReport> {
  requireEngine();
  return invoke<SheetReport>("layout_sheet", { folder, path, settings });
}

// --- Assemblage de photos de façade (tâche 6.4) -------------------------------

/** Un point commun : dans la photo (`here`) et dans sa photo d'appui (`there`), en pixels. */
export interface AssemblyPair {
  here: SourcePoint;
  there: SourcePoint;
}

export interface AssemblyPairReport {
  /** Écart en pixels de la photo d'appui ; `null` avec moins de 5 points. */
  residualPx: number | null;
  outlier: boolean;
}

export interface AssemblyPhotoReport {
  id: string;
  name: string;
  width: number;
  height: number;
  anchor: string | null;
  pairs: AssemblyPairReport[];
  placed: boolean;
  /** Coins dans le repère de la photo de référence, en pixels. */
  corners: [SourcePoint, SourcePoint, SourcePoint, SourcePoint] | null;
  meanResidualPx: number | null;
  problem: string | null;
}

export interface AssemblyReport {
  photos: AssemblyPhotoReport[];
  /** Toutes les photos sont placées : l'assemblage peut être calculé. */
  ready: boolean;
}

export interface AssemblyRenderReport {
  width: number;
  height: number;
  scale: number;
  keptScale: boolean;
  seconds: number;
}

export async function fetchAssembly(folder: string): Promise<AssemblyReport> {
  requireEngine();
  return invoke<AssemblyReport>("assembly_show", { folder });
}

/** Ajoute une photo ; elle s'appuie sur `anchor`, ou sur la dernière ajoutée. */
export async function addAssemblyPhoto(folder: string, path: string, anchor: string | null): Promise<{ project: Project; id: string }> {
  requireEngine();
  const [project, id] = await invoke<[Project, string]>("assembly_add", { folder, path, anchor });
  return { project, id };
}

export async function removeAssemblyPhoto(folder: string, photo: string): Promise<Project> {
  requireEngine();
  return invoke<Project>("assembly_remove", { folder, photo });
}

/** Remplace la photo d'appui et les points communs d'une photo ; le projet est enregistré. */
export async function saveAssemblyPairs(
  folder: string,
  photo: string,
  anchor: string,
  pairs: AssemblyPair[],
): Promise<{ project: Project; report: AssemblyReport }> {
  requireEngine();
  const [project, report] = await invoke<[Project, AssemblyReport]>("assembly_set_pairs", { folder, photo, anchor, pairs });
  return { project, report };
}

/** Ce que le raccord automatique dit d'une photo (tâche 6.6). */
export interface AutoPhotoReport {
  id: string;
  name: string;
  /** Photo d'appui choisie ; `null` pour la référence et une photo non raccordée. */
  anchor: string | null;
  /** Détails communs trouvés avec l'appui. */
  matches: number;
  /** Points communs enregistrés. */
  pairs: number;
  /** Raccordée : ses points ont été remplacés. */
  linked: boolean;
}

export interface AutoReport {
  photos: AutoPhotoReport[];
  /** Photos sans détail commun avec les autres (leurs points sont gardés). */
  unlinked: string[];
  seconds: number;
  assembly: AssemblyReport;
}

/** Raccord automatique : le moteur trouve les points communs et l'appui de chaque photo. */
export async function autoAssembly(folder: string): Promise<{ project: Project; report: AutoReport }> {
  requireEngine();
  const [project, report] = await invoke<[Project, AutoReport]>("assembly_auto", { folder });
  return { project, report };
}

/** Calcule l'image assemblée ; elle devient la façade du projet. */
export async function renderAssembly(folder: string): Promise<{ project: Project; report: AssemblyRenderReport }> {
  requireEngine();
  const [project, report] = await invoke<[Project, AssemblyRenderReport]>("assembly_render", { folder });
  return { project, report };
}

/** Demande une ou plusieurs photos de façade ; liste vide si l'utilisateur renonce. */
export async function chooseFacadePhotos(): Promise<string[]> {
  requireEngine();
  const { open } = await import("@tauri-apps/plugin-dialog");
  const chosen = await open({
    multiple: true,
    directory: false,
    title: "Ajouter des photos de la façade",
    filters: [{ name: "Photo (HEIC, JPEG, PNG, TIFF)", extensions: PHOTO_EXTENSIONS }],
  });
  if (Array.isArray(chosen)) return chosen;
  return typeof chosen === "string" ? [chosen] : [];
}

// --- Mise à jour automatique (D16) ------------------------------------------

/** Une version plus récente, publiée dans les Releases du dépôt. */
export interface UpdateInfo {
  currentVersion: string;
  version: string;
  notes: string | null;
}

/** Avancement du téléchargement d'une mise à jour, en octets. */
export interface UpdateProgress {
  done: number;
  total: number | null;
}

/**
 * Cherche une version plus récente que celle de l'app. `null` : l'app est à
 * jour, ou la recherche est impossible (navigateur seul, pas de réseau,
 * fabrication sans clé de lecture). Ne lève jamais d'erreur : une recherche
 * ratée ne doit pas gêner le travail.
 */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  if (!isTauri()) return null;
  try {
    return await invoke<UpdateInfo | null>("update_check");
  } catch (error) {
    console.warn("Recherche de mise à jour impossible :", error);
    return null;
  }
}

/**
 * Télécharge, vérifie et installe la mise à jour trouvée par
 * `checkForUpdate`, puis relance l'app (la promesse ne se résout alors
 * jamais). `onProgress` reçoit l'avancement du téléchargement.
 */
export async function installUpdate(onProgress: (progress: UpdateProgress) => void): Promise<void> {
  requireEngine();
  const { listen } = await import("@tauri-apps/api/event");
  const stop = await listen<UpdateProgress>("mise-a-jour", (event) => onProgress(event.payload));
  try {
    await invoke("update_install");
  } finally {
    stop();
  }
}

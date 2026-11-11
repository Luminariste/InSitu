// Géométrie de la visionneuse : fonctions pures, sans écran ni WebGL, donc
// testables (voir geometry.test.ts).
//
// Trois repères :
// - le repère commun, en MILLIMÈTRES, origine en haut à gauche, y vers le bas.
//   Tous les calques y sont placés ;
// - l'écran, en pixels CSS, origine au coin haut-gauche de la visionneuse ;
// - les pixels d'une image, propres à chaque calque et à chaque niveau de sa
//   pyramide.

/** Descripteur d'une pyramide de tuiles, tel que l'écrit le moteur (`pyramid.json`). */
export interface Pyramid {
  version: number;
  /** Taille de l'image d'origine, en pixels. */
  width: number;
  height: number;
  tileSize: number;
  format: "png" | "jpeg";
  channels: number;
  /** Du niveau 0 (pleine résolution) au plus réduit ; le niveau n est réduit de 2ⁿ. */
  levels: PyramidLevel[];
}

export interface PyramidLevel {
  level: number;
  width: number;
  height: number;
  columns: number;
  rows: number;
}

export interface Point {
  x: number;
  y: number;
}

export interface Size {
  width: number;
  height: number;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Ce que montre la visionneuse. */
export interface ViewState {
  /** Point du repère commun (mm) affiché au centre de la visionneuse. */
  centerX: number;
  centerY: number;
  /** Pixels CSS par millimètre. */
  zoom: number;
}

/** Position d'une image dans le repère commun. */
export interface Placement {
  /** Coin haut-gauche de l'image, en mm. */
  x: number;
  y: number;
  /** Côté d'un pixel de l'image d'origine, en mm. */
  mmPerPixel: number;
}

/** Place une image de `pixelWidth` pixels de large sur `widthMm` millimètres. */
export function placementForWidth(pixelWidth: number, widthMm: number, x = 0, y = 0): Placement {
  return { x, y, mmPerPixel: widthMm / pixelWidth };
}

/** Rectangle occupé par une image dans le repère commun, en mm. */
export function placedBounds(pyramid: Pick<Pyramid, "width" | "height">, placement: Placement): Rect {
  return {
    x: placement.x,
    y: placement.y,
    width: pyramid.width * placement.mmPerPixel,
    height: pyramid.height * placement.mmPerPixel,
  };
}

export function unionRects(rects: Rect[]): Rect | null {
  if (rects.length === 0) return null;
  let left = Infinity;
  let top = Infinity;
  let right = -Infinity;
  let bottom = -Infinity;
  for (const r of rects) {
    left = Math.min(left, r.x);
    top = Math.min(top, r.y);
    right = Math.max(right, r.x + r.width);
    bottom = Math.max(bottom, r.y + r.height);
  }
  return { x: left, y: top, width: right - left, height: bottom - top };
}

// --- Conversions mm ↔ écran -------------------------------------------------

export function mmToScreen(view: ViewState, viewport: Size, mm: Point): Point {
  return {
    x: (mm.x - view.centerX) * view.zoom + viewport.width / 2,
    y: (mm.y - view.centerY) * view.zoom + viewport.height / 2,
  };
}

export function screenToMm(view: ViewState, viewport: Size, screen: Point): Point {
  return {
    x: (screen.x - viewport.width / 2) / view.zoom + view.centerX,
    y: (screen.y - viewport.height / 2) / view.zoom + view.centerY,
  };
}

/** Partie du repère commun visible à l'écran, en mm. */
export function visibleRect(view: ViewState, viewport: Size): Rect {
  const width = viewport.width / view.zoom;
  const height = viewport.height / view.zoom;
  return { x: view.centerX - width / 2, y: view.centerY - height / 2, width, height };
}

// --- Déplacement et zoom ----------------------------------------------------

/** Déplace le contenu de `dx`, `dy` pixels CSS (positif : vers la droite, vers le bas). */
export function panBy(view: ViewState, dx: number, dy: number): ViewState {
  return { ...view, centerX: view.centerX - dx / view.zoom, centerY: view.centerY - dy / view.zoom };
}

export interface ZoomLimits {
  min: number;
  max: number;
}

/**
 * Multiplie le zoom par `factor` en gardant fixe le point du contenu qui se
 * trouve sous `anchor` (position du curseur, en pixels CSS).
 */
export function zoomAt(
  view: ViewState,
  viewport: Size,
  anchor: Point,
  factor: number,
  limits?: ZoomLimits,
): ViewState {
  return setZoomAt(view, viewport, anchor, view.zoom * factor, limits);
}

/** Comme `zoomAt`, avec le zoom visé donné directement. */
export function setZoomAt(
  view: ViewState,
  viewport: Size,
  anchor: Point,
  zoom: number,
  limits?: ZoomLimits,
): ViewState {
  const clamped = limits ? Math.min(limits.max, Math.max(limits.min, zoom)) : zoom;
  const fixed = screenToMm(view, viewport, anchor);
  return {
    zoom: clamped,
    centerX: fixed.x - (anchor.x - viewport.width / 2) / clamped,
    centerY: fixed.y - (anchor.y - viewport.height / 2) / clamped,
  };
}

/**
 * Ramène la vue vers le contenu : le centre de l'écran ne sort jamais de
 * `bounds`. On ne peut donc pas perdre l'image en la poussant hors champ.
 */
export function keepInBounds(view: ViewState, bounds: Rect): ViewState {
  const centerX = Math.min(bounds.x + bounds.width, Math.max(bounds.x, view.centerX));
  const centerY = Math.min(bounds.y + bounds.height, Math.max(bounds.y, view.centerY));
  if (centerX === view.centerX && centerY === view.centerY) return view;
  return { ...view, centerX, centerY };
}

/** Vue qui montre `bounds` en entier, centré, avec une marge (part de l'écran). */
export function fitView(bounds: Rect, viewport: Size, margin = 0.04): ViewState {
  const usable = 1 - 2 * margin;
  const zoom = Math.min(
    (viewport.width * usable) / bounds.width,
    (viewport.height * usable) / bounds.height,
  );
  return {
    centerX: bounds.x + bounds.width / 2,
    centerY: bounds.y + bounds.height / 2,
    zoom,
  };
}

/**
 * Zoom « 100 % » d'une image : un pixel de l'image d'origine sur un pixel
 * physique de l'écran (sur un écran Retina, un demi-pixel CSS).
 */
export function actualSizeZoom(mmPerPixel: number, devicePixelRatio: number): number {
  return 1 / (mmPerPixel * devicePixelRatio);
}

/** Zoom exprimé en pourcentage de la taille réelle d'une image. */
export function zoomPercent(zoom: number, mmPerPixel: number, devicePixelRatio: number): number {
  return zoom * mmPerPixel * devicePixelRatio * 100;
}

// --- Molette et trackpad ----------------------------------------------------

/** Ce qu'on lit d'un événement `wheel` pour décider de son sens. */
export interface WheelInput {
  deltaX: number;
  deltaY: number;
  /** 0 : pixels, 1 : lignes, 2 : pages. */
  deltaMode: number;
  ctrlKey: boolean;
  metaKey: boolean;
  /** Ancienne valeur propre à WebKit et Chromium : multiples de 120 pour une molette à crans. */
  wheelDeltaY?: number;
}

export type WheelIntent = "zoom-pinch" | "zoom-wheel" | "pan";

/**
 * Molette et trackpad produisent le même événement. On les distingue ainsi :
 *
 * - pincement du trackpad : Chromium l'annonce comme une molette avec
 *   `ctrlKey` (WebKit, lui, envoie des événements `gesture` traités à part) ;
 * - Cmd ou Ctrl + molette : zoom, toujours ;
 * - molette à crans d'une souris : pas de composante horizontale, et un
 *   déplacement en lignes ou en multiples de 120 : zoom ;
 * - tout le reste (défilement à deux doigts) : déplacement.
 */
export function classifyWheel(input: WheelInput): WheelIntent {
  if (input.ctrlKey) return "zoom-pinch";
  if (input.metaKey) return "zoom-wheel";
  if (input.deltaX !== 0 || input.deltaY === 0) return "pan";
  if (input.deltaMode !== 0) return "zoom-wheel";
  const legacy = input.wheelDeltaY;
  if (legacy !== undefined && legacy !== 0 && Math.abs(legacy) >= 120 && legacy % 120 === 0) {
    return "zoom-wheel";
  }
  return "pan";
}

/** Facteur de zoom d'un événement de molette (deltaY négatif : on agrandit). */
export function wheelZoomFactor(input: WheelInput, intent: WheelIntent): number {
  // Un cran de molette vaut environ 100 px ou 3 lignes ; un pincement envoie
  // de petites valeurs, d'où une sensibilité plus forte.
  const pixels = input.deltaMode === 1 ? input.deltaY * 33 : input.deltaMode === 2 ? input.deltaY * 300 : input.deltaY;
  const bounded = Math.max(-300, Math.min(300, pixels));
  return Math.exp(-bounded * (intent === "zoom-pinch" ? 0.01 : 0.002));
}

// --- Tuiles -----------------------------------------------------------------

/**
 * Niveau de pyramide à afficher : le plus réduit dont la résolution atteint
 * encore celle de l'écran. Les tuiles ne sont donc jamais agrandies (sauf au
 * niveau 0, au-delà de 100 %), et jamais réduites de plus de la moitié.
 */
export function pickLevel(
  levelCount: number,
  mmPerPixel: number,
  zoom: number,
  devicePixelRatio: number,
): number {
  // Pixels physiques de l'écran par pixel de l'image d'origine.
  const scale = mmPerPixel * zoom * devicePixelRatio;
  if (!(scale > 0) || scale >= 1) return 0;
  const level = Math.floor(Math.log2(1 / scale) + 1e-9);
  return Math.min(levelCount - 1, Math.max(0, level));
}

export interface TileRange {
  level: number;
  columnMin: number;
  columnMax: number;
  rowMin: number;
  rowMax: number;
}

/**
 * Tuiles du niveau `level` qui touchent la partie visible de l'écran, ou
 * `null` si l'image est entièrement hors champ.
 */
export function visibleTileRange(
  pyramid: Pyramid,
  level: number,
  placement: Placement,
  view: ViewState,
  viewport: Size,
): TileRange | null {
  const info = pyramid.levels[level];
  const visible = visibleRect(view, viewport);
  // Côté d'un pixel de ce niveau, en mm : exactement 2ⁿ pixels d'origine.
  const pixel = placement.mmPerPixel * 2 ** level;
  const left = (visible.x - placement.x) / pixel;
  const top = (visible.y - placement.y) / pixel;
  const right = left + visible.width / pixel;
  const bottom = top + visible.height / pixel;
  // Étendue réelle de l'image à ce niveau : le dernier pixel d'un niveau
  // arrondi au pixel supérieur dépasse un peu de l'image.
  const extentX = pyramid.width / 2 ** level;
  const extentY = pyramid.height / 2 ** level;
  if (right <= 0 || bottom <= 0 || left >= extentX || top >= extentY) return null;
  const size = pyramid.tileSize;
  return {
    level,
    columnMin: Math.max(0, Math.floor(left / size)),
    columnMax: Math.min(info.columns - 1, Math.floor((right - 1e-9) / size)),
    rowMin: Math.max(0, Math.floor(top / size)),
    rowMax: Math.min(info.rows - 1, Math.floor((bottom - 1e-9) / size)),
  };
}

export function tileCount(range: TileRange): number {
  return (range.columnMax - range.columnMin + 1) * (range.rowMax - range.rowMin + 1);
}

/** Rectangle d'une tuile dans l'image de son niveau, en pixels de ce niveau. */
export function tileRect(pyramid: Pyramid, level: number, column: number, row: number): Rect {
  const info = pyramid.levels[level];
  const x = column * pyramid.tileSize;
  const y = row * pyramid.tileSize;
  return {
    x,
    y,
    width: Math.min(pyramid.tileSize, info.width - x),
    height: Math.min(pyramid.tileSize, info.height - y),
  };
}

export interface TileAncestor {
  level: number;
  column: number;
  row: number;
  /** Partie de la tuile ancêtre qui recouvre la tuile demandée, en pixels de l'ancêtre. */
  frame: Rect;
}

/**
 * Tuile du niveau `level + up` qui contient la tuile donnée, et la partie de
 * cette tuile qui lui correspond. Sert à afficher une version moins fine en
 * attendant qu'une tuile soit chargée.
 */
export function tileAncestor(
  pyramid: Pyramid,
  level: number,
  column: number,
  row: number,
  up: number,
): TileAncestor {
  const factor = 2 ** up;
  const rect = tileRect(pyramid, level, column, row);
  const ancestorColumn = Math.floor(column / factor);
  const ancestorRow = Math.floor(row / factor);
  const ancestorRect = tileRect(pyramid, level + up, ancestorColumn, ancestorRow);
  const x = rect.x / factor - ancestorRect.x;
  const y = rect.y / factor - ancestorRect.y;
  return {
    level: level + up,
    column: ancestorColumn,
    row: ancestorRow,
    frame: {
      x,
      y,
      // Les niveaux sont arrondis au pixel supérieur : au bord, la tuile
      // ancêtre peut s'arrêter une fraction de pixel plus tôt.
      width: Math.min(rect.width / factor, ancestorRect.width - x),
      height: Math.min(rect.height / factor, ancestorRect.height - y),
    },
  };
}

/** Nom du fichier d'une tuile dans le dossier de sa pyramide. */
export function tilePath(pyramid: Pyramid, level: number, column: number, row: number): string {
  return `${level}/${column}_${row}.${pyramid.format === "png" ? "png" : "jpg"}`;
}

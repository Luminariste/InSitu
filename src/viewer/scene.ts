// La scène de la visionneuse : le rendu WebGL (PixiJS), la vue, les gestes.
// Aucun lien avec React : le composant `Viewer` ne fait que la créer et lui
// transmettre ses calques.

import { autoDetectRenderer, Container, ImageSource, Texture, type Renderer } from "pixi.js";
import {
  actualSizeZoom,
  classifyWheel,
  fitView,
  keepInBounds,
  panBy,
  screenToMm,
  setZoomAt,
  unionRects,
  wheelZoomFactor,
  zoomAt,
  zoomPercent,
  type Point,
  type Rect,
  type Size,
  type ViewState,
  type ZoomLimits,
} from "./geometry";
import type { FrameContext, LayerSpec, SceneNode } from "./nodes";
import { TiledLayerNode } from "./tiledLayer";
import { TileStore, type TileRequest, type TileStoreStatus } from "./tileStore";

export interface SceneOptions {
  /** Couleur du fond, hors de toute image. */
  background?: number;
  /** Tuiles gardées en mémoire au-delà de celles qui sont à l'écran. */
  tileCapacity?: number;
  /** Chargements de tuiles simultanés. */
  tileConcurrency?: number;
}

/** Mesure de fluidité d'un geste (suite d'images dessinées sans pause). */
export interface FrameStats {
  /** Nombre d'images dessinées pendant le geste. */
  frames: number;
  /** Durée moyenne entre deux images, en ms (16,7 ms = 60 images par seconde). */
  meanMs: number;
  /** Pire durée entre deux images, en ms. */
  worstMs: number;
  /** Nombre d'images arrivées avec plus de 25 ms de retard (saccades visibles). */
  slowFrames: number;
  /** Temps de calcul moyen d'une image par l'interface, en ms. */
  meanWorkMs: number;
}

export interface SceneStatus {
  view: ViewState;
  /** Zoom en pourcentage de la taille réelle du calque le plus fin ; `null` sans calque. */
  zoomPercent: number | null;
  tiles: TileStoreStatus;
}

export interface SceneListeners {
  /** À chaque image dessinée. Ne rien y faire de lourd. */
  onFrame?: (status: SceneStatus) => void;
  /** Position du curseur dans le repère commun, en mm ; `null` quand il sort. */
  onCursor?: (mm: Point | null) => void;
  /** Quand un geste se termine (plus aucune image depuis un quart de seconde). */
  onGestureEnd?: (stats: FrameStats) => void;
}

/** Une image dessinée après une pause plus longue ne compte pas dans le geste. */
// Après le dernier mouvement, la scène continue d'être redessinée pendant ce
// délai : la mesure porte ainsi sur le rythme réel de l'écran, pas sur le
// rythme auquel les mouvements arrivent.
const GESTURE_TAIL_MS = 180;
const SLOW_FRAME_MS = 25;

/** Événement de pincement propre à WebKit (Safari, fenêtre Tauri sur Mac). */
interface GestureEvent extends UIEvent {
  scale: number;
  clientX: number;
  clientY: number;
}

export class ViewerScene {
  /** Repère commun, gradué en millimètres. Tout objet affiché y est posé. */
  readonly world = new Container();
  private readonly stage = new Container();
  private readonly renderer: Renderer;
  private readonly host: HTMLElement;
  private readonly tiles: TileStore<Texture>;
  private readonly nodes = new Map<string, { kind: string; node: SceneNode }>();
  private readonly extraNodes = new Set<SceneNode>();
  private readonly resizeObserver: ResizeObserver;
  private readonly disposers: Array<() => void> = [];
  private listeners: SceneListeners = {};

  private view: ViewState = { centerX: 0, centerY: 0, zoom: 1 };
  private viewport: Size = { width: 0, height: 0 };
  /** Vrai tant que l'utilisateur n'a pas bougé : la vue se cale sur « tout voir ». */
  private autoFit = true;
  private frameRequest = 0;
  private destroyed = false;

  private lastFrameAt = 0;
  private gesture = { frames: 0, total: 0, worst: 0, slow: 0, work: 0 };
  private activeUntil = 0;
  private inGesture = false;

  /** Crée la scène dans `host`, qui doit avoir une taille. */
  static async create(host: HTMLElement, options: SceneOptions = {}): Promise<ViewerScene> {
    const renderer = await autoDetectRenderer({
      // WebGL partout : c'est le moteur de rendu disponible dans la fenêtre
      // Tauri (WebKit) quelle que soit la version de macOS.
      preference: "webgl",
      width: Math.max(1, host.clientWidth),
      height: Math.max(1, host.clientHeight),
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
      // Pas de lissage des bords : deux tuiles voisines doivent se toucher
      // sans laisser de liseré.
      antialias: false,
      backgroundColor: options.background ?? 0x1d1d20,
      powerPreference: "high-performance",
    });
    return new ViewerScene(host, renderer, options);
  }

  private constructor(host: HTMLElement, renderer: Renderer, options: SceneOptions) {
    this.host = host;
    this.renderer = renderer;
    this.stage.addChild(this.world);
    this.tiles = new TileStore<Texture>({
      capacity: options.tileCapacity ?? 256,
      concurrency: options.tileConcurrency ?? 6,
      load: loadTileTexture,
      dispose: disposeTileTexture,
      onChange: () => this.invalidate(),
    });

    const canvas = renderer.canvas as HTMLCanvasElement;
    canvas.style.display = "block";
    canvas.style.touchAction = "none";
    host.appendChild(canvas);
    this.viewport = { width: host.clientWidth, height: host.clientHeight };

    this.resizeObserver = new ResizeObserver(() => this.resize());
    this.resizeObserver.observe(host);
    this.bindGestures(canvas);
    this.invalidate();
  }

  // --- Contenu ---------------------------------------------------------------

  /** Remplace la liste des calques, du fond vers le dessus. */
  setLayers(specs: LayerSpec[]): void {
    const kept = new Set<string>();
    specs.forEach((spec, index) => {
      kept.add(spec.id);
      const existing = this.nodes.get(spec.id);
      let node: SceneNode;
      if (existing && existing.kind === spec.kind && updateNode(existing.node, spec)) {
        node = existing.node;
      } else {
        if (existing) this.dropNode(existing.node);
        node = createNode(spec);
        this.nodes.set(spec.id, { kind: spec.kind, node });
        this.world.addChild(node.container);
      }
      node.container.zIndex = index;
    });
    for (const [id, { node }] of this.nodes) {
      if (kept.has(id)) continue;
      this.dropNode(node);
      this.nodes.delete(id);
    }
    this.world.sortableChildren = true;
    this.invalidate();
  }

  /**
   * Ajoute un objet libre au-dessus des calques (voir `SceneNode`). Renvoie
   * la fonction qui le retire.
   */
  addNode(node: SceneNode): () => void {
    this.extraNodes.add(node);
    node.container.zIndex = 1_000_000;
    this.world.addChild(node.container);
    this.invalidate();
    return () => {
      if (!this.extraNodes.delete(node)) return;
      this.dropNode(node);
      this.invalidate();
    };
  }

  setListeners(listeners: SceneListeners): void {
    this.listeners = listeners;
  }

  // --- Vue -------------------------------------------------------------------

  getView(): ViewState {
    return this.view;
  }

  getViewport(): Size {
    return this.viewport;
  }

  setView(view: ViewState): void {
    this.autoFit = false;
    const bounds = this.contentBounds();
    this.view = bounds ? keepInBounds(view, bounds) : view;
    this.activeUntil = performance.now() + GESTURE_TAIL_MS;
    this.invalidate();
  }

  /** Montre tous les calques visibles en entier. */
  fitAll(): void {
    this.autoFit = true;
    this.applyAutoFit();
    this.invalidate();
  }

  /**
   * Zoom 100 % : un pixel du calque le plus fin sur un pixel physique de
   * l'écran, en gardant fixe le point donné (le centre par défaut).
   */
  zoomToActualSize(anchor?: Point): void {
    const detail = this.finestDetail();
    if (detail === null) return;
    const zoom = actualSizeZoom(detail, this.pixelRatio());
    const center = { x: this.viewport.width / 2, y: this.viewport.height / 2 };
    this.setView(setZoomAt(this.view, this.viewport, anchor ?? center, zoom));
  }

  /** Demande une nouvelle image. Plusieurs demandes n'en produisent qu'une. */
  invalidate(): void {
    if (this.destroyed || this.frameRequest !== 0) return;
    this.frameRequest = requestAnimationFrame((time) => {
      this.frameRequest = 0;
      this.renderFrame(time);
    });
  }

  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    cancelAnimationFrame(this.frameRequest);
    this.resizeObserver.disconnect();
    for (const dispose of this.disposers) dispose();
    for (const { node } of this.nodes.values()) node.destroy();
    for (const node of this.extraNodes) node.destroy();
    this.nodes.clear();
    this.extraNodes.clear();
    this.tiles.destroy();
    this.stage.destroy({ children: true });
    this.renderer.destroy(true);
  }

  // --- Rendu -----------------------------------------------------------------

  private renderFrame(time: number): void {
    if (this.destroyed) return;
    const started = performance.now();
    if (this.autoFit) this.applyAutoFit();

    const ratio = this.pixelRatio();
    const { view, viewport } = this;
    this.world.scale.set(view.zoom);
    // Calé sur un pixel physique : à 100 %, chaque pixel de l'image tombe
    // exactement sur un pixel de l'écran, sans flou d'interpolation.
    this.world.position.set(
      Math.round((viewport.width / 2 - view.centerX * view.zoom) * ratio) / ratio,
      Math.round((viewport.height / 2 - view.centerY * view.zoom) * ratio) / ratio,
    );

    const wanted = new Map<string, TileRequest>();
    const frame: FrameContext = {
      view,
      viewport,
      devicePixelRatio: ratio,
      tiles: this.tiles,
      wantTile: (url, priority) => {
        const known = wanted.get(url);
        if (!known || priority < known.priority) wanted.set(url, { url, priority });
      },
    };
    for (const { node } of this.nodes.values()) node.update(frame);
    for (const node of this.extraNodes) node.update(frame);
    this.tiles.sync(wanted);

    this.renderer.render(this.stage);
    // Pendant un geste, une image est dessinée à chaque rafraîchissement de
    // l'écran, même si aucun mouvement n'est arrivé entre-temps.
    const active = performance.now() < this.activeUntil;
    this.recordFrame(time, performance.now() - started, active);
    if (active) this.invalidate();

    const detail = this.finestDetail();
    this.listeners.onFrame?.({
      view,
      zoomPercent: detail === null ? null : zoomPercent(view.zoom, detail, ratio),
      tiles: this.tiles.status(),
    });
  }

  /**
   * Mesure la fluidité d'un geste : l'écart entre deux images successives
   * tant que le geste dure. Sur un écran à 60 Hz l'écart idéal est de
   * 16,7 ms, sur un écran à 120 Hz de 8,3 ms.
   */
  private recordFrame(time: number, workMs: number, active: boolean): void {
    const g = this.gesture;
    if (active) {
      if (this.inGesture) {
        const gap = time - this.lastFrameAt;
        g.frames += 1;
        g.total += gap;
        g.worst = Math.max(g.worst, gap);
        g.work += workMs;
        if (gap > SLOW_FRAME_MS) g.slow += 1;
      }
      this.inGesture = true;
      this.lastFrameAt = time;
      return;
    }
    if (!this.inGesture) return;
    this.inGesture = false;
    this.gesture = { frames: 0, total: 0, worst: 0, slow: 0, work: 0 };
    if (g.frames < 5) return;
    this.listeners.onGestureEnd?.({
      frames: g.frames,
      meanMs: g.total / g.frames,
      worstMs: g.worst,
      slowFrames: g.slow,
      meanWorkMs: g.work / g.frames,
    });
  }

  private resize(): void {
    if (this.destroyed) return;
    const width = this.host.clientWidth;
    const height = this.host.clientHeight;
    if (width === 0 || height === 0) return;
    this.viewport = { width, height };
    // Le rapport change quand la fenêtre passe d'un écran Retina à un autre.
    this.renderer.resize(width, height, this.pixelRatio());
    this.invalidate();
  }

  private pixelRatio(): number {
    return window.devicePixelRatio || 1;
  }

  private contentBounds(): Rect | null {
    const rects: Rect[] = [];
    for (const { node } of this.nodes.values()) {
      const bounds = node.bounds();
      if (bounds) rects.push(bounds);
    }
    for (const node of this.extraNodes) {
      const bounds = node.bounds();
      if (bounds) rects.push(bounds);
    }
    return unionRects(rects);
  }

  private finestDetail(): number | null {
    let finest: number | null = null;
    const consider = (node: SceneNode) => {
      const detail = node.finestDetail();
      if (detail !== null && (finest === null || detail < finest)) finest = detail;
    };
    for (const { node } of this.nodes.values()) consider(node);
    for (const node of this.extraNodes) consider(node);
    return finest;
  }

  private applyAutoFit(): void {
    const bounds = this.contentBounds();
    if (!bounds || this.viewport.width === 0 || this.viewport.height === 0) return;
    this.view = fitView(bounds, this.viewport);
  }

  /** Du quart de « tout voir » jusqu'à 16 pixels d'écran par pixel d'image. */
  private zoomLimits(): ZoomLimits | undefined {
    const bounds = this.contentBounds();
    const detail = this.finestDetail();
    if (!bounds || detail === null) return undefined;
    const fit = fitView(bounds, this.viewport).zoom;
    const max = actualSizeZoom(detail, this.pixelRatio()) * 16;
    return { min: Math.min(fit / 4, max), max };
  }

  private dropNode(node: SceneNode): void {
    this.world.removeChild(node.container);
    node.destroy();
  }

  // --- Gestes ----------------------------------------------------------------

  private bindGestures(canvas: HTMLCanvasElement): void {
    const listen = <K extends keyof HTMLElementEventMap>(
      type: K,
      handler: (event: HTMLElementEventMap[K]) => void,
      options?: AddEventListenerOptions,
    ) => {
      canvas.addEventListener(type, handler as EventListener, options);
      this.disposers.push(() => canvas.removeEventListener(type, handler as EventListener, options));
    };
    const local = (event: { clientX: number; clientY: number }): Point => {
      const box = canvas.getBoundingClientRect();
      return { x: event.clientX - box.left, y: event.clientY - box.top };
    };

    // Molette, défilement à deux doigts, pincement (Chromium).
    listen(
      "wheel",
      (event) => {
        event.preventDefault();
        const input = {
          deltaX: event.deltaX,
          deltaY: event.deltaY,
          deltaMode: event.deltaMode,
          ctrlKey: event.ctrlKey,
          metaKey: event.metaKey,
          wheelDeltaY: (event as WheelEvent & { wheelDeltaY?: number }).wheelDeltaY,
        };
        const intent = classifyWheel(input);
        if (intent === "pan") {
          this.setView(panBy(this.view, -event.deltaX, -event.deltaY));
        } else {
          const factor = wheelZoomFactor(input, intent);
          this.setView(zoomAt(this.view, this.viewport, local(event), factor, this.zoomLimits()));
        }
      },
      { passive: false },
    );

    // Pincement (WebKit) : `scale` est le rapport depuis le début du geste.
    let pinchScale = 1;
    const onGestureStart = (event: Event) => {
      event.preventDefault();
      pinchScale = 1;
    };
    const onGestureChange = (event: Event) => {
      event.preventDefault();
      const gesture = event as GestureEvent;
      if (!(gesture.scale > 0)) return;
      const factor = gesture.scale / pinchScale;
      pinchScale = gesture.scale;
      this.setView(zoomAt(this.view, this.viewport, local(gesture), factor, this.zoomLimits()));
    };
    const onGestureEnd = (event: Event) => event.preventDefault();
    const gestures: Array<[string, (event: Event) => void]> = [
      ["gesturestart", onGestureStart],
      ["gesturechange", onGestureChange],
      ["gestureend", onGestureEnd],
    ];
    for (const [type, handler] of gestures) {
      canvas.addEventListener(type, handler, { passive: false });
      this.disposers.push(() => canvas.removeEventListener(type, handler));
    }

    // Glisser pour déplacer.
    let drag: { pointerId: number; x: number; y: number } | null = null;
    listen("pointerdown", (event) => {
      if (event.button !== 0 && event.button !== 1) return;
      drag = { pointerId: event.pointerId, x: event.clientX, y: event.clientY };
      canvas.setPointerCapture(event.pointerId);
      canvas.style.cursor = "grabbing";
    });
    listen("pointermove", (event) => {
      if (drag && drag.pointerId === event.pointerId) {
        this.setView(panBy(this.view, event.clientX - drag.x, event.clientY - drag.y));
        drag = { pointerId: event.pointerId, x: event.clientX, y: event.clientY };
      }
      this.listeners.onCursor?.(screenToMm(this.view, this.viewport, local(event)));
    });
    const endDrag = (event: PointerEvent) => {
      if (!drag || drag.pointerId !== event.pointerId) return;
      drag = null;
      canvas.style.cursor = "";
    };
    listen("pointerup", endDrag);
    listen("pointercancel", endDrag);
    listen("pointerleave", () => this.listeners.onCursor?.(null));

    // Double-clic : 100 % sur le point visé.
    listen("dblclick", (event) => this.zoomToActualSize(local(event)));
  }
}

/** Fabrique l'objet de scène d'un calque. Un nouveau type de calque s'ajoute ici. */
function createNode(spec: LayerSpec): SceneNode {
  switch (spec.kind) {
    case "tiles":
      return new TiledLayerNode(spec);
  }
}

/** Met à jour un objet existant ; renvoie `false` s'il faut le recréer. */
function updateNode(node: SceneNode, spec: LayerSpec): boolean {
  switch (spec.kind) {
    case "tiles":
      if (!(node instanceof TiledLayerNode) || !node.accepts(spec)) return false;
      node.apply(spec);
      return true;
  }
}

/** Charge une tuile et la prépare pour la carte graphique. */
async function loadTileTexture(url: string, signal: AbortSignal): Promise<Texture> {
  const response = await fetch(url, { signal });
  if (!response.ok) throw new Error(`${url} : ${response.status}`);
  const blob = await response.blob();
  // Décodage hors du fil d'affichage : il ne provoque pas de saccade.
  const bitmap = await createImageBitmap(blob);
  if (signal.aborted) {
    bitmap.close();
    throw new DOMException("Chargement annulé", "AbortError");
  }
  const source = new ImageSource({
    resource: bitmap,
    scaleMode: "linear",
    // La réserve de tuiles décide seule de ce qui reste en mémoire.
    autoGarbageCollect: false,
  });
  return new Texture({ source });
}

function disposeTileTexture(texture: Texture): void {
  const resource = texture.source.resource as ImageBitmap | undefined;
  texture.destroy(true);
  resource?.close?.();
}

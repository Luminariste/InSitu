// Composition sur la façade (tâche 4.2) : les dessins posés, leur sélection,
// les gestes à la souris et au clavier. L'écran Façade branche ce crochet sur
// sa visionneuse ; les calculs sont dans `placement.ts` et `gesture.ts`.
//
// Chaque geste terminé envoie calques et dessins posés au moteur, qui les
// contrôle et les enregistre (et garde l'état d'avant pour ⌘Z, tâche 4.3). Dans un simple navigateur, les dessins posés restent en
// mémoire (bibliothèque inventée), pour juger des gestes sans l'app.

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type DragEvent,
  type PointerEvent,
  type RefObject,
} from "react";
import {
  cacheFileUrl,
  fetchDrawingPreview,
  fetchLibrary,
  saveComposition,
  type LibraryEntry,
  type Project,
} from "../engine";
import { demoLibrary } from "../library/demo";
import type { LayerSpec, ViewerHandle } from "../viewer";
import { mmToScreen, screenToMm, visibleRect } from "../viewer/geometry";
import {
  applyGesture,
  frameCorners,
  handleAt,
  repeatGesture,
  type Gesture,
  type GestureStep,
  type Repeatable,
} from "./gesture";
import {
  NUDGE_MM,
  NUDGE_SHIFT_FACTOR,
  PASTE_OFFSET_MM,
  align,
  arrange,
  bounds,
  copyOf,
  corners,
  distribute,
  dropped,
  expandToGroups,
  flip,
  group,
  hitTest,
  newId,
  rectCenter,
  remove,
  rotate,
  scale,
  translate,
  ungroup,
  type Alignment,
  type Arrange,
  type FlipAxis,
  type Placement,
  type Point,
  type Rect,
} from "./placement";
import {
  addLayer,
  effectiveLayers,
  isLocked,
  isShown,
  layerOf,
  moveLayer,
  moveToLayer,
  pickable,
  removeLayer,
  shown,
  stack,
  updateLayer,
} from "./layers";
import type { Layer } from "./placement";
import { SelectionNode } from "./SelectionNode";

/** Type des données glissées depuis la liste des dessins. */
export const DRAWING_DRAG_TYPE = "application/x-insitu-drawing";

export type ComposeDrawing = LibraryEntry & { url?: string | null };

interface Options {
  engine: boolean;
  folder: string | null;
  project: Project | null;
  onProject: (project: Project) => void;
  viewer: RefObject<ViewerHandle | null>;
  stage: RefObject<HTMLDivElement | null>;
  /** Faux pendant un autre geste de l'écran (pose des points d'échelle) ou sans façade à l'échelle. */
  enabled: boolean;
  onError: (message: string | null) => void;
}

/** Ce que l'inspecteur peut faire de la sélection. */
export interface CompositionActions {
  remove(): void;
  flip(axis: FlipAxis): void;
  rotateBy(deg: number): void;
  align(how: Alignment): void;
  distribute(axis: "horizontal" | "vertical"): void;
  group(): void;
  ungroup(): void;
  arrange(how: Arrange): void;
  /** Déplace le cadre de la sélection pour que son centre soit en `center`. */
  moveTo(center: Point): void;
  /** Change la largeur du cadre de la sélection (proportions gardées). */
  resizeTo(width: number): void;
  /** Rotation absolue d'un dessin seul. */
  setRotation(deg: number): void;
  /** Pose un dessin au centre de la vue. */
  place(drawing: string): void;
  /** Range la sélection sur un calque. */
  moveToLayer(layer: string): void;
}

/** Ce que la rubrique « Calques » peut faire (tâche 4.3). */
export interface LayerActions {
  /** Le calque où se posent les nouveaux dessins. */
  activate(id: string): void;
  add(): void;
  remove(id: string): void;
  rename(id: string, name: string): void;
  toggleVisible(id: string): void;
  toggleLocked(id: string): void;
  /** +1 : vers l'avant. */
  move(id: string, step: -1 | 1): void;
}

export interface Composition {
  drawings: ComposeDrawing[];
  placements: Placement[];
  selected: Placement[];
  frame: Rect | null;
  /** Calques de la composition, du fond vers l'avant (un seul, implicite, s'il n'y en a pas). */
  composeLayers: Layer[];
  activeLayer: string;
  /** Nombre de dessins posés par calque. */
  layerCounts: Map<string, number>;
  layerActions: LayerActions;
  /** Ce que la visionneuse doit afficher. */
  layers: LayerSpec[];
  /** Consigne du geste possible, pour la barre d'état. */
  hint: string | null;
  actions: CompositionActions;
  /** À brancher sur la scène de la visionneuse ; vrai si le geste est pris. */
  onPointerDown(event: PointerEvent<HTMLDivElement>): boolean;
  onPointerMove(event: PointerEvent<HTMLDivElement>): void;
  onPointerUp(event: PointerEvent<HTMLDivElement>): boolean;
  onDragOver(event: DragEvent<HTMLDivElement>): void;
  onDrop(event: DragEvent<HTMLDivElement>): void;
  shortcuts: { keys: string; run: () => void; enabled?: boolean }[];
}

interface Active {
  gesture: Gesture;
  down: { x: number; y: number; t: number };
  step: GestureStep | null;
  /** Identifiants des copies (⌥), stables pendant tout le geste. */
  ids: string[];
}

const CLICK_PX = 4;

export function useComposition(options: Options): Composition {
  const { engine, folder, project, onProject, viewer, stage, enabled, onError } = options;
  const [drawings, setDrawings] = useState<ComposeDrawing[]>([]);
  const [placements, setPlacements] = useState<Placement[]>([]);
  const [layerList, setLayerList] = useState<Layer[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [selection, setSelection] = useState<Set<string>>(new Set());
  const [marquee, setMarquee] = useState<Rect | null>(null);
  const [previews, setPreviews] = useState<Map<string, string>>(new Map());
  const active = useRef<Active | null>(null);
  const lastRepeat = useRef<Repeatable | null>(null);
  const clipboard = useRef<Placement[]>([]);
  const spaceDown = useRef(false);
  const saving = useRef<Promise<unknown>>(Promise.resolve());
  const selectionNode = useRef<SelectionNode | null>(null);

  // --- Bibliothèque et dessins posés ----------------------------------------------------

  useEffect(() => {
    if (!engine) {
      setDrawings(demoLibrary(12));
      return;
    }
    setDrawings([]);
    if (!folder) return;
    let cancelled = false;
    fetchLibrary(folder).then(
      (entries) =>
        !cancelled && setDrawings(entries.map((e) => ({ ...e, url: e.thumbnail ? cacheFileUrl(e.thumbnail) : null }))),
      (failure: unknown) => !cancelled && onError(String(failure)),
    );
    return () => {
      cancelled = true;
    };
  }, [engine, folder, onError]);

  // Le projet fait foi, sauf pendant un geste.
  const projectPlacements = project?.placements;
  const projectLayers = project?.layers;
  useEffect(() => {
    if (!engine || active.current) return;
    setPlacements(projectPlacements ?? []);
    setLayerList(projectLayers ?? []);
  }, [engine, projectPlacements, projectLayers]);

  useEffect(() => {
    setSelection(new Set());
    setPreviews(new Map());
    lastRepeat.current = null;
  }, [folder]);

  // Aperçus (1 024 px, fond transparent) des dessins posés.
  useEffect(() => {
    const wanted = [...new Set(placements.map((p) => p.drawing))].filter((id) => !previews.has(id));
    if (wanted.length === 0) return;
    if (!engine) {
      const next = new Map(previews);
      for (const id of wanted) {
        const url = drawings.find((d) => d.id === id)?.url;
        if (url) next.set(id, url);
      }
      if (next.size !== previews.size) setPreviews(next);
      return;
    }
    if (!folder) return;
    let cancelled = false;
    Promise.all(wanted.map(async (id) => [id, await fetchDrawingPreview(folder, id)] as const)).then(
      (found) => {
        if (cancelled) return;
        setPreviews((current) => {
          const next = new Map(current);
          for (const [id, url] of found) next.set(id, url);
          return next;
        });
      },
      (failure: unknown) => !cancelled && onError(String(failure)),
    );
    return () => {
      cancelled = true;
    };
  }, [placements, previews, drawings, engine, folder, onError]);

  /** Enregistre calques et dessins (rangés par calque) ; en cas de refus, revient au projet. */
  const commit = useCallback(
    (next: Placement[], nextSelection?: Set<string>, nextLayers: Layer[] = layerList) => {
      const stacked = stack(next, nextLayers);
      setPlacements(stacked);
      setLayerList(nextLayers);
      if (nextSelection) setSelection(nextSelection);
      if (!engine || !folder) return;
      saving.current = saving.current.then(() =>
        saveComposition(folder, nextLayers, stacked).then(onProject, (failure: unknown) => {
          onError(String(failure).replace(/^Error: /, ""));
          setPlacements(project?.placements ?? []);
          setLayerList(project?.layers ?? []);
        }),
      );
    },
    [engine, folder, onProject, onError, project?.placements, project?.layers, layerList],
  );

  // --- Calques ------------------------------------------------------------------------------

  const composeLayers = useMemo(() => effectiveLayers(layerList), [layerList]);
  const activeLayer = composeLayers.find((l) => l.id === activeId) ?? composeLayers[composeLayers.length - 1];
  const picks = useMemo(() => pickable(placements, layerList), [placements, layerList]);
  const layerCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const p of placements) {
      const id = layerOf(layerList, p).id;
      counts.set(id, (counts.get(id) ?? 0) + 1);
    }
    return counts;
  }, [placements, layerList]);

  // Un dessin masqué ou verrouillé sort de la sélection.
  useEffect(() => {
    setSelection((current) => {
      const allowed = new Set(picks.map((p) => p.id));
      const kept = [...current].filter((id) => allowed.has(id));
      return kept.length === current.size ? current : new Set(kept);
    });
  }, [picks]);

  const commitLayers = (c: { layers: Layer[]; placements: Placement[] }) => {
    if (c.layers === layerList && c.placements === placements) return;
    commit(c.placements, undefined, c.layers);
  };
  const current = { layers: layerList, placements };
  const layerActions: LayerActions = {
    activate: (id) => setActiveId(id),
    add: () => {
      const r = addLayer(current, activeLayer.id);
      commitLayers(r);
      setActiveId(r.added.id);
    },
    remove: (id) => commitLayers(removeLayer(current, id)),
    rename: (id, name) => {
      const trimmed = name.trim();
      const layer = composeLayers.find((l) => l.id === id);
      if (trimmed && layer && trimmed !== layer.name) commitLayers(updateLayer(current, id, { name: trimmed }));
    },
    toggleVisible: (id) => {
      const layer = composeLayers.find((l) => l.id === id);
      if (layer) commitLayers(updateLayer(current, id, { visible: !isShown(layer) }));
    },
    toggleLocked: (id) => {
      const layer = composeLayers.find((l) => l.id === id);
      if (layer) commitLayers(updateLayer(current, id, { locked: !isLocked(layer) }));
    },
    move: (id, step) => commitLayers(moveLayer(current, id, step)),
  };

  // --- Affichage --------------------------------------------------------------------------

  const selected = useMemo(() => placements.filter((p) => selection.has(p.id)), [placements, selection]);
  const frame = useMemo(() => bounds(selected), [selected]);

  const layers = useMemo<LayerSpec[]>(
    () =>
      shown(placements, layerList).flatMap((p) => {
        const url = previews.get(p.drawing);
        if (!url) return [];
        const [tl, tr, br, bl] = corners(p);
        return [
          { id: `pose:${p.id}`, kind: "mesh-image", url, divisions: 1, points: [tl, tr, bl, br] } satisfies LayerSpec,
        ];
      }),
    [placements, layerList, previews],
  );

  // Le cadre de sélection, dessiné par-dessus tout.
  useEffect(() => {
    let detach: (() => void) | null = null;
    let frameRequest = 0;
    const attach = () => {
      const scene = viewer.current?.scene();
      if (!scene) {
        frameRequest = requestAnimationFrame(attach);
        return;
      }
      const node = new SelectionNode();
      selectionNode.current = node;
      detach = scene.addNode(node);
    };
    attach();
    return () => {
      cancelAnimationFrame(frameRequest);
      selectionNode.current = null;
      detach?.();
    };
  }, [viewer]);
  useEffect(() => {
    selectionNode.current?.set({ outlines: selected.map((p) => corners(p)), frame, marquee });
    viewer.current?.scene()?.invalidate();
  }, [selected, frame, marquee, viewer, layers]);

  // --- Pointeur ------------------------------------------------------------------------------

  const pointerAt = (event: { clientX: number; clientY: number }) => {
    const scene = viewer.current?.scene();
    const canvas = stage.current?.querySelector(".viewer-canvas");
    if (!scene || !canvas) return null;
    const box = canvas.getBoundingClientRect();
    const screen = { x: event.clientX - box.left, y: event.clientY - box.top };
    const view = scene.getView();
    const viewport = scene.getViewport();
    return { screen, mm: screenToMm(view, viewport, screen), toScreen: (p: Point) => mmToScreen(view, viewport, p) };
  };

  const handleUnder = (at: NonNullable<ReturnType<typeof pointerAt>>) =>
    frame ? handleAt(frameCorners(frame).map(at.toScreen), at.screen) : null;

  const setCursor = (cursor: string) => {
    if (stage.current) stage.current.style.cursor = cursor;
  };

  const onPointerDown = (event: PointerEvent<HTMLDivElement>): boolean => {
    if (!enabled || event.button !== 0 || spaceDown.current) return false;
    const at = pointerAt(event);
    if (!at) return false;
    const handle = handleUnder(at);
    let gesture: Gesture | null = null;
    if (handle && frame) {
      gesture =
        handle.kind === "scale"
          ? { kind: "scale", start: at.mm, before: placements, ids: selection, frame, corner: handle.corner }
          : { kind: "rotate", start: at.mm, before: placements, ids: selection, pivot: rectCenter(frame) };
    } else {
      const hit = hitTest(picks, at.mm);
      if (hit) {
        const unit = expandToGroups(placements, [hit.id]);
        if (event.shiftKey) {
          // ⇧-clic : ajoute ou retire le dessin (et son groupe) de la sélection.
          const next = new Set(selection);
          const has = selection.has(hit.id);
          for (const id of unit) {
            if (has) next.delete(id);
            else next.add(id);
          }
          setSelection(next);
          event.stopPropagation();
          event.preventDefault();
          return true;
        }
        const ids = selection.has(hit.id) ? selection : unit;
        setSelection(ids);
        gesture = { kind: "move", start: at.mm, before: placements, ids };
      } else if (picks.length > 0) {
        gesture = { kind: "marquee", start: at.mm, before: placements, ids: event.shiftKey ? selection : new Set() };
      }
    }
    if (!gesture) return false;
    event.stopPropagation();
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    active.current = { gesture, down: { x: event.clientX, y: event.clientY, t: event.timeStamp }, step: null, ids: [] };
    return true;
  };

  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const at = pointerAt(event);
    if (!at) return;
    const current = active.current;
    if (!current) {
      if (!enabled || spaceDown.current) return;
      const handle = handleUnder(at);
      setCursor(
        handle?.kind === "scale"
          ? handle.corner % 2 === 0
            ? "nwse-resize"
            : "nesw-resize"
          : handle?.kind === "rotate"
            ? "alias"
            : hitTest(picks, at.mm)
              ? "move"
              : "",
      );
      return;
    }
    if (Math.hypot(event.clientX - current.down.x, event.clientY - current.down.y) < CLICK_PX && !current.step) return;
    let n = 0;
    const makeId = () => (current.ids[n++] ??= newId());
    const step = applyGesture(current.gesture, at.mm, { shift: event.shiftKey, alt: event.altKey }, makeId);
    current.step = step;
    setPlacements(step.placements);
    if (current.gesture.kind === "marquee") {
      // Le cadre ne prend pas les dessins masqués ou verrouillés.
      const allowed = new Set(picks.map((p) => p.id));
      setSelection(new Set([...step.selection].filter((id) => allowed.has(id))));
    } else setSelection(step.selection);
    setMarquee(step.marquee ?? null);
  };

  const onPointerUp = (event: PointerEvent<HTMLDivElement>): boolean => {
    const current = active.current;
    if (!current) return false;
    active.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId);
    setMarquee(null);
    const { gesture, step } = current;
    if (!step) {
      // Un simple clic : dans le vide, il vide la sélection.
      if (gesture.kind === "marquee" && !event.shiftKey) setSelection(new Set());
      return true;
    }
    if (gesture.kind !== "marquee") {
      if (step.repeat) lastRepeat.current = step.repeat;
      commit(step.placements, step.selection);
    }
    return true;
  };

  // Espace enfoncée : la main, pour déplacer la vue comme dans Illustrator.
  useEffect(() => {
    const down = (event: KeyboardEvent) => {
      if (event.code !== "Space" || event.repeat) return;
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) return;
      spaceDown.current = true;
      if (stage.current) stage.current.style.cursor = "grab";
    };
    const up = (event: KeyboardEvent) => {
      if (event.code !== "Space") return;
      spaceDown.current = false;
      if (stage.current) stage.current.style.cursor = "";
    };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
    };
  }, [stage]);

  // --- Glisser un dessin depuis la liste ------------------------------------------------------

  const viewCenterAndWidth = () => {
    const scene = viewer.current?.scene();
    if (!scene) return null;
    const visible = visibleRect(scene.getView(), scene.getViewport());
    return { center: rectCenter(visible), width: visible.width };
  };

  const placeDrawing = (id: string, at?: Point) => {
    const drawing = drawings.find((d) => d.id === id);
    const view = viewCenterAndWidth();
    if (!drawing || !view) return;
    if (!isShown(activeLayer) || isLocked(activeLayer)) {
      onError(
        `Le calque « ${activeLayer.name} » est ${isShown(activeLayer) ? "verrouillé" : "masqué"} : choisir un autre calque pour poser le dessin.`,
      );
      return;
    }
    const p = dropped(drawing, at ?? view.center, view.width);
    if (layerList.length > 0) p.layer = activeLayer.id;
    commit([...placements, p], new Set([p.id]));
  };

  const onDragOver = (event: DragEvent<HTMLDivElement>) => {
    if (!enabled || !event.dataTransfer.types.includes(DRAWING_DRAG_TYPE)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
  };

  const onDrop = (event: DragEvent<HTMLDivElement>) => {
    const id = event.dataTransfer.getData(DRAWING_DRAG_TYPE);
    if (!enabled || !id) return;
    event.preventDefault();
    const at = pointerAt(event);
    if (at) placeDrawing(id, at.mm);
  };

  // --- Actions sur la sélection ---------------------------------------------------------------

  const has = selection.size > 0;
  const apply = (next: Placement[], nextSelection?: Set<string>) => {
    if (next !== placements) commit(next, nextSelection);
  };

  const paste = (where: "center" | "front" | "back") => {
    if (clipboard.current.length === 0) return;
    let offset = { x: 0, y: 0 };
    if (where === "center") {
      const view = viewCenterAndWidth();
      const box = bounds(clipboard.current);
      if (view && box) {
        const c = rectCenter(box);
        offset = { x: view.center.x - c.x, y: view.center.y - c.y };
      }
    }
    const copies = copyOf(clipboard.current, offset);
    const ids = new Set(copies.map((p) => p.id));
    apply(where === "back" ? [...copies, ...placements] : [...placements, ...copies], ids);
  };

  const nudge = (dx: number, dy: number) => () => apply(translate(placements, selection, dx, dy));
  const big = NUDGE_MM * NUDGE_SHIFT_FACTOR;

  const actions: CompositionActions = {
    remove: () => apply(remove(placements, selection), new Set()),
    flip: (axis) => apply(flip(placements, selection, axis)),
    rotateBy: (deg) => frame && apply(rotate(placements, selection, deg, rectCenter(frame))),
    align: (how) => apply(align(placements, selection, how)),
    distribute: (axis) => apply(distribute(placements, selection, axis)),
    group: () => apply(group(placements, selection)),
    ungroup: () => apply(ungroup(placements, selection)),
    arrange: (how) => apply(arrange(placements, selection, how)),
    moveTo: (center) => {
      if (!frame) return;
      const c = rectCenter(frame);
      apply(translate(placements, selection, center.x - c.x, center.y - c.y));
    },
    resizeTo: (width) =>
      frame && frame.width > 0 && apply(scale(placements, selection, width / frame.width, rectCenter(frame))),
    setRotation: (deg) => {
      if (selected.length !== 1) return;
      const p = selected[0];
      apply(rotate(placements, selection, deg - p.rotation, p.center));
    },
    place: (id) => placeDrawing(id),
    moveToLayer: (id) => {
      const r = moveToLayer(current, selection, id);
      if (r !== current) commit(r.placements, undefined, r.layers);
    },
  };

  const shortcuts = [
    {
      keys: "mod+a",
      run: () => setSelection(new Set(picks.map((p) => p.id))),
      enabled: enabled && picks.length > 0,
    },
    { keys: "mod+shift+a", run: () => setSelection(new Set()), enabled: enabled && has },
    { keys: "escape", run: () => setSelection(new Set()), enabled: enabled && has },
    { keys: "backspace", run: actions.remove, enabled: enabled && has },
    { keys: "arrowleft", run: nudge(-NUDGE_MM, 0), enabled: enabled && has },
    { keys: "arrowright", run: nudge(NUDGE_MM, 0), enabled: enabled && has },
    { keys: "arrowup", run: nudge(0, -NUDGE_MM), enabled: enabled && has },
    { keys: "arrowdown", run: nudge(0, NUDGE_MM), enabled: enabled && has },
    { keys: "shift+arrowleft", run: nudge(-big, 0), enabled: enabled && has },
    { keys: "shift+arrowright", run: nudge(big, 0), enabled: enabled && has },
    { keys: "shift+arrowup", run: nudge(0, -big), enabled: enabled && has },
    { keys: "shift+arrowdown", run: nudge(0, big), enabled: enabled && has },
    { keys: "mod+c", run: () => (clipboard.current = selected), enabled: enabled && has },
    {
      keys: "mod+x",
      run: () => {
        clipboard.current = selected;
        actions.remove();
      },
      enabled: enabled && has,
    },
    { keys: "mod+v", run: () => paste("center"), enabled },
    { keys: "mod+f", run: () => paste("front"), enabled },
    { keys: "mod+b", run: () => paste("back"), enabled },
    {
      keys: "mod+d",
      run: () => {
        const r = lastRepeat.current ?? { kind: "move", dx: PASTE_OFFSET_MM, dy: PASTE_OFFSET_MM, copy: true };
        const result = repeatGesture(placements, selection, r);
        apply(result.placements, result.selection);
      },
      enabled: enabled && has,
    },
    { keys: "mod+g", run: actions.group, enabled: enabled && selection.size > 1 },
    { keys: "mod+shift+g", run: actions.ungroup, enabled: enabled && has },
    { keys: "mod+shift+]", run: () => actions.arrange("front"), enabled: enabled && has },
    { keys: "mod+]", run: () => actions.arrange("forward"), enabled: enabled && has },
    { keys: "mod+[", run: () => actions.arrange("backward"), enabled: enabled && has },
    { keys: "mod+shift+[", run: () => actions.arrange("back"), enabled: enabled && has },
  ];

  const hint = !enabled
    ? null
    : has
      ? "Glisser : déplacer (⌥ copie, ⇧ droit) · coin : agrandir · près d'un coin, dehors : tourner · Espace : déplacer la vue"
      : picks.length > 0
        ? "Cliquer un dessin pour le choisir, ou glisser autour de plusieurs"
        : drawings.length > 0
          ? "Glisser un dessin de la liste sur la façade"
          : null;

  return {
    drawings,
    placements,
    selected,
    frame,
    composeLayers,
    activeLayer: activeLayer.id,
    layerCounts,
    layerActions,
    layers,
    hint,
    actions,
    onPointerDown,
    onPointerMove,
    onPointerUp,
    onDragOver,
    onDrop,
    shortcuts,
  };
}

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
  fetchAssignments,
  fetchDrawingPreview,
  fetchLibrary,
  fetchSilhouettes,
  saveComposition,
  type Assignment,
  type LibraryEntry,
  type Project,
  type SilhouettePolygon,
} from "../engine";
import { projectorColor } from "../facade/overview";
import { demoLibrary, demoSilhouette } from "../library/demo";
import type { LayerSpec, ViewerHandle } from "../viewer";
import { mmToScreen, screenToMm, visibleRect } from "../viewer/geometry";
import {
  applyGesture,
  frameHandles,
  handleAt,
  nodeAt,
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
  assignTo,
  bounds,
  copyOf,
  deform,
  distribute,
  dropped,
  expandToGroups,
  facadePoint,
  flip,
  group,
  hitTest,
  newId,
  outline,
  rectCenter,
  remove,
  rotate,
  scale,
  sharedDeformation,
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
import { meshLines, nodePoints } from "./deformation";
import { SelectionNode } from "./SelectionNode";
import { dropInTree, setFlag, type TreeDrop } from "./slideTree";
import { DRAWING_DRAG_TYPE, draggedDrawing, droppedDrawingId, endDrawingDrag } from "./dragDrawing";

export { DRAWING_DRAG_TYPE };

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
  /** Choisit à la main la diapositive de la sélection ; `null` : la proposition du moteur (tâche 4.5). */
  assignTo(projector: string | null): void;
  /**
   * Tend sur la sélection une grille au repos de `columns × rows` cases
   * (1 × 1 : quatre coins), d'un bloc ; 0 retire la déformation (tâche 4.4).
   */
  deform(columns: number, rows: number): void;
  /** Choisit ces dessins posés (ceux qui existent), par identifiant (tâche 6.1). */
  select(ids: string[]): void;
  /** Un dessin lâché dans l'arborescence des diapositives : autre diapositive, ou devant un autre dessin. */
  dropInTree(dragged: string, drop: TreeDrop): void;
  /** Choisit ces dessins depuis l'arborescence, groupes compris ; ni masqués ni verrouillés (D-4.5-g). */
  selectInTree(ids: Iterable<string>): void;
  /** Masque (vrai) ou affiche ces dessins un à un ; masqués, ils ne sont plus choisis. */
  setHidden(ids: Iterable<string>, hidden: boolean): void;
  /** Verrouille (vrai) ou déverrouille ces dessins un à un ; verrouillés, ils ne sont plus choisis. */
  setLocked(ids: Iterable<string>, locked: boolean): void;
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
  /** Diapositive de chaque dessin posé, par identifiant (tâche 4.5) ; vide sans moteur. */
  assignments: Map<string, Assignment>;
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
/** Cases par côté de la maille qui affiche un dessin déformé. */
const PREVIEW_DIVISIONS = 16;

/** Points de la maille d'affichage d'un dessin posé, ligne par ligne. */
function previewMesh(p: Placement): { divisions: number; points: Point[] } {
  const n = p.deformation ? PREVIEW_DIVISIONS : 1;
  const points: Point[] = [];
  for (let j = 0; j <= n; j++) for (let i = 0; i <= n; i++) points.push(facadePoint(p, i / n, j / n));
  return { divisions: n, points };
}

export function useComposition(options: Options): Composition {
  const { engine, folder, project, onProject, viewer, stage, enabled, onError } = options;
  const [drawings, setDrawings] = useState<ComposeDrawing[]>([]);
  const [placements, setPlacements] = useState<Placement[]>([]);
  const [layerList, setLayerList] = useState<Layer[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [selection, setSelection] = useState<Set<string>>(new Set());
  const [marquee, setMarquee] = useState<Rect | null>(null);
  const [previews, setPreviews] = useState<Map<string, string>>(new Map());
  const [assignments, setAssignments] = useState<Map<string, Assignment>>(new Map());
  const [detected, setDetected] = useState<Map<string, SilhouettePolygon[]>>(new Map());
  const requested = useRef<Set<string>>(new Set());
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

  // Diapositive de chaque dessin : recalculée par le moteur à chaque
  // enregistrement (dessins posés ou recalage des projecteurs).
  const projectProjectors = project?.projectors;
  useEffect(() => {
    if (!engine || !folder) {
      setAssignments(new Map());
      return;
    }
    let cancelled = false;
    fetchAssignments(folder).then(
      (list) => !cancelled && setAssignments(new Map(list.map((a) => [a.placement, a]))),
      (failure: unknown) => !cancelled && onError(String(failure)),
    );
    return () => {
      cancelled = true;
    };
  }, [engine, folder, projectPlacements, projectProjectors, onError]);

  useEffect(() => {
    setSelection(new Set());
    setPreviews(new Map());
    setDetected(new Map());
    requested.current = new Set();
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

  // Silhouettes (tâche 4.7) : celle retouchée dans la bibliothèque (le projet
  // fait foi), sinon celle que le moteur détecte, demandée une fois par dessin.
  const projectLibrary = project?.library;
  const manualSilhouettes = useMemo(
    () => new Map((projectLibrary ?? []).flatMap((d) => (d.silhouette ? [[d.id, d.silhouette] as const] : []))),
    [projectLibrary],
  );
  useEffect(() => {
    // Une silhouette rendue à la détection se redemande.
    requested.current = new Set();
  }, [projectLibrary]);
  useEffect(() => {
    const wanted = [...new Set(placements.map((p) => p.drawing))].filter(
      (id) => !manualSilhouettes.has(id) && !detected.has(id) && !requested.current.has(id),
    );
    if (wanted.length === 0) return;
    for (const id of wanted) requested.current.add(id);
    if (!engine) {
      setDetected((current) => new Map([...current, ...wanted.map((id) => [id, demoSilhouette(id)] as const)]));
      return;
    }
    if (!folder) return;
    fetchSilhouettes(folder, wanted).then(
      (list) =>
        setDetected((current) => {
          const next = new Map(current);
          for (const s of list) if (!s.manual) next.set(s.drawing, s.polygons);
          return next;
        }),
      (failure: unknown) => onError(String(failure)),
    );
  }, [placements, manualSilhouettes, detected, engine, folder, onError]);

  // --- Affichage --------------------------------------------------------------------------

  const selected = useMemo(() => placements.filter((p) => selection.has(p.id)), [placements, selection]);
  const frame = useMemo(() => bounds(selected), [selected]);
  // Les nœuds ne se tirent que sur une sélection qui partage une même déformation.
  const editable = useMemo(() => sharedDeformation(selected), [selected]);
  const editableNodes = useMemo(() => (editable ? nodePoints(editable) : []), [editable]);

  const layers = useMemo<LayerSpec[]>(
    () =>
      shown(placements, layerList).flatMap((p) => {
        const url = previews.get(p.drawing);
        if (!url) return [];
        const pose = { id: `pose:${p.id}`, kind: "mesh-image", url, ...previewMesh(p) } satisfies LayerSpec;
        // Juste sous le dessin, sa silhouette en noir cache ce qui est derrière.
        const polygons = manualSilhouettes.get(p.drawing) ?? detected.get(p.drawing) ?? [];
        if (polygons.length === 0) return [pose];
        const mask = {
          id: `silhouette:${p.id}`,
          kind: "polygons",
          shapes: polygons.map((polygon) => ({
            points: polygon.map(([u, v]) => facadePoint(p, u, v)),
            color: 0x000000,
            fillAlpha: 1,
            strokeAlpha: 0,
          })),
        } satisfies LayerSpec;
        return [mask, pose];
      }),
    [placements, layerList, previews, manualSilhouettes, detected],
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
    const colors = new Map((project?.projectors ?? []).map((q, index) => [q.id, projectorColor(index)]));
    const slides = shown(placements, layerList).flatMap((p) => {
      const a = assignments.get(p.id);
      if (!a) return [];
      return [{ outline: outline(p), color: a.projector ? (colors.get(a.projector) ?? null) : null, whole: a.whole }];
    });
    selectionNode.current?.set({
      slides,
      outlines: selected.map((p) => outline(p)),
      frame,
      marquee,
      mesh: editable ? { lines: meshLines(editable), nodes: editableNodes } : null,
    });
    viewer.current?.scene()?.invalidate();
  }, [
    selected,
    frame,
    marquee,
    viewer,
    layers,
    editable,
    editableNodes,
    assignments,
    placements,
    layerList,
    project?.projectors,
  ]);

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
    frame ? handleAt(frameHandles(frame).map(at.toScreen), at.screen) : null;

  const nodeUnder = (at: NonNullable<ReturnType<typeof pointerAt>>) =>
    editable ? nodeAt(editableNodes.map(at.toScreen), at.screen) : null;

  const setCursor = (cursor: string) => {
    if (stage.current) stage.current.style.cursor = cursor;
  };

  const onPointerDown = (event: PointerEvent<HTMLDivElement>): boolean => {
    if (!enabled || event.button !== 0 || spaceDown.current) return false;
    const at = pointerAt(event);
    if (!at) return false;
    const node = nodeUnder(at);
    const handle = node === null ? handleUnder(at) : null;
    let gesture: Gesture | null = null;
    if (node !== null && editable) {
      gesture = {
        kind: "node",
        start: at.mm,
        before: placements,
        ids: selection,
        deformation: editable,
        index: node,
        origin: editableNodes[node],
      };
    } else if (handle && frame) {
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
        nodeUnder(at) !== null
          ? "crosshair"
          : handle?.kind === "scale"
            ? ([
                "nwse-resize",
                "nesw-resize",
                "nwse-resize",
                "nesw-resize",
                "ns-resize",
                "ew-resize",
                "ns-resize",
                "ew-resize",
              ][handle.corner] ?? "nwse-resize")
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
    if (!enabled) return;
    if (!event.dataTransfer.types.includes(DRAWING_DRAG_TYPE) && draggedDrawing() === null) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
  };

  const onDrop = (event: DragEvent<HTMLDivElement>) => {
    const id = droppedDrawingId(event.dataTransfer, draggedDrawing());
    endDrawingDrag();
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
    deform: (columns, rows) => apply(deform(placements, selection, columns, rows)),
    assignTo: (projector) => apply(assignTo(placements, selection, projector)),
    select: (ids) => setSelection(new Set(ids.filter((id) => placements.some((p) => p.id === id)))),
    dropInTree: (dragged, drop) => apply(dropInTree(placements, assignments, dragged, drop), new Set([dragged])),
    selectInTree: (ids) => {
      const free = new Set(placements.filter((p) => !p.hidden && !p.locked).map((p) => p.id));
      setSelection(new Set([...expandToGroups(placements, ids)].filter((id) => free.has(id))));
    },
    setHidden: (ids, value) => {
      const set = new Set(ids);
      const kept = value ? new Set([...selection].filter((id) => !set.has(id))) : undefined;
      apply(setFlag(placements, set, "hidden", value), kept);
    },
    setLocked: (ids, value) => {
      const set = new Set(ids);
      const kept = value ? new Set([...selection].filter((id) => !set.has(id))) : undefined;
      apply(setFlag(placements, set, "locked", value), kept);
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
      ? editable
        ? "Glisser un point bleu : déformer · glisser le dessin : déplacer · coin du cadre : agrandir · Espace : déplacer la vue"
        : "Glisser : déplacer (⌥ copie, ⇧ droit) · coin : agrandir · près d'un coin, dehors : tourner · Espace : déplacer la vue"
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
    assignments,
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

// Outils des dessins dans la vue Diapositive (tâche 4.6, puis demande de
// Benjamin du 13 novembre, D-3.6-f) : les mêmes gestes que sur la façade
// (choisir, ⇧-clic, glisser, cadre et poignées, rotation, mode 4 coins,
// grilles, alignement), mais pour cette diapositive seulement. Rien ne
// bouge sur la façade ni dans les autres diapositives : chaque geste change
// la retouche des dessins choisis (une grille en mm de diapositive). Les
// calculs sont dans `tools.ts` et `adjust.ts`.
//
// À la fin de chaque geste, les retouches changées partent au moteur en un
// seul enregistrement (une seule étape d'annulation) ; l'aperçu de la
// diapositive est alors recalculé. Les dessins retouchés s'affichent en
// bleu pendant le geste et jusqu'à l'arrivée de l'aperçu recalculé.

import { useCallback, useEffect, useMemo, useRef, useState, type PointerEvent, type RefObject } from "react";
import { fetchDrawingPreview, saveComposition, type Project, type SlideDrawing, type SlideView } from "../engine";
import { meshLines, moveNode, nodePoints, type Deformation } from "../compose/deformation";
import { frameHandles, handleAt, nodeAt, rotateCursor, scaleCursor, KNOB } from "../compose/gesture";
import type { Alignment, Point, Rect } from "../compose/placement";
import { SelectionNode } from "../compose/SelectionNode";
import { demoLibrary } from "../library/demo";
import { NudgeSettle, nudged, setViewerCursor, useArrowNudge, type LayerSpec, type ViewerHandle } from "../viewer";
import { mmToScreen, screenToMm } from "../viewer/geometry";
import { adjustedOutline, adjustedPoints, drawingAt, drawingsAt, restAdjustment } from "./adjust";
import { nextBehind } from "../compose/placement";
import { commandDown } from "../ui/shortcuts";
import {
  alignShifts,
  applySlideGesture,
  boxOf,
  distributeShifts,
  regrid,
  shiftAdjustment,
  slideCopies,
  SLIDE_COPY_OFFSET_MM,
  turnAdjustment,
  type SlideGesture,
  type SlideUnit,
} from "./tools";

/** Couleur de la retouche en cours, sur la photo. */
const LIVE_TINT = 0x0a84ff;
const CLICK_PX = 4;

interface Options {
  engine: boolean;
  folder: string | null;
  view: SlideView | null;
  project: Project | null;
  /** Sélection venue d'ailleurs (onglet Calques) ; suivie quand elle change. */
  external: Set<string>;
  /** Fait connaître la sélection d'ici (outils qui changent le dessin partout). */
  onSelect: (ids: string[]) => void;
  /** Choisira ces dessins dès que la composition les connaîtra (copies tout juste enregistrées). */
  selectLater: (ids: string[]) => void;
  onProject: (project: Project) => void;
  viewer: RefObject<ViewerHandle | null>;
  stage: RefObject<HTMLDivElement | null>;
  onError: (message: string | null) => void;
  /** Silhouette d'un dessin de la bibliothèque, ou null tant qu'elle manque. */
  silhouetteOf: (drawing: string) => [number, number][][] | null;
}

export interface SlideActions {
  rotateBy(deg: number): void;
  align(how: Alignment): void;
  distribute(axis: "horizontal" | "vertical"): void;
  /** Grille de `columns × rows` cases (1 × 1 : quatre coins) qui garde la retouche ; 0 la retire. */
  deform(columns: number, rows: number): void;
  setMode(mode: "normal" | "corners"): void;
  /** ⌘D : copie les dessins choisis dans cette diapositive, un peu décalée. */
  duplicate(): void;
}

export interface SlideAdjust {
  /** Les dessins choisis dans cette diapositive. */
  selected: SlideDrawing[];
  /** Au moins un dessin choisi est retouché dans cette diapositive. */
  adjusted: boolean;
  /** Mode 4 coins : les points de la retouche se tirent, sans cadre. */
  corners: boolean;
  actions: SlideActions;
  layers: LayerSpec[];
  hint: string;
  /** Vrai pendant un geste et jusqu'à l'arrivée de l'aperçu recalculé. */
  pending: boolean;
  onPointerDown(event: PointerEvent<HTMLDivElement>): void;
  onPointerMove(event: PointerEvent<HTMLDivElement>): void;
  onPointerUp(event: PointerEvent<HTMLDivElement>): void;
}

const sameSet = (a: Set<string>, b: Set<string>) => a.size === b.size && [...a].every((x) => b.has(x));

export function useSlideAdjust(options: Options): SlideAdjust {
  const { engine, folder, view, project, external, onSelect, selectLater, onProject, viewer, stage, onError, silhouetteOf } =
    options;
  const [selection, setSelection] = useState<Set<string>>(new Set());
  /** Retouches tenues à jour à l'écran (le moteur fait foi après chaque enregistrement). */
  const [local, setLocal] = useState<Map<string, Deformation | null>>(new Map());
  const [previews, setPreviews] = useState<Map<string, string>>(new Map());
  /** `copy` : ⌥ au début du glisser, les dessins déplacés sont des copies. */
  const active = useRef<{ gesture: SlideGesture; down: Point; moved: boolean; copy: boolean } | null>(null);
  const [pending, setPending] = useState(false);
  const node = useRef<SelectionNode | null>(null);
  /** Point cliqué en mode 4 coins : les flèches le déplacent. */
  const [activeNode, setActiveNode] = useState<number | null>(null);
  /** Sélection où l'on a choisi le mode 4 coins. */
  const [cornersFor, setCornersFor] = useState<string | null>(null);
  const settle = useRef(new NudgeSettle<null>());

  const projector = view?.projectorId ?? null;
  const drawings = useMemo(() => view?.drawings ?? [], [view]);
  const placements = useMemo(() => project?.placements ?? [], [project]);

  // Le projet fait foi : on oublie l'état local à chaque nouvel état du projet.
  useEffect(() => setLocal(new Map()), [placements]);
  useEffect(() => setSelection(new Set()), [projector]);
  useEffect(() => () => settle.current.flush(), []);
  useEffect(() => setPending(false), [view]);
  // La sélection faite dans l'onglet Calques (sans moteur, il n'y en a pas).
  useEffect(() => {
    if (engine) setSelection((current) => (sameSet(current, external) ? current : new Set(external)));
  }, [engine, external]);

  const choose = useCallback(
    (ids: Iterable<string>) => {
      const next = new Set(ids);
      setSelection(next);
      onSelect([...next]);
    },
    [onSelect],
  );

  const adjustmentOf = useCallback(
    (placement: string): Deformation | null => {
      if (local.has(placement)) return local.get(placement) ?? null;
      const a = placements.find((p) => p.id === placement)?.adjustment;
      return a && a.projector === projector ? a.deformation : null;
    },
    [local, placements, projector],
  );

  const selected = useMemo(() => drawings.filter((d) => selection.has(d.placement)), [drawings, selection]);
  const selectionKey = useMemo(() => selected.map((d) => d.placement).sort().join(","), [selected]);
  const corners = selected.length === 1 && cornersFor === selectionKey;
  useEffect(() => setActiveNode(null), [selectionKey, corners]);
  const meshOf = corners ? adjustmentOf(selected[0].placement) : null;

  /** Le dessin et ses compagnons de groupe présents dans cette diapositive. */
  const withGroup = useCallback(
    (id: string): string[] => {
      const group = placements.find((p) => p.id === id)?.group;
      if (!group) return [id];
      const mates = new Set(placements.filter((p) => p.group === group).map((p) => p.id));
      return drawings.filter((d) => mates.has(d.placement)).map((d) => d.placement);
    },
    [placements, drawings],
  );

  /** Un groupe compte pour un (⌘-clic : le dessin suivant en dessous). */
  const groupKey = useCallback(
    (id: string) => {
      const group = placements.find((p) => p.id === id)?.group;
      return group ? `g:${group}` : `p:${id}`;
    },
    [placements],
  );

  const outlineOf = useCallback((d: SlideDrawing) => adjustedOutline(d, adjustmentOf(d.placement)), [adjustmentOf]);
  const frame: Rect | null = useMemo(
    () => (corners ? null : boxOf(selected.flatMap(outlineOf))),
    [corners, selected, outlineOf],
  );

  // Aperçus des dessins choisis, pour les montrer retouchés pendant le geste.
  useEffect(() => {
    const wanted = [...new Set(selected.map((d) => d.drawing))].filter((id) => !previews.has(id));
    if (wanted.length === 0) return;
    if (!engine) {
      const demo = demoLibrary(12);
      setPreviews((m) => {
        const next = new Map(m);
        for (const id of wanted) next.set(id, demo.find((d) => d.id === id)?.url ?? "");
        return next;
      });
      return;
    }
    if (!folder) return;
    let cancelled = false;
    for (const id of wanted)
      fetchDrawingPreview(folder, id).then(
        (url) => !cancelled && setPreviews((m) => new Map(m).set(id, url)),
        (failure: unknown) => !cancelled && onError(String(failure)),
      );
    return () => {
      cancelled = true;
    };
  }, [selected, previews, engine, folder, onError]);

  const layers = useMemo<LayerSpec[]>(() => {
    // Sans moteur, l'aperçu ne se recalcule pas : le bleu montre la retouche.
    if (engine && !pending) return [];
    return selected.flatMap((d) => {
      const url = previews.get(d.drawing);
      if (!url) return [];
      return [
        {
          id: `retouche:${d.placement}`,
          kind: "mesh-image",
          url,
          divisions: d.divisions,
          points: adjustedPoints(d, adjustmentOf(d.placement)),
          tint: LIVE_TINT,
          opacity: 0.9,
        } satisfies LayerSpec,
      ];
    });
  }, [selected, previews, adjustmentOf, engine, pending]);

  // Contours, cadre et points, par-dessus.
  useEffect(() => {
    let detach: (() => void) | null = null;
    let frameRequest = 0;
    const attach = () => {
      const scene = viewer.current?.scene();
      if (!scene) {
        frameRequest = requestAnimationFrame(attach);
        return;
      }
      const n = new SelectionNode();
      node.current = n;
      detach = scene.addNode(n);
    };
    attach();
    return () => {
      cancelAnimationFrame(frameRequest);
      node.current = null;
      detach?.();
    };
  }, [viewer]);
  useEffect(() => {
    node.current?.set({
      slides: [],
      lost: [],
      outlines: selected.map(outlineOf),
      frame,
      marquee: null,
      mesh: meshOf ? { lines: meshLines(meshOf), nodes: nodePoints(meshOf), active: activeNode } : null,
    });
    viewer.current?.scene()?.invalidate();
  }, [selected, outlineOf, frame, meshOf, viewer, layers, activeNode]);

  /** Enregistre d'un coup les retouches changées (aucune : retirée). */
  const save = (changes: Map<string, Deformation | null>) => {
    if (changes.size === 0) return;
    setLocal((m) => new Map([...m, ...changes]));
    if (!engine || !folder || !projector || !project) return;
    setPending(true);
    const next = placements.map((p) => {
      if (!changes.has(p.id)) return p;
      const deformation = changes.get(p.id) ?? null;
      return { ...p, adjustment: deformation ? { projector, deformation } : null };
    });
    saveComposition(folder, project.layers ?? [], next).then(onProject, (failure: unknown) => {
      onError(String(failure).replace(/^Error: /, ""));
      setPending(false);
      setLocal(new Map());
    });
  };

  /** Copies des dessins, avec leur retouche (⌘D, ⌥ + glisser) ; les copies sont choisies. */
  const saveCopies = (retouches: Map<string, Deformation | null>) => {
    const kept = new Map([...retouches].flatMap(([id, d]) => (d ? [[id, d] as const] : [])));
    setLocal(new Map());
    if (kept.size === 0 || !engine || !folder || !projector || !project) return;
    const r = slideCopies(placements, kept, projector);
    setPending(true);
    setSelection(new Set(r.ids));
    selectLater(r.ids);
    saveComposition(folder, project.layers ?? [], r.placements).then(onProject, (failure: unknown) => {
      onError(String(failure).replace(/^Error: /, ""));
      setPending(false);
    });
  };

  /** La retouche de départ d'un dessin : la sienne, sinon quatre coins au repos. */
  const baseOf = (d: SlideDrawing): Deformation | null => adjustmentOf(d.placement) ?? restAdjustment(d, 1, 1);
  const befores = (list: SlideDrawing[]) => {
    const out = new Map<string, Deformation>();
    for (const d of list) {
      const b = baseOf(d);
      if (b) out.set(d.placement, b);
    }
    return out;
  };
  const shifted = (shifts: Map<string, Point>) => {
    const out = new Map<string, Deformation | null>();
    for (const d of selected) {
      const s = shifts.get(d.placement);
      const b = baseOf(d);
      if (s && b && (s.x !== 0 || s.y !== 0)) out.set(d.placement, shiftAdjustment(b, s.x, s.y));
    }
    return out;
  };
  /** Les blocs à aligner : un groupe compte pour un. */
  const units = (): SlideUnit[] => {
    const byKey = new Map<string, SlideDrawing[]>();
    for (const d of selected) {
      const key = placements.find((p) => p.id === d.placement)?.group ?? `seul:${d.placement}`;
      byKey.set(key, [...(byKey.get(key) ?? []), d]);
    }
    return [...byKey.values()].flatMap((list) => {
      const box = boxOf(list.flatMap(outlineOf));
      return box ? [{ ids: list.map((d) => d.placement), box }] : [];
    });
  };

  const actions: SlideActions = {
    rotateBy: (deg) => {
      if (!frame && selected.length === 0) return;
      const box = frame ?? boxOf(selected.flatMap(outlineOf));
      if (!box) return;
      const pivot = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
      const out = new Map<string, Deformation | null>();
      for (const [id, b] of befores(selected)) out.set(id, turnAdjustment(b, deg, pivot));
      save(out);
    },
    align: (how) => save(shifted(alignShifts(units(), how))),
    distribute: (axis) => save(shifted(distributeShifts(units(), axis))),
    deform: (columns, rows) => {
      const out = new Map<string, Deformation | null>();
      for (const d of selected) {
        if (columns === 0) {
          if (adjustmentOf(d.placement)) out.set(d.placement, null);
          continue;
        }
        const b = baseOf(d);
        if (b) out.set(d.placement, regrid(b, columns, rows));
      }
      save(out);
      if (columns === 0) setCornersFor(null);
      else if (selected.length === 1) setCornersFor(selectionKey);
    },
    duplicate: () => {
      const out = new Map<string, Deformation | null>();
      for (const [id, b] of befores(selected)) out.set(id, shiftAdjustment(b, SLIDE_COPY_OFFSET_MM, SLIDE_COPY_OFFSET_MM));
      saveCopies(out);
    },
    setMode: (mode) => {
      if (mode === "normal" || selected.length !== 1) {
        setCornersFor(null);
        return;
      }
      if (!adjustmentOf(selected[0].placement)) {
        const b = restAdjustment(selected[0], 1, 1);
        if (b) save(new Map([[selected[0].placement, b]]));
      }
      setCornersFor(selectionKey);
    },
  };

  const pointerAt = (event: { clientX: number; clientY: number }) => {
    const scene = viewer.current?.scene();
    const canvas = stage.current?.querySelector(".viewer-canvas");
    if (!scene || !canvas) return null;
    const box = canvas.getBoundingClientRect();
    const screen = { x: event.clientX - box.left, y: event.clientY - box.top };
    const v = scene.getView();
    const viewport = scene.getViewport();
    return { screen, mm: screenToMm(v, viewport, screen), toScreen: (p: Point) => mmToScreen(v, viewport, p) };
  };

  // Échap : quitter le mode 4 coins.
  useEffect(() => {
    if (!corners) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      event.preventDefault();
      setCornersFor(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [corners]);

  // Flèches : le point choisi bouge d'un point d'écran (dix avec ⇧) ; sans
  // point choisi, les dessins choisis se décalent dans cette diapositive.
  useArrowNudge(selected.length > 0, (step) => {
    const zoom = viewer.current?.scene()?.getView().zoom;
    if (selected.length === 0 || !zoom) return false;
    const out = new Map<string, Deformation | null>();
    if (meshOf && activeNode !== null && activeNode < nodePoints(meshOf).length) {
      const at = nodePoints(meshOf)[activeNode];
      out.set(selected[0].placement, moveNode(meshOf, activeNode, nudged(at, step, zoom)));
    } else {
      for (const [id, b] of befores(selected)) out.set(id, shiftAdjustment(b, step.x / zoom, step.y / zoom));
    }
    setLocal((m) => new Map([...m, ...out]));
    setPending(true);
    settle.current.push(null, () => save(out));
    return true;
  });

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    const at = pointerAt(event);
    if (!at) return;
    settle.current.flush();
    let gesture: SlideGesture | null = null;
    if (meshOf) {
      const nodes = nodePoints(meshOf);
      const index = nodeAt(nodes.map(at.toScreen), at.screen);
      setActiveNode(index);
      if (index !== null)
        gesture = {
          kind: "node",
          start: at.mm,
          id: selected[0].placement,
          index,
          origin: nodes[index],
          before: new Map([[selected[0].placement, meshOf]]),
        };
    }
    if (!gesture && frame) {
      const handle = handleAt(frameHandles(frame).map(at.toScreen), at.screen);
      if (handle?.kind === "rotate")
        gesture = {
          kind: "rotate",
          start: at.mm,
          pivot: { x: frame.x + frame.width / 2, y: frame.y + frame.height / 2 },
          before: befores(selected),
        };
      else if (handle?.kind === "scale")
        gesture = { kind: "scale", start: at.mm, frame, corner: handle.corner, before: befores(selected) };
    }
    if (!gesture) {
      // Un clic ne prend que ce qu'on voit d'un dessin (sa silhouette) ;
      // ⌘-clic : le dessin suivant en dessous, comme dans Illustrator.
      const behind = commandDown(event);
      const hit = behind
        ? nextBehind(
            drawingsAt(drawings, adjustmentOf, at.mm, silhouetteOf),
            (d) => groupKey(d.placement),
            (d) => selection.has(d.placement),
          )
        : drawingAt(drawings, adjustmentOf, at.mm, silhouetteOf);
      if (!hit) {
        if (!event.shiftKey) choose([]);
        return;
      }
      const ids = withGroup(hit.placement);
      let next: Set<string>;
      if (event.shiftKey && !behind) {
        // ⇧-clic : ajoute ou retire le dessin (et son groupe) de la sélection.
        next = new Set(selection);
        const remove = selection.has(hit.placement);
        for (const id of ids) {
          if (remove) next.delete(id);
          else next.add(id);
        }
        choose(next);
        if (remove) return;
      } else {
        next = selection.has(hit.placement) && !behind ? selection : new Set(ids);
        if (next !== selection) choose(next);
      }
      gesture = { kind: "move", start: at.mm, before: befores(drawings.filter((d) => next.has(d.placement))) };
    }
    event.stopPropagation();
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    // ⌥ en glissant les dessins : on en glisse des copies (comme sur la façade).
    const copy = gesture.kind === "move" && event.altKey;
    active.current = { gesture, down: { x: event.clientX, y: event.clientY }, moved: false, copy };
  };

  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const at = pointerAt(event);
    if (!at) return;
    const current = active.current;
    if (!current) {
      let cursor = "";
      if (meshOf && nodeAt(nodePoints(meshOf).map(at.toScreen), at.screen) !== null) cursor = "crosshair";
      else if (frame) {
        const handle = handleAt(frameHandles(frame).map(at.toScreen), at.screen);
        if (handle?.kind === "rotate") cursor = rotateCursor(handle.corner === KNOB ? 0 : handle.corner);
        else if (handle?.kind === "scale") cursor = scaleCursor(handle.corner);
      }
      if (!cursor && drawingAt(drawings, adjustmentOf, at.mm, silhouetteOf)) cursor = "move";
      setViewerCursor(stage.current, cursor);
      return;
    }
    if (!current.moved && Math.hypot(event.clientX - current.down.x, event.clientY - current.down.y) < CLICK_PX) return;
    if (!current.moved) setPending(true);
    current.moved = true;
    const next = applySlideGesture(current.gesture, at.mm, { shift: event.shiftKey, alt: event.altKey });
    setLocal((m) => new Map([...m, ...next]));
  };

  const onPointerUp = (event: PointerEvent<HTMLDivElement>) => {
    const current = active.current;
    if (!current) return;
    active.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId);
    if (!current.moved) return;
    const at = pointerAt(event);
    if (!at) {
      setPending(false);
      setLocal(new Map());
      return;
    }
    const result = applySlideGesture(current.gesture, at.mm, { shift: event.shiftKey, alt: event.altKey });
    if (current.copy) saveCopies(result);
    else save(result);
  };

  const adjusted = selected.some((d) => adjustmentOf(d.placement) !== null);
  const hint =
    selected.length > 0
      ? corners
        ? "Mode 4 coins : glisser un point bleu, ou le cliquer puis flèches (⇧ : pas plus grand) · Échap : retrouver le cadre"
        : "Glisser : décaler dans cette diapositive (⌥ copie, ⇧ droit) · ⌘D : copier · coin : agrandir · rond au-dessus du cadre : tourner (⇧ par 5°) · ⇧-clic : ajouter · ⌘-clic : dessin du dessous"
      : drawings.length > 0
        ? "Cliquer un dessin pour le retoucher dans cette diapositive (⌘-clic : celui du dessous)"
        : "";

  return {
    selected,
    adjusted,
    corners,
    actions,
    layers,
    hint,
    pending,
    onPointerDown,
    onPointerMove,
    onPointerUp,
  };
}

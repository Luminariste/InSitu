// Retouche d'un dessin dans la vue Diapositive (tâche 4.6) : choisir un
// dessin d'un clic, glisser pour le décaler dans cette diapositive, tirer
// les points d'une grille, et tout retirer d'un clic. Rien ne change sur la
// façade ni dans les autres diapositives. À la fin de chaque geste, la
// retouche est envoyée au moteur, qui la contrôle et l'enregistre ; l'aperçu
// de la diapositive est alors recalculé.
//
// Le dessin retouché s'affiche en bleu pendant le geste et jusqu'à ce que
// l'aperçu recalculé arrive ; ensuite seul l'aperçu le montre (retour
// d'essai du 13 novembre : le bleu, resté par-dessus, cachait l'aperçu à
// jour tant que le dessin était choisi).

import { useCallback, useEffect, useMemo, useRef, useState, type PointerEvent, type RefObject } from "react";
import { fetchDrawingPreview, saveAdjustment, type Project, type SlideDrawing, type SlideView } from "../engine";
import { meshLines, nodePoints, type Deformation } from "../compose/deformation";
import { nodeAt } from "../compose/gesture";
import type { Placement, Point } from "../compose/placement";
import { SelectionNode } from "../compose/SelectionNode";
import { demoLibrary } from "../library/demo";
import { NudgeSettle, nudged, setViewerCursor, useArrowNudge, type LayerSpec, type ViewerHandle } from "../viewer";
import { mmToScreen, screenToMm } from "../viewer/geometry";
import {
  adjustedCenter,
  adjustedOutline,
  adjustedPoints,
  applyAdjustGesture,
  drawingAt,
  restAdjustment,
  rotateAdjustment,
  shiftNodes,
  type AdjustGesture,
} from "./adjust";

/** Couleur de la retouche en cours, sur la photo. */
const LIVE_TINT = 0x0a84ff;
const CLICK_PX = 4;

interface Options {
  engine: boolean;
  folder: string | null;
  view: SlideView | null;
  placements: Placement[];
  onProject: (project: Project) => void;
  viewer: RefObject<ViewerHandle | null>;
  stage: RefObject<HTMLDivElement | null>;
  onError: (message: string | null) => void;
}

export interface SlideAdjust {
  /** Le dessin choisi dans cette diapositive. */
  selected: SlideDrawing | null;
  /** Sa retouche dans cette diapositive, s'il en a une. */
  adjustment: Deformation | null;
  layers: LayerSpec[];
  hint: string;
  /** Vrai pendant un geste et jusqu'à l'arrivée de l'aperçu recalculé. */
  pending: boolean;
  /** Grille au repos (1 × 1 : quatre coins) ; 0 retire la retouche. */
  setGrid(columns: number, rows: number): void;
  /** Tourne le dessin dans cette diapositive seulement. */
  rotateBy(deg: number): void;
  onPointerDown(event: PointerEvent<HTMLDivElement>): void;
  onPointerMove(event: PointerEvent<HTMLDivElement>): void;
  onPointerUp(event: PointerEvent<HTMLDivElement>): void;
}

export function useSlideAdjust(options: Options): SlideAdjust {
  const { engine, folder, view, placements, onProject, viewer, stage, onError } = options;
  const [selectedId, setSelectedId] = useState<string | null>(null);
  /** Retouches tenues à jour à l'écran (le moteur fait foi après chaque enregistrement). */
  const [local, setLocal] = useState<Map<string, Deformation | null>>(new Map());
  const [previews, setPreviews] = useState<Map<string, string>>(new Map());
  const active = useRef<{ gesture: AdjustGesture; down: Point; moved: boolean } | null>(null);
  /** Un geste est en cours, ou son aperçu recalculé n'est pas encore là. */
  const [pending, setPending] = useState(false);
  const node = useRef<SelectionNode | null>(null);
  /** Point cliqué : les flèches le déplacent (sans point, elles décalent le dessin). */
  const [activeNode, setActiveNode] = useState<number | null>(null);
  const settle = useRef(new NudgeSettle<null>());

  const projector = view?.projectorId ?? null;
  const drawings = useMemo(() => view?.drawings ?? [], [view]);

  // Le projet fait foi : on oublie l'état local à chaque nouvel état du projet.
  useEffect(() => setLocal(new Map()), [placements]);
  useEffect(() => setSelectedId(null), [projector]);
  useEffect(() => setActiveNode(null), [selectedId]);
  useEffect(() => () => settle.current.flush(), []);
  // Un nouvel état de la diapositive (aperçu recalculé) : le bleu s'efface.
  useEffect(() => setPending(false), [view]);

  const adjustmentOf = useCallback(
    (placement: string): Deformation | null => {
      if (local.has(placement)) return local.get(placement) ?? null;
      const a = placements.find((p) => p.id === placement)?.adjustment;
      return a && a.projector === projector ? a.deformation : null;
    },
    [local, placements, projector],
  );

  const selected = drawings.find((d) => d.placement === selectedId) ?? null;
  const adjustment = selected ? adjustmentOf(selected.placement) : null;

  // Aperçu du dessin choisi, pour le montrer retouché pendant le geste.
  useEffect(() => {
    if (!selected || previews.has(selected.drawing)) return;
    const id = selected.drawing;
    if (!engine) {
      const url = demoLibrary(12).find((d) => d.id === id)?.url;
      if (url) setPreviews((m) => new Map(m).set(id, url));
      return;
    }
    if (!folder) return;
    let cancelled = false;
    fetchDrawingPreview(folder, id).then(
      (url) => !cancelled && setPreviews((m) => new Map(m).set(id, url)),
      (failure: unknown) => !cancelled && onError(String(failure)),
    );
    return () => {
      cancelled = true;
    };
  }, [selected, previews, engine, folder, onError]);

  const layers = useMemo<LayerSpec[]>(() => {
    const url = selected ? previews.get(selected.drawing) : undefined;
    // Sans moteur, l'aperçu ne se recalcule pas : le bleu montre la retouche.
    if (!selected || !url || (engine && !pending)) return [];
    return [
      {
        id: `retouche:${selected.placement}`,
        kind: "mesh-image",
        url,
        divisions: selected.divisions,
        points: adjustedPoints(selected, adjustment),
        tint: LIVE_TINT,
        opacity: 0.9,
      },
    ];
  }, [selected, previews, adjustment, engine, pending]);

  // Contour, grille et points, par-dessus.
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
      outlines: selected ? [adjustedOutline(selected, adjustment)] : [],
      frame: null,
      marquee: null,
      mesh: adjustment ? { lines: meshLines(adjustment), nodes: nodePoints(adjustment), active: activeNode } : null,
    });
    viewer.current?.scene()?.invalidate();
  }, [selected, adjustment, viewer, layers, activeNode]);

  const save = (placement: string, next: Deformation | null) => {
    setLocal((m) => new Map(m).set(placement, next));
    if (!engine || !folder || !projector) return;
    setPending(true);
    saveAdjustment(folder, placement, next ? { projector, deformation: next } : null).then(
      onProject,
      (failure: unknown) => {
        onError(String(failure).replace(/^Error: /, ""));
        setPending(false);
        setLocal((m) => {
          const copy = new Map(m);
          copy.delete(placement);
          return copy;
        });
      },
    );
  };

  const setGrid = (columns: number, rows: number) => {
    if (!selected) return;
    save(selected.placement, columns === 0 ? null : restAdjustment(selected, columns, rows));
  };

  const rotateBy = (deg: number) => {
    if (!selected) return;
    const base = adjustment ?? restAdjustment(selected, 1, 1);
    if (!base) return;
    save(selected.placement, rotateAdjustment(base, deg, adjustedCenter(selected, base)));
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

  // Flèches : le point choisi bouge d'un point d'écran (dix avec ⇧) ; sans
  // point choisi, tout le dessin se décale dans cette diapositive.
  useArrowNudge(selected !== null, (step) => {
    const zoom = viewer.current?.scene()?.getView().zoom;
    if (!selected || !zoom) return false;
    const before = adjustment ?? restAdjustment(selected, 1, 1);
    if (!before) return false;
    const nodes = nodePoints(before);
    const next =
      adjustment && activeNode !== null && activeNode < nodes.length
        ? applyAdjustGesture(
            { kind: "node", start: nodes[activeNode], before, index: activeNode, origin: nodes[activeNode] },
            nudged(nodes[activeNode], step, zoom),
          )
        : shiftNodes(before, step.x / zoom, step.y / zoom);
    const id = selected.placement;
    setLocal((m) => new Map(m).set(id, next));
    setPending(true);
    settle.current.push(null, () => save(id, next));
    return true;
  });

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    const at = pointerAt(event);
    if (!at) return;
    settle.current.flush();
    let gesture: AdjustGesture | null = null;
    if (selected && adjustment) {
      const nodes = nodePoints(adjustment);
      const index = nodeAt(nodes.map(at.toScreen), at.screen);
      setActiveNode(index);
      if (index !== null) gesture = { kind: "node", start: at.mm, before: adjustment, index, origin: nodes[index] };
    }
    if (!gesture) {
      const hit = drawingAt(drawings, adjustmentOf, at.mm);
      setSelectedId(hit?.placement ?? null);
      if (!hit) return;
      // Glisser le dessin le décale dans cette diapositive (quatre coins au repos s'il n'a pas de retouche).
      const before = adjustmentOf(hit.placement) ?? restAdjustment(hit, 1, 1);
      if (before) gesture = { kind: "move", start: at.mm, before };
    }
    if (!gesture) return;
    event.stopPropagation();
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    active.current = { gesture, down: { x: event.clientX, y: event.clientY }, moved: false };
  };

  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const at = pointerAt(event);
    if (!at) return;
    const current = active.current;
    if (!current) {
      const overNode =
        adjustment && nodeAt(nodePoints(adjustment).map(at.toScreen), at.screen) !== null ? "crosshair" : null;
      const overDrawing = drawingAt(drawings, adjustmentOf, at.mm) ? "move" : "";
      setViewerCursor(stage.current, overNode ?? overDrawing);
      return;
    }
    if (!current.moved && Math.hypot(event.clientX - current.down.x, event.clientY - current.down.y) < CLICK_PX) return;
    if (!current.moved) setPending(true);
    current.moved = true;
    const id = selectedId ?? drawingAt(drawings, adjustmentOf, current.gesture.start)?.placement;
    if (!id) return;
    const next = applyAdjustGesture(current.gesture, at.mm);
    setLocal((m) => new Map(m).set(id, next));
  };

  const onPointerUp = (event: PointerEvent<HTMLDivElement>) => {
    const current = active.current;
    if (!current) return;
    active.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId);
    if (!current.moved) return;
    if (!selectedId) {
      setPending(false);
      return;
    }
    const at = pointerAt(event);
    if (!at) {
      setPending(false);
      return;
    }
    save(selectedId, applyAdjustGesture(current.gesture, at.mm));
  };

  const hint = selected
    ? adjustment
      ? "Glisser un point bleu : retoucher (ou le cliquer puis flèches) · glisser le dessin ou flèches : le décaler · ⇧ : pas plus grand"
      : "Glisser le dessin ou flèches (⇧ : pas plus grand) pour le décaler dans cette diapositive, ou choisir une grille"
    : drawings.length > 0
      ? "Cliquer un dessin pour le retoucher dans cette diapositive"
      : "";

  return { selected, adjustment, layers, hint, pending, setGrid, rotateBy, onPointerDown, onPointerMove, onPointerUp };
}

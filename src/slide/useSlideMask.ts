// Masque général de la diapositive (D-3.6-h) : tracer ses formes sur la
// vue Diapositive, au lasso (glisser) ou à la plume (un clic par point ;
// clic sur le premier point, double-clic ou Entrée pour fermer ; ⌫ retire
// le dernier point ; Échap abandonne). Chaque forme fermée, et chaque
// réglage, est enregistré par le moteur (une étape d'annulation).

import { useEffect, useMemo, useRef, useState, type PointerEvent, type RefObject } from "react";
import { saveSlideMask, type Project, type SlideMask } from "../engine";
import type { Point } from "../compose/placement";
import { setViewerCursor, type LayerSpec, type ViewerHandle } from "../viewer";
import { mmToScreen, screenToMm } from "../viewer/geometry";
import { addShape, closesOnFirst, maskLayers, removeShape, type MaskPen } from "./slideMask";

interface Options {
  engine: boolean;
  folder: string | null;
  projector: string | null;
  mask: SlideMask | null;
  /** Le carré de la diapositive (mm), pour noircir l'extérieur. */
  square: Point[];
  viewer: RefObject<ViewerHandle | null>;
  stage: RefObject<HTMLDivElement | null>;
  onProject: (project: Project) => void;
  onError: (message: string | null) => void;
}

export interface SlideMaskTool {
  mask: SlideMask | null;
  /** Outil de tracé pris (null : aucun). */
  pen: MaskPen | null;
  setPen(pen: MaskPen | null): void;
  /** Forme choisie dans la liste (contour en bleu). */
  selected: number | null;
  setSelected(index: number | null): void;
  save(next: SlideMask | null): void;
  toggleHidden(): void;
  setOutside(outside: boolean): void;
  remove(index: number): void;
  layers: LayerSpec[];
  hint: string | null;
  onPointerDown(event: PointerEvent<HTMLDivElement>): boolean;
  onPointerMove(event: PointerEvent<HTMLDivElement>): boolean;
  onPointerUp(event: PointerEvent<HTMLDivElement>): boolean;
  shortcuts: { keys: string; run: () => void; enabled?: boolean }[];
}

export function useSlideMask({ engine, folder, projector, mask: saved, square, viewer, stage, onProject, onError }: Options): SlideMaskTool {
  // Sans moteur (navigateur seul), le masque reste en mémoire, pour essayer l'outil.
  const [memory, setMemory] = useState<Map<string, SlideMask | null>>(new Map());
  const mask = engine ? saved : (projector ? memory.get(projector) ?? null : null);
  const [pen, setPenState] = useState<MaskPen | null>(null);
  const [points, setPoints] = useState<Point[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const dragging = useRef(false);
  const lastClick = useRef(0);

  useEffect(() => {
    setPenState(null);
    setPoints([]);
    setSelected(null);
  }, [projector]);

  const setPen = (next: MaskPen | null) => {
    setPoints([]);
    setPenState(next);
  };

  useEffect(() => {
    if (!pen) return;
    setViewerCursor(stage.current, "crosshair");
    return () => setViewerCursor(stage.current, "");
  }, [pen, stage]);

  const save = (next: SlideMask | null) => {
    if (!projector) return;
    if (!engine) {
      setMemory((m) => new Map(m).set(projector, next));
      return;
    }
    if (!folder) return;
    saveSlideMask(folder, projector, next).then(onProject, (failure: unknown) => onError(String(failure).replace(/^Error: /, "")));
  };

  const close = (shape: Point[]) => {
    const next = addShape(mask, shape);
    setPoints([]);
    if (next) save(next);
  };

  const geometry = () => {
    const scene = viewer.current?.scene();
    const box = stage.current?.querySelector(".viewer-canvas")?.getBoundingClientRect();
    if (!scene || !box) return null;
    const view = scene.getView();
    const viewport = scene.getViewport();
    return {
      toMm: (e: { clientX: number; clientY: number }) => screenToMm(view, viewport, { x: e.clientX - box.left, y: e.clientY - box.top }),
      toScreen: (p: Point) => mmToScreen(view, viewport, p),
    };
  };

  const onPointerDown = (event: PointerEvent<HTMLDivElement>): boolean => {
    if (!pen || event.button !== 0) return false;
    const g = geometry();
    if (!g) return false;
    event.stopPropagation();
    event.preventDefault();
    const at = g.toMm(event);
    if (pen === "lasso") {
      dragging.current = true;
      (event.target as Element).setPointerCapture?.(event.pointerId);
      setPoints([at]);
      return true;
    }
    // Plume : double-clic ou clic sur le premier point ferme la forme.
    const now = event.timeStamp;
    const here = g.toScreen(at);
    const previous = points.length > 0 ? g.toScreen(points[points.length - 1]) : null;
    const double = now - lastClick.current < 350 && previous !== null && Math.hypot(here.x - previous.x, here.y - previous.y) < 6;
    lastClick.current = now;
    if (double && points.length >= 3) {
      close(points);
      return true;
    }
    if (closesOnFirst(points, at, g.toScreen)) {
      close(points);
      return true;
    }
    setPoints([...points, at]);
    return true;
  };

  const onPointerMove = (event: PointerEvent<HTMLDivElement>): boolean => {
    if (!pen) return false;
    if (pen === "lasso" && dragging.current) {
      const g = geometry();
      if (!g) return true;
      const at = g.toMm(event);
      setPoints((current) => {
        const last = current[current.length - 1];
        if (last) {
          const a = g.toScreen(last);
          const b = g.toScreen(at);
          if (Math.hypot(a.x - b.x, a.y - b.y) < 3) return current;
        }
        return [...current, at];
      });
      event.stopPropagation();
    }
    return true;
  };

  const onPointerUp = (event: PointerEvent<HTMLDivElement>): boolean => {
    if (!pen) return false;
    event.stopPropagation();
    if (pen === "lasso" && dragging.current) {
      dragging.current = false;
      close(points);
    }
    return true;
  };

  const layers = useMemo(() => maskLayers(mask, square, points, selected), [mask, square, points, selected]);

  const hint = !pen
    ? null
    : pen === "lasso"
      ? "Masque de la diapositive : entourer une zone en glissant ; Échap : terminer"
      : points.length === 0
        ? "Masque de la diapositive, plume : cliquer les points de la forme ; Échap : terminer"
        : "Plume : cliquer le premier point, double-cliquer ou Entrée pour fermer ; ⌫ : retirer le dernier point ; Échap : abandonner";

  const shortcuts = [
    { keys: "escape", run: () => (points.length > 0 ? setPoints([]) : setPen(null)), enabled: pen !== null },
    { keys: "enter", run: () => close(points), enabled: pen === "pen" && points.length >= 3 },
    { keys: "backspace", run: () => setPoints(points.slice(0, -1)), enabled: pen === "pen" && points.length > 0 },
    {
      keys: "backspace",
      run: () => {
        if (mask && selected !== null) save(removeShape(mask, selected));
        setSelected(null);
      },
      enabled: pen === null && selected !== null && !!mask,
    },
  ];

  return {
    mask,
    pen,
    setPen,
    selected,
    setSelected,
    save,
    toggleHidden: () => mask && save({ ...mask, hidden: !mask.hidden }),
    setOutside: (outside) => save({ ...(mask ?? { shapes: [] }), outside }),
    remove: (index) => {
      if (mask) save(removeShape(mask, index));
      setSelected(null);
    },
    layers,
    hint,
    onPointerDown,
    onPointerMove,
    onPointerUp,
    shortcuts,
  };
}

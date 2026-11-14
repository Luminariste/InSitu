// Outil « Masque » (demande de Benjamin du 14 novembre, D-4.2-u), le même en
// Façade et en Diapositive : avec un dessin choisi, on peint pour le
// cacher ; ⌥ + pinceau fait réapparaître ; le lasso cache d'un coup la zone
// entourée (⌥ : la fait réapparaître). Le geste se dessine par-dessus la
// visionneuse pendant qu'on le trace ; au lâcher, il est ramené dans chaque
// dessin choisi (`strokeFor`) et enregistré en une seule étape d'annulation.

import { useCallback, useEffect, useRef, useState, type PointerEvent, type ReactNode, type RefObject } from "react";
import { setViewerCursor, type ViewerHandle } from "../viewer";
import { mmToScreen, screenToMm } from "../viewer/geometry";
import { DEFAULT_BRUSH_PX, nextBrush, strokeFor, type MaskShape, type MaskStroke } from "./mask";
import type { Point } from "./placement";

/** Un dessin choisi que l'outil peut masquer : la chaîne qui le pose (fractions → mm de la vue) et son masque. */
export interface MaskTarget {
  id: string;
  map: (u: number, v: number) => Point;
  mask: MaskStroke[];
}

interface Options {
  viewer: RefObject<ViewerHandle | null>;
  stage: RefObject<HTMLDivElement | null>;
  /** Faux : l'outil ne peut pas servir (écran occupé, pas de composition). */
  available: boolean;
  targets: MaskTarget[];
  onCommit: (masks: Map<string, MaskStroke[]>) => void;
}

export interface MaskTool {
  on: boolean;
  setOn(on: boolean): void;
  shape: MaskShape;
  setShape(shape: MaskShape): void;
  brushPx: number;
  setBrushPx(px: number): void;
  /** Au moins un dessin choisi a un masque. */
  masked: boolean;
  /** Tout réafficher sur les dessins choisis. */
  clear(): void;
  /** Vrai si le geste est pris par l'outil. */
  onPointerDown(event: PointerEvent<HTMLDivElement>): boolean;
  onPointerMove(event: PointerEvent<HTMLDivElement>): boolean;
  onPointerUp(event: PointerEvent<HTMLDivElement>): boolean;
  /** Le tracé en cours, à poser par-dessus la visionneuse. */
  overlay: ReactNode;
  hint: string | null;
  shortcuts: { keys: string; run: () => void; enabled?: boolean }[];
}

const HIDE = "rgba(255, 69, 58, 0.45)";
const SHOW = "rgba(48, 209, 88, 0.45)";

export function useMaskTool({ viewer, stage, available, targets, onCommit }: Options): MaskTool {
  const [on, setOnState] = useState(false);
  const [shape, setShape] = useState<MaskShape>("brush");
  const [brushPx, setBrushPx] = useState(DEFAULT_BRUSH_PX);
  const canvas = useRef<HTMLCanvasElement | null>(null);
  const stroke = useRef<{ points: Point[]; reveal: boolean } | null>(null);
  const hover = useRef<{ x: number; y: number; reveal: boolean } | null>(null);
  const active = on && available;
  const ready = active && targets.length > 0;

  const setOn = useCallback((next: boolean) => {
    stroke.current = null;
    setOnState(next);
  }, []);
  useEffect(() => {
    if (!available) setOnState(false);
  }, [available]);

  const geometry = () => {
    const scene = viewer.current?.scene();
    const box = stage.current?.querySelector(".viewer-canvas")?.getBoundingClientRect();
    if (!scene || !box) return null;
    const view = scene.getView();
    const viewport = scene.getViewport();
    return {
      box,
      toMm: (e: { clientX: number; clientY: number }) => screenToMm(view, viewport, { x: e.clientX - box.left, y: e.clientY - box.top }),
      toScreen: (p: Point) => mmToScreen(view, viewport, p),
      pxPerMm: () => {
        const a = mmToScreen(view, viewport, { x: 0, y: 0 });
        const b = mmToScreen(view, viewport, { x: 1, y: 0 });
        return Math.hypot(b.x - a.x, b.y - a.y) || 1;
      },
    };
  };

  const draw = () => {
    const c = canvas.current;
    const g = geometry();
    if (!c) return;
    const ratio = window.devicePixelRatio || 1;
    const width = c.clientWidth;
    const height = c.clientHeight;
    if (c.width !== Math.round(width * ratio) || c.height !== Math.round(height * ratio)) {
      c.width = Math.round(width * ratio);
      c.height = Math.round(height * ratio);
    }
    const ctx = c.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    ctx.clearRect(0, 0, width, height);
    if (!g || !ready) return;
    const s = stroke.current;
    if (s && s.points.length > 0) {
      const pts = s.points.map(g.toScreen);
      ctx.fillStyle = s.reveal ? SHOW : HIDE;
      ctx.strokeStyle = s.reveal ? SHOW : HIDE;
      if (shape === "lasso") {
        ctx.beginPath();
        pts.forEach((p, i) => (i === 0 ? ctx.moveTo(p.x, p.y) : ctx.lineTo(p.x, p.y)));
        ctx.closePath();
        ctx.fill("evenodd");
        ctx.setLineDash([4, 4]);
        ctx.lineWidth = 1;
        ctx.strokeStyle = "#fff";
        ctx.stroke();
        ctx.setLineDash([]);
      } else {
        ctx.lineCap = "round";
        ctx.lineJoin = "round";
        ctx.lineWidth = brushPx;
        ctx.beginPath();
        pts.forEach((p, i) => (i === 0 ? ctx.moveTo(p.x, p.y) : ctx.lineTo(p.x, p.y)));
        if (pts.length === 1) ctx.lineTo(pts[0].x + 0.01, pts[0].y);
        ctx.stroke();
      }
    }
    // Le pinceau sous la souris : un cercle de son épaisseur.
    const h = hover.current;
    if (h && shape === "brush") {
      ctx.lineWidth = 1;
      ctx.strokeStyle = "rgba(0, 0, 0, 0.8)";
      ctx.beginPath();
      ctx.arc(h.x, h.y, brushPx / 2 + 1, 0, 2 * Math.PI);
      ctx.stroke();
      ctx.strokeStyle = h.reveal ? "#30d158" : "#fff";
      ctx.beginPath();
      ctx.arc(h.x, h.y, brushPx / 2, 0, 2 * Math.PI);
      ctx.stroke();
    }
  };

  // Le curseur de la visionneuse : aucun (on voit le cercle) au pinceau, une croix au lasso.
  useEffect(() => {
    if (!ready) {
      hover.current = null;
      draw();
      return;
    }
    setViewerCursor(stage.current, shape === "brush" ? "none" : "crosshair");
    draw();
    return () => setViewerCursor(stage.current, "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, shape, brushPx]);

  const onPointerDown = (event: PointerEvent<HTMLDivElement>): boolean => {
    if (!ready || event.button !== 0) return false;
    const g = geometry();
    if (!g) return false;
    stroke.current = { points: [g.toMm(event)], reveal: event.altKey };
    (event.target as Element).setPointerCapture?.(event.pointerId);
    event.preventDefault();
    event.stopPropagation();
    draw();
    return true;
  };

  const onPointerMove = (event: PointerEvent<HTMLDivElement>): boolean => {
    if (!ready) return false;
    const g = geometry();
    if (!g) return false;
    hover.current = { x: event.clientX - g.box.left, y: event.clientY - g.box.top, reveal: event.altKey };
    const s = stroke.current;
    if (s) {
      const p = g.toMm(event);
      const last = g.toScreen(s.points[s.points.length - 1]);
      const here = g.toScreen(p);
      if (Math.hypot(here.x - last.x, here.y - last.y) >= 2) s.points.push(p);
      event.stopPropagation();
    }
    draw();
    return s !== null || ready;
  };

  const onPointerUp = (event: PointerEvent<HTMLDivElement>): boolean => {
    const s = stroke.current;
    if (!s) return false;
    stroke.current = null;
    event.stopPropagation();
    const g = geometry();
    if (g) {
      const widthMm = brushPx / g.pxPerMm();
      const masks = new Map<string, MaskStroke[]>();
      for (const t of targets) {
        const next = strokeFor(t.map, s.points, shape, s.reveal, widthMm);
        if (next) masks.set(t.id, [...t.mask, next]);
      }
      if (masks.size > 0) onCommit(masks);
    }
    draw();
    return true;
  };

  const masked = targets.some((t) => t.mask.length > 0);
  const clear = () => {
    const masks = new Map(targets.filter((t) => t.mask.length > 0).map((t) => [t.id, [] as MaskStroke[]]));
    if (masks.size > 0) onCommit(masks);
  };

  const overlay = active ? <canvas ref={canvas} className="compose-mask-overlay" aria-hidden="true" /> : null;
  const hint = !active
    ? null
    : targets.length === 0
      ? "Masque : choisir d'abord un dessin (M ou Échap pour quitter l'outil)"
      : shape === "brush"
        ? "Masque : peindre pour cacher, ⌥ + peindre pour faire réapparaître ; [ et ] : épaisseur ; Z : lasso ; Échap : quitter"
        : "Masque : entourer une zone pour la cacher, ⌥ pour la faire réapparaître ; T : pinceau ; Échap : quitter";

  const shortcuts = [
    { keys: "m", run: () => setOn(!on), enabled: available },
    { keys: "escape", run: () => setOn(false), enabled: active },
    { keys: "t", run: () => setShape("brush"), enabled: active },
    { keys: "z", run: () => setShape("lasso"), enabled: active },
    { keys: "[", run: () => setBrushPx(nextBrush(brushPx, -1)), enabled: active },
    { keys: "]", run: () => setBrushPx(nextBrush(brushPx, 1)), enabled: active },
  ];

  return {
    on: active,
    setOn,
    shape,
    setShape,
    brushPx,
    setBrushPx,
    masked,
    clear,
    onPointerDown,
    onPointerMove,
    onPointerUp,
    overlay,
    hint,
    shortcuts,
  };
}

// Gomme et crayon sur un dessin de la bibliothèque (tâche 4.8), en plein
// écran comme la retouche de silhouette. La gomme efface des traits du
// dessin d'origine, le crayon en ajoute ; chacun trace un trait de
// l'épaisseur choisie, ou remplit une zone qu'on entoure. Chaque retouche
// s'enregistre seule, peu après le geste ; ⌘Z la retire, « Revenir au
// dessin d'origine » les retire toutes. Le fichier importé n'est jamais
// modifié : le moteur fabrique à part le dessin retouché, que l'app lit
// partout ensuite (vignette, façade, vecteurs, silhouette, export).

import { useCallback, useEffect, useLayoutEffect, useRef, useState, type PointerEvent, type WheelEvent } from "react";
import type { RetouchStroke } from "../engine";
import {
  Button,
  IconButton,
  Panel,
  Property,
  Section,
  Segmented,
  ToolGroup,
  ToolSeparator,
  ToolSpacer,
  Toolbar,
  useShortcuts,
} from "../ui";
import { classifyWheel, wheelZoomFactor } from "../viewer/geometry";
import {
  addPoint,
  clearStrokes,
  defaultSize,
  finishStroke,
  nextSize,
  paintStrokes,
  pushStroke,
  redoStroke,
  retouchLine,
  sizeSteps,
  startHistory,
  undoStroke,
  type RetouchHistory,
  type RetouchShape,
  type RetouchTool,
} from "./retouchLogic";
import { fitView, scaleLabel, zoomAt, type View } from "./vectorLogic";
import { BACKDROPS, type Backdrop } from "./backdrop";

const TOOLS: { value: RetouchTool; label: string; title: string; shortcut: string }[] = [
  { value: "erase", label: "Gomme", title: "Effacer des traits du dessin", shortcut: "e" },
  { value: "draw", label: "Crayon", title: "Ajouter des traits ou des zones au dessin", shortcut: "n" },
];

const SHAPES: { value: RetouchShape; label: string; title: string; shortcut: string }[] = [
  { value: "trait", label: "Trait", title: "Peindre le long du geste, à l'épaisseur choisie", shortcut: "t" },
  { value: "zone", label: "Zone", title: "Entourer une zone : elle est remplie au lâcher", shortcut: "z" },
];

/** Écart minimal entre deux points enregistrés d'un tracé, en pixels d'écran. */
const MIN_SCREEN_PX = 1.5;

interface Props {
  name: string;
  /** Aperçu du dessin sans ses retouches. */
  originalUrl: string | null;
  /** Taille du dessin, dans son unité. */
  width: number;
  height: number;
  /** Retouches enregistrées à l'ouverture. */
  strokes: RetouchStroke[];
  backdrop: Backdrop;
  /** Change le fond (le même que celui de la bibliothèque). */
  onBackdrop: (backdrop: Backdrop) => void;
  /** Faux dans le navigateur seul : rien ne s'enregistre. */
  editable: boolean;
  saving: boolean;
  error: string | null;
  /** Appelé après chaque retouche, annulation ou retour à l'origine. */
  onChange: (strokes: RetouchStroke[]) => void;
  onClose: () => void;
}

export function RetouchEditor(props: Props) {
  const { name, originalUrl, width, height, backdrop, onBackdrop, editable, saving, error, onChange, onClose } = props;
  const size = { width, height };
  const [tool, setTool] = useState<RetouchTool>("erase");
  const [shape, setShape] = useState<RetouchShape>("trait");
  const [brush, setBrush] = useState(() => defaultSize(width, height));
  const [history, setHistory] = useState<RetouchHistory>(() => startHistory(props.strokes));
  const [view, setView] = useState<View | null>(null);
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [pointer, setPointer] = useState<{ x: number; y: number } | null>(null);
  const [spaceDown, setSpaceDown] = useState(false);
  const stage = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const drag = useRef<
    | { kind: "pan"; x: number; y: number; view: View }
    | { kind: "paint"; points: [number, number][] }
    | null
  >(null);
  const frame = useRef(0);

  const fit = useCallback(() => {
    const box = stage.current;
    if (!box) return;
    setView(fitView(width, height, box.clientWidth, box.clientHeight, 32));
  }, [width, height]);

  useLayoutEffect(() => {
    fit();
  }, [fit]);

  // L'aperçu d'origine, chargé une fois : le canevas y peint les retouches.
  useEffect(() => {
    setImage(null);
    if (!originalUrl) return;
    const img = new Image();
    img.onload = () => setImage(img);
    img.src = originalUrl;
  }, [originalUrl]);

  /** Redessine le dessin et ses retouches, plus le tracé en cours. */
  const render = useCallback(() => {
    const target = canvas.current;
    if (!target || !image) return;
    const w = image.naturalWidth;
    const h = image.naturalHeight;
    if (target.width !== w || target.height !== h) {
      target.width = w;
      target.height = h;
    }
    const context = target.getContext("2d");
    if (!context) return;
    context.clearRect(0, 0, w, h);
    context.drawImage(image, 0, 0);
    // Épaisseurs en fractions : le même rendu que le moteur, à la taille
    // de l'aperçu.
    paintStrokes(context, history.strokes, w, h);
    const current = drag.current;
    if (current?.kind === "paint" && current.points.length > 0) {
      if (shape === "zone") {
        context.save();
        context.beginPath();
        current.points.forEach(([u, v], i) => (i === 0 ? context.moveTo(u * w, v * h) : context.lineTo(u * w, v * h)));
        context.closePath();
        context.fillStyle = tool === "erase" ? "rgba(255, 69, 58, 0.25)" : "rgba(255, 255, 255, 0.35)";
        context.fill("evenodd");
        context.setLineDash([6, 4]);
        context.lineWidth = Math.max(1, w / 400);
        context.strokeStyle = tool === "erase" ? "#ff453a" : "#ffffff";
        context.stroke();
        context.restore();
      } else {
        const live = finishStroke(tool, "trait", brush, current.points, size);
        if (live) paintStrokes(context, [live], w, h);
      }
    }
    // `size` suit `width` et `height`.
  }, [image, history.strokes, tool, shape, brush, width, height]);

  useEffect(() => {
    render();
  }, [render]);

  const scheduleRender = () => {
    cancelAnimationFrame(frame.current);
    frame.current = requestAnimationFrame(render);
  };

  const commit = (next: RetouchHistory) => {
    setHistory(next);
    onChange(next.strokes);
  };

  const undo = () => history.strokes.length > 0 && commit(undoStroke(history));
  const redo = () => history.undone.length > 0 && commit(redoStroke(history));

  useShortcuts([
    { keys: "mod+z", run: undo, enabled: editable && history.strokes.length > 0 },
    { keys: "mod+shift+z", run: redo, enabled: editable && history.undone.length > 0 },
    { keys: "e", run: () => setTool("erase") },
    { keys: "n", run: () => setTool("draw") },
    { keys: "t", run: () => setShape("trait") },
    { keys: "z", run: () => setShape("zone") },
    { keys: "[", run: () => setBrush((b) => nextSize(b, -1, width, height)) },
    { keys: "]", run: () => setBrush((b) => nextSize(b, 1, width, height)) },
    { keys: "mod+0", run: fit },
  ]);

  useEffect(() => {
    const onKey = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") {
        // Échap abandonne le tracé en cours ; sinon il ferme.
        if (drag.current?.kind === "paint") {
          drag.current = null;
          render();
        } else onClose();
      }
      if (event.key === " " && !event.repeat) {
        setSpaceDown(true);
        event.preventDefault();
      }
    };
    const onKeyUp = (event: globalThis.KeyboardEvent) => {
      if (event.key === " ") setSpaceDown(false);
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("keyup", onKeyUp);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("keyup", onKeyUp);
    };
  }, [onClose, render]);

  /** Position du pointeur en fractions du dessin. */
  const toDrawing = (clientX: number, clientY: number): [number, number] | null => {
    const box = stage.current;
    if (!box || !view) return null;
    const rect = box.getBoundingClientRect();
    return [
      (clientX - rect.left - view.x) / (view.scale * width),
      (clientY - rect.top - view.y) / (view.scale * height),
    ];
  };

  const minScale = () => {
    const box = stage.current;
    if (!box) return 1e-3;
    return Math.min(1, fitView(width, height, box.clientWidth, box.clientHeight).scale / 2);
  };

  const zoomBy = (factor: number) => {
    const box = stage.current;
    if (!box || !view) return;
    setView(zoomAt(view, factor, box.clientWidth / 2, box.clientHeight / 2, minScale()));
  };

  const onWheel = (event: WheelEvent<HTMLDivElement>) => {
    const box = stage.current;
    if (!box || !view) return;
    const input = {
      deltaX: event.deltaX,
      deltaY: event.deltaY,
      deltaMode: event.deltaMode,
      ctrlKey: event.ctrlKey,
      metaKey: event.metaKey,
      wheelDeltaY: (event.nativeEvent as unknown as { wheelDeltaY?: number }).wheelDeltaY,
    };
    const intent = classifyWheel(input);
    if (intent === "pan") {
      setView({ ...view, x: view.x - event.deltaX, y: view.y - event.deltaY });
      return;
    }
    const rect = box.getBoundingClientRect();
    setView(zoomAt(view, wheelZoomFactor(input, intent), event.clientX - rect.left, event.clientY - rect.top, minScale()));
  };

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (!view) return;
    // Espace + glisser, ou bouton du milieu : déplacer la vue.
    if (event.button === 1 || (event.button === 0 && (spaceDown || !editable || !image))) {
      drag.current = { kind: "pan", x: event.clientX, y: event.clientY, view };
      event.currentTarget.setPointerCapture(event.pointerId);
      return;
    }
    if (event.button !== 0) return;
    const at = toDrawing(event.clientX, event.clientY);
    if (!at) return;
    drag.current = { kind: "paint", points: [at] };
    event.currentTarget.setPointerCapture(event.pointerId);
    scheduleRender();
  };

  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const box = stage.current;
    if (box) {
      const rect = box.getBoundingClientRect();
      setPointer({ x: event.clientX - rect.left, y: event.clientY - rect.top });
    }
    const current = drag.current;
    if (!current) return;
    if (current.kind === "pan") {
      setView({ ...current.view, x: current.view.x + event.clientX - current.x, y: current.view.y + event.clientY - current.y });
      return;
    }
    const at = toDrawing(event.clientX, event.clientY);
    if (!at || !view) return;
    // Points espacés d'environ un pixel et demi d'écran, au plus fin d'un
    // pixel du dessin.
    const minPx = Math.max(MIN_SCREEN_PX / view.scale, 0.5);
    current.points = addPoint(current.points, at, size, minPx);
    scheduleRender();
  };

  const onPointerUp = () => {
    const current = drag.current;
    drag.current = null;
    if (current?.kind !== "paint") return;
    const stroke = finishStroke(tool, shape, brush, current.points, size);
    if (stroke) commit(pushStroke(history, stroke));
    else render();
  };

  const layer = view
    ? { left: view.x, top: view.y, width: width * view.scale, height: height * view.scale }
    : { display: "none" as const };
  const panning = spaceDown || drag.current?.kind === "pan";
  const cursorDiameter = view ? brush * view.scale : 0;
  const steps = sizeSteps(width, height);
  const stepIndex = Math.max(0, steps.indexOf(brush));

  return (
    <div className="vc" role="dialog" aria-label={`Gomme et crayon : ${name}`}>
      <Toolbar label="Gomme et crayon">
        <strong className="vc-title" title={name}>
          {name}
        </strong>
        <ToolSeparator />
        <Segmented label="Outil" value={tool} onChange={setTool} options={TOOLS} />
        <Segmented label="Forme" value={shape} onChange={setShape} options={SHAPES} />
        <ToolSeparator />
        <ToolGroup label="Retouches">
          <IconButton icon="undo" label="Annuler la dernière retouche" shortcut="mod+z" onClick={undo} disabled={!editable || history.strokes.length === 0} />
          <IconButton icon="redo" label="Rétablir" shortcut="mod+shift+z" onClick={redo} disabled={!editable || history.undone.length === 0} />
        </ToolGroup>
        <ToolSpacer />
        <ToolGroup label="Fond">
          <Segmented label="Fond derrière le dessin" value={backdrop} onChange={onBackdrop} options={BACKDROPS} />
        </ToolGroup>
        <ToolSeparator />
        <ToolGroup label="Zoom">
          <IconButton icon="minus" label="Zoom arrière" onClick={() => zoomBy(1 / 1.5)} />
          <span className="vc-scale">{view ? scaleLabel(view.scale) : "—"}</span>
          <IconButton icon="plus" label="Zoom avant" onClick={() => zoomBy(1.5)} />
          <Button icon="fit" shortcut="mod+0" onClick={fit}>
            Ajuster
          </Button>
        </ToolGroup>
        <ToolSeparator />
        <Button icon="close" onClick={onClose} title="Revenir à la bibliothèque (Échap)">
          Fermer
        </Button>
      </Toolbar>

      <div className="vc-body">
        <div
          className={`vc-stage retouche-stage fond-${backdrop}${panning ? " retouche-pan" : ""}`}
          ref={stage}
          onWheel={onWheel}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={onPointerUp}
          onPointerLeave={() => setPointer(null)}
        >
          <div className="vc-layer" style={layer}>
            <canvas ref={canvas} className="retouche-canvas" />
          </div>
          {pointer && !panning && shape === "trait" && editable && image && (
            <div
              className={tool === "erase" ? "retouche-cursor gomme" : "retouche-cursor"}
              style={{
                left: pointer.x,
                top: pointer.y,
                width: Math.max(4, cursorDiameter),
                height: Math.max(4, cursorDiameter),
              }}
            />
          )}
          {!image && <p className="vc-wait">Préparation de l'aperçu…</p>}
          {saving && <p className="vc-busy">Enregistrement…</p>}
        </div>

        <Panel side="right" title="Gomme et crayon">
          <Section>
            <p className="vc-note">{retouchLine(history.strokes)}</p>
            <p className="vc-note">
              Le fichier importé n'est pas modifié. Le dessin retouché remplace l'original partout : vignette, façade,
              vecteurs, silhouette, diapositives et export.
            </p>
          </Section>
          <Section title="Épaisseur du trait">
            <Property label={`${brush.toLocaleString("fr-FR")} px`} hint="En pixels du dessin. Raccourcis : [ et ].">
              <input
                type="range"
                min={0}
                max={Math.max(0, steps.length - 1)}
                value={stepIndex}
                disabled={shape === "zone"}
                onChange={(event) => setBrush(steps[Number(event.target.value)] ?? brush)}
                aria-label="Épaisseur du trait"
              />
            </Property>
          </Section>
          <Section title="Retouches">
            <div className="vc-actions">
              <Button
                disabled={!editable || history.strokes.length === 0}
                onClick={() => commit(clearStrokes(history))}
                title="Retirer toutes les retouches (⌘Z les remet)"
              >
                Revenir au dessin d'origine
              </Button>
            </div>
            {error && <p className="vc-erreur">{error}</p>}
            {!editable && <p className="vc-note">Dans l'app seulement : ici, rien ne s'enregistre.</p>}
          </Section>
          <Section>
            <p className="vc-note">
              Gomme (E) ou crayon (N) ; trait (T) ou zone (Z). Glisser pour peindre ; en zone, entourer ce qu'il faut
              effacer ou remplir. ⌘Z : annuler la dernière retouche. Espace + glisser : déplacer la vue ; molette ou
              pincement : zoom.
            </p>
          </Section>
        </Panel>
      </div>
    </div>
  );
}

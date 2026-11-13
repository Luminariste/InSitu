// Retouche de la silhouette d'un dessin (tâche 4.7), en plein écran sur la
// bibliothèque. Le contour est posé sur le dessin ; on tire un point pour le
// déplacer, on double-clique sur un bord pour ajouter un point (ailleurs :
// un nouveau contour carré), on clique un point avec ⌥ pour le retirer.
// « Enregistrer » écrit la silhouette dans le projet ; « Détecter à
// nouveau » revient à la détection automatique du moteur.

import { useCallback, useEffect, useLayoutEffect, useRef, useState, type PointerEvent, type WheelEvent } from "react";
import type { DrawingSilhouette, SilhouettePolygon } from "../engine";
import { Button, IconButton, Panel, Section, Segmented, ToolGroup, ToolSeparator, ToolSpacer, Toolbar } from "../ui";
import { classifyWheel, wheelZoomFactor } from "../viewer/geometry";
import {
  edgeAt,
  insertVertex,
  moveVertex,
  polygonsPath,
  removeVertex,
  samePolygons,
  silhouetteLine,
  squareAt,
  vertexAt,
  type VertexRef,
} from "./silhouetteLogic";
import { fitView, scaleLabel, zoomAt, type View } from "./vectorLogic";
import { BACKDROPS, type Backdrop } from "./backdrop";

type Mode = "contour" | "resultat";

const MODES: { value: Mode; label: string; title: string }[] = [
  { value: "contour", label: "Contour", title: "Le contour de la silhouette, posé sur le dessin" },
  { value: "resultat", label: "Résultat", title: "La silhouette en noir sous le dessin : ce qu'elle cache" },
];

/** Rayon de prise d'un point ou d'un bord, en pixels d'écran. */
const HIT_PX = 8;

interface Props {
  name: string;
  imageUrl: string | null;
  /** Taille du dessin, dans son unité. */
  width: number;
  height: number;
  silhouette: DrawingSilhouette | null;
  backdrop: Backdrop;
  /** Change le fond (le même que celui de la bibliothèque). */
  onBackdrop: (backdrop: Backdrop) => void;
  /** Faux dans le navigateur seul : rien ne s'enregistre. */
  editable: boolean;
  busy: boolean;
  error: string | null;
  onSave: (polygons: SilhouettePolygon[] | null) => void;
  onClose: () => void;
}

export function SilhouetteEditor(props: Props) {
  const { name, imageUrl, width, height, silhouette, backdrop, onBackdrop, editable, busy, error, onSave, onClose } = props;
  const [mode, setMode] = useState<Mode>("contour");
  const [view, setView] = useState<View | null>(null);
  const [draft, setDraft] = useState<SilhouettePolygon[]>(silhouette?.polygons ?? []);
  const [active, setActive] = useState<VertexRef | null>(null);
  const stage = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number; view: View; vertex: VertexRef | null } | null>(null);
  const size = { width, height };

  const fit = useCallback(() => {
    const box = stage.current;
    if (!box) return;
    setView(fitView(width, height, box.clientWidth, box.clientHeight, 32));
  }, [width, height]);

  useLayoutEffect(() => {
    fit();
  }, [fit]);

  // Une silhouette neuve (enregistrée, détectée à nouveau) remplace le brouillon.
  useEffect(() => {
    setDraft(silhouette?.polygons ?? []);
    setActive(null);
  }, [silhouette]);

  useEffect(() => {
    const onKey = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

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
  const radius = () => HIT_PX / (view?.scale ?? 1);

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
    setView(
      zoomAt(view, wheelZoomFactor(input, intent), event.clientX - rect.left, event.clientY - rect.top, minScale()),
    );
  };

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (!view || event.button !== 0) return;
    const at = toDrawing(event.clientX, event.clientY);
    const vertex = at && editable && mode === "contour" ? vertexAt(draft, at, size, radius()) : null;
    if (vertex && event.altKey) {
      setDraft(removeVertex(draft, vertex));
      setActive(null);
      return;
    }
    setActive(vertex);
    drag.current = { x: event.clientX, y: event.clientY, view, vertex };
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const start = drag.current;
    if (!start) return;
    if (start.vertex) {
      const at = toDrawing(event.clientX, event.clientY);
      if (at) setDraft((current) => moveVertex(current, start.vertex as VertexRef, at));
      return;
    }
    setView({ ...start.view, x: start.view.x + event.clientX - start.x, y: start.view.y + event.clientY - start.y });
  };

  const onPointerUp = () => {
    drag.current = null;
  };

  const onDoubleClick = (event: React.MouseEvent<HTMLDivElement>) => {
    if (!editable || mode !== "contour") return;
    const at = toDrawing(event.clientX, event.clientY);
    if (!at || at[0] < 0 || at[0] > 1 || at[1] < 0 || at[1] > 1) return;
    const edge = edgeAt(draft, at, size, radius());
    if (edge) {
      const { polygons, added } = insertVertex(draft, edge, edge.point);
      setDraft(polygons);
      setActive(added);
    } else {
      setDraft([...draft, squareAt(at)]);
      setActive(null);
    }
  };

  const original = silhouette?.polygons ?? [];
  const changed = !samePolygons(draft, original);
  const layer = view
    ? { left: view.x, top: view.y, width: width * view.scale, height: height * view.scale }
    : { display: "none" as const };
  const handle = view ? 4 / view.scale : 4;
  // Un tracé par contour : des contours qui se chevauchent se réunissent,
  // comme dans le moteur.
  const paths = draft.map((polygon) => polygonsPath([polygon], size));

  return (
    <div className="vc" role="dialog" aria-label={`Retoucher la silhouette : ${name}`}>
      <Toolbar label="Silhouette">
        <strong className="vc-title" title={name}>
          {name}
        </strong>
        <ToolSeparator />
        <Segmented label="Affichage" value={mode} onChange={setMode} options={MODES} />
        <ToolSpacer />
        <ToolGroup label="Fond">
          <Segmented label="Fond derrière le dessin" value={backdrop} onChange={onBackdrop} options={BACKDROPS} />
        </ToolGroup>
        <ToolSeparator />
        <ToolGroup label="Zoom">
          <IconButton icon="minus" label="Zoom arrière" onClick={() => zoomBy(1 / 1.5)} />
          <span className="vc-scale">{view ? scaleLabel(view.scale) : "—"}</span>
          <IconButton icon="plus" label="Zoom avant" onClick={() => zoomBy(1.5)} />
          <Button icon="fit" onClick={fit}>
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
          className={`vc-stage fond-${backdrop}${mode === "resultat" ? " silhouette-resultat" : ""}`}
          ref={stage}
          onWheel={onWheel}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={onPointerUp}
          onDoubleClick={onDoubleClick}
        >
          <div className="vc-layer" style={layer}>
            {mode === "resultat" && (
              <svg className="silhouette-svg" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none">
                {paths.map((d, i) => (
                  <path key={i} d={d} fill="black" />
                ))}
              </svg>
            )}
            {imageUrl && <img src={imageUrl} alt="" draggable={false} />}
            {mode === "contour" && (
              <svg className="silhouette-svg" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none">
                {paths.map((d, i) => (
                  <path
                    key={i}
                    d={d}
                    fill="rgba(255, 69, 58, 0.18)"
                    stroke="#ff453a"
                    strokeWidth={1.5}
                    vectorEffect="non-scaling-stroke"
                    strokeLinejoin="round"
                  />
                ))}
                {editable &&
                  draft.map((polygon, p) =>
                    polygon.map(([u, v], i) => {
                      const on = active?.polygon === p && active.index === i;
                      return (
                        <rect
                          key={`${p}-${i}`}
                          x={u * width - handle}
                          y={v * height - handle}
                          width={2 * handle}
                          height={2 * handle}
                          fill={on ? "#ff453a" : "white"}
                          stroke="#ff453a"
                          strokeWidth={1}
                          vectorEffect="non-scaling-stroke"
                        />
                      );
                    }),
                  )}
              </svg>
            )}
          </div>
          {!imageUrl && <p className="vc-wait">Préparation de l'aperçu…</p>}
          {busy && <p className="vc-busy">Enregistrement…</p>}
        </div>

        <Panel side="right" title="Silhouette">
          <Section>
            <p className="vc-note">
              {silhouette ? silhouetteLine(original, silhouette.manual) : "Détection en cours…"}
              {changed && " · retouches non enregistrées"}
            </p>
            <p className="vc-note">
              La silhouette est posée en noir sous le dessin : elle cache, sur la façade et dans les diapositives, la
              fresque et les dessins placés derrière lui.
            </p>
          </Section>
          <Section title="Retoucher">
            <div className="vc-actions">
              <Button variant="primary" disabled={!editable || busy || !changed} onClick={() => onSave(draft)}>
                Enregistrer
              </Button>
              <Button disabled={busy || !changed} onClick={() => setDraft(original)}>
                Annuler les retouches
              </Button>
            </div>
            <div className="vc-actions">
              <Button
                disabled={!editable || busy || (silhouette !== null && !silhouette.manual)}
                onClick={() => onSave(null)}
                title="Oublier les retouches et reprendre le contour détecté par l'app"
              >
                Détecter à nouveau
              </Button>
              <Button
                disabled={!editable || busy || draft.length === 0}
                onClick={() => setDraft([])}
                title="Ce dessin ne cachera rien (à enregistrer ensuite)"
              >
                Aucune silhouette
              </Button>
            </div>
            {error && <p className="vc-erreur">{error}</p>}
            {!editable && <p className="vc-note">Dans l'app seulement : ici, la silhouette est un exemple.</p>}
          </Section>
          <Section>
            <p className="vc-note">
              Tirer un point pour le déplacer. Double-clic sur un bord : un point de plus ; ailleurs : un nouveau
              contour. ⌥-clic sur un point : le retirer. Molette ou pincement : zoom ; glisser le fond : déplacer.
            </p>
          </Section>
        </Panel>
      </div>
    </div>
  );
}

// Visionneuse haute résolution : composant React autonome.
//
//   <Viewer layers={[{ id, kind: "tiles", source, placement, opacity }]} />
//
// Gestes : glisser ou défiler à deux doigts pour se déplacer ; pincer, tourner
// la molette ou Cmd + défilement pour zoomer sur le curseur ; double-clic pour
// passer à 100 %.
//
// Le composant ne contient aucun calcul : il crée une `ViewerScene` et lui
// passe ses calques. La barre d'état est mise à jour directement, sans passer
// par React, pour ne rien coûter pendant un geste.

import { useEffect, useImperativeHandle, useRef, useState, type Ref } from "react";
import type { Point } from "./geometry";
import type { LayerSpec } from "./nodes";
import { ViewerScene, type FrameStats, type SceneOptions, type SceneStatus } from "./scene";
import "./viewer.css";

export interface ViewerHandle {
  /** Montre tous les calques visibles en entier. */
  fitAll(): void;
  /** Un pixel d'image pour un pixel d'écran. */
  zoomToActualSize(): void;
  /** La scène, pour y ajouter des objets ou piloter la vue ; `null` avant qu'elle soit prête. */
  scene(): ViewerScene | null;
}

export interface ViewerProps {
  /** Calques, du fond vers le dessus. */
  layers: LayerSpec[];
  ref?: Ref<ViewerHandle>;
  className?: string;
  options?: SceneOptions;
  /** Affiche dans la barre d'état la mesure de fluidité du dernier geste. */
  showFrameStats?: boolean;
  onStatus?: (status: SceneStatus) => void;
  onCursor?: (mm: Point | null) => void;
  onGestureEnd?: (stats: FrameStats) => void;
}

const decimal = new Intl.NumberFormat("fr-FR", { minimumFractionDigits: 1, maximumFractionDigits: 1 });
const precise = new Intl.NumberFormat("fr-FR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

function formatZoom(percent: number | null): string {
  if (percent === null) return "—";
  return `${percent < 10 ? decimal.format(percent) : Math.round(percent)} %`;
}

export function formatFrameStats(stats: FrameStats): string {
  const slow = stats.slowFrames === 0 ? "aucune saccade" : `${stats.slowFrames} en retard`;
  const perSecond = Math.round(1000 / stats.meanMs);
  return (
    `${stats.frames} images, ${perSecond} par seconde ` +
    `(${decimal.format(stats.meanMs)} ms en moyenne, ${decimal.format(stats.worstMs)} ms au pire), ` +
    `${slow}, calcul : ${decimal.format(stats.meanWorkMs)} ms`
  );
}

export function Viewer(props: ViewerProps) {
  const { layers, ref, className, options, showFrameStats } = props;
  const hostRef = useRef<HTMLDivElement>(null);
  const zoomRef = useRef<HTMLSpanElement>(null);
  const cursorRef = useRef<HTMLSpanElement>(null);
  const tilesRef = useRef<HTMLSpanElement>(null);
  const statsRef = useRef<HTMLSpanElement>(null);
  const sceneRef = useRef<ViewerScene | null>(null);
  const [scene, setScene] = useState<ViewerScene | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  // Toujours les derniers rappels, sans recréer la scène quand ils changent.
  const callbacks = useRef(props);
  callbacks.current = props;

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    let cancelled = false;
    let created: ViewerScene | null = null;
    ViewerScene.create(host, options).then(
      (instance) => {
        if (cancelled) {
          instance.destroy();
          return;
        }
        created = instance;
        sceneRef.current = instance;
        instance.setListeners({
          onFrame: (status) => {
            if (zoomRef.current) zoomRef.current.textContent = formatZoom(status.zoomPercent);
            if (tilesRef.current) {
              const { loading, failed } = status.tiles;
              tilesRef.current.textContent =
                failed > 0
                  ? `${failed} tuile${failed > 1 ? "s" : ""} illisible${failed > 1 ? "s" : ""}`
                  : loading > 0
                    ? `chargement de ${loading} tuile${loading > 1 ? "s" : ""}…`
                    : "";
            }
            callbacks.current.onStatus?.(status);
          },
          onCursor: (mm) => {
            if (cursorRef.current) {
              cursorRef.current.textContent = mm
                ? `x ${precise.format(mm.x)} mm · y ${precise.format(mm.y)} mm`
                : "";
            }
            callbacks.current.onCursor?.(mm);
          },
          onGestureEnd: (stats) => {
            if (statsRef.current) statsRef.current.textContent = formatFrameStats(stats);
            callbacks.current.onGestureEnd?.(stats);
          },
        });
        setScene(instance);
      },
      (error: unknown) => {
        if (!cancelled) setFailure(String(error));
      },
    );
    return () => {
      cancelled = true;
      sceneRef.current = null;
      setScene(null);
      created?.destroy();
    };
    // Les options ne sont lues qu'à la création.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    scene?.setLayers(layers);
  }, [scene, layers]);

  useImperativeHandle(
    ref,
    () => ({
      fitAll: () => sceneRef.current?.fitAll(),
      zoomToActualSize: () => sceneRef.current?.zoomToActualSize(),
      scene: () => sceneRef.current,
    }),
    [],
  );

  return (
    <div className={`viewer${className ? ` ${className}` : ""}`}>
      <div className="viewer-canvas" ref={hostRef} />
      {failure && <p className="viewer-failure">L'affichage n'a pas pu démarrer : {failure}</p>}
      <div className="viewer-status">
        <span className="viewer-zoom" ref={zoomRef} title="Zoom, en proportion de la taille réelle" />
        <span className="viewer-cursor" ref={cursorRef} />
        <span className="viewer-tiles" ref={tilesRef} />
        {showFrameStats && (
          <span className="viewer-stats" title="Fluidité du dernier geste">
            Dernier geste : <span ref={statsRef}>—</span>
          </span>
        )}
      </div>
    </div>
  );
}

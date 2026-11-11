// Écran de démonstration de la visionneuse (tâche 1.5) : pour un projecteur
// d'Épinal, la photo redressée et le Verre superposés sur les mêmes 280 mm.
//
// D'où viennent les tuiles :
// - dans l'application, le moteur les fabrique au premier affichage à partir
//   du jeu d'essai du dépôt, et les garde dans le cache de l'app ;
// - sinon (navigateur seul, ou moteur en échec), elles sont lues dans le
//   dossier `tuiles-demo/`, rempli à la main par `insitu tiles`.

import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  fetchEngineInfo,
  fetchTestSetDir,
  hasEngine,
  openTiledImage,
  openTileFolder,
} from "./engine";
import { placementForWidth, Viewer, type LayerSpec, type TileSource, type ViewerHandle } from "./viewer";

const PROJECTORS = ["a", "b", "c", "d"] as const;
type Projector = (typeof PROJECTORS)[number];

/**
 * Côté d'une diapositive quand l'interface tourne sans moteur (navigateur
 * seul). Dans l'application, la valeur vient du moteur.
 */
const SLIDE_SIZE_MM_WITHOUT_ENGINE = 280;

interface DemoSources {
  rectifiedPhoto: TileSource;
  engraving: TileSource;
  slideSizeMm: number;
  /** Temps de préparation, en secondes. */
  seconds: number;
  origin: "moteur" | "dossier tuiles-demo";
}

type DemoState =
  | { status: "loading" }
  | { status: "ready"; sources: DemoSources }
  | { status: "missing"; detail: string | null };

const photoFile = (p: Projector) => `epinal/references/photos-redressees/${p}.jpg`;
const engravingFile = (p: Projector) => `epinal/references/verres/verre-${p}.png`;

async function loadSources(projector: Projector): Promise<DemoState> {
  const started = performance.now();
  const seconds = () => (performance.now() - started) / 1000;
  let detail: string | null = null;

  if (hasEngine()) {
    try {
      const [info, dir] = await Promise.all([fetchEngineInfo(), fetchTestSetDir()]);
      if (!info || !dir) throw new Error("le dossier jeu-essai du dépôt est introuvable");
      const [rectifiedPhoto, engraving] = await Promise.all([
        openTiledImage(`${dir}/${photoFile(projector)}`),
        openTiledImage(`${dir}/${engravingFile(projector)}`),
      ]);
      return {
        status: "ready",
        sources: { rectifiedPhoto, engraving, slideSizeMm: info.slideSizeMm, seconds: seconds(), origin: "moteur" },
      };
    } catch (error) {
      detail = String(error);
    }
  }

  const [rectifiedPhoto, engraving] = await Promise.all([
    openTileFolder(`/tuiles/photo-${projector}`),
    openTileFolder(`/tuiles/verre-${projector}`),
  ]);
  if (rectifiedPhoto && engraving) {
    return {
      status: "ready",
      sources: {
        rectifiedPhoto,
        engraving,
        slideSizeMm: SLIDE_SIZE_MM_WITHOUT_ENGINE,
        seconds: seconds(),
        origin: "dossier tuiles-demo",
      },
    };
  }
  return { status: "missing", detail };
}

export function ViewerDemo() {
  const viewer = useRef<ViewerHandle>(null);
  const [projector, setProjector] = useState<Projector>("a");
  const [state, setState] = useState<DemoState>({ status: "loading" });
  const [photoVisible, setPhotoVisible] = useState(true);
  const [photoOpacity, setPhotoOpacity] = useState(1);
  const [engravingVisible, setEngravingVisible] = useState(true);
  const [engravingOpacity, setEngravingOpacity] = useState(1);
  const [blackTransparent, setBlackTransparent] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setState({ status: "loading" });
    loadSources(projector).then((next) => {
      if (!cancelled) setState(next);
    });
    return () => {
      cancelled = true;
    };
  }, [projector]);

  const sources = state.status === "ready" ? state.sources : null;
  const layers = useMemo<LayerSpec[]>(() => {
    if (!sources) return [];
    // Les deux images couvrent la même diapositive : chacune est étalée sur
    // 280 mm à partir du même coin, quelle que soit sa taille en pixels.
    const { rectifiedPhoto, engraving, slideSizeMm } = sources;
    return [
      {
        id: "rectified-photo",
        kind: "tiles",
        source: rectifiedPhoto,
        placement: placementForWidth(rectifiedPhoto.pyramid.width, slideSizeMm),
        opacity: photoOpacity,
        visible: photoVisible,
      },
      {
        id: "engraving",
        kind: "tiles",
        source: engraving,
        placement: placementForWidth(engraving.pyramid.width, slideSizeMm),
        opacity: engravingOpacity,
        visible: engravingVisible,
        blend: blackTransparent ? "screen" : "normal",
      },
    ];
  }, [sources, photoOpacity, photoVisible, engravingOpacity, engravingVisible, blackTransparent]);

  return (
    <main className="demo">
      <header className="demo-bar">
        <h1>InSitu</h1>

        <div className="demo-group" role="group" aria-label="Projecteur">
          <span className="demo-label">Projecteur</span>
          {PROJECTORS.map((p) => (
            <button
              key={p}
              type="button"
              className={p === projector ? "actif" : ""}
              aria-pressed={p === projector}
              onClick={() => setProjector(p)}
            >
              {p.toUpperCase()}
            </button>
          ))}
        </div>

        <LayerControl
          label="Photo redressée"
          visible={photoVisible}
          opacity={photoOpacity}
          onVisible={setPhotoVisible}
          onOpacity={setPhotoOpacity}
        />
        <LayerControl
          label="Verre"
          visible={engravingVisible}
          opacity={engravingOpacity}
          onVisible={setEngravingVisible}
          onOpacity={setEngravingOpacity}
        >
          <label className="demo-check" title="Le noir du Verre laisse voir la photo ; seuls les traits restent">
            <input
              type="checkbox"
              checked={blackTransparent}
              onChange={(event) => setBlackTransparent(event.target.checked)}
            />
            noir transparent
          </label>
        </LayerControl>

        <div className="demo-group demo-zoom">
          <button type="button" onClick={() => viewer.current?.fitAll()}>
            Tout voir
          </button>
          <button type="button" onClick={() => viewer.current?.zoomToActualSize()}>
            100 %
          </button>
        </div>
      </header>

      <div className="demo-stage">
        <Viewer ref={viewer} layers={layers} showFrameStats />

        {state.status === "loading" && (
          <p className="demo-message">
            Préparation des tuiles du projecteur {projector.toUpperCase()}…
            <br />
            <small>Quelques secondes la première fois, immédiat ensuite.</small>
          </p>
        )}

        {state.status === "missing" && (
          <div className="demo-message demo-missing">
            <p>
              <strong>Les tuiles du projecteur {projector.toUpperCase()} sont introuvables.</strong>
            </p>
            {state.detail && <p>Le moteur a répondu : {state.detail}</p>}
            <p>
              Pour les fabriquer à la main, lancer ces deux commandes dans le dossier du dépôt, puis
              recharger la page :
            </p>
            <pre>
              {`cargo run --release -p insitu-cli -- tiles jeu-essai/${photoFile(projector)} tuiles-demo/photo-${projector}\n` +
                `cargo run --release -p insitu-cli -- tiles jeu-essai/${engravingFile(projector)} tuiles-demo/verre-${projector}`}
            </pre>
          </div>
        )}
      </div>

      {sources && (
        <footer className="demo-foot">
          Photo redressée : {sources.rectifiedPhoto.pyramid.width.toLocaleString("fr-FR")} px · Verre :{" "}
          {sources.engraving.pyramid.width.toLocaleString("fr-FR")} px · tous deux sur {sources.slideSizeMm} mm ·
          tuiles : {sources.origin}, prêtes en {sources.seconds.toLocaleString("fr-FR", { maximumFractionDigits: 1 })} s
        </footer>
      )}
    </main>
  );
}

interface LayerControlProps {
  label: string;
  visible: boolean;
  opacity: number;
  onVisible: (visible: boolean) => void;
  onOpacity: (opacity: number) => void;
  children?: ReactNode;
}

function LayerControl({ label, visible, opacity, onVisible, onOpacity, children }: LayerControlProps) {
  return (
    <div className="demo-group demo-layer">
      <label className="demo-check">
        <input type="checkbox" checked={visible} onChange={(event) => onVisible(event.target.checked)} />
        {label}
      </label>
      <input
        type="range"
        min={0}
        max={100}
        value={Math.round(opacity * 100)}
        aria-label={`Opacité : ${label}`}
        onChange={(event) => onOpacity(Number(event.target.value) / 100)}
      />
      <span className="demo-value">{Math.round(opacity * 100)} %</span>
      {children}
    </div>
  );
}

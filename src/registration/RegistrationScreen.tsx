// Écran Recalage (tâche 3.2) : pour chaque projecteur, des points communs
// entre sa photo redressée (à gauche, en mm de diapositive) et la façade (à
// droite, en mm réels). Quatre points donnent le trapèze ; les suivants
// suivent le relief. Le moteur mesure l'écart de chaque point et signale
// ceux qui semblent mal placés ; l'emprise du projecteur se dessine sur la
// façade.
//
// Tout le calcul et l'enregistrement passent par le moteur (`../engine.ts`).
// Dans un simple navigateur (`npm run dev`), l'écran montre un exemple déjà
// tuilé dans `tuiles-demo/` s'il existe (voir `docs/recalage.md`) : on peut y
// poser des points, mais rien n'est calculé ni enregistré.

import { useCallback, useEffect, useMemo, useRef, useState, type PointerEvent, type RefObject } from "react";
import {
  fetchRegistration,
  hasEngine,
  openFacadeView,
  openProject,
  openTiledImage,
  openTileFolder,
  projectFile,
  saveRegistration,
  type Project,
  type RegistrationPair,
  type RegistrationReport,
} from "../engine";
import { Viewer, placementForWidth, type LayerSpec, type Placement, type TileSource, type ViewerHandle } from "../viewer";
import { mmToScreen, screenToMm } from "../viewer/geometry";
import { hitHandle } from "../rectification/logic";
import { recalledProject } from "../project/current";
import { facadePlacement, isClick, mmPerSourceUnit } from "../facade/scale";
import {
  GRAB_RADIUS_PX,
  NO_PENDING,
  SLIDE_MM,
  click,
  formatFacadeCm,
  formatSlideMm,
  hint,
  onSlide,
  pairStatus,
  movePoint,
  removePair,
  summary,
  type Pending,
  type Side,
} from "./logic";
import { EMPTY_MARKERS, PairMarkersNode, type MarkersContent } from "./markers";
import "./registration.css";

/** Échelle supposée de la façade de l'exemple du navigateur (mm par pixel). */
const DEMO_FACADE_MM_PER_PIXEL = 4;

/** Installe un objet de repères dans la scène d'une visionneuse, dès qu'elle existe. */
function useMarkers(viewer: RefObject<ViewerHandle | null>, name: string) {
  const node = useRef<PairMarkersNode | null>(null);
  const content = useRef<MarkersContent>(EMPTY_MARKERS);
  useEffect(() => {
    let remove: (() => void) | null = null;
    let frame = 0;
    const attach = () => {
      const scene = viewer.current?.scene();
      if (!scene) {
        frame = requestAnimationFrame(attach);
        return;
      }
      // La scène détruit l'objet qu'on lui retire : un nouveau à chaque pose.
      const created = new PairMarkersNode(name);
      created.set(content.current);
      node.current = created;
      remove = scene.addNode(created);
    };
    attach();
    return () => {
      cancelAnimationFrame(frame);
      node.current = null;
      remove?.();
    };
  }, [viewer, name]);
  return useCallback(
    (next: MarkersContent) => {
      content.current = next;
      node.current?.set(next);
      viewer.current?.scene()?.invalidate();
    },
    [viewer],
  );
}

export function RegistrationScreen() {
  const engine = hasEngine();
  const slideViewer = useRef<ViewerHandle>(null);
  const facadeViewer = useRef<ViewerHandle>(null);
  const slidePane = useRef<HTMLDivElement>(null);
  const facadePane = useRef<HTMLDivElement>(null);
  const pointerDown = useRef<{ x: number; y: number; t: number } | null>(null);
  const setSlideMarkers = useMarkers(slideViewer, "diapositive");
  const setFacadeMarkers = useMarkers(facadeViewer, "façade");

  const [folder, setFolder] = useState<string | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [selectedProjector, setSelectedProjector] = useState<string | null>(null);
  const [photo, setPhoto] = useState<TileSource | null>(null);
  const [facade, setFacade] = useState<{ source: TileSource; placement: Placement } | null>(null);
  const [pairs, setPairs] = useState<RegistrationPair[]>([]);
  const [report, setReport] = useState<RegistrationReport | null>(null);
  const [pending, setPending] = useState<Pending>(NO_PENDING);
  const [selected, setSelected] = useState<number | null>(null);
  /** Point saisi pour être glissé : son côté, son numéro, les paires d'avant. */
  const [dragging, setDragging] = useState<{ side: Side; index: number; before: RegistrationPair[] } | null>(null);
  const [grabbable, setGrabbable] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const projector = project?.projectors.find((p) => p.id === selectedProjector) ?? null;
  const facadeInfo = project?.facade ?? null;
  const scaled = mmPerSourceUnit(facadeInfo?.scale ?? null) !== null;
  const ready = busy === null;

  const run = useCallback(async (label: string | null, task: () => Promise<void>) => {
    if (label) setBusy(label);
    setError(null);
    try {
      await task();
    } catch (failure) {
      setError(String(failure).replace(/^Error: /, ""));
    } finally {
      if (label) setBusy(null);
    }
  }, []);

  // --- Chargement ---------------------------------------------------------

  const showProjector = useCallback(async (dir: string, opened: Project, id: string | null) => {
    setPending(NO_PENDING);
    setSelected(null);
    setPhoto(null);
    setReport(null);
    const p = opened.projectors.find((q) => q.id === id) ?? null;
    setPairs(p?.registration ?? []);
    if (!p?.rectifiedPhoto) return;
    setBusy("Préparation de la photo redressée…");
    setPhoto(await openTiledImage(projectFile(dir, p.rectifiedPhoto)));
    setReport(await fetchRegistration(dir, p.id));
  }, []);

  const load = useCallback(
    (path: string) =>
      run("Ouverture du projet…", async () => {
        const opened = await openProject(path);
        setFolder(path);
        setProject(opened);
        setFacade(null);
        const k = mmPerSourceUnit(opened.facade?.scale ?? null);
        if (opened.facade && k !== null) {
          setBusy("Préparation de l'affichage du plan…");
          const view = await openFacadeView(path);
          setFacade({ source: view.source, placement: facadePlacement(view.sourceUnitsPerPixel, k) });
        }
        const chosen =
          opened.projectors.find((p) => p.rectifiedPhoto && (p.registration?.length ?? 0) > 0) ??
          opened.projectors.find((p) => p.rectifiedPhoto) ??
          null;
        setSelectedProjector(chosen?.id ?? null);
        await showProjector(path, opened, chosen?.id ?? null);
      }),
    [run, showProjector],
  );

  // Au lancement : le projet ouvert dans les autres écrans ; l'exemple sinon.
  useEffect(() => {
    if (engine) {
      const last = recalledProject();
      if (last) void load(last);
      return;
    }
    void (async () => {
      setPhoto(await openTileFolder("/tuiles/recalage-photo"));
      const plan = await openTileFolder("/tuiles/recalage-facade");
      if (plan) setFacade({ source: plan, placement: { x: 0, y: 0, mmPerPixel: DEMO_FACADE_MM_PER_PIXEL } });
    })();
  }, [engine, load]);

  // --- Repères -------------------------------------------------------------

  useEffect(() => {
    const statuses = pairs.map((_, i) => pairStatus(report?.pairs[i]));
    setSlideMarkers({
      points: pairs.map((p, i) => ({ ...p.slide, status: statuses[i] })),
      selected,
      pending: pending.slide,
      footprint: null,
    });
    setFacadeMarkers({
      points: pairs.map((p, i) => ({ ...p.facade, status: statuses[i] })),
      selected,
      pending: pending.facade,
      footprint: report && report.footprint.length > 1 ? report.footprint : null,
    });
  }, [pairs, report, pending, selected, setSlideMarkers, setFacadeMarkers]);

  // Nouvelle image : on recadre.
  useEffect(() => {
    const id = window.setTimeout(() => slideViewer.current?.fitAll(), 0);
    return () => window.clearTimeout(id);
  }, [photo]);
  useEffect(() => {
    const id = window.setTimeout(() => facadeViewer.current?.fitAll(), 0);
    return () => window.clearTimeout(id);
  }, [facade]);

  const slideLayers = useMemo<LayerSpec[]>(
    () =>
      photo
        ? [
            {
              id: "photo-redressee",
              kind: "tiles",
              source: photo,
              placement: placementForWidth(photo.pyramid.width, SLIDE_MM),
            },
          ]
        : [],
    [photo],
  );
  const facadeLayers = useMemo<LayerSpec[]>(
    () => (facade ? [{ id: "facade-plan", kind: "tiles", source: facade.source, placement: facade.placement }] : []),
    [facade],
  );

  // --- Actions -------------------------------------------------------------

  /** Enregistre les paires et relit le compte rendu du moteur. */
  const commit = (next: RegistrationPair[], previous: RegistrationPair[] = pairs) => {
    setPairs(next);
    if (!engine || !folder || !projector) return;
    saveRegistration(folder, projector.id, next).then(
      (saved) => {
        setProject(saved.project);
        setReport(saved.report);
      },
      (failure: unknown) => {
        // Refusé par le moteur : rien n'est enregistré, on revient en arrière.
        setPairs(previous);
        setError(String(failure).replace(/^Error: /, ""));
      },
    );
  };

  const chooseProjector = (id: string) =>
    run("Ouverture du projecteur…", async () => {
      if (!folder || !project) return;
      setSelectedProjector(id);
      await showProjector(folder, project, id);
    });

  const remove = (index: number) => {
    setSelected(null);
    commit(removePair(pairs, index));
  };

  /** Le pointeur, en mm de la vue `side` et en pixels de son écran. */
  const pointerAt = (side: Side, event: PointerEvent) => {
    const handle = side === "slide" ? slideViewer.current : facadeViewer.current;
    const pane = side === "slide" ? slidePane.current : facadePane.current;
    const scene = handle?.scene();
    const canvas = pane?.querySelector(".viewer-canvas");
    if (!scene || !canvas) return null;
    const box = canvas.getBoundingClientRect();
    const screen = { x: event.clientX - box.left, y: event.clientY - box.top };
    const view = scene.getView();
    const viewport = scene.getViewport();
    return { screen, mm: screenToMm(view, viewport, screen), toScreen: (p: { x: number; y: number }) => mmToScreen(view, viewport, p) };
  };

  /** Numéro du point de ce côté sous le pointeur, ou rien. */
  const pointUnder = (side: Side, at: NonNullable<ReturnType<typeof pointerAt>>) =>
    hitHandle(
      pairs.map((pair) => at.toScreen(pair[side])),
      at.screen,
      GRAB_RADIUS_PX,
    );

  const onPointerDown = (side: Side) => (event: PointerEvent<HTMLDivElement>) => {
    pointerDown.current = { x: event.clientX, y: event.clientY, t: event.timeStamp };
    if (!ready) return;
    const at = pointerAt(side, event);
    const hit = at ? pointUnder(side, at) : null;
    if (hit === null) return;
    // Le point est saisi : la visionneuse ne doit pas se déplacer.
    event.stopPropagation();
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    setDragging({ side, index: hit, before: pairs });
  };

  const onPointerMove = (side: Side) => (event: PointerEvent<HTMLDivElement>) => {
    const at = pointerAt(side, event);
    if (!at) return;
    if (dragging && dragging.side === side) {
      setPairs(movePoint(dragging.before, dragging.index, side, at.mm));
      return;
    }
    setGrabbable(ready && pointUnder(side, at) !== null);
  };

  const onPointerUp = (side: Side) => (event: PointerEvent<HTMLDivElement>) => {
    const down = pointerDown.current;
    pointerDown.current = null;
    const up = { x: event.clientX, y: event.clientY, t: event.timeStamp };
    if (dragging) {
      event.currentTarget.releasePointerCapture(event.pointerId);
      const { index, before } = dragging;
      setDragging(null);
      if (down && isClick(down, up)) {
        // Simple clic sur un point : il est choisi (ou ne l'est plus).
        setPairs(before);
        setSelected(selected === index ? null : index);
        return;
      }
      setSelected(index);
      commit(pairs, before);
      return;
    }
    if (!down || !ready || !isClick(down, up)) return;
    const at = pointerAt(side, event)?.mm;
    if (!at) return;
    if (side === "slide" && !onSlide(at)) {
      setError("Ce point est hors de la diapositive : cliquer dans le carré de la photo redressée.");
      return;
    }
    setError(null);
    const result = click(pairs, pending, selected, side, at);
    setPending(result.pending);
    if (result.changed) commit(result.pairs);
  };

  // Échap : abandonne le point en attente ou la sélection.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setPending(NO_PENDING);
      setSelected(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // --- Affichage -------------------------------------------------------------

  const rectified = project?.projectors.filter((p) => p.rectifiedPhoto) ?? [];
  const message = !engine
    ? photo
      ? null
      : "Dans un simple navigateur, le moteur n'est pas là : ouvrir l'app pour recaler un projecteur."
    : !folder
      ? "Ouvrir un projet dans l'écran Façade."
      : !facadeInfo
        ? "Importer le plan de la façade dans l'écran Façade."
        : !scaled
          ? "Mettre la façade à l'échelle dans l'écran Façade : les points de façade sont en millimètres réels."
          : rectified.length === 0
            ? "Redresser d'abord la photo de mire d'un projecteur (écran Redressement)."
            : null;
  const instruction = hint(pending, selected);
  const stageClass = `registration-stage${dragging ? " registration-dragging" : grabbable ? " registration-grab" : ""}`;

  return (
    <main className="registration">
      <header className="registration-bar">
        <div className="registration-group">
          <span className="registration-label">Projet</span>
          <span className="registration-value">{project?.name ?? (engine ? "aucun" : "exemple")}</span>
          {engine && (
            <button type="button" onClick={() => folder && void load(folder)} disabled={!folder || !ready}>
              Relire
            </button>
          )}
        </div>
        <div className="registration-group">
          <span className="registration-label">Projecteur</span>
          <select
            value={selectedProjector ?? ""}
            onChange={(event) => void chooseProjector(event.target.value)}
            disabled={!ready || rectified.length === 0}
            aria-label="Projecteur"
          >
            {rectified.length === 0 && <option value="">aucun</option>}
            {rectified.map((p) => (
              <option key={p.id} value={p.id}>
                {p.label} — {(p.registration?.length ?? 0) > 0 ? `${p.registration?.length} points` : "à recaler"}
              </option>
            ))}
          </select>
        </div>
        <div className="registration-group">
          <span className="registration-label">Voir</span>
          <button type="button" onClick={() => slideViewer.current?.fitAll()} disabled={!photo}>
            Toute la photo
          </button>
          <button type="button" onClick={() => facadeViewer.current?.fitAll()} disabled={!facade}>
            Toute la façade
          </button>
        </div>
      </header>

      <div className="registration-body">
        <div className="registration-views">
          <section className="registration-pane" ref={slidePane}>
            <h2>Photo redressée {projector ? `— ${projector.label}` : ""}</h2>
            <div
              className={stageClass}
              onPointerDownCapture={onPointerDown("slide")}
              onPointerMove={onPointerMove("slide")}
              onPointerUp={onPointerUp("slide")}
            >
              <Viewer ref={slideViewer} layers={slideLayers} cursorUnit="mm" />
            </div>
          </section>
          <section className="registration-pane" ref={facadePane}>
            <h2>Façade</h2>
            <div
              className={stageClass}
              onPointerDownCapture={onPointerDown("facade")}
              onPointerMove={onPointerMove("facade")}
              onPointerUp={onPointerUp("facade")}
            >
              <Viewer ref={facadeViewer} layers={facadeLayers} cursorUnit="mm" />
            </div>
          </section>
          {(message || busy) && <p className="registration-message">{busy ?? message}</p>}
        </div>

        <aside className="registration-list" aria-label="Points communs">
          <table>
            <thead>
              <tr>
                <th>N°</th>
                <th title="Écart sur la diapositive">Diapo</th>
                <th title="Écart sur la façade">Façade</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {pairs.map((_, i) => {
                const r = report?.pairs[i];
                const status = pairStatus(r);
                return (
                  <tr
                    key={i}
                    className={`${selected === i ? "selectionne " : ""}etat-${status}`}
                    onClick={() => {
                      setPending(NO_PENDING);
                      setSelected(selected === i ? null : i);
                    }}
                    title={r?.outlier ? "Point douteux : à vérifier, déplacer ou retirer" : "Cliquer pour déplacer ce point"}
                  >
                    <td>
                      <i className={`pastille ${status}`} /> {i + 1}
                    </td>
                    <td>{formatSlideMm(r?.slideResidualMm)}</td>
                    <td>{formatFacadeCm(r?.facadeResidualMm)}</td>
                    <td>
                      <button
                        type="button"
                        aria-label={`Retirer le point ${i + 1}`}
                        onClick={(event) => {
                          event.stopPropagation();
                          remove(i);
                        }}
                        disabled={!ready}
                      >
                        ×
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          {pairs.length === 0 && <p className="registration-empty">Aucun point pour l'instant.</p>}
        </aside>
      </div>

      <footer className="registration-foot">
        {error ? (
          <span className="registration-error">{error}</span>
        ) : instruction ? (
          <span className="registration-hint">{instruction} (Échap : annuler)</span>
        ) : (
          <span>{summary(report, pairs.length)}</span>
        )}
        <span className="registration-legend">
          <i className="pastille bon" /> &lt; 1 mm <i className="pastille moyen" /> au-delà{" "}
          <i className="pastille aberrant" /> douteux <i className="pastille inconnu" /> pas encore mesuré
        </span>
      </footer>
    </main>
  );
}

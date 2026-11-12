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
  cacheFileUrl,
  fetchFacadeOverview,
  fetchRegistration,
  hasEngine,
  openFacadeView,
  openProject,
  openTiledImage,
  openTileFolder,
  projectFile,
  saveRegistration,
  type FacadeOverview,
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
  POINT_ADVICE,
  SLIDE_MM,
  click,
  facadePhotoLayer,
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
import { ProjectMenu } from "../project/ProjectMenu";
import {
  Banner,
  Busy,
  Button,
  Dot,
  EmptyState,
  IconButton,
  List,
  ListRow,
  Panel,
  Screen,
  StatusBar,
  StatusText,
  ToolGroup,
  ToolSpacer,
  Toolbar,
  useShortcuts,
  type Tone,
} from "../ui";
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
  /** Projecteurs posés sur la façade, pour y voir la photo recalée. */
  const [overview, setOverview] = useState<FacadeOverview | null>(null);
  const [showPhoto, setShowPhoto] = useState(true);
  const [photoOpacity, setPhotoOpacity] = useState(0.6);

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

  /** Relit la pose des photos sur la façade (après chaque recalage enregistré). */
  const refreshOverview = useCallback(async (dir: string) => {
    try {
      setOverview(await fetchFacadeOverview(dir));
    } catch {
      // Sans aperçu, le recalage reste possible : on n'affiche pas la photo.
      setOverview(null);
    }
  }, []);

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
        setOverview(null);
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
        await refreshOverview(path);
      }),
    [run, showProjector, refreshOverview],
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
  const facadeLayers = useMemo<LayerSpec[]>(() => {
    if (!facade) return [];
    const plan: LayerSpec = { id: "facade-plan", kind: "tiles", source: facade.source, placement: facade.placement };
    // La photo recalée par-dessus le plan, pour juger le recalage d'un coup d'œil.
    const recaled = facadePhotoLayer(overview, selectedProjector, showPhoto, photoOpacity, cacheFileUrl);
    return recaled ? [plan, recaled] : [plan];
  }, [facade, overview, selectedProjector, showPhoto, photoOpacity]);

  // --- Actions -------------------------------------------------------------

  /** Enregistre les paires et relit le compte rendu du moteur. */
  const commit = (next: RegistrationPair[], previous: RegistrationPair[] = pairs) => {
    setPairs(next);
    if (!engine || !folder || !projector) return;
    saveRegistration(folder, projector.id, next).then(
      (saved) => {
        setProject(saved.project);
        setReport(saved.report);
        void refreshOverview(folder);
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

  const tones: Record<string, Tone> = { bon: "success", moyen: "warning", aberrant: "danger", inconnu: "info" };
  const moveProjector = (step: -1 | 1) => {
    const at = rectified.findIndex((p) => p.id === selectedProjector);
    const next = rectified[Math.min(rectified.length - 1, Math.max(0, at + step))];
    if (next && next.id !== selectedProjector && ready) void chooseProjector(next.id);
  };
  const stepPoint = (step: -1 | 1) => {
    if (pairs.length === 0) return;
    setPending(NO_PENDING);
    setSelected(selected === null ? 0 : Math.min(pairs.length - 1, Math.max(0, selected + step)));
  };

  useShortcuts([
    {
      keys: "mod+0",
      run: () => {
        slideViewer.current?.fitAll();
        facadeViewer.current?.fitAll();
      },
    },
    { keys: "backspace", run: () => selected !== null && remove(selected), enabled: selected !== null && ready },
  ]);

  const toolbar = (
    <Toolbar>
      <ProjectMenu
        name={project?.name ?? null}
        folder={folder}
        ready={ready}
        placeholder="Exemple sans moteur"
        onOpen={(path) => void load(path)}
        onReload={() => folder && void load(folder)}
        onError={setError}
      />
      <ToolSpacer />
      <ToolGroup label="Photo sur la façade">
        <label className="ui-check" title="Poser la photo redressée sur la façade, d'après les points (à partir de 4)">
          <input type="checkbox" checked={showPhoto} onChange={(e) => setShowPhoto(e.target.checked)} disabled={!engine} />
          Photo recalée
        </label>
        <input
          type="range"
          className="registration-opacity"
          aria-label="Opacité de la photo recalée"
          title="Opacité de la photo recalée"
          min={0.1}
          max={1}
          step={0.05}
          value={photoOpacity}
          disabled={!engine || !showPhoto}
          onChange={(e) => setPhotoOpacity(Number(e.target.value))}
        />
      </ToolGroup>
      <ToolGroup label="Voir">
        <Button icon="fit" onClick={() => slideViewer.current?.fitAll()} disabled={!photo}>
          Toute la photo
        </Button>
        <Button icon="fit" onClick={() => facadeViewer.current?.fitAll()} disabled={!facade} shortcut="mod+0" title="⌘0 cadre les deux vues">
          Toute la façade
        </Button>
      </ToolGroup>
    </Toolbar>
  );

  const sidebar = (
    <Panel side="left" title="Projecteurs">
      {rectified.length > 0 ? (
        <List label="Projecteurs" onMove={moveProjector}>
          {rectified.map((p) => {
            const count = p.registration?.length ?? 0;
            return (
              <ListRow
                key={p.id}
                selected={p.id === selectedProjector}
                onSelect={() => ready && p.id !== selectedProjector && void chooseProjector(p.id)}
                leading={<Dot tone={count >= 4 ? "success" : count > 0 ? "warning" : undefined} />}
                title={p.label}
                detail={count > 0 ? `${count} point${count > 1 ? "s" : ""}` : "à recaler"}
              />
            );
          })}
        </List>
      ) : (
        <p className="ui-list-empty">{engine ? "Aucun projecteur redressé." : "Exemple sans moteur."}</p>
      )}
    </Panel>
  );

  const inspector = (
    <Panel side="right" title="Points communs">
      {pairs.length < 4 && (
        <div className="registration-advice">
          <p className="ui-note">{POINT_ADVICE}</p>
          <p className="ui-note">
            Cliquer le détail sur la photo, puis le même sur la façade. Il en faut 4, répartis vers les quatre coins de la
            photo ; la photo se pose alors sur la façade.
          </p>
        </div>
      )}
      {pairs.length > 0 ? (
        <List label="Points communs" onMove={stepPoint}>
          {pairs.map((_, i) => {
            const r = report?.pairs[i];
            const status = pairStatus(r);
            return (
              <ListRow
                key={i}
                selected={selected === i}
                tone={status === "aberrant" ? "danger" : undefined}
                onSelect={() => {
                  setPending(NO_PENDING);
                  setSelected(selected === i ? null : i);
                }}
                tooltip={r?.outlier ? "Point douteux : à vérifier, déplacer ou supprimer" : "Cliquer pour choisir ce point, puis le glisser pour le déplacer"}
                leading={<Dot tone={tones[status]} />}
                title={`Point ${i + 1}`}
                detail={`diapo ${formatSlideMm(r?.slideResidualMm)} · façade ${formatFacadeCm(r?.facadeResidualMm)}`}
                actions={<IconButton icon="trash" label={`Supprimer le point ${i + 1}`} shortcut="backspace" size="sm" onClick={() => remove(i)} disabled={!ready} />}
              />
            );
          })}
        </List>
      ) : (
        <p className="ui-list-empty">Aucun point pour l'instant.</p>
      )}
    </Panel>
  );

  const status = (
    <StatusBar>
      {instruction ? <StatusText tone="info">{instruction} (Échap : annuler)</StatusText> : <StatusText>{summary(report, pairs.length)}</StatusText>}
      <span className="ui-legend push">
        <span>
          <Dot tone="success" /> &lt; 1 mm
        </span>
        <span>
          <Dot tone="warning" /> au-delà
        </span>
        <span>
          <Dot tone="danger" /> douteux
        </span>
        <span>
          <Dot tone="info" /> pas encore mesuré
        </span>
      </span>
    </StatusBar>
  );

  return (
    <Screen
      className="registration"
      toolbar={toolbar}
      banner={
        error && (
          <Banner tone="danger" onClose={() => setError(null)}>
            {error}
          </Banner>
        )
      }
      sidebar={sidebar}
      inspector={inspector}
      status={status}
    >
      <div className="registration-views">
        <section className="registration-pane" ref={slidePane}>
          <h2>Photo redressée {projector ? `· ${projector.label}` : ""}</h2>
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
        {message && !busy && (
          <EmptyState icon="link" title="Recalage pas encore possible">
            <p>{message}</p>
          </EmptyState>
        )}
        {busy && <Busy>{busy}</Busy>}
      </div>
    </Screen>
  );
}

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
  previewRegistration,
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
  cornerPairs,
  demoTrapezeMesh,
  facadeGrab,
  facadePhotoLayer,
  initialSide,
  meshCenter,
  pairTitle,
  translateFacade,
  type MeshLike,
  type XYPoint,
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
/** Aperçu de la photo de l'exemple du navigateur (voir `docs/recalage.md`). */
const DEMO_PHOTO_PREVIEW = "/tuiles/recalage-photo/apercu.jpg";

/** Ce qu'on est en train de glisser. */
type Drag =
  /** Un point posé, d'un côté ou de l'autre. */
  | { kind: "point"; side: Side; index: number; before: RegistrationPair[] }
  /** Toute la photo, par la poignée du centre (façade). */
  | { kind: "move"; start: XYPoint; before: RegistrationPair[] }
  /** Un détail de la photo attrapé sur la façade : nouvelle paire au premier mouvement. */
  | { kind: "pin"; slide: XYPoint; before: RegistrationPair[] };

/** Contour d'un maillage (pour l'emprise de l'exemple du navigateur). */
function meshOutline(mesh: MeshLike): XYPoint[] {
  const n = mesh.divisions;
  const at = (i: number, j: number) => mesh.facade[j * (n + 1) + i];
  const out: XYPoint[] = [];
  for (let i = 0; i < n; i++) out.push(at(i, 0));
  for (let j = 0; j < n; j++) out.push(at(n, j));
  for (let i = n; i > 0; i--) out.push(at(i, n));
  for (let j = n; j > 0; j--) out.push(at(0, j));
  out.push(at(0, 0));
  return out;
}

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
  /** Ce qui est saisi pour être glissé, et les paires d'avant. */
  const [dragging, setDragging] = useState<Drag | null>(null);
  /** Pose provisoire calculée par le moteur pendant un glisser (D-3.2-h). */
  const [live, setLive] = useState<{ mesh: MeshLike | null; footprint: XYPoint[] } | null>(null);
  const previewing = useRef<{ busy: boolean; next: RegistrationPair[] | null }>({ busy: false, next: null });
  const spaceHeld = useRef(false);
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
    setLive(null);
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

  /** Dans l'exemple du navigateur, un simple trapèze par les 4 premiers points. */
  const demoMesh = useMemo(() => (engine ? null : demoTrapezeMesh(pairs)), [engine, pairs]);
  const savedMesh = overview?.projectors.find((q) => q.projectorId === selectedProjector)?.mesh ?? null;
  /** La photo telle qu'elle est posée sur la façade en ce moment. */
  const currentMesh: MeshLike | null = demoMesh ?? live?.mesh ?? (pairs.length >= 4 ? savedMesh : null);
  const photoOnFacade = showPhoto && currentMesh !== null && (engine ? true : photo !== null);

  useEffect(() => {
    const statuses = pairs.map((_, i) => pairStatus(report?.pairs[i]));
    const footprint = demoMesh
      ? meshOutline(demoMesh)
      : live
        ? live.footprint
        : report && report.footprint.length > 1
          ? report.footprint
          : null;
    setSlideMarkers({
      points: pairs.map((p, i) => ({ ...p.slide, status: statuses[i] })),
      selected,
      pending: pending.slide,
      footprint: null,
      handle: null,
    });
    setFacadeMarkers({
      points: pairs.map((p, i) => ({ ...p.facade, status: statuses[i] })),
      selected,
      pending: pending.facade,
      footprint: footprint && footprint.length > 1 ? footprint : null,
      handle: photoOnFacade && currentMesh ? meshCenter(currentMesh) : null,
    });
  }, [pairs, report, pending, selected, live, demoMesh, currentMesh, photoOnFacade, setSlideMarkers, setFacadeMarkers]);

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
    const recaled = demoMesh
      ? ({
          id: "photo-recalee",
          kind: "mesh-image",
          url: DEMO_PHOTO_PREVIEW,
          divisions: demoMesh.divisions,
          points: demoMesh.facade,
          opacity: photoOpacity,
          visible: showPhoto,
        } satisfies LayerSpec)
      : pairs.length >= 4
        ? facadePhotoLayer(overview, selectedProjector, showPhoto, photoOpacity, cacheFileUrl, live?.mesh ?? null)
        : null;
    return recaled ? [plan, recaled] : [plan];
  }, [facade, overview, selectedProjector, showPhoto, photoOpacity, live, demoMesh, pairs.length]);

  // --- Actions -------------------------------------------------------------

  /** Enregistre les paires et relit le compte rendu du moteur. */
  const commit = (next: RegistrationPair[], previous: RegistrationPair[] = pairs) => {
    setPairs(next);
    if (!engine || !folder || !projector) return;
    saveRegistration(folder, projector.id, next).then(
      (saved) => {
        setProject(saved.project);
        setReport(saved.report);
        // La pose provisoire reste affichée jusqu'à ce que la vraie la remplace.
        void refreshOverview(folder).then(() => setLive(null));
      },
      (failure: unknown) => {
        // Refusé par le moteur : rien n'est enregistré, on revient en arrière.
        setPairs(previous);
        setLive(null);
        setError(String(failure).replace(/^Error: /, ""));
      },
    );
  };

  /**
   * Redessine la photo sur la façade pour des paires en cours de glisser.
   * Le moteur calcule ; une seule demande à la fois, la dernière gagne.
   */
  const requestPreview = useCallback(
    (next: RegistrationPair[]) => {
      if (!engine || next.length < 4) return;
      const queue = previewing.current;
      queue.next = next;
      if (queue.busy) return;
      const pump = async () => {
        while (queue.next) {
          const asked = queue.next;
          queue.next = null;
          queue.busy = true;
          try {
            const result = await previewRegistration(asked);
            setLive({ mesh: result.mesh, footprint: result.footprint });
          } catch {
            // Pas d'aperçu pour cette position : on garde le précédent.
          }
        }
        queue.busy = false;
      };
      void pump();
    },
    [engine],
  );

  /** Pose la photo sur la façade par ses quatre coins, au milieu de la vue. */
  const placeByCorners = () => {
    const scene = facadeViewer.current?.scene();
    if (!scene) return;
    const view = scene.getView();
    const viewport = scene.getViewport();
    const a = screenToMm(view, viewport, { x: 0, y: 0 });
    const b = screenToMm(view, viewport, { x: viewport.width, y: viewport.height });
    const rect = { x: a.x, y: a.y, width: b.x - a.x, height: b.y - a.y };
    const others = (overview?.projectors ?? []).filter((q) => q.projectorId !== selectedProjector).map((q) => q.footprint);
    const side = initialSide(rect, others);
    const next = cornerPairs({ x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 }, side);
    setPending(NO_PENDING);
    setSelected(null);
    setShowPhoto(true);
    requestPreview(next);
    commit(next);
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

  /** Ce qu'un appui attrape à cet endroit (point posé, poignée, détail de la photo). */
  const grabAt = (side: Side, at: NonNullable<ReturnType<typeof pointerAt>>) => {
    const hit = pointUnder(side, at);
    if (side === "slide") return hit === null ? ({ kind: "none" } as const) : ({ kind: "point", index: hit } as const);
    const center = currentMesh ? meshCenter(currentMesh) : null;
    const c = center ? at.toScreen(center) : null;
    return facadeGrab({
      pointHit: hit,
      centerDistancePx: c ? Math.hypot(c.x - at.screen.x, c.y - at.screen.y) : null,
      mesh: currentMesh,
      at: at.mm,
      photoShown: photoOnFacade,
      spaceHeld: spaceHeld.current,
    });
  };

  const onPointerDown = (side: Side) => (event: PointerEvent<HTMLDivElement>) => {
    pointerDown.current = { x: event.clientX, y: event.clientY, t: event.timeStamp };
    if (!ready || spaceHeld.current) return;
    const at = pointerAt(side, event);
    if (!at) return;
    const grab = grabAt(side, at);
    if (grab.kind === "none") return;
    // Quelque chose est saisi : la visionneuse ne doit pas se déplacer.
    event.stopPropagation();
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    if (grab.kind === "point") setDragging({ kind: "point", side, index: grab.index, before: pairs });
    else if (grab.kind === "move") setDragging({ kind: "move", start: at.mm, before: pairs });
    else setDragging({ kind: "pin", slide: grab.slide, before: pairs });
  };

  const onPointerMove = (side: Side) => (event: PointerEvent<HTMLDivElement>) => {
    const at = pointerAt(side, event);
    if (!at) return;
    if (dragging) {
      const down = pointerDown.current;
      const moved = down ? !isClick(down, { x: event.clientX, y: event.clientY, t: event.timeStamp }) : true;
      let next: RegistrationPair[] | null = null;
      if (dragging.kind === "point" && dragging.side === side) {
        next = movePoint(dragging.before, dragging.index, side, at.mm);
      } else if (dragging.kind === "move" && side === "facade") {
        next = translateFacade(dragging.before, at.mm.x - dragging.start.x, at.mm.y - dragging.start.y);
      } else if (dragging.kind === "pin" && side === "facade" && moved) {
        next = [...dragging.before, { slide: dragging.slide, facade: at.mm }];
      }
      if (next) {
        setPairs(next);
        requestPreview(next);
      }
      return;
    }
    setGrabbable(ready && !spaceHeld.current && grabAt(side, at).kind !== "none");
  };

  const onPointerUp = (side: Side) => (event: PointerEvent<HTMLDivElement>) => {
    const down = pointerDown.current;
    pointerDown.current = null;
    const up = { x: event.clientX, y: event.clientY, t: event.timeStamp };
    const clicked = down !== null && isClick(down, up);
    if (dragging) {
      event.currentTarget.releasePointerCapture(event.pointerId);
      const drag = dragging;
      setDragging(null);
      if (drag.kind === "point") {
        if (clicked) {
          // Simple clic sur un point : il est choisi (ou ne l'est plus).
          setPairs(drag.before);
          setSelected(selected === drag.index ? null : drag.index);
          return;
        }
        setSelected(drag.index);
        commit(pairs, drag.before);
        return;
      }
      if (clicked) {
        setPairs(drag.before);
        // Un simple clic sur la photo (sans glisser) reste un clic de pose.
        if (drag.kind === "move") return;
      } else {
        // Une épingle n'est pas choisie après la pose : le clic suivant pose
        // un nouveau point au lieu de la déplacer.
        commit(pairs, drag.before);
        return;
      }
    }
    if (!down || !ready || !clicked) return;
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

  // Barre d'espace enfoncée : glisser déplace la vue, comme dans Illustrator.
  useEffect(() => {
    const isTyping = (event: KeyboardEvent) =>
      event.target instanceof HTMLElement && /^(INPUT|TEXTAREA|SELECT)$/.test(event.target.tagName);
    const down = (event: KeyboardEvent) => {
      if (event.code !== "Space" || isTyping(event)) return;
      event.preventDefault();
      spaceHeld.current = true;
      setGrabbable(false);
    };
    const up = (event: KeyboardEvent) => {
      if (event.code === "Space") spaceHeld.current = false;
    };
    const blur = () => (spaceHeld.current = false);
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    window.addEventListener("blur", blur);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
      window.removeEventListener("blur", blur);
    };
  }, []);

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
      {pairs.length === 0 && (
        <div className="registration-advice">
          <Button
            variant="primary"
            icon="link"
            onClick={placeByCorners}
            disabled={!ready || !photo || !facade || (engine && !projector)}
            title="Pose la photo redressée au milieu de la vue de la façade, par ses quatre coins"
          >
            Poser la photo sur la façade
          </Button>
          <p className="ui-note">
            Puis glisser ses quatre coins sur la façade jusqu'à ce que les fenêtres de la photo tombent sur celles du plan. La
            poignée du centre déplace toute la photo.
          </p>
        </div>
      )}
      {pairs.length > 0 && pairs.length < 4 && (
        <div className="registration-advice">
          <p className="ui-note">{POINT_ADVICE}</p>
          <p className="ui-note">
            Cliquer le détail sur la photo, puis le même sur la façade. Il en faut 4, répartis vers les quatre coins de la
            photo. Plus simple : supprimer ces points et poser la photo par ses coins.
          </p>
        </div>
      )}
      {pairs.length >= 4 && (
        <div className="registration-advice">
          <p className="ui-note">
            Pour affiner : sur la façade, attraper un détail de la photo (coin de fenêtre…) et le lâcher sur le même détail du
            plan. Chaque détail ainsi posé devient un point. Barre d'espace + glisser : déplacer la vue.
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
                title={pairTitle(pairs, i)}
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

// Écran Recalage (tâche 3.2) : pour chaque projecteur, des points communs
// entre sa photo redressée (à gauche, en mm de diapositive) et la façade (à
// droite, en mm réels). Quatre points donnent le trapèze ; au-delà, il est
// ajusté au mieux sur tous (perspective pure, D-3.2-l). Le moteur mesure l'écart de chaque point et signale
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
  adjustRegistration,
  previewRegistration,
  projectFile,
  saveRegistration,
  type FacadeOverview,
  type Project,
  type RegistrationLine,
  type RegistrationPair,
  type RegistrationReport,
} from "../engine";
import { NudgeSettle, Viewer, nudged, placementForWidth, useArrowNudge, type LayerSpec, type Placement, type TileSource, type ViewerHandle } from "../viewer";
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
  ADJUST_SHOWN,
  constraintsText,
  ensureGuides,
  followConstraints,
  isGuide,
  lineDrawn,
  removePair,
  summary,
  type Pending,
  type Side,
  readRegistrationLook,
  saveRegistrationLook,
  type RegistrationLook,
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
  Segmented,
  StatusBar,
  StatusText,
  ToolGroup,
  ToolSpacer,
  Toolbar,
  useShortcuts,
  type Tone,
} from "../ui";
import "./registration.css";
import { asString, keep as keepSetting, projectKey, recall } from "../ui/remember";

const SELECTED_KEY = "insitu.projecteurChoisi.recalage";

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
  | { kind: "pin"; slide: XYPoint; before: RegistrationPair[] }
  /** Un trait en cours de tracé, sur la photo ou sur le plan (mode Lignes, D-3.2-k). */
  | { kind: "draw"; side: Side; start: XYPoint; end: XYPoint; before: RegistrationPair[] }
  /** Un bout d'une ligne déjà posée. */
  | { kind: "lineEnd"; side: Side; line: number; end: 0 | 1; before: RegistrationPair[]; beforeLines: RegistrationLine[] };

/** Façon de caler la photo : par points sûrs ou par lignes communes (D-3.2-k). */
type Mode = "points" | "lines";

/** Ramène un point dans le carré de la diapositive. */
const clampSlide = (p: XYPoint): XYPoint => ({
  x: Math.min(SLIDE_MM, Math.max(0, p.x)),
  y: Math.min(SLIDE_MM, Math.max(0, p.y)),
});

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
  useEffect(() => {
    if (folder && selectedProjector) keepSetting(projectKey(SELECTED_KEY, folder), selectedProjector);
  }, [folder, selectedProjector]);
  const [photo, setPhoto] = useState<TileSource | null>(null);
  const [facade, setFacade] = useState<{ source: TileSource; placement: Placement } | null>(null);
  const [pairs, setPairs] = useState<RegistrationPair[]>([]);
  /** Lignes communes (bord de fenêtre, corniche…) : un trait sur la photo, le même sur le plan. */
  const [lines, setLines] = useState<RegistrationLine[]>([]);
  /** Trait déjà tracé d'un côté, qui attend le même de l'autre. */
  const [halfLine, setHalfLine] = useState<{ side: Side; seg: [XYPoint, XYPoint] } | null>(null);
  const [mode, setMode] = useState<Mode>("points");
  const [report, setReport] = useState<RegistrationReport | null>(null);
  const [pending, setPending] = useState<Pending>(NO_PENDING);
  const [selected, setSelected] = useState<number | null>(null);
  /** Côté du point choisi que les flèches déplacent : le dernier cliqué. */
  const [selectedSide, setSelectedSide] = useState<Side>("facade");
  const settle = useRef(new NudgeSettle<RegistrationPair[]>());
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
  // Photo recalée et son opacité : retenues au changement d'écran et d'un lancement à l'autre.
  const [look, setLookState] = useState<RegistrationLook>(readRegistrationLook);
  const { showPhoto, photoOpacity } = look;
  const setLook = useCallback((update: (current: RegistrationLook) => RegistrationLook) => {
    setLookState((current) => {
      const next = update(current);
      saveRegistrationLook(next);
      return next;
    });
  }, []);
  const setShowPhoto = useCallback((show: boolean) => setLook((current) => ({ ...current, showPhoto: show })), [setLook]);
  const setPhotoOpacity = useCallback((o: number) => setLook((current) => ({ ...current, photoOpacity: o })), [setLook]);
  /** Points d'avant le dernier « Ajuster », tant qu'on peut l'annuler (D-3.2-i). */
  const [adjustUndo, setAdjustUndo] = useState<RegistrationPair[] | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  /** Enregistrements en cours : ⌘Z attend qu'ils soient finis (sinon il annulerait l'action d'avant). */
  const [saving, setSaving] = useState(0);

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
    setAdjustUndo(null);
    setNotice(null);
    const p = opened.projectors.find((q) => q.id === id) ?? null;
    setPairs(p?.registration ?? []);
    setLines(p?.registrationLines ?? []);
    setHalfLine(null);
    if (!p?.rectifiedPhoto) return;
    setBusy("Préparation de la photo redressée…");
    setPhoto(await openTiledImage(projectFile(dir, p.rectifiedPhoto)));
    setReport(await fetchRegistration(dir, p.id));
  }, []);

  const load = useCallback(
    (path: string) =>
      run("Ouverture du projet…", async () => {
        const kept = recall(projectKey(SELECTED_KEY, path), asString, null);
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
        // Le projecteur regardé la dernière fois dans ce projet (D15-v), sinon un recalé.
        const chosen =
          opened.projectors.find((p) => p.id === kept && p.rectifiedPhoto) ??
          opened.projectors.find((p) => p.rectifiedPhoto && (p.registration?.length ?? 0) > 0) ??
          opened.projectors.find((p) => p.rectifiedPhoto) ??
          null;
        setSelectedProjector(chosen?.id ?? null);
        await showProjector(path, opened, chosen?.id ?? null);
        await refreshOverview(path);
      }),
    [run, showProjector, refreshOverview],
  );

  /**
   * Relit le projet après ⌘Z, ⇧⌘Z ou ⌘R sans changer de projecteur ni de
   * vue : seuls les points, le compte rendu et la photo posée changent. Si
   * le plan lui-même a changé, on rouvre tout.
   */
  const reload = useCallback(
    (path: string, current: Project | null, keep: string | null) =>
      run(null, async () => {
        const opened = await openProject(path);
        const p = opened.projectors.find((q) => q.id === keep) ?? null;
        if (!current || !p || JSON.stringify(opened.facade ?? null) !== JSON.stringify(current.facade ?? null)) {
          await load(path);
          return;
        }
        setProject(opened);
        setPending(NO_PENDING);
        setSelected(null);
        setDragging(null);
        setLive(null);
        setAdjustUndo(null);
        setPairs(p.registration ?? []);
        setLines(p.registrationLines ?? []);
        setHalfLine(null);
        setReport(p.rectifiedPhoto ? await fetchRegistration(path, p.id) : null);
        await refreshOverview(path);
      }),
    [run, load, refreshOverview],
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

  /** Trait en attente ou en cours de tracé, de ce côté. */
  const draftOn = (side: Side): [XYPoint, XYPoint] | null =>
    dragging?.kind === "draw" && dragging.side === side
      ? [dragging.start, dragging.end]
      : halfLine?.side === side
        ? halfLine.seg
        : null;

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
      lines: lines.map((l) => l.slide),
      draft: draftOn("slide"),
    });
    setFacadeMarkers({
      points: pairs.map((p, i) => ({ ...p.facade, status: statuses[i] })),
      selected,
      pending: pending.facade,
      footprint: footprint && footprint.length > 1 ? footprint : null,
      handle: photoOnFacade && currentMesh ? meshCenter(currentMesh) : null,
      lines: lines.map((l) => l.facade),
      draft: draftOn("facade"),
    });
  }, [pairs, lines, halfLine, dragging, report, pending, selected, live, demoMesh, currentMesh, photoOnFacade, setSlideMarkers, setFacadeMarkers]);

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

  /** Enregistre les paires et les lignes, et relit le compte rendu du moteur. */
  const commit = (
    next: RegistrationPair[],
    previous: RegistrationPair[] = pairs,
    nextLines: RegistrationLine[] = lines,
    previousLines: RegistrationLine[] = lines,
  ) => {
    // Toute autre retouche rend l'ajustement définitif.
    setAdjustUndo(null);
    setPairs(next);
    setLines(nextLines);
    if (!engine || !folder || !projector) return;
    setSaving((n) => n + 1);
    saveRegistration(folder, projector.id, next, nextLines)
      .then(
      (saved) => {
        setProject(saved.project);
        setReport(saved.report);
        // La pose provisoire reste affichée jusqu'à ce que la vraie la remplace.
        void refreshOverview(folder).then(() => setLive(null));
      },
      (failure: unknown) => {
        // Refusé par le moteur : rien n'est enregistré, on revient en arrière.
        setPairs(previous);
        setLines(previousLines);
        setLive(null);
        setError(String(failure).replace(/^Error: /, ""));
      },
      )
      .finally(() => setSaving((n) => n - 1));
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

  /** La partie de la façade visible à l'écran, en mm. */
  const visibleFacade = () => {
    const scene = facadeViewer.current?.scene();
    if (!scene) return null;
    const view = scene.getView();
    const viewport = scene.getViewport();
    const a = screenToMm(view, viewport, { x: 0, y: 0 });
    const b = screenToMm(view, viewport, { x: viewport.width, y: viewport.height });
    return { x: a.x, y: a.y, width: b.x - a.x, height: b.y - a.y };
  };

  /** Pose la photo sur la façade par ses quatre coins, au milieu de la vue. */
  const placeByCorners = () => {
    const rect = visibleFacade();
    if (!rect) return;
    const others = (overview?.projectors ?? []).filter((q) => q.projectorId !== selectedProjector).map((q) => q.footprint);
    const side = initialSide(rect, others);
    const next = cornerPairs({ x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 }, side);
    setPending(NO_PENDING);
    setSelected(null);
    setShowPhoto(true);
    requestPreview(next);
    commit(next);
  };

  /**
   * Cale la pose affichée sur les lignes du plan (D-3.2-i) : le moteur part
   * des points actuels et rend les quatre coins ajustés, qu'on enregistre.
   */
  const adjustNow = () =>
    run("Ajustement sur les lignes du plan…", async () => {
      if (!folder || !projector || pairs.length < 4) return;
      setNotice(null);
      const result = await adjustRegistration(folder, projector.id, pairs);
      if (result.movedMm < 1) {
        setNotice("Rien de mieux autour de cette pose : la photo reste où elle est. La rapprocher à la main et réessayer.");
        return;
      }
      const before = pairs;
      setPending(NO_PENDING);
      setSelected(null);
      setShowPhoto(true);
      requestPreview(result.pairs);
      commit(result.pairs, before);
      setAdjustUndo(before);
      setNotice(
        `Photo calée sur le plan : un coin a bougé de ${(result.movedMm / 10).toFixed(0)} cm au plus. Vérifier que les fenêtres tombent juste ; sinon « Annuler l'ajustement ».`,
      );
    });

  const undoAdjust = () => {
    if (!adjustUndo) return;
    setNotice(null);
    requestPreview(adjustUndo);
    commit(adjustUndo);
  };

  const chooseProjector = (id: string) =>
    run("Ouverture du projecteur…", async () => {
      if (!folder || !project) return;
      setSelectedProjector(id);
      await showProjector(folder, project, id);
    });

  /** Les coins guides recalculés d'après les points sûrs et les lignes. */
  // Des points posés sans les coins guides (un à un, ou guides supprimés)
  // les retrouvent dès qu'il y en a 4 : les points restent libres (D-3.2-m).
  const follow = (next: RegistrationPair[], withLines: RegistrationLine[] = lines) =>
    followConstraints(ensureGuides(next), withLines);

  const remove = (index: number) => {
    setSelected(null);
    commit(isGuide(pairs[index]) ? removePair(pairs, index) : follow(removePair(pairs, index)));
  };

  const removeLine = (index: number) => {
    const nextLines = lines.filter((_, i) => i !== index);
    const next = follow(pairs, nextLines);
    requestPreview(next);
    commit(next, pairs, nextLines, lines);
  };

  /** Quatre coins guides : la condition pour caler par lignes. */
  const guideCount = pairs.filter(isGuide).length;
  // Sans les quatre coins, ils sont remis au premier trait (`ensureGuides`).
  const lineMode = mode === "lines" && pairs.length >= 4;

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

  /** Le bout de ligne de ce côté sous le pointeur, ou rien. */
  const lineEndUnder = (side: Side, at: NonNullable<ReturnType<typeof pointerAt>>) => {
    const hit = hitHandle(
      lines.flatMap((l) => l[side].map((p) => at.toScreen(p))),
      at.screen,
      GRAB_RADIUS_PX,
    );
    return hit === null ? null : { line: Math.floor(hit / 2), end: (hit % 2) as 0 | 1 };
  };

  /** Ce qu'un appui attrape à cet endroit (point posé, poignée, détail de la photo). */
  const grabAt = (side: Side, at: NonNullable<ReturnType<typeof pointerAt>>) => {
    const end = lineEndUnder(side, at);
    if (end) return { kind: "lineEnd", ...end } as const;
    const hit = pointUnder(side, at);
    if (lineMode) {
      // Mode Lignes : un appui ailleurs que sur un point ou la poignée trace un trait.
      if (hit !== null) return { kind: "point", index: hit } as const;
      if (spaceHeld.current) return { kind: "none" } as const;
      if (side === "facade" && photoOnFacade && currentMesh) {
        const center = meshCenter(currentMesh);
        const c = center ? at.toScreen(center) : null;
        if (c && Math.hypot(c.x - at.screen.x, c.y - at.screen.y) <= GRAB_RADIUS_PX) return { kind: "move" } as const;
      }
      if (side === "slide" && !onSlide(at.mm)) return { kind: "none" } as const;
      return { kind: "draw" } as const;
    }
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
    settle.current.flush();
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
    else if (grab.kind === "lineEnd")
      setDragging({ kind: "lineEnd", side, line: grab.line, end: grab.end, before: ensureGuides(pairs), beforeLines: lines });
    else if (grab.kind === "draw") setDragging({ kind: "draw", side, start: at.mm, end: at.mm, before: pairs });
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
        // Un point sûr entraîne les coins guides ; un coin guide se glisse seul.
        if (!isGuide(dragging.before[dragging.index])) next = follow(next);
      } else if (dragging.kind === "move" && side === "facade") {
        next = translateFacade(dragging.before, at.mm.x - dragging.start.x, at.mm.y - dragging.start.y);
      } else if (dragging.kind === "pin" && side === "facade" && moved) {
        next = follow([...dragging.before, { slide: dragging.slide, facade: at.mm }]);
      } else if (dragging.kind === "draw" && dragging.side === side) {
        setDragging({ ...dragging, end: side === "slide" ? clampSlide(at.mm) : at.mm });
      } else if (dragging.kind === "lineEnd" && dragging.side === side) {
        const nextLines = dragging.beforeLines.map((l, i) => {
          if (i !== dragging.line) return l;
          const seg: [XYPoint, XYPoint] = [l[side][0], l[side][1]];
          seg[dragging.end] = side === "slide" ? clampSlide(at.mm) : at.mm;
          return { ...l, [side]: seg };
        });
        setLines(nextLines);
        next = follow(dragging.before, nextLines);
      }
      if (next) {
        setPairs(next);
        requestPreview(next);
      }
      return;
    }
    const kind = grabAt(side, at).kind;
    setGrabbable(ready && !spaceHeld.current && kind !== "none" && kind !== "draw");
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
      if (drag.kind === "draw") {
        const seg: [XYPoint, XYPoint] = [drag.start, drag.end];
        if (clicked || !lineDrawn(drag.start, drag.end)) return;
        if (halfLine && halfLine.side !== side) {
          // Les deux traits sont là : la ligne commune est posée.
          const line: RegistrationLine =
            side === "slide" ? { slide: seg, facade: halfLine.seg } : { slide: halfLine.seg, facade: seg };
          const nextLines = [...lines, line];
          const next = follow(ensureGuides(pairs), nextLines);
          setHalfLine(null);
          requestPreview(next);
          commit(next, pairs, nextLines, lines);
        } else {
          setHalfLine({ side, seg });
        }
        return;
      }
      if (drag.kind === "lineEnd") {
        if (clicked) {
          setLines(drag.beforeLines);
          setPairs(drag.before);
          return;
        }
        commit(pairs, drag.before, lines, drag.beforeLines);
        return;
      }
      if (drag.kind === "point") {
        setSelectedSide(drag.side);
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
    if (result.changed && (selected === null || !isGuide(pairs[selected]))) result.pairs = follow(result.pairs);
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
      setHalfLine(null);
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
  const instruction = lineMode
    ? halfLine
      ? halfLine.side === "slide"
        ? "Tracer maintenant la même ligne sur le plan : appuyer sur un bout, glisser jusqu'à l'autre."
        : "Tracer maintenant la même ligne sur la photo : appuyer sur un bout, glisser jusqu'à l'autre."
      : "Glisser sur la photo, à gauche, le long d'un bord sûr (bord de fenêtre, corniche), puis le même sur le plan, à droite."
    : hint(pending, selected, selectedSide);
  const stageClass = `registration-stage${dragging && dragging.kind !== "draw" ? " registration-dragging" : grabbable ? " registration-grab" : ""}`;

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

  // Flèches : le point choisi bouge d'un point d'écran (dix avec ⇧), du côté
  // cliqué en dernier ; la photo suit sur la façade, l'enregistrement attend
  // la fin de la rafale (D15-p).
  useArrowNudge(selected !== null && ready, (step) => {
    if (selected === null || selected >= pairs.length) return false;
    const zoom = (selectedSide === "slide" ? slideViewer : facadeViewer).current?.scene()?.getView().zoom;
    if (!zoom) return false;
    const moved = movePoint(pairs, selected, selectedSide, nudged(pairs[selected][selectedSide], step, zoom));
    const next = isGuide(pairs[selected]) ? moved : follow(moved);
    setPairs(next);
    requestPreview(next);
    settle.current.push(pairs, (before) => commit(next, before));
    return true;
  });
  useEffect(() => () => settle.current.flush(), []);

  useShortcuts([
    {
      keys: "mod+0",
      run: () => {
        slideViewer.current?.fitAll();
        facadeViewer.current?.fitAll();
      },
    },
    {
      // ⌘Z pendant un glisser : le point revient où il était (rien n'est encore enregistré).
      keys: "mod+z",
      run: () => {
        if (!dragging) return;
        setPairs(dragging.before);
        if (dragging.kind === "lineEnd") setLines(dragging.beforeLines);
        setDragging(null);
        setLive(null);
      },
      enabled: dragging !== null,
    },
    { keys: "backspace", run: () => selected !== null && remove(selected), enabled: selected !== null && ready },
  ]);

  const toolbar = (
    <Toolbar>
      <ProjectMenu
        name={project?.name ?? null}
        folder={folder}
        ready={ready && dragging === null && saving === 0}
        placeholder="Exemple sans moteur"
        onOpen={(path) => void load(path)}
        onReload={() => folder && void reload(folder, project, selectedProjector)}
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
      <div className="registration-advice">
        <Segmented<Mode>
          label="Caler par"
          value={mode}
          onChange={(value) => {
            setMode(value);
            setHalfLine(null);
            setPending(NO_PENDING);
            setSelected(null);
            // Les lignes s'appuient sur les quatre coins guides : on les remet
            // s'ils manquent (points posés un à un, coin déplacé sur la photo).
            if (value === "lines" && guideCount !== 4) {
              const next = ensureGuides(pairs);
              if (next !== pairs) {
                requestPreview(next);
                commit(next);
              }
            }
          }}
          options={[
            { value: "points", label: "Points", title: "Caler par des points sûrs : un détail de la photo lâché sur le même détail du plan" },
            { value: "lines", label: "Lignes", title: "Caler par des lignes : un bord tracé sur la photo, le même tracé sur le plan" },
          ]}
        />
        {mode === "lines" && pairs.length < 4 && (
          <p className="ui-note">Pour caler par lignes, poser d'abord la photo sur la façade, par ses quatre coins.</p>
        )}
      </div>
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
            Puis l'amener à peu près à sa place (poignée du centre, coins), et caler par des points sûrs : attraper un détail
            de la photo et le lâcher sur le même détail du plan.
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
          {ADJUST_SHOWN && (
          <Button
            variant="primary"
            icon="search"
            onClick={() => void adjustNow()}
            disabled={!engine || !ready || !photo || !facade || !projector}
            title={
              !engine
                ? "Demande l'application (le moteur n'est pas là dans un simple navigateur)"
                : pairs.length > 4
                  ? "Cale la photo sur les lignes du plan, autour de la pose actuelle ; remplace les points par les quatre coins"
                  : "Cale la photo sur les lignes du plan, autour de la pose actuelle"
            }
          >
            Ajuster sur le plan
          </Button>
          )}
          {adjustUndo && (
            <Button icon="refresh" onClick={undoAdjust} disabled={!ready} title="Remet les points d'avant l'ajustement">
              Annuler l'ajustement
            </Button>
          )}
          {mode === "points" ? (
            <p className="ui-note">
              Pour caler : sur la façade, attraper un détail sûr de la photo (coin de fenêtre, pointe d'un arc…), où qu'il soit,
              et le lâcher sur le même détail du plan. Les coins ne sont qu'un guide : ils suivent vos points. Barre d'espace +
              glisser : déplacer la vue.
            </p>
          ) : (
            <p className="ui-note">
              Tracer un bord sûr sur la photo (à gauche), puis le même sur le plan (à droite) : une verticale (bord de fenêtre),
              une horizontale (corniche)… Les deux traits n'ont pas besoin d'avoir la même longueur. Deux horizontales et deux
              verticales suffisent à retrouver le trapèze ; les points sûrs comptent aussi. Barre d'espace + glisser : déplacer
              la vue.
            </p>
          )}
          {constraintsText(pairs, lines) && <p className="ui-note">{constraintsText(pairs, lines)}</p>}
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
      {lines.length > 0 && (
        <List label="Lignes communes">
          {lines.map((l, i) => (
            <ListRow
              key={`ligne-${i}`}
              title={`Ligne ${i + 1}`}
              detail={`photo ${(Math.hypot(l.slide[1].x - l.slide[0].x, l.slide[1].y - l.slide[0].y)).toFixed(0)} mm · plan ${(Math.hypot(l.facade[1].x - l.facade[0].x, l.facade[1].y - l.facade[0].y) / 1000).toFixed(1)} m`}
              tooltip="Glisser un bout de la ligne, sur la photo ou sur le plan, pour la corriger"
              actions={<IconButton icon="trash" label={`Supprimer la ligne ${i + 1}`} size="sm" onClick={() => removeLine(i)} disabled={!ready} />}
            />
          ))}
        </List>
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
        error ? (
          <Banner tone="danger" onClose={() => setError(null)}>
            {error}
          </Banner>
        ) : (
          notice && (
            <Banner tone="success" onClose={() => setNotice(null)}>
              {notice}
            </Banner>
          )
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

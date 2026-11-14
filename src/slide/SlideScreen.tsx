// Écran Diapositive (tâche 3.6) : la diapositive calculée d'un projecteur,
// posée sur sa photo redressée, pour vérifier que les dessins tombent sur
// l'architecture. On passe d'un projecteur à l'autre d'un clic ou avec les
// flèches ← → du clavier.
//
// Les deux images sont en mm de diapositive (carré de 280 mm) : la photo
// redressée par tuiles, la diapositive calculée (aperçu de 2 048 px fait par
// le moteur, même calcul que l'export) colorée par-dessus.
//
// Un clic sur un dessin le choisit ; les outils de la barre sont ceux de la
// Façade, mais tourner, étirer, déformer, aligner agissent dans cette
// diapositive seulement (tâche 4.6, D-3.6-f, `useSlideAdjust.ts`). À
// droite, comme dans la Façade : Dessins (poser un dessin dans un
// projecteur recalé), Calques (ceux de cette diapositive), Export.

import { useCallback, useEffect, useMemo, useRef, useState, type DragEvent } from "react";
import {
  cacheFileUrl,
  chooseExportFolder,
  exportSlides,
  fetchSlideViews,
  hasEngine,
  openProject,
  openTiledImage,
  placeInSlide,
  projectFile,
  type ExportSettings,
  type Placement,
  type Project,
  type SlideContent,
  type SlideView,
} from "../engine";
import { useSlideAdjust } from "./useSlideAdjust";
import { slidePoint } from "./adjust";
import { MaskBar } from "../compose/MaskBar";
import { SlideMaskSection } from "./SlideMaskSection";
import { useSlideMask } from "./useSlideMask";
import { useMaskTool } from "../compose/useMaskTool";
import { SlideExportSection, startSettings } from "./SlideExportSection";
import { controlPlanSummary, exportSummary, parseDpi, validDpi } from "../facade/exportLogic";
import { CompositionToolbar } from "../compose/CompositionToolbar";
import { DrawingsSection } from "../compose/DrawingsSection";
import { LayersSection } from "../compose/LayersSection";
import { SlidesSection } from "../compose/SlidesSection";
import { slideTree } from "../compose/slideTree";
import { useComposition, type CompositionActions } from "../compose/useComposition";
import { draggedDrawing, droppedDrawingId, endDrawingDrag, DRAWING_DRAG_TYPE } from "../compose/dragDrawing";
import { screenToMm, visibleRect } from "../viewer/geometry";
import { recalledProject } from "../project/current";
import { Viewer, placementForWidth, type LayerSpec, type TileSource, type ViewerHandle } from "../viewer";
import { demoSlideViews, demoImages } from "./demo";
import {
  LIGHTBURN_LOOK,
  DRAWING_OPACITIES,
  cutNotice,
  nearestDrawingOpacity,
  placeHere,
  placeNote,
  slideBlocked,
  SLIDE_BACKDROPS,
  SLIDE_MM,
  SLIDE_SQUARE,
  TINTS,
  backdropLayer,
  drawingsLayer,
  isLightBurnLook,
  nextProjector,
  planLayer,
  readLook,
  saveLook,
  viewProblem,
  withBackdrop,
  type SlideLook,
} from "./logic";
import { ProjectMenu } from "../project/ProjectMenu";
import { readViewOptions, saveViewOptions } from "../facade/viewOptions";
import {
  Banner,
  Busy,
  Button,
  Dot,
  EmptyState,
  IconButton,
  List,
  MenuButton,
  ListRow,
  Panel,
  Screen,
  StatusBar,
  StatusText,
  TabbedPanel,
  ToolSeparator,
  ToolSpacer,
  Toolbar,
  useShortcuts,
} from "../ui";
import "./slide.css";

type SlideTab = "dessins" | "calques" | "export";
const SLIDE_TABS: SlideTab[] = ["dessins", "calques", "export"];
const TAB_KEY = "insitu.ongletDiapositive";
function readSlideTab(): SlideTab {
  try {
    const saved = localStorage.getItem(TAB_KEY);
    if (saved && (SLIDE_TABS as string[]).includes(saved)) return saved as SlideTab;
  } catch {
    // Stockage indisponible : onglet par défaut.
  }
  return "dessins";
}
function saveSlideTab(tab: SlideTab) {
  try {
    localStorage.setItem(TAB_KEY, tab);
  } catch {
    // Sans conséquence.
  }
}

/** Raccourcis de la composition qui gardent leur sens dans une diapositive : ordre et groupe. */
/** Le carré de la diapositive en contour (SLIDE_SQUARE est rangé ligne par ligne, pour un maillage). */
const SLIDE_OUTLINE = [SLIDE_SQUARE[0], SLIDE_SQUARE[1], SLIDE_SQUARE[3], SLIDE_SQUARE[2]];

const SLIDE_SHORTCUTS = new Set(["mod+g", "mod+shift+g", "mod+shift+]", "mod+]", "mod+[", "mod+shift+["]);

/**
 * Le réglage d'avant « Comme dans LightBurn », pour y revenir ; gardé hors
 * de l'écran pour survivre à un passage par un autre onglet.
 */
const beforeLightBurn: { look: SlideLook | null } = { look: null };

export function SlideScreen() {
  const engine = hasEngine();
  const viewer = useRef<ViewerHandle>(null);
  const stage = useRef<HTMLDivElement>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [folder, setFolder] = useState<string | null>(null);
  const [views, setViews] = useState<SlideView[]>([]);
  const [current, setCurrent] = useState<string | null>(null);
  const [photos, setPhotos] = useState<Record<string, TileSource>>({});
  // Ce qu'on regarde : photo ou non, fond, couleur des dessins ; retenu.
  const [look, setLookState] = useState<SlideLook>(readLook);
  const setLook = useCallback((next: SlideLook) => {
    setLookState(next);
    saveLook(next);
  }, []);
  const { showDrawings, opacity, tint } = look;
  const lightBurn = isLightBurnLook(look);
  const toggleLightBurn = () => {
    if (lightBurn) {
      setLook(beforeLightBurn.look ?? { ...look, photo: true });
      beforeLightBurn.look = null;
    } else {
      beforeLightBurn.look = look;
      setLook(LIGHTBURN_LOOK);
    }
  };
  // Fresque et plan : les mêmes cases que le menu Affichage de la Façade
  // (D-3.6-d, D-3.6-e) ; cachés là-bas, cachés ici, et inversement.
  const [content, setContentState] = useState<SlideContent>(() => {
    const options = readViewOptions();
    return { fresco: options.fresco, plan: options.plan };
  });
  const contentRef = useRef(content);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  /** Dans le navigateur seul : photo et dessins inventés. */
  const [demo, setDemo] = useState<{ photo: string; drawings: Record<string, string> } | null>(null);

  const load = useCallback(async (path: string) => {
    setBusy("Calcul des diapositives…");
    setError(null);
    try {
      const opened = await openProject(path);
      setFolder(path);
      setProject(opened);
      const list = await fetchSlideViews(path, contentRef.current);
      setViews(list);
      setCurrent((before) =>
        list.some((v) => v.projectorId === before)
          ? before
          : ((list.find((v) => v.preview && v.rectifiedPhoto) ?? list[0])?.projectorId ?? null),
      );
    } catch (failure) {
      setError(String(failure).replace(/^Error: /, ""));
    } finally {
      setBusy(null);
    }
  }, []);

  useEffect(() => {
    if (engine) {
      const last = recalledProject();
      if (last) void load(last);
      return;
    }
    const list = demoSlideViews();
    setViews(list);
    setCurrent(list[0].projectorId);
    setDemo(demoImages(list));
  }, [engine, load]);

  const view = views.find((v) => v.projectorId === current) ?? null;

  // Le projet a changé (retouche, ordre, dessin posé…) : l'aperçu est recalculé.
  const onProject = useCallback(
    (next: Project) => {
      setProject(next);
      if (!folder) return;
      fetchSlideViews(folder, contentRef.current).then(setViews, (failure: unknown) => setError(String(failure)));
    },
    [folder],
  );

  // Montrer ou cacher la fresque ou le plan : retenu pour la Façade aussi ;
  // l'aperçu est recalculé sans la fresque, ou avec le plan s'il manque.
  const setContent = (next: SlideContent) => {
    const before = contentRef.current;
    contentRef.current = next;
    setContentState(next);
    saveViewOptions({ ...readViewOptions(), fresco: next.fresco, plan: next.plan });
    const missingPlan = next.plan && views.some((v) => v.preview !== null && v.plan === null);
    if (!engine || !folder || (next.fresco === before.fresco && !missingPlan)) return;
    setBusy(next.plan && !before.plan ? "Calcul du plan dans les diapositives…" : "Calcul des diapositives…");
    fetchSlideViews(folder, next)
      .then(setViews, (failure: unknown) => setError(String(failure).replace(/^Error: /, "")))
      .finally(() => setBusy(null));
  };

  // La composition de la façade, sans l'afficher : bibliothèque, sélection,
  // calques, et les outils qui changent le dessin partout (miroir, ordre,
  // groupe, masquer, verrouiller).
  const composition = useComposition({
    engine,
    folder,
    project,
    onProject,
    viewer,
    stage,
    enabled: busy === null,
    headless: true,
    onError: setError,
  });
  const compositionIds = useMemo(() => new Set(composition.selected.map((p) => p.id)), [composition.selected]);
  const selectInComposition = composition.actions.selectInTree;
  const onSelect = useCallback((ids: string[]) => selectInComposition(ids), [selectInComposition]);
  // Un dessin tout juste posé ou copié : choisi dès que la composition le connaît.
  const wanted = useRef<string[] | null>(null);
  const selectLater = useCallback((ids: string[]) => {
    wanted.current = ids;
  }, []);
  useEffect(() => {
    const ids = wanted.current;
    if (!ids || !ids.every((id) => composition.placements.some((p) => p.id === id))) return;
    wanted.current = null;
    selectInComposition(ids);
  }, [composition.placements, selectInComposition]);
  const adjust = useSlideAdjust({
    engine,
    folder,
    view,
    project,
    external: compositionIds,
    onSelect,
    selectLater,
    onProject,
    viewer,
    stage,
    onError: setError,
    silhouetteOf: composition.silhouetteOf,
  });

  // Outil « Masque » (D-4.2-u) : le masque est celui du dessin posé, le même
  // que sur la façade ; on le peint ici sur la diapositive.
  const composed = composition.placements;
  const maskTargets = useMemo(
    () =>
      adjust.selected.flatMap((d) => {
        const p = composed.find((q) => q.id === d.placement);
        if (!p) return [];
        const deformation = p.adjustment && p.adjustment.projector === view?.projectorId ? p.adjustment.deformation : null;
        return [{ id: p.id, map: (u: number, v: number) => slidePoint(d, deformation, u, v), mask: p.mask ?? [] }];
      }),
    [adjust.selected, composed, view?.projectorId],
  );
  // Masque général de la diapositive (D-3.6-h) : calque « Masque » de l'onglet Calques.
  const slideMask = useSlideMask({
    engine,
    folder,
    projector: view?.projectorId ?? null,
    mask: (project?.projectors ?? []).find((q) => q.id === view?.projectorId)?.mask ?? null,
    square: SLIDE_OUTLINE,
    viewer,
    stage,
    onProject,
    onError: setError,
  });
  const maskTool = useMaskTool({
    viewer,
    stage,
    available: engine && busy === null && view !== null,
    targets: maskTargets,
    onCommit: composition.actions.setMasks,
  });

  // Tuiles de la photo redressée, gardées pour basculer sans attendre.
  useEffect(() => {
    if (!engine || !folder || !view?.rectifiedPhoto || photos[view.projectorId]) return;
    const id = view.projectorId;
    let cancelled = false;
    openTiledImage(projectFile(folder, view.rectifiedPhoto)).then(
      (source) => !cancelled && setPhotos((all) => ({ ...all, [id]: source })),
      (failure: unknown) => !cancelled && setError(String(failure)),
    );
    return () => {
      cancelled = true;
    };
  }, [engine, folder, view, photos]);

  // Flèches gauche et droite : projecteur précédent ou suivant, sauf quand
  // un dessin est choisi (les flèches le décalent alors, D15-p).
  const drawingChosen = useRef(false);
  drawingChosen.current = adjust.selected.length > 0;
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || drawingChosen.current) return;
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) return;
      const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
      if (step === 0) return;
      event.preventDefault();
      setCurrent((id) => nextProjector(views, id, step));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [views]);

  const layers = useMemo<LayerSpec[]>(() => {
    if (!view) return [];
    // Le fond remplit le carré de 280 mm ; la photo, opaque, le couvre quand elle est montrée.
    const out: LayerSpec[] = [backdropLayer(look.backdrop)];
    if (demo) {
      out.push({ id: "photo-demo", kind: "mesh-image", url: demo.photo, divisions: 1, points: SLIDE_SQUARE, visible: look.photo });
      out.push(...drawingsLayer(demo.drawings[view.projectorId] ?? null, showDrawings, opacity, tint));
      out.push(...adjust.layers);
      out.push(...slideMask.layers);
      return out;
    }
    const photo = photos[view.projectorId];
    if (photo) {
      out.push({
        id: `photo:${view.projectorId}`,
        kind: "tiles",
        source: photo,
        placement: placementForWidth(photo.pyramid.width, SLIDE_MM),
        visible: look.photo,
      });
    }
    // Le plan, repère qui n'est pas gravé : jamais « comme dans LightBurn ».
    out.push(...planLayer(view.plan ? cacheFileUrl(view.plan) : null, content.plan && !lightBurn, look.photo ? "noir" : look.backdrop));
    out.push(...drawingsLayer(view.preview ? cacheFileUrl(view.preview) : null, showDrawings, opacity, tint));
    out.push(...adjust.layers);
    out.push(...slideMask.layers);
    return out;
  }, [view, photos, demo, showDrawings, opacity, tint, look.photo, look.backdrop, adjust.layers, content.plan, lightBurn, slideMask.layers]);

  // Cadrer la diapositive à l'ouverture.
  useEffect(() => {
    const id = window.setTimeout(() => viewer.current?.fitAll(), 0);
    return () => window.clearTimeout(id);
  }, [folder, demo]);

  const problem = viewProblem(view);
  const cut = cutNotice(view);
  const ready = busy === null;

  // --- Poser un dessin (D-3.6-f) ------------------------------------------------------------

  const projectorOf = project?.projectors?.find((q) => q.id === view?.projectorId) ?? null;
  const registered = (projectorOf?.registration?.length ?? 0) >= 4;
  const placeBlocked = slideBlocked(engine, view?.label ?? null);
  const place = async (drawing: string, at?: { x: number; y: number }) => {
    if (placeBlocked || !folder || !view) return;
    const scene = viewer.current?.scene();
    const visible = scene ? visibleRect(scene.getView(), scene.getViewport()) : null;
    const { center, width } = placeHere(visible, at);
    setError(null);
    try {
      const { project: next, id } = await placeInSlide(folder, view.projectorId, drawing, center, width, composition.activeLayer);
      selectLater([id]);
      onProject(next);
    } catch (failure) {
      setError(String(failure).replace(/^Error: /, ""));
    }
  };
  const [dropping, setDropping] = useState(false);
  const carriesDrawing = (event: DragEvent) =>
    draggedDrawing() !== null || event.dataTransfer.types.includes(DRAWING_DRAG_TYPE);
  const onDragOver = (event: DragEvent<HTMLDivElement>) => {
    if (!carriesDrawing(event)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = placeBlocked ? "none" : "copy";
    if (!dropping) setDropping(true);
  };
  const onDrop = (event: DragEvent<HTMLDivElement>) => {
    setDropping(false);
    const id = droppedDrawingId(event.dataTransfer, draggedDrawing());
    endDrawingDrag();
    if (!id) return;
    event.preventDefault();
    if (placeBlocked) {
      setError(placeBlocked);
      return;
    }
    const scene = viewer.current?.scene();
    const canvas = stage.current?.querySelector(".viewer-canvas");
    if (!scene || !canvas) return;
    const box = canvas.getBoundingClientRect();
    const at = screenToMm(scene.getView(), scene.getViewport(), { x: event.clientX - box.left, y: event.clientY - box.top });
    void place(id, at);
  };

  // --- Export de cette diapositive ------------------------------------------------------------

  // Les réglages de l'onglet Export de la Façade, la fresque cochée si elle
  // se voit à l'écran ; ils deviennent ceux du projet (D-3.5-o).
  const [settings, setSettings] = useState<ExportSettings>(() => startSettings(null, content.fresco));
  const [dpiText, setDpiText] = useState(String(settings.dpi));
  useEffect(() => {
    const next = startSettings(project, content.fresco);
    setSettings(next);
    setDpiText(String(next.dpi));
    // Un autre projet, ou la fresque montrée ou cachée : on repart de là.
  }, [project?.id, Boolean(project?.fresco), content.fresco]);
  const exportBlocked = !engine
    ? "Exporter demande l'application."
    : !view
      ? "Choisir d'abord un projecteur, à gauche."
      : view.preview === null
        ? registered
          ? `${view.label} : ${view.problem ?? "diapositive non calculée"}.`
          : `${view.label} n'est pas recalé : seuls les dessins posés dans cette diapositive s'exportent. En poser d'abord (onglet Dessins).`
        : null;
  const canControlPlan =
    engine && folder !== null && ready && view !== null && !(view.problem ?? "").startsWith("recalage incomplet");
  const exportThis = async (controlPlan: boolean, chosen?: ExportSettings) => {
    if (!folder || !view) return;
    setError(null);
    setNotice(null);
    try {
      const destination = await chooseExportFolder();
      if (!destination) return;
      setBusy(controlPlan ? `Export du plan de contrôle ${view.label}…` : `Export de la diapositive ${view.label}…`);
      const used = chosen ?? settings;
      const report = await exportSlides(folder, destination, used, () => {}, [view.projectorId], controlPlan);
      setNotice(controlPlan ? controlPlanSummary(report, destination) : exportSummary(report, destination));
      if (!controlPlan) onProject(await openProject(folder));
    } catch (failure) {
      setError(String(failure).replace(/^Error: /, ""));
    } finally {
      setBusy(null);
    }
  };
  const dpi = parseDpi(dpiText);

  useShortcuts([
    { keys: "mod+0", run: () => viewer.current?.fitAll(), enabled: view !== null },
    { keys: "mod+d", run: () => adjust.actions.duplicate(), enabled: ready && adjust.selected.length > 0 },
    {
      keys: "mod+e",
      run: () => void exportThis(false, { ...settings, dpi }),
      enabled: exportBlocked === null && ready && validDpi(dpi),
    },
  ]);
  // Un outil à la fois : le tracé du masque de la diapositive ou le masque d'un dessin.
  const penOn = slideMask.pen !== null;
  const setMaskToolOn = maskTool.setOn;
  const setPen = slideMask.setPen;
  useEffect(() => {
    if (penOn) setMaskToolOn(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [penOn]);
  useEffect(() => {
    if (maskTool.on) setPen(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [maskTool.on]);
  useShortcuts(slideMask.shortcuts);
  useShortcuts(maskTool.shortcuts);
  useShortcuts(composition.shortcuts.filter((s) => SLIDE_SHORTCUTS.has(s.keys)));

  // --- Barre d'outils -----------------------------------------------------------------------

  // Les outils de la Façade ; ceux qui déplacent ou déforment agissent dans
  // cette diapositive seulement (D-3.6-f).
  const slideActions: CompositionActions = {
    ...composition.actions,
    rotateBy: adjust.actions.rotateBy,
    align: adjust.actions.align,
    distribute: adjust.actions.distribute,
    deform: adjust.actions.deform,
    setMode: adjust.actions.setMode,
  };
  // Sans moteur, la composition n'a pas les dessins de l'exemple : la barre
  // suit la sélection de la diapositive (miroir, ordre, groupe n'y font rien).
  const toolSelection: Placement[] = engine
    ? composition.selected.filter((p) => adjust.selected.some((d) => d.placement === p.id))
    : adjust.selected.map((d) => ({ id: d.placement, drawing: d.drawing }) as unknown as Placement);
  const opacityChoice = nearestDrawingOpacity(opacity);

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
      <ToolSeparator />
      <CompositionToolbar
        selected={toolSelection}
        actions={slideActions}
        enabled={ready && view !== null}
        corners={adjust.corners}
        slide={{ adjusted: adjust.adjusted }}
        mask={engine ? { on: maskTool.on, toggle: () => maskTool.setOn(!maskTool.on) } : undefined}
      />
      {maskTool.on && (
        <>
          <ToolSeparator />
          <MaskBar tool={maskTool} count={maskTargets.length} />
        </>
      )}
      <ToolSpacer />
      <Button
        active={lightBurn}
        onClick={toggleLightBurn}
        title={
          lightBurn
            ? "Revenir à l'affichage d'avant (photo, couleurs)"
            : "Voir la diapositive telle qu'elle part dans LightBurn : traits blancs sur fond noir, sans photo"
        }
      >
        Comme dans LightBurn
      </Button>
      <MenuButton
        label="Affichage"
        icon={look.photo && showDrawings ? "eye" : "eyeOff"}
        align="end"
        title="Photo redressée, fond, fresque, plan, couleur et opacité des dessins"
        items={[
          { heading: "Montrer" },
          { label: "La photo redressée", checked: look.photo, onSelect: () => setLook({ ...look, photo: !look.photo }) },
          {
            label: "La fresque (comme dans la Façade ; gravée si cochée à l'export)",
            checked: content.fresco,
            onSelect: () => setContent({ ...content, fresco: !content.fresco }),
            disabled: !ready,
          },
          {
            label: "Le plan de la façade vu par ce projecteur",
            checked: content.plan,
            onSelect: () => setContent({ ...content, plan: !content.plan }),
            disabled: !ready || lightBurn,
          },
          { label: "Les dessins", checked: showDrawings, onSelect: () => setLook({ ...look, showDrawings: !showDrawings }) },
          { separator: true },
          { heading: "Fond (sans la photo)" },
          ...SLIDE_BACKDROPS.map((b) => ({
            label: b.label,
            checked: look.backdrop === b.value,
            onSelect: () => setLook(withBackdrop(look, b.value)),
            disabled: look.photo,
          })),
          { separator: true },
          { heading: "Couleur des dessins" },
          ...TINTS.map((t) => ({ label: t.label, checked: tint === t.value, onSelect: () => setLook({ ...look, tint: t.value }) })),
          { separator: true },
          { heading: "Opacité des dessins" },
          ...DRAWING_OPACITIES.map((o) => ({
            label: `${Math.round(o * 100)} %`,
            checked: opacityChoice === o,
            onSelect: () => setLook({ ...look, opacity: o }),
          })),
        ]}
      />
      <IconButton icon="fit" label="Tout voir" shortcut="mod+0" variant="default" onClick={() => viewer.current?.fitAll()} disabled={!view} />
    </Toolbar>
  );

  const sidebar = (
    <Panel side="left" title="Projecteurs">
      {views.length > 0 ? (
        <List label="Projecteurs" onMove={(step) => setCurrent((id) => nextProjector(views, id, step))}>
          {views.map((v) => (
            <ListRow
              key={v.projectorId}
              selected={v.projectorId === current}
              onSelect={() => setCurrent(v.projectorId)}
              leading={<Dot tone={v.preview ? "success" : v.problem ? "warning" : undefined} />}
              title={v.label}
              detail={v.problem ?? "diapositive calculée"}
              tone={v.problem ? "warning" : undefined}
              tooltip={v.problem ?? undefined}
            />
          ))}
        </List>
      ) : (
        <p className="ui-list-empty">{engine && !folder ? "Pas de projet ouvert." : "Aucun projecteur."}</p>
      )}
    </Panel>
  );

  // --- Panneau de droite : Dessins, Calques, Export (de cette diapositive) ------------------

  const [tab, setTab] = useState<SlideTab>(readSlideTab);
  const chooseTab = (next: SlideTab) => {
    setTab(next);
    saveSlideTab(next);
  };
  const tree = useMemo(
    () =>
      slideTree(
        composition.placements,
        composition.composeLayers,
        (project?.projectors ?? []).map((q) => ({ id: q.id, label: q.label, registered: (q.registration?.length ?? 0) >= 4 })),
        composition.assignments,
      ).filter((g) => g.projector !== null && g.projector === view?.projectorId),
    [composition.placements, composition.composeLayers, composition.assignments, project?.projectors, view?.projectorId],
  );
  const describePlacement = useCallback(
    (p: { drawing: string }) => {
      const d = composition.drawings.find((x) => x.id === p.drawing);
      return { name: d?.name ?? p.drawing, url: d?.url };
    },
    [composition.drawings],
  );
  const layersBlocked = !engine ? "Les calques demandent l'application." : !folder ? "Pas de projet ouvert." : null;

  const inspector = (
    <TabbedPanel
      label="Propriétés de la diapositive"
      side="right"
      value={tab}
      onChange={chooseTab}
      tabs={[
        { value: "dessins", label: "Dessins", icon: "image", title: "Bibliothèque : glisser un dessin sur la diapositive, ou double-cliquer" },
        { value: "calques", label: "Calques", icon: "layers", title: "Les dessins de cette diapositive, et les calques" },
        { value: "export", label: "Export", icon: "export", title: "Exporter cette diapositive, plan de contrôle" },
      ]}
      panes={{
        dessins: (
          <>
            {!placeBlocked && view && <p className="ui-note slide-place-note">{placeNote(view.label, registered)}</p>}
            <DrawingsSection drawings={composition.drawings} blocked={placeBlocked} onPlace={(id) => void place(id)} />
          </>
        ),
        calques: (
          <>
            <SlideMaskSection tool={slideMask} blocked={engine ? layersBlocked : view ? null : "Aucun projecteur."} />
            <SlidesSection
              groups={tree}
              selected={compositionIds}
              describe={describePlacement}
              actions={composition.actions}
              blocked={layersBlocked}
            />
            {!layersBlocked && tree.length === 0 && <p className="ui-list-empty">Aucun dessin dans cette diapositive.</p>}
            <LayersSection
              layers={composition.composeLayers}
              active={composition.activeLayer}
              counts={composition.layerCounts}
              actions={composition.layerActions}
              blocked={layersBlocked}
            />
          </>
        ),
        export: (
          <SlideExportSection
            label={view?.label ?? null}
            project={project}
            blocked={exportBlocked}
            canControlPlan={canControlPlan}
            busy={!ready}
            settings={settings}
            onSettings={setSettings}
            dpiText={dpiText}
            onDpiText={setDpiText}
            onExport={(chosen) => void exportThis(false, chosen)}
            onControlPlan={() => void exportThis(true)}
          />
        ),
      }}
    />
  );

  return (
    <Screen
      className="slide"
      toolbar={toolbar}
      banner={
        error ? (
          <Banner tone="danger" onClose={() => setError(null)}>
            {error}
          </Banner>
        ) : problem ? (
          <Banner tone="warning">{problem}</Banner>
        ) : notice ? (
          <Banner tone="success" onClose={() => setNotice(null)}>
            {notice}
          </Banner>
        ) : cut ? (
          <Banner tone="warning">{cut}</Banner>
        ) : null
      }
      sidebar={sidebar}
      inspector={inspector}
      status={
        <StatusBar>
          <StatusText>
            {view
              ? lightBurn
                ? `${view.label} : diapositive telle qu'elle part dans LightBurn (280 mm)`
                : look.photo
                  ? `${view.label} : diapositive calculée sur la photo redressée (280 mm)`
                  : `${view.label} : diapositive calculée, sans la photo (280 mm)`
              : "Aucun projecteur choisi"}
          </StatusText>
          {adjust.pending && engine && <StatusText>Recalcul de la diapositive…</StatusText>}
          <StatusText push>
            {slideMask.hint ? slideMask.hint : maskTool.hint ? maskTool.hint : adjust.selected.length > 0
              ? `${adjust.hint} · cliquer dans le vide : ← → changent de projecteur`
              : `${adjust.hint ? `${adjust.hint} · ` : ""}← → : projecteur précédent ou suivant`}
          </StatusText>
        </StatusBar>
      }
    >
      <div
        className="slide-stage"
        ref={stage}
        onPointerDownCapture={(event) => slideMask.onPointerDown(event) || maskTool.onPointerDown(event) || adjust.onPointerDown(event)}
        onPointerMoveCapture={(event) => slideMask.onPointerMove(event) || maskTool.onPointerMove(event) || adjust.onPointerMove(event)}
        onPointerUpCapture={(event) => slideMask.onPointerUp(event) || maskTool.onPointerUp(event) || adjust.onPointerUp(event)}
        onDragOver={onDragOver}
        onDragLeave={() => setDropping(false)}
        onDrop={onDrop}
      >
        <Viewer ref={viewer} layers={layers} cursorUnit="mm" />
        {maskTool.overlay}
        {dropping && <div className="slide-drop-veil">{placeBlocked ?? "Déposer pour poser le dessin dans cette diapositive"}</div>}
        {engine && !folder && !busy && (
          <EmptyState icon="folder" title="Aucun projet ouvert">
            <p>Ouvrir un projet (menu Fichier du Mac, ⌘O), ou en créer un (⌘N).</p>
          </EmptyState>
        )}
        {busy && <Busy>{busy}</Busy>}
      </div>
    </Screen>
  );
}

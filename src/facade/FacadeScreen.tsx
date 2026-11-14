// Écran Façade (tâche 3.1) : ouvrir ou créer un projet, importer le plan de
// façade (PDF, image, SVG, DXF), le mettre à l'échelle par deux points et
// une cote.
//
// Tout ce qui est calcul et enregistrement passe par le moteur
// (`../engine.ts`). Dans un simple navigateur (`npm run dev`), l'écran sait
// seulement montrer un plan déjà tuilé dans `tuiles-demo/facade/` : de quoi
// contrôler l'ergonomie sans l'app ; rien n'y est enregistré.

import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent, type PointerEvent } from "react";
import { CompositionToolbar } from "../compose/CompositionToolbar";
import { DrawingsSection } from "../compose/DrawingsSection";
import { useIllustratorImport } from "../compose/IllustratorImport";
import { LayersSection } from "../compose/LayersSection";
import { PlacementBar } from "../compose/PlacementBar";
import { SlidesSection } from "../compose/SlidesSection";
import { slideTree } from "../compose/slideTree";
import { useComposition } from "../compose/useComposition";
import "../compose/compose.css";
import {
  chooseNewProjectPath,
  choosePlanFile,
  chooseProjectFolder,
  createProject,
  hasEngine,
  importDroppedDrawings,
  vectorizeDrawings,
  importFacade,
  removeFacade,
  rotateFacade,
  inspectPlan,
  openFacadeView,
  openProject,
  openTileFolder,
  setFacadeScale,
  type FacadeScale,
  type FacadeView,
  type PlanInfo,
  type Project,
  type SourcePoint,
} from "../engine";
import {
  cacheFileUrl,
  fetchFacadeOverview,
  fetchSimulation,
  openTiledImage,
  projectFile,
  type FacadeOverview,
  type Simulation,
  type SimulationAlert,
} from "../engine";
import type { TileSource } from "../viewer";
import { useExportPanel } from "./ExportPanel";
import { viewToRestore, type KeptView } from "./keepView";
import { contextTab, readFacadeTab, saveFacadeTab, type FacadeTab, type TabContext } from "./inspectorTabs";
import { frescoPlacement } from "./exportLogic";
import { rotationLabel } from "./rotation";
import { DEFAULT_DISPLAY, PHOTO_OPACITIES, changedDisplay, nearestOpacity, overviewLayers, readDisplay, saveDisplay, type OverviewDisplay } from "./overview";
import { demoOverview, demoSimulation } from "./overviewDemo";
import { OverviewPanel } from "./OverviewPanel";
import { SimulationSection } from "./SimulationSection";
import { DEFAULT_VIEW_OPTIONS, changedViewOptions, readViewOptions, saveViewOptions, toggleViewOption, type ViewOptions } from "./viewOptions";
import { alertTarget, simulationLayers } from "./simulation";
import { Viewer, type LayerSpec, type ViewerHandle } from "../viewer";
import { fitView, screenToMm } from "../viewer/geometry";
import { ScaleMarkersNode } from "./markers";
import { AddPlanDialog } from "./plans/AddPlanDialog";
import { PlansSection } from "./plans/PlansSection";
import { usePlanGroups } from "./plans/usePlanGroups";
import {
  facadePlacement,
  formatLength,
  isClick,
  mmPerSourceUnit,
  parseDistanceMm,
  projectNameFromPath,
  sourceDistance,
  showScaleMarkers,
  sourceToView,
  viewToSource,
  withProjectExtension,
} from "./scale";
import { AssemblyScreen } from "../assembly/AssemblyScreen";
import { ignoredNotice } from "../library/drop";
import { importSummary } from "../library/logic";
import { useFinderDrop } from "../library/useFinderDrop";
import { recalledProject as recalled, rememberProject as remember } from "../project/current";
import { ProjectMenu } from "../project/ProjectMenu";
import {
  Banner,
  Busy,
  Button,
  ConfirmDialog,
  DropVeil,
  Dialog,
  Dot,
  EmptyState,
  IconButton,
  MenuButton,
  Property,
  Screen,
  Section,
  StatusBar,
  TabbedPanel,
  StatusText,
  ToolSeparator,
  ToolSpacer,
  Toolbar,
  useShortcuts,
} from "../ui";
import "./facade.css";

const KIND_LABEL: Record<string, string> = { pdf: "PDF", svg: "SVG", dxf: "DXF", image: "image" };

/** Plan en attente du choix de sa page (PDF de plusieurs pages). */
interface PendingPlan {
  path: string;
  info: PlanInfo;
  page: number;
}

/** Étape de la pose des points de mise à l'échelle. */
type Picking = "off" | "a" | "b";

export function FacadeScreen() {
  const engine = hasEngine();
  const viewer = useRef<ViewerHandle>(null);
  const stage = useRef<HTMLDivElement>(null);
  const markers = useRef<ScaleMarkersNode | null>(null);
  const markerPoints = useRef<{ x: number; y: number }[]>([]);
  const pointerDown = useRef<{ x: number; y: number; t: number } | null>(null);

  const [folder, setFolder] = useState<string | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [view, setView] = useState<FacadeView | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState<PendingPlan | null>(null);
  const [picking, setPicking] = useState<Picking>("off");
  const [points, setPoints] = useState<SourcePoint[]>([]);
  const [distanceText, setDistanceText] = useState("");
  /** Échelle en mémoire seulement, dans le navigateur sans moteur. */
  const [browserScale, setBrowserScale] = useState<FacadeScale | null>(null);
  /** Vue façade (tâche 3.3) : les projecteurs recalés posés sur le plan. */
  const [overview, setOverview] = useState<FacadeOverview | null>(null);
  const [overviewBusy, setOverviewBusy] = useState(false);
  /** Photos, emprises, recouvrements, opacité, projecteurs décochés : retenus (overview.ts). */
  const [display, setDisplayState] = useState<OverviewDisplay>(() => readDisplay(engine ? recalled() : null));
  const displayFolder = useRef<string | null>(null);
  const setDisplay = useCallback((next: OverviewDisplay) => {
    setDisplayState(next);
    saveDisplay(displayFolder.current, next);
  }, []);
  /** Simulation de tous les projecteurs et ses alertes (tâche 6.1). */
  const [simulation, setSimulation] = useState<Simulation | null>(null);
  const [simulating, setSimulating] = useState(false);
  const [showSlides, setShowSlides] = useState(true);
  const [chosenAlert, setChosenAlert] = useState<number | null>(null);
  /** Fresque en une image (tâche 3.5), affichée par tuiles sur la façade. */
  const [frescoSource, setFrescoSource] = useState<TileSource | null>(null);
  /** Menu Affichage : plan, fresque, dessins posés, négatifs ; retenu d'un lancement à l'autre. */
  const [viewOptions, setViewOptionsState] = useState<ViewOptions>(readViewOptions);
  const setViewOptions = useCallback((update: (current: ViewOptions) => ViewOptions) => {
    setViewOptionsState((current) => {
      const next = update(current);
      saveViewOptions(next);
      return next;
    });
  }, []);
  const showFresco = viewOptions.fresco;
  const setShowFresco = useCallback((fresco: boolean) => setViewOptions((current) => ({ ...current, fresco })), [setViewOptions]);
  /** Assemblage de photos (tâche 6.4), en plein écran par-dessus. */
  const [assembling, setAssembling] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [removingPlan, setRemovingPlan] = useState(false);

  const facade = project?.facade ?? null;
  const scale = engine ? (facade?.scale ?? null) : browserScale;
  const k = mmPerSourceUnit(scale);

  const run = useCallback(async (label: string, task: () => Promise<void>) => {
    setBusy(label);
    setError(null);
    try {
      await task();
    } catch (failure) {
      setError(String(failure).replace(/^Error: /, ""));
    } finally {
      setBusy(null);
    }
  }, []);

  // Emprises, photos et recouvrements de tous les projecteurs.
  const loadOverview = useCallback(async (path: string) => {
    setOverviewBusy(true);
    try {
      setOverview(await fetchFacadeOverview(path));
    } catch (failure) {
      setOverview(null);
      setError(String(failure).replace(/^Error: /, ""));
    } finally {
      setOverviewBusy(false);
    }
  }, []);

  // Ouvre un projet et prépare l'affichage de sa façade.
  const load = useCallback(
    (path: string, opened?: Project) =>
      run("Ouverture du projet…", async () => {
        const next = opened ?? (await openProject(path));
        setFolder(path);
        setProject(next);
        remember(path);
        if (displayFolder.current !== path) {
          displayFolder.current = path;
          setDisplayState(readDisplay(path));
        }
        setPicking("off");
        setPoints(next.facade?.scale ? [next.facade.scale.pointA, next.facade.scale.pointB] : []);
        setDistanceText(next.facade?.scale ? formatLength(next.facade.scale.distanceMm) : "");
        setView(null);
        if (next.facade) {
          setBusy("Préparation de l'affichage du plan…");
          setView(await openFacadeView(path));
        }
        setOverview(null);
        setSimulation(null);
        setChosenAlert(null);
        void loadOverview(path);
      }),
    [run, loadOverview],
  );

  // Relit le projet (après ⌘Z, ⇧⌘Z, ⌘R) en gardant le zoom et l'endroit vus (D15-l).
  const keptView = useRef<KeptView | null>(null);
  const reload = () => {
    if (!folder) return;
    const scene = viewer.current?.scene();
    keptView.current = scene ? { view: scene.getView(), k } : null;
    void load(folder);
  };

  // Au lancement : le dernier projet dans l'app, le plan d'essai sinon.
  useEffect(() => {
    if (engine) {
      const last = recalled();
      if (last) void load(last).catch(() => remember(null));
      return;
    }
    setOverview(demoOverview());
    void openTileFolder("/tuiles/facade").then((source) => {
      if (!source) return;
      setView({
        kind: "image",
        source,
        sourceWidth: source.pyramid.width,
        sourceHeight: source.pyramid.height,
        sourceUnitsPerPixel: 1,
      });
    });
  }, [engine, load]);

  // Les repères suivent les points et l'échelle.
  // La scène de la visionneuse se crée de façon asynchrone : on attend
  // qu'elle existe pour y poser les repères.
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
      const node = new ScaleMarkersNode();
      node.setPoints(markerPoints.current);
      markers.current = node;
      remove = scene.addNode(node);
    };
    attach();
    return () => {
      cancelAnimationFrame(frame);
      markers.current = null;
      remove?.();
    };
  }, []);
  useEffect(() => {
    // Le trait AB disparaît une fois l'échelle validée (D15-r).
    markerPoints.current = showScaleMarkers(picking !== "off", points, scale) ? points.map((p) => sourceToView(p, k)) : [];
    markers.current?.setPoints(markerPoints.current);
    viewer.current?.scene()?.invalidate();
  }, [points, k, picking, scale]);

  // Un changement d'unité (mise à l'échelle) change la taille du plan dans
  // la visionneuse : on recadre.
  // Après une relecture du projet (⌘Z, ⇧⌘Z, ⌘R), la vue reste où elle
  // était (D15-l).
  useEffect(() => {
    if (!view) return;
    const restore = viewToRestore(keptView.current, k);
    keptView.current = null;
    const id = window.setTimeout(() => {
      const scene = viewer.current?.scene();
      if (restore && scene) scene.setView(restore);
      else viewer.current?.fitAll();
    }, 0);
    return () => window.clearTimeout(id);
  }, [k, view]);

  // Tuiles de la fresque, refaites quand elle change.
  const frescoPath = project?.fresco?.source ?? null;
  useEffect(() => {
    setFrescoSource(null);
    if (!engine || !folder || !frescoPath) return;
    let cancelled = false;
    openTiledImage(projectFile(folder, frescoPath)).then(
      (source) => !cancelled && setFrescoSource(source),
      (failure: unknown) => !cancelled && setError(String(failure)),
    );
    return () => {
      cancelled = true;
    };
  }, [engine, folder, frescoPath]);

  // Plans ajoutés par-dessus le plan de référence (D-PL-a à g).
  const planGroups = usePlanGroups({
    engine,
    folder,
    project,
    onProject: setProject,
    viewer,
    stage,
    negative: viewOptions.planNegative,
    scaled: k !== null,
    onError: setError,
    onNotice: setNotice,
  });

  const layers = useMemo<LayerSpec[]>(() => {
    const plan: LayerSpec[] = view
      ? [
          {
            id: "facade-plan",
            kind: "tiles",
            source: view.source,
            placement: facadePlacement(view.sourceUnitsPerPixel, k),
            visible: viewOptions.plan,
            invert: viewOptions.planNegative,
          },
        ]
      : [];
    // Les emprises sont dans le repère où le recalage a été fait : celui de
    // la visionneuse, le même que pour le plan.
    const fresco = project?.fresco;
    if (fresco && frescoSource) {
      plan.push({
        id: "fresque",
        kind: "tiles",
        source: frescoSource,
        placement: frescoPlacement(fresco, frescoSource.pyramid.width),
        visible: showFresco,
        // Le noir de la fresque laisse voir le plan.
        blend: "screen",
      });
    }
    const url = engine ? cacheFileUrl : (request: string) => request;
    // Plans ajoutés (D-PL-a) : leurs feuilles sous tout, leurs vues juste
    // au-dessus du plan de référence.
    const reference = view ? plan.slice(0, 1) : [];
    const rest = view ? plan.slice(1) : plan;
    return [
      ...planGroups.sheets,
      ...reference,
      ...planGroups.layers,
      ...rest,
      ...overviewLayers(overview, display, url),
      ...simulationLayers(overview, simulation, display, showSlides, url),
    ];
  }, [view, k, overview, display, engine, project?.fresco, frescoSource, showFresco, simulation, showSlides, viewOptions.plan, viewOptions.planNegative, planGroups.sheets, planGroups.layers]);

  // Dessins posés sur la façade (tâche 4.2) : il faut une façade à l'échelle
  // (dans le navigateur sans moteur, le plan d'essai suffit).
  const composeBlocked = !view
    ? "Importer d'abord un plan."
    : engine && !k
      ? "Mettre d'abord le plan à l'échelle (onglet Plan, à droite, rubrique Mise en plan)."
      : null;
  const composition = useComposition({
    engine,
    folder,
    project,
    onProject: setProject,
    viewer,
    stage,
    enabled: composeBlocked === null && picking === "off" && planGroups.mode === "off" && busy === null && viewOptions.drawings,
    visible: viewOptions.drawings,
    checkOverlap: viewOptions.overlap,
    onError: setError,
  });
  const viewerLayers = useMemo(() => [...layers, ...composition.layers], [layers, composition.layers]);

  // Arborescence des dessins par diapositive : il faut les affectations du moteur.
  const tree = useMemo(
    () =>
      slideTree(
        composition.placements,
        composition.composeLayers,
        (project?.projectors ?? []).map((q) => ({ id: q.id, label: q.label, registered: (q.registration?.length ?? 0) >= 4 })),
        composition.assignments,
      ),
    [composition.placements, composition.composeLayers, project?.projectors, composition.assignments],
  );
  const selectedIds = useMemo(() => new Set(composition.selected.map((p) => p.id)), [composition.selected]);
  const describePlacement = useCallback(
    (p: { drawing: string }) => {
      const d = composition.drawings.find((x) => x.id === p.drawing);
      return { name: d?.name ?? p.drawing, url: d?.url };
    },
    [composition.drawings],
  );

  // --- Simulation (tâche 6.1) ----------------------------------------------

  const simulationBlocked = !overview
    ? "Ouvrir un projet pour simuler ses projecteurs."
    : overview.projectors.every((p) => p.footprint.length < 3)
      ? "Recaler d'abord au moins un projecteur."
      : null;

  const simulate = () => {
    if (!engine) {
      if (overview) setSimulation(demoSimulation(overview));
      return;
    }
    if (!folder) return;
    setSimulating(true);
    setChosenAlert(null);
    fetchSimulation(folder)
      .then((result) => {
        setSimulation(result);
        setShowSlides(true);
      })
      .catch((failure) => setError(String(failure).replace(/^Error: /, "")))
      .finally(() => setSimulating(false));
  };

  // Une alerte mène au dessin concerné : il est choisi, et la vue le montre.
  const goToAlert = (alert: SimulationAlert, index: number) => {
    setChosenAlert(index);
    const target = alertTarget(alert);
    composition.actions.select(target.selection);
    const scene = viewer.current?.scene();
    if (scene) scene.setView(fitView(target.rect, scene.getViewport()));
  };

  // --- Actions -------------------------------------------------------------

  const newProject = () =>
    run("Création du projet…", async () => {
      const chosen = await chooseNewProjectPath();
      if (!chosen) return;
      const path = withProjectExtension(chosen);
      const created = await createProject(path, projectNameFromPath(path));
      await load(path, created);
    });

  const openExisting = () =>
    run("Ouverture du projet…", async () => {
      const chosen = await chooseProjectFolder();
      if (chosen) await load(chosen);
    });

  const importPlan = (path: string, page?: number) =>
    run("Import du plan…", async () => {
      if (!folder) return;
      const next = await importFacade(folder, path, page);
      setPending(null);
      await load(folder, next);
    });

  // Retirer le plan (D15-h) : le fichier reste dans facade/ ; ce qui est en
  // mm de façade (dessins posés, recalage, fresque) est gardé.
  const confirmRemovePlan = () =>
    run("Retrait du plan…", async () => {
      setRemovingPlan(false);
      if (!folder) return;
      const next = await removeFacade(folder);
      await load(folder, next);
      setNotice("Plan retiré du projet. Le fichier reste dans le dossier facade/ du projet.");
    });

  // Tourner le plan d'un quart de tour (D15-k) : l'échelle, les dessins
  // posés et les points de recalage tournent avec lui (moteur).
  const rotatePlan = (turns: number) =>
    run("Rotation du plan…", async () => {
      if (!folder) return;
      const next = await rotateFacade(folder, turns);
      await load(folder, next);
    });

  const choosePlan = () =>
    run("Lecture du plan…", async () => {
      const path = await choosePlanFile();
      if (!path) return;
      const info = await inspectPlan(path);
      if (info.kind === "pdf" && (info.pageCount ?? 1) > 1) {
        setPending({ path, info, page: 1 });
        return;
      }
      await importPlan(path);
    });

  const validateScale = (event: FormEvent) => {
    event.preventDefault();
    const distanceMm = parseDistanceMm(distanceText);
    if (points.length < 2) {
      setError("Placer d'abord les deux points A et B sur le plan.");
      return;
    }
    if (distanceMm === null) {
      setError("Cote illisible : écrire par exemple 12,5 m, 1250 cm ou 12500 (mm).");
      return;
    }
    const [a, b] = points;
    if (!engine) {
      setBrowserScale({ pointA: a, pointB: b, distanceMm });
      return;
    }
    void run("Enregistrement de l'échelle…", async () => {
      if (!folder) return;
      setProject(await setFacadeScale(folder, a, b, distanceMm));
      setPicking("off");
    });
  };

  const startPicking = () => {
    setPoints([]);
    setPicking("a");
    setError(null);
  };

  // --- Clics sur le plan ------------------------------------------------------

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (picking === "off" && planGroups.onPointerDown(event)) return;
    if (picking === "off" && planGroups.mode === "off" && composition.onPointerDown(event)) return;
    pointerDown.current = { x: event.clientX, y: event.clientY, t: event.timeStamp };
  };

  const onPointerUp = (event: PointerEvent<HTMLDivElement>) => {
    if (planGroups.onPointerUp(event)) return;
    if (composition.onPointerUp(event)) return;
    const down = pointerDown.current;
    pointerDown.current = null;
    if (picking === "off" || !down) return;
    if (!isClick(down, { x: event.clientX, y: event.clientY, t: event.timeStamp })) return;
    const scene = viewer.current?.scene();
    const canvas = stage.current?.querySelector(".viewer-canvas");
    if (!scene || !canvas) return;
    const box = canvas.getBoundingClientRect();
    const inScene = screenToMm(scene.getView(), scene.getViewport(), {
      x: event.clientX - box.left,
      y: event.clientY - box.top,
    });
    const point = viewToSource(inScene, k);
    if (picking === "a") {
      setPoints([point]);
      setPicking("b");
    } else {
      setPoints((previous) => [previous[0], point]);
      setPicking("off");
    }
  };

  // --- Affichage ---------------------------------------------------------------

  const unit = view ? (view.kind === "image" ? "px" : view.kind === "pdf" ? "pt" : view.kind === "svg" ? "px CSS" : "unités") : "";
  const sizeMm = k && view ? { width: view.sourceWidth * k, height: view.sourceHeight * k } : null;
  const span = points.length === 2 ? sourceDistance(points[0], points[1]) : null;
  const projectName = project?.name ?? (folder ? projectNameFromPath(folder) : null);

  const planName = facade
    ? `${facade.source.replace(/^facade\//, "")}${facade.page ? `, page ${facade.page}` : ""}`
    : view
      ? "plan d'essai"
      : null;
  const ready = busy === null;
  const canImport = engine && folder !== null && ready;

  useShortcuts([
    { keys: "mod+i", run: () => void choosePlan(), enabled: canImport && !assembling },
    { keys: "mod+0", run: () => viewer.current?.fitAll(), enabled: view !== null && !assembling },
    { keys: "escape", run: () => setPicking("off"), enabled: picking !== "off" && !assembling },
    { keys: "escape", run: () => planGroups.setMode("off"), enabled: planGroups.mode !== "off" && !assembling },
  ]);
  useShortcuts(composition.shortcuts);

  const changed = changedViewOptions(viewOptions) + changedDisplay(display);
  const toggleView = (key: keyof ViewOptions) => setViewOptions((current) => toggleViewOption(current, key));
  // Tout remontrer ; les projecteurs décochés à gauche restent cachés.
  const showEverything = () => {
    setViewOptions(() => DEFAULT_VIEW_OPTIONS);
    setDisplay({ ...DEFAULT_DISPLAY, hidden: display.hidden });
  };
  const photoOpacity = nearestOpacity(display.photoOpacity);

  const composing = composeBlocked === null && picking === "off" && ready && viewOptions.drawings;
  const toolbar = (
    <div className="facade-toolbars">
    <Toolbar>
      <ProjectMenu
        name={projectName}
        folder={folder}
        ready={ready}
        placeholder="Essai sans moteur"
        onOpen={(path) => void load(path)}
        onReload={reload}
        onError={setError}
      />
      <ToolSeparator />
      {/* Outils des dessins posés (D15-j) : agissent sur la sélection. Le
          plan et l'assemblage de photos sont dans l'onglet Plan, à droite. */}
      <CompositionToolbar
        selected={composition.selected}
        actions={composition.actions}
        enabled={composing}
        corners={composition.deforming}
      />
      <ToolSpacer />
      <MenuButton
        label={changed > 0 ? `Affichage (${changed})` : "Affichage"}
        icon={viewOptions.plan && viewOptions.fresco && viewOptions.drawings ? "eye" : "eyeOff"}
        align="end"
        title="Montrer ou cacher le plan, la fresque, les dessins, les photos et emprises des projecteurs ; passer en négatif"
        items={[
          { heading: "Montrer" },
          { label: "Le plan", checked: viewOptions.plan, onSelect: () => toggleView("plan"), disabled: !view },
          {
            label: "La fresque",
            checked: viewOptions.fresco,
            onSelect: () => toggleView("fresco"),
            disabled: !project?.fresco,
          },
          { label: "Les dessins posés", checked: viewOptions.drawings, onSelect: () => toggleView("drawings") },
          { label: "Les photos des projecteurs", checked: display.photos, onSelect: () => setDisplay({ ...display, photos: !display.photos }) },
          { label: "Les emprises", checked: display.footprints, onSelect: () => setDisplay({ ...display, footprints: !display.footprints }) },
          { label: "Les recouvrements", checked: display.overlaps, onSelect: () => setDisplay({ ...display, overlaps: !display.overlaps }) },
          { separator: true },
          { heading: "Opacité des photos" },
          ...PHOTO_OPACITIES.map((o) => ({
            label: `${Math.round(o * 100)} %`,
            checked: photoOpacity === o,
            onSelect: () => setDisplay({ ...display, photoOpacity: o }),
            disabled: !display.photos,
          })),
          { separator: true },
          { heading: "Couleurs" },
          {
            label: "Plan en négatif (traits blancs sur noir)",
            checked: viewOptions.planNegative,
            onSelect: () => toggleView("planNegative"),
            disabled: !view,
          },
          { label: "Tout en négatif (traits noirs sur blanc)", checked: viewOptions.negative, onSelect: () => toggleView("negative") },
          { separator: true },
          { heading: "Vérifier" },
          {
            label: "Chevauchement : diapositive de chaque dessin, en rouge ce qui ne sera pas projeté",
            checked: viewOptions.overlap,
            onSelect: () => toggleView("overlap"),
            disabled: !viewOptions.drawings,
          },
          { separator: true },
          { label: "Tout montrer, sans négatif", onSelect: showEverything, disabled: changed === 0 },
        ]}
      />
      <IconButton icon="fit" label="Tout voir" shortcut="mod+0" variant="default" onClick={() => viewer.current?.fitAll()} disabled={!view} />
    </Toolbar>
    {/* Réglages du dessin posé, comme le panneau Contrôle d'Illustrator (D-4.2-p). */}
    {composing && (
      <Toolbar label="Dessin posé">
        <PlacementBar
          selected={composition.selected}
          frame={composition.frame}
          drawings={composition.drawings}
          actions={composition.actions}
          layers={composition.composeLayers}
          projectors={project?.projectors ?? []}
          assignments={composition.assignments}
          enabled={composing}
          deforming={composition.deforming}
        />
      </Toolbar>
    )}
    </div>
  );

  // Inspecteur en onglets (D15-i) : un onglet par sujet, choisi par le
  // contexte (dessins choisis : Dessins ; pose de A et B : Plan) ou
  // à la main ; le dernier choisi est retenu d'un lancement à l'autre.
  const illustrator = useIllustratorImport({ folder, onProject: setProject, onFresco: () => setShowFresco(true) });
  const exportPanel = useExportPanel({
    engine,
    folder,
    project,
    onProject: setProject,
    showFresco,
    onShowFresco: setShowFresco,
    illustrator,
  });
  const [tab, setTab] = useState<FacadeTab>(readFacadeTab);
  const chooseTab = (next: FacadeTab) => {
    setTab(next);
    saveFacadeTab(next);
  };
  const tabContext: TabContext = { hasPlan: view !== null, picking: picking !== "off", selection: composition.selected.length };
  const previousContext = useRef<TabContext | null>(null);
  useEffect(() => {
    if (busy !== null) return;
    const previous = previousContext.current;
    previousContext.current = tabContext;
    setTab((current) => contextTab(previous, tabContext, current));
  }, [tabContext.hasPlan, tabContext.picking, tabContext.selection, busy]);

  // Dessins glissés depuis le Finder (D-4.1-f) : ils entrent dans la
  // bibliothèque ; lâchés sur la façade, ils y sont aussi posés, à l'endroit
  // du lâcher ; lâchés ailleurs (onglet Dessins…), l'onglet Dessins les montre.
  const dropping = useFinderDrop(engine && folder !== null && !assembling, (files, ignored, place) => {
    if (files.length === 0) {
      setError(ignoredNotice(Math.max(ignored, 1)));
      return;
    }
    const onFacade = place.target instanceof Node && (stage.current?.contains(place.target) ?? false);
    void run(files.length > 1 ? `Import de ${files.length} dessins…` : "Import du dessin…", async () => {
      if (!folder) return;
      const { project: updated, report, drawings } = await importDroppedDrawings(folder, files, () => {});
      setProject(updated);
      const entries = drawings
        .map((id) => updated.library?.find((d) => d.id === id))
        .filter((d): d is NonNullable<typeof d> => Boolean(d));
      const placed = onFacade && composition.actions.placeNew(entries, place);
      if (!placed) chooseTab("dessins");
      setNotice([importSummary(report), placed ? "Posés sur la façade." : null, ignoredNotice(ignored)].filter(Boolean).join(" "));
      if (report.added.length > 0) void vectorizeDrawings(folder, report.added, () => {}).catch(() => {});
    });
  });

  const inspector = (
    <TabbedPanel
      label="Propriétés de la façade"
      side="right"
      value={tab}
      onChange={chooseTab}
      tabs={[
        { value: "plan", label: "Plan", icon: "document", title: "Le plan de la façade, sa mise en plan (échelle) et la fresque", badge: view && !k ? "warning" : undefined },
        { value: "dessins", label: "Dessins", icon: "image", title: "Bibliothèque : les dessins à poser, classés par dossier" },
        { value: "calques", label: "Calques", icon: "layers", title: "Ranger les dessins posés" },
        { value: "export", label: "Export", icon: "export", title: "Simulation, export des diapositives, fiche de calepinage" },
      ]}
      panes={{
        plan: (
          <>
            <Section>
              <div className="facade-plan-actions">
                <Button icon="import" shortcut="mod+i" title="Importer le plan de la façade : PDF, image, SVG ou DXF" onClick={() => void choosePlan()} disabled={!canImport}>
                  {facade ? "Remplacer le plan…" : "Importer un plan…"}
                </Button>
                <Button
                  icon="photos"
                  title="Sans plan : assembler plusieurs photos de face de la façade en une seule image"
                  onClick={() => setAssembling(true)}
                  disabled={(engine && folder === null) || !ready}
                >
                  Assembler des photos…
                </Button>
              </div>
              {view ? (
                <dl className="ui-facts">
                  <dt>Fichier</dt>
                  <dd title={planName ?? undefined}>{planName}</dd>
                  <dt>Format</dt>
                  <dd>{KIND_LABEL[view.kind]}</dd>
                  <dt>Taille</dt>
                  <dd>
                    {Math.round(view.sourceWidth).toLocaleString("fr-FR")} × {Math.round(view.sourceHeight).toLocaleString("fr-FR")} {unit}
                  </dd>
                  {sizeMm && (
                    <>
                      <dt>Façade</dt>
                      <dd>
                        {formatLength(sizeMm.width)} × {formatLength(sizeMm.height)}
                      </dd>
                    </>
                  )}
                </dl>
              ) : (
                <p>Aucun plan pour l'instant.</p>
              )}
              {engine && facade && (
                <Property label="Orientation" hint={rotationLabel(facade.rotation)}>
                  <div className="facade-rotate">
                    <IconButton
                      icon="rotateLeft"
                      label="Tourner le plan d'un quart de tour à gauche"
                      variant="default"
                      size="sm"
                      onClick={() => rotatePlan(-1)}
                      disabled={!ready || picking !== "off"}
                    />
                    <IconButton
                      icon="rotateRight"
                      label="Tourner le plan d'un quart de tour à droite"
                      variant="default"
                      size="sm"
                      onClick={() => rotatePlan(1)}
                      disabled={!ready || picking !== "off"}
                    />
                  </div>
                </Property>
              )}
              {engine && facade && (
                <Button variant="danger" size="sm" icon="trash" onClick={() => setRemovingPlan(true)} disabled={!ready}>
                  Retirer le plan…
                </Button>
              )}
            </Section>
            <Section title="Mise en plan">
              <form className="facade-scale" onSubmit={validateScale}>
                <p className={k ? "facade-scale-ok" : undefined}>
                  {k ? `1 ${unit} = ${formatLength(k)}` : "Pas encore à l'échelle : cliquer deux points dont on connaît la distance réelle."}
                </p>
                <Property label="Points A et B">
                  <Button
                    icon="ruler"
                    active={picking !== "off"}
                    onClick={picking === "off" ? startPicking : () => setPicking("off")}
                    disabled={!view || !ready}
                    title="Cliquer deux points du plan dont on connaît la distance réelle"
                  >
                    {picking === "a" ? "Cliquer A…" : picking === "b" ? "Cliquer B…" : points.length === 2 ? "Reposer A et B" : "Poser A et B"}
                  </Button>
                </Property>
                <Property label="Distance A–B" hint={span !== null ? `Sur le plan : ${span.toLocaleString("fr-FR", { maximumFractionDigits: 1 })} ${unit}` : "Par exemple 12,5 m, 1250 cm ou 12500 (mm)."}>
                  <input
                    className="ui-input"
                    type="text"
                    inputMode="decimal"
                    placeholder="ex. 12,5 m"
                    value={distanceText}
                    onChange={(event) => setDistanceText(event.target.value)}
                    disabled={!view}
                    aria-label="Distance réelle entre A et B"
                  />
                </Property>
                <div className="facade-scale-actions">
                  <Button type="submit" variant="primary" disabled={!view || points.length < 2 || !ready}>
                    {k ? "Mettre à jour l'échelle" : "Valider l'échelle"}
                  </Button>
                </div>
              </form>
            </Section>
            <PlansSection
              state={planGroups}
              referenceName={facade ? planName : null}
              referenceShown={viewOptions.plan}
              onToggleReference={() => toggleView("plan")}
              blocked={
                !engine
                  ? "Ajouter des plans demande l'application."
                  : !facade
                    ? "Importer d'abord le plan de référence."
                    : !k
                      ? "Mettre d'abord le plan de référence à l'échelle (rubrique Mise en plan) : les autres plans se posent dessus."
                      : null
              }
              disabled={!ready || planGroups.busy !== null || picking !== "off"}
            />
            {exportPanel.fresco}
          </>
        ),
        dessins: (
          <>
            <DrawingsSection
              drawings={composition.drawings}
              blocked={composeBlocked ?? (viewOptions.drawings ? null : "Les dessins posés sont cachés : les montrer (menu Affichage, en haut à droite).")}
              onPlace={composition.actions.place}
            />
          </>
        ),
        calques: (
          <>
            <SlidesSection
              groups={composition.placements.length > 0 ? tree : []}
              selected={selectedIds}
              describe={describePlacement}
              actions={composition.actions}
              blocked={composeBlocked}
            />
            <LayersSection
              layers={composition.composeLayers}
              active={composition.activeLayer}
              counts={composition.layerCounts}
              actions={composition.layerActions}
              blocked={composeBlocked}
            />
          </>
        ),
        export: (
          <>
            <SimulationSection
              simulation={simulation}
              blocked={simulationBlocked}
              busy={simulating}
              onRun={simulate}
              showSlides={showSlides}
              onShowSlides={setShowSlides}
              chosen={chosenAlert}
              onAlert={goToAlert}
            />
            {exportPanel.export}
          </>
        ),
      }}
    />
  );

  const status = (
    <StatusBar>
      {view ? (
        <>
          {sizeMm ? (
            <StatusText tone="success">
              <Dot tone="success" /> À l'échelle : façade de {formatLength(sizeMm.width)} × {formatLength(sizeMm.height)}
            </StatusText>
          ) : (
            <StatusText tone="warning">
              <Dot tone="warning" /> Pas encore à l'échelle : les longueurs sont en {unit}, pas en mm.
            </StatusText>
          )}
          {span !== null && (
            <StatusText>
              A–B : {span.toLocaleString("fr-FR", { maximumFractionDigits: 1 })} {unit}
              {k ? ` = ${formatLength(span * k)}` : ""}
            </StatusText>
          )}
          {picking !== "off" ? (
            <StatusText tone="info" push>
              Cliquer le point {picking === "a" ? "A" : "B"} sur le plan (Échap : annuler)
            </StatusText>
          ) : planGroups.hint ? (
            <StatusText tone="info" push>
              {planGroups.hint}
            </StatusText>
          ) : (
            composition.hint && (
              <StatusText tone="info" push>
                {composition.hint}
              </StatusText>
            )
          )}
        </>
      ) : (
        <StatusText>{folder ? "Pas de plan" : "Pas de projet ouvert"}</StatusText>
      )}
    </StatusBar>
  );

  return (
    <Screen
      className="facade"
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
      sidebar={
        <OverviewPanel
          overview={overview}
          display={display}
          onDisplay={setDisplay}
          onReload={engine && folder ? () => void loadOverview(folder) : null}
          busy={overviewBusy}
        />
      }
      inspector={inspector}
      status={status}
    >
      <div
        className={`facade-stage${picking !== "off" ? " facade-picking" : ""}${viewOptions.negative ? " facade-negative" : ""}`}
        ref={stage}
        onPointerDownCapture={onPointerDown}
        onPointerMoveCapture={(event) => {
          if (!planGroups.onPointerMove(event)) composition.onPointerMove(event);
        }}
        onPointerUpCapture={onPointerUp}
        onDragOver={composition.onDragOver}
        onDrop={composition.onDrop}
      >
        <Viewer ref={viewer} layers={viewerLayers} />
        {composition.turn && (
          <div className="compose-turn" style={{ left: composition.turn.x + 18, top: composition.turn.y + 18 }}>
            {composition.turn.text}
          </div>
        )}
        {!view && !busy && (
          !engine ? (
            <EmptyState icon="document" title="Pas de moteur dans ce navigateur">
              <p>Ouvrir l'app pour importer un plan.</p>
              <small>
                Pour essayer l'écran ici, tuiler un plan dans <code>tuiles-demo/facade</code> (voir <code>docs/facade.md</code>).
              </small>
            </EmptyState>
          ) : !folder ? (
            <EmptyState
              icon="folder"
              title="Aucun projet ouvert"
              actions={
                <>
                  <Button variant="primary" onClick={() => void newProject()}>
                    Nouveau projet…
                  </Button>
                  <Button onClick={() => void openExisting()}>Ouvrir un projet…</Button>
                </>
              }
            >
              <p>Un projet InSitu est un dossier .insitu, partagé par Dropbox.</p>
            </EmptyState>
          ) : (
            <EmptyState
              icon="document"
              title="Pas encore de plan"
              actions={
                <Button variant="primary" icon="import" onClick={() => void choosePlan()}>
                  Importer un plan…
                </Button>
              }
            >
              <p>PDF d'architecte, image (PNG, JPEG, TIFF, HEIC), SVG ou DXF.</p>
              <small>Un DWG est à exporter d'abord en PDF ou en DXF.</small>
            </EmptyState>
          )
        )}
        {(busy ?? planGroups.busy) && <Busy>{busy ?? planGroups.busy}</Busy>}
        {dropping && (
          <DropVeil>
            {composeBlocked === null && viewOptions.drawings
              ? "Déposer pour ajouter à la bibliothèque et poser sur la façade"
              : "Déposer pour ajouter à la bibliothèque"}
          </DropVeil>
        )}
      </div>
      {assembling && (
        <AssemblyScreen
          folder={engine ? folder : null}
          project={project}
          onClose={(changed) => {
            setAssembling(false);
            if (changed) setProject(changed);
          }}
          onRendered={(next, message) => {
            setAssembling(false);
            if (folder) void load(folder, next);
            setNotice(message);
          }}
        />
      )}
      {removingPlan && (
        <ConfirmDialog
          title="Retirer le plan de la façade ?"
          confirmLabel="Retirer le plan"
          onConfirm={() => void confirmRemovePlan()}
          onCancel={() => setRemovingPlan(false)}
        >
          <p>Le plan et sa mise à l'échelle sortent du projet. Le fichier reste dans le dossier facade/ du projet.</p>
          <p>Les dessins posés, les points de recalage et la fresque sont gardés, en millimètres : ils retrouvent leur place sur le prochain plan une fois mis à l'échelle.</p>
        </ConfirmDialog>
      )}
      {planGroups.pending && (
        <AddPlanDialog pending={planGroups.pending} onConfirm={(choices) => void planGroups.actions.confirmPending(choices)} onCancel={planGroups.cancelPending} />
      )}
      {pending && (
        <Dialog
          title="Quelle page est la façade ?"
          onClose={() => setPending(null)}
          actions={
            <>
              <Button onClick={() => setPending(null)}>Annuler</Button>
              <Button variant="primary" onClick={() => void importPlan(pending.path, pending.page)}>
                Importer cette page
              </Button>
            </>
          }
        >
          <p>Ce PDF a {pending.info.pageCount} pages.</p>
          <select
            className="ui-select"
            value={pending.page}
            onChange={(event) => setPending({ ...pending, page: Number(event.target.value) })}
            aria-label="Page"
          >
            {Array.from({ length: pending.info.pageCount ?? 1 }, (_, i) => (
              <option key={i + 1} value={i + 1}>
                Page {i + 1}
              </option>
            ))}
          </select>
        </Dialog>
      )}
    </Screen>
  );
}

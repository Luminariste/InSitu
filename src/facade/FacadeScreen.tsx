// Écran Façade (tâche 3.1) : ouvrir ou créer un projet, importer le plan de
// façade (PDF, image, SVG, DXF), le mettre à l'échelle par deux points et
// une cote.
//
// Tout ce qui est calcul et enregistrement passe par le moteur
// (`../engine.ts`). Dans un simple navigateur (`npm run dev`), l'écran sait
// seulement montrer un plan déjà tuilé dans `tuiles-demo/facade/` : de quoi
// contrôler l'ergonomie sans l'app ; rien n'y est enregistré.

import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent, type PointerEvent } from "react";
import { DrawingsSection } from "../compose/DrawingsSection";
import { PlacementInspector } from "../compose/PlacementInspector";
import { useComposition } from "../compose/useComposition";
import "../compose/compose.css";
import {
  chooseNewProjectPath,
  choosePlanFile,
  chooseProjectFolder,
  createProject,
  hasEngine,
  importFacade,
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
import { cacheFileUrl, fetchFacadeOverview, openTiledImage, projectFile, type FacadeOverview } from "../engine";
import type { TileSource } from "../viewer";
import { ExportPanel } from "./ExportPanel";
import { frescoPlacement } from "./exportLogic";
import { DEFAULT_DISPLAY, overviewLayers, type OverviewDisplay } from "./overview";
import { demoOverview } from "./overviewDemo";
import { OverviewPanel } from "./OverviewPanel";
import { Viewer, type LayerSpec, type ViewerHandle } from "../viewer";
import { screenToMm } from "../viewer/geometry";
import { ScaleMarkersNode } from "./markers";
import {
  facadePlacement,
  formatLength,
  isClick,
  mmPerSourceUnit,
  parseDistanceMm,
  projectNameFromPath,
  sourceDistance,
  sourceToView,
  viewToSource,
  withProjectExtension,
} from "./scale";
import { recalledProject as recalled, rememberProject as remember } from "../project/current";
import { ProjectMenu } from "../project/ProjectMenu";
import {
  Banner,
  Busy,
  Button,
  Dialog,
  Dot,
  EmptyState,
  Panel,
  Property,
  Screen,
  Section,
  StatusBar,
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
  const [display, setDisplay] = useState<OverviewDisplay>(DEFAULT_DISPLAY);
  /** Fresque en une image (tâche 3.5), affichée par tuiles sur la façade. */
  const [frescoSource, setFrescoSource] = useState<TileSource | null>(null);
  const [showFresco, setShowFresco] = useState(true);

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
        setPicking("off");
        setPoints(next.facade?.scale ? [next.facade.scale.pointA, next.facade.scale.pointB] : []);
        setDistanceText(next.facade?.scale ? formatLength(next.facade.scale.distanceMm) : "");
        setView(null);
        if (next.facade) {
          setBusy("Préparation de l'affichage du plan…");
          setView(await openFacadeView(path));
        }
        setOverview(null);
        void loadOverview(path);
      }),
    [run, loadOverview],
  );

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
    markerPoints.current = points.map((p) => sourceToView(p, k));
    markers.current?.setPoints(markerPoints.current);
    viewer.current?.scene()?.invalidate();
  }, [points, k]);

  // Un changement d'unité (mise à l'échelle) change la taille du plan dans
  // la visionneuse : on recadre.
  useEffect(() => {
    const id = window.setTimeout(() => viewer.current?.fitAll(), 0);
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

  const layers = useMemo<LayerSpec[]>(() => {
    const plan: LayerSpec[] = view
      ? [
          {
            id: "facade-plan",
            kind: "tiles",
            source: view.source,
            placement: facadePlacement(view.sourceUnitsPerPixel, k),
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
    return [...plan, ...overviewLayers(overview, display, engine ? cacheFileUrl : (request) => request)];
  }, [view, k, overview, display, engine, project?.fresco, frescoSource, showFresco]);

  // Dessins posés sur la façade (tâche 4.2) : il faut une façade à l'échelle
  // (dans le navigateur sans moteur, le plan d'essai suffit).
  const composeBlocked = !view
    ? "Importer d'abord un plan."
    : engine && !k
      ? "Mettre d'abord le plan à l'échelle (rubrique Échelle, à droite)."
      : null;
  const composition = useComposition({
    engine,
    folder,
    project,
    onProject: setProject,
    viewer,
    stage,
    enabled: composeBlocked === null && picking === "off" && busy === null,
    onError: setError,
  });
  const viewerLayers = useMemo(() => [...layers, ...composition.layers], [layers, composition.layers]);

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
    if (picking === "off" && composition.onPointerDown(event)) return;
    pointerDown.current = { x: event.clientX, y: event.clientY, t: event.timeStamp };
  };

  const onPointerUp = (event: PointerEvent<HTMLDivElement>) => {
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
    { keys: "mod+i", run: () => void choosePlan(), enabled: canImport },
    { keys: "mod+0", run: () => viewer.current?.fitAll(), enabled: view !== null },
    { keys: "escape", run: () => setPicking("off"), enabled: picking !== "off" },
  ]);
  useShortcuts(composition.shortcuts);

  const toolbar = (
    <Toolbar>
      <ProjectMenu
        name={projectName}
        folder={folder}
        ready={ready}
        placeholder="Essai sans moteur"
        onOpen={(path) => void load(path)}
        onReload={() => folder && void load(folder)}
        onError={setError}
      />
      <ToolSeparator />
      <Button icon="import" shortcut="mod+i" title="Importer le plan de la façade" onClick={() => void choosePlan()} disabled={!canImport}>
        {facade ? "Remplacer le plan…" : "Importer un plan…"}
      </Button>
      <ToolSpacer />
      <Button icon="fit" shortcut="mod+0" title="Voir toute la façade" onClick={() => viewer.current?.fitAll()} disabled={!view}>
        Tout voir
      </Button>
    </Toolbar>
  );

  const inspector = (
    <Panel side="right" title="Plan">
      <PlacementInspector selected={composition.selected} frame={composition.frame} drawings={composition.drawings} actions={composition.actions} />
      <Section>
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
      </Section>
      <Section title="Échelle">
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
      <ExportPanel
        engine={engine}
        folder={folder}
        project={project}
        onProject={setProject}
        showFresco={showFresco}
        onShowFresco={setShowFresco}
      />
    </Panel>
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
      banner={error && <Banner tone="danger" onClose={() => setError(null)}>{error}</Banner>}
      sidebar={
        <OverviewPanel
          overview={overview}
          display={display}
          onDisplay={setDisplay}
          onReload={engine && folder ? () => void loadOverview(folder) : null}
          busy={overviewBusy}
        >
          <DrawingsSection drawings={composition.drawings} blocked={composeBlocked} onPlace={composition.actions.place} />
        </OverviewPanel>
      }
      inspector={inspector}
      status={status}
    >
      <div
        className={`facade-stage${picking !== "off" ? " facade-picking" : ""}`}
        ref={stage}
        onPointerDownCapture={onPointerDown}
        onPointerMoveCapture={composition.onPointerMove}
        onPointerUpCapture={onPointerUp}
        onDragOver={composition.onDragOver}
        onDrop={composition.onDrop}
      >
        <Viewer ref={viewer} layers={viewerLayers} />
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
        {busy && <Busy>{busy}</Busy>}
      </div>
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

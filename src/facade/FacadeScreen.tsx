// Écran Façade (tâche 3.1) : ouvrir ou créer un projet, importer le plan de
// façade (PDF, image, SVG, DXF), le mettre à l'échelle par deux points et
// une cote.
//
// Tout ce qui est calcul et enregistrement passe par le moteur
// (`../engine.ts`). Dans un simple navigateur (`npm run dev`), l'écran sait
// seulement montrer un plan déjà tuilé dans `tuiles-demo/facade/` : de quoi
// contrôler l'ergonomie sans l'app ; rien n'y est enregistré.

import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent, type PointerEvent } from "react";
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
import { cacheFileUrl, fetchFacadeOverview, type FacadeOverview } from "../engine";
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
    return [...plan, ...overviewLayers(overview, display, engine ? cacheFileUrl : (request) => request)];
  }, [view, k, overview, display, engine]);

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

  const onPointerDown = (event: PointerEvent) => {
    pointerDown.current = { x: event.clientX, y: event.clientY, t: event.timeStamp };
  };

  const onPointerUp = (event: PointerEvent) => {
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

  return (
    <main className="facade">
      <header className="facade-bar">
        <div className="facade-group">
          <span className="facade-label">Projet</span>
          <strong className="facade-project" title={folder ?? undefined}>
            {projectName ?? (engine ? "aucun" : "essai sans moteur")}
          </strong>
          <button type="button" onClick={newProject} disabled={!engine || busy !== null}>
            Nouveau…
          </button>
          <button type="button" onClick={openExisting} disabled={!engine || busy !== null}>
            Ouvrir…
          </button>
        </div>

        <div className="facade-group">
          <span className="facade-label">Plan</span>
          <span className="facade-value">
            {facade
              ? `${facade.source.replace(/^facade\//, "")}${facade.page ? `, page ${facade.page}` : ""}`
              : view
                ? "plan d'essai"
                : "aucun"}
          </span>
          <button type="button" onClick={choosePlan} disabled={!engine || !folder || busy !== null}>
            Importer un plan…
          </button>
        </div>

        <form className="facade-group facade-scale" onSubmit={validateScale}>
          <span className="facade-label">Échelle</span>
          <button
            type="button"
            className={picking !== "off" ? "actif" : ""}
            onClick={startPicking}
            disabled={!view || busy !== null}
            title="Cliquer deux points du plan dont on connaît la distance réelle"
          >
            {picking === "a" ? "Cliquer le point A…" : picking === "b" ? "Cliquer le point B…" : "Poser A et B"}
          </button>
          <label className="facade-distance">
            cote A–B
            <input
              type="text"
              inputMode="decimal"
              placeholder="ex. 12,5 m"
              value={distanceText}
              onChange={(event) => setDistanceText(event.target.value)}
              disabled={!view}
            />
          </label>
          <button type="submit" disabled={!view || points.length < 2 || busy !== null}>
            Valider
          </button>
        </form>

        <div className="facade-group facade-zoom">
          <button type="button" onClick={() => viewer.current?.fitAll()} disabled={!view}>
            Tout voir
          </button>
        </div>
      </header>

      <div className="facade-body">
        <div
          className={`facade-stage${picking !== "off" ? " facade-picking" : ""}`}
          ref={stage}
          onPointerDownCapture={onPointerDown}
          onPointerUpCapture={onPointerUp}
        >
          <Viewer ref={viewer} layers={layers} />
          {!view && !busy && (
            <div className="facade-message">
              {!engine ? (
                <p>
                  Dans un simple navigateur, le moteur n'est pas là : ouvrir l'app pour importer un plan.
                  <br />
                  <small>
                    Pour essayer l'écran ici, tuiler un plan dans <code>tuiles-demo/facade</code> (voir{" "}
                    <code>docs/facade.md</code>).
                  </small>
                </p>
              ) : !folder ? (
                <p>Créer un projet ou ouvrir un projet existant, puis importer le plan de la façade.</p>
              ) : (
                <p>
                  Importer le plan de la façade : PDF d'architecte, image (PNG, JPEG, TIFF), SVG ou DXF.
                  <br />
                  <small>Un DWG est à exporter d'abord en PDF ou en DXF.</small>
                </p>
              )}
            </div>
          )}
          {busy && <p className="facade-message">{busy}</p>}
          {pending && (
            <div className="facade-message facade-pages">
              <p>
                Ce PDF a {pending.info.pageCount} pages. Laquelle est la façade ?
              </p>
              <p>
                <select
                  value={pending.page}
                  onChange={(event) => setPending({ ...pending, page: Number(event.target.value) })}
                >
                  {Array.from({ length: pending.info.pageCount ?? 1 }, (_, i) => (
                    <option key={i + 1} value={i + 1}>
                      Page {i + 1}
                    </option>
                  ))}
                </select>{" "}
                <button type="button" onClick={() => void importPlan(pending.path, pending.page)}>
                  Importer cette page
                </button>{" "}
                <button type="button" onClick={() => setPending(null)}>
                  Annuler
                </button>
              </p>
            </div>
          )}
        </div>
        <OverviewPanel
          overview={overview}
          display={display}
          onDisplay={setDisplay}
          onReload={engine && folder ? () => void loadOverview(folder) : null}
          busy={overviewBusy}
        />
      </div>

      <footer className="facade-foot">
        {error && <span className="facade-error">{error}</span>}
        {!error && view && (
          <>
            <span>
              {KIND_LABEL[view.kind]} de {Math.round(view.sourceWidth).toLocaleString("fr-FR")} ×{" "}
              {Math.round(view.sourceHeight).toLocaleString("fr-FR")} {unit}
            </span>
            {sizeMm ? (
              <span className="facade-ok">
                À l'échelle : façade de {formatLength(sizeMm.width)} × {formatLength(sizeMm.height)} ·{" "}
                1 {unit} = {formatLength(k ?? 0)}
              </span>
            ) : (
              <span className="facade-warning">
                Pas encore à l'échelle : les longueurs affichées sont en {unit}, pas en mm.
              </span>
            )}
            {span !== null && (
              <span>
                A–B : {span.toLocaleString("fr-FR", { maximumFractionDigits: 1 })} {unit}
                {k ? ` = ${formatLength(span * k)}` : ""}
              </span>
            )}
          </>
        )}
      </footer>
    </main>
  );
}

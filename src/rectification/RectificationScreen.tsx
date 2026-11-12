// Écran Redressement (tâche 2.5) : pour chaque projecteur, importer la photo
// de la mire (HEIC compris), la faire détecter, corriger au besoin par
// quatre coins posés à la main (la grille affine ensuite), vérifier à la
// loupe, puis exporter la photo redressée en TIFF ou en PNG.
//
// Tout le calcul (détection, redressement, export) et l'enregistrement
// passent par le moteur (`../engine.ts`). Dans un simple navigateur
// (`npm run dev`), l'écran montre un exemple déjà préparé dans
// `tuiles-demo/` s'il existe (voir `docs/redressement.md`) : de quoi
// contrôler l'affichage et les poignées sans l'app ; rien n'y est calculé.

import { useCallback, useEffect, useMemo, useRef, useState, type PointerEvent, type RefObject } from "react";
import {
  addProjector,
  chooseExportPath,
  choosePhotoFile,
  chooseProjectFolder,
  exportRectified,
  fetchProjectorView,
  hasEngine,
  importProjectorPhoto,
  openProject,
  openTiledImage,
  openTileFolder,
  projectFile,
  rectifyProjector,
  type Project,
  type Projector,
  type RectificationView,
  type XY,
} from "../engine";
import { Viewer, placementForWidth, type LayerSpec, type Point, type TileSource, type ViewerHandle } from "../viewer";
import { mmToScreen, screenToMm } from "../viewer/geometry";
import { recalledProject, rememberProject } from "../project/current";
import {
  EXPORT_DPI_CHOICES,
  defaultCorners,
  exportFileName,
  fromXY,
  hitHandle,
  isConvexQuad,
  orderCorners,
  nextProjectorLabel,
  nodeQuality,
  shortestSide,
  slidePixels,
  summarize,
  toXY,
  type Corners,
} from "./logic";
import { Loupe } from "./Loupe";
import { RectificationOverlay, type OverlayContent } from "./overlay";
import "./rectification.css";

type Mode = "brute" | "redressee";

/** Grossissement de la loupe : pixels d'écran par pixel de photo. */
const LOUPE_ZOOM = 4;
/** Côté de la loupe, en pixels d'écran. */
const LOUPE_SIZE = 240;
/** Distance à laquelle on saisit une poignée, en pixels d'écran. */
const HANDLE_RADIUS = 16;
/** Côté minimal du quadrilatère des coins, en pixels de photo. */
const MIN_SIDE_PX = 100;

const SLIDE_MM = 280;

function projectorStatus(p: Projector): string {
  if (p.rectification) return p.rectification.method === "automatic" ? "redressé" : "redressé (coins)";
  if (p.rawPhoto) return "photo à redresser";
  return "sans photo";
}

/** Installe un objet de repères dans la scène d'une visionneuse, dès qu'elle existe. */
function useOverlay(viewer: RefObject<ViewerHandle | null>) {
  const overlay = useRef<RectificationOverlay | null>(null);
  const content = useRef<OverlayContent | null>(null);
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
      const node = new RectificationOverlay();
      if (content.current) node.set(content.current);
      overlay.current = node;
      remove = scene.addNode(node);
    };
    attach();
    return () => {
      cancelAnimationFrame(frame);
      overlay.current = null;
      remove?.();
    };
  }, [viewer]);
  return useCallback(
    (next: OverlayContent) => {
      content.current = next;
      overlay.current?.set(next);
      viewer.current?.scene()?.invalidate();
    },
    [viewer],
  );
}

export function RectificationScreen() {
  const engine = hasEngine();
  const viewer = useRef<ViewerHandle>(null);
  const stage = useRef<HTMLDivElement>(null);
  const setOverlay = useOverlay(viewer);

  const [folder, setFolder] = useState<string | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [view, setView] = useState<RectificationView | null>(null);
  const [rawSource, setRawSource] = useState<TileSource | null>(null);
  const [rectifiedSource, setRectifiedSource] = useState<TileSource | null>(null);
  const [mode, setMode] = useState<Mode>("brute");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [corners, setCorners] = useState<Corners | null>(null);
  const [dragging, setDragging] = useState<number | null>(null);
  const [loupeCenter, setLoupeCenter] = useState<Point | null>(null);
  const [dpi, setDpi] = useState<number>(500);
  const [format, setFormat] = useState<"tif" | "png">("tif");

  const projector = project?.projectors.find((p) => p.id === selected) ?? null;
  const photoSize = rawSource ? { width: rawSource.pyramid.width, height: rawSource.pyramid.height } : null;

  const run = useCallback(async (label: string, task: () => Promise<void>) => {
    setBusy(label);
    setError(null);
    setNotice(null);
    try {
      await task();
    } catch (failure) {
      setError(String(failure).replace(/^Error: /, ""));
    } finally {
      setBusy(null);
    }
  }, []);

  // --- Chargement ---------------------------------------------------------

  /** Prépare l'affichage d'un projecteur : sa photo, son redressement. */
  const showProjector = useCallback(async (dir: string, p: Projector | null) => {
    setEditing(false);
    setCorners(null);
    setView(null);
    setRawSource(null);
    setRectifiedSource(null);
    if (!p?.rawPhoto) return;
    setBusy("Préparation de l'affichage de la photo…");
    setRawSource(await openTiledImage(projectFile(dir, p.rawPhoto)));
    if (p.rectification) setView(await fetchProjectorView(dir, p.id));
    if (p.rectifiedPhoto) setRectifiedSource(await openTiledImage(projectFile(dir, p.rectifiedPhoto)));
  }, []);

  const load = useCallback(
    (path: string, keep?: string | null) =>
      run("Ouverture du projet…", async () => {
        const opened = await openProject(path);
        setFolder(path);
        setProject(opened);
        rememberProject(path);
        const chosen =
          opened.projectors.find((p) => p.id === keep) ??
          opened.projectors.find((p) => p.rawPhoto) ??
          opened.projectors[0] ??
          null;
        setSelected(chosen?.id ?? null);
        setMode("brute");
        await showProjector(path, chosen);
      }),
    [run, showProjector],
  );

  // Au lancement : le dernier projet dans l'app, l'exemple sinon.
  useEffect(() => {
    if (engine) {
      const last = recalledProject();
      if (last) void load(last);
      return;
    }
    void (async () => {
      const raw = await openTileFolder("/tuiles/redressement-brute");
      if (!raw) return;
      setRawSource(raw);
      setRectifiedSource(await openTileFolder("/tuiles/redressement-redressee"));
      const response = await fetch("/tuiles/redressement-vue.json").catch(() => null);
      if (response?.ok) setView((await response.json()) as RectificationView);
    })();
  }, [engine, load]);

  // --- Repères et loupe -------------------------------------------------------

  const nodesInView = useMemo(() => {
    if (!view) return [];
    return view.nodes.map((n) => {
      const [x, y] = mode === "brute" ? n.photo : n.slide;
      return { x, y, quality: nodeQuality(n.errorMm) };
    });
  }, [view, mode]);

  useEffect(() => {
    const handles = editing && mode === "brute" ? corners : null;
    setOverlay({
      nodes: nodesInView,
      outline: view && mode === "brute" && !editing ? view.outline.map(fromXY) : null,
      handles,
      activeHandle: dragging,
    });
  }, [nodesInView, view, mode, editing, corners, dragging, setOverlay]);

  // Changement d'image : on recadre.
  useEffect(() => {
    const id = window.setTimeout(() => viewer.current?.fitAll(), 0);
    return () => window.clearTimeout(id);
  }, [mode, rawSource, rectifiedSource]);

  const layers = useMemo<LayerSpec[]>(() => {
    if (mode === "brute" && rawSource) {
      // Un pixel de photo par unité de la scène : les positions sont en px.
      return [{ id: "photo-brute", kind: "tiles", source: rawSource, placement: { x: 0, y: 0, mmPerPixel: 1 } }];
    }
    if (mode === "redressee" && rectifiedSource) {
      return [
        {
          id: "photo-redressee",
          kind: "tiles",
          source: rectifiedSource,
          placement: placementForWidth(rectifiedSource.pyramid.width, SLIDE_MM),
        },
      ];
    }
    return [];
  }, [mode, rawSource, rectifiedSource]);

  // --- Actions -----------------------------------------------------------------

  const openExisting = () =>
    run("Ouverture du projet…", async () => {
      const chosen = await chooseProjectFolder();
      if (chosen) await load(chosen);
    });

  const choose = (id: string) =>
    run("Ouverture du projecteur…", async () => {
      if (!folder || !project) return;
      setSelected(id);
      setMode("brute");
      await showProjector(folder, project.projectors.find((p) => p.id === id) ?? null);
    });

  const newProjector = () =>
    run("Ajout d'un projecteur…", async () => {
      if (!folder || !project) return;
      const label = nextProjectorLabel(project.projectors.map((p) => p.label));
      const { project: next, id } = await addProjector(folder, label);
      setProject(next);
      setSelected(id);
      await showProjector(folder, next.projectors.find((p) => p.id === id) ?? null);
    });

  /** Détection, automatique ou à partir des coins ; en cas d'échec, propose les coins. */
  const detect = async (dir: string, id: string, from: Corners | null, size: { width: number; height: number } | null) => {
    setBusy(from ? "Détection à partir des quatre coins…" : "Détection de la mire…");
    try {
      const { project: next, view: detected } = await rectifyProjector(dir, id, from ? (from.map(toXY) as [XY, XY, XY, XY]) : null);
      setProject(next);
      setView(detected);
      setEditing(false);
      setMode("brute");
      const p = next.projectors.find((q) => q.id === id);
      if (p?.rectifiedPhoto) setRectifiedSource(await openTiledImage(projectFile(dir, p.rectifiedPhoto)));
    } catch (failure) {
      const message = String(failure).replace(/^Error: /, "");
      if (from || !size) throw failure;
      // La recherche automatique a échoué : on passe à la pose des coins.
      setError(`${message}. Glisser les quatre poignées sur les coins de la mire, puis « Détecter depuis les coins ».`);
      setCorners(defaultCorners(size.width, size.height));
      setEditing(true);
    }
  };

  const importPhoto = () =>
    run("Import de la photo…", async () => {
      if (!folder || !selected) return;
      const path = await choosePhotoFile();
      if (!path) return;
      setBusy("Copie de la photo dans le projet…");
      const next = await importProjectorPhoto(folder, selected, path);
      setProject(next);
      const p = next.projectors.find((q) => q.id === selected) ?? null;
      await showProjector(folder, p);
      const raw = p?.rawPhoto ? await openTiledImage(projectFile(folder, p.rawPhoto)) : null;
      await detect(folder, selected, null, raw ? { width: raw.pyramid.width, height: raw.pyramid.height } : null);
    });

  const detectAgain = () =>
    run("Détection de la mire…", async () => {
      if (!folder || !selected) return;
      await detect(folder, selected, null, photoSize);
    });

  const startCorners = () => {
    if (!photoSize) return;
    setMode("brute");
    setError(null);
    setCorners(view ? (view.corners.map(fromXY) as Corners) : defaultCorners(photoSize.width, photoSize.height));
    setEditing(true);
  };

  const cancelCorners = () => {
    setEditing(false);
    setDragging(null);
    setLoupeCenter(null);
  };

  const applyCorners = () => {
    if (!corners) return;
    // Les poignées ont pu être glissées dans n'importe quel ordre.
    const ordered = orderCorners(corners) as Corners;
    if (!isConvexQuad(ordered)) {
      setError("Les quatre poignées ne forment pas un quadrilatère : poser chacune sur un coin différent de la mire.");
      return;
    }
    if (shortestSide(ordered) < MIN_SIDE_PX) {
      setError("Les coins sont trop rapprochés : les poser sur les quatre coins de la mire.");
      return;
    }
    if (!engine) {
      setNotice("Dans un simple navigateur, la détection n'est pas disponible : ouvrir l'app.");
      return;
    }
    void run("Détection à partir des quatre coins…", async () => {
      if (!folder || !selected) return;
      await detect(folder, selected, ordered, photoSize);
      setLoupeCenter(null);
    });
  };

  const exportImage = () =>
    run("Export de la photo redressée…", async () => {
      if (!folder || !projector) return;
      const destination = await chooseExportPath(exportFileName(projector.label, dpi, format));
      if (!destination) return;
      setBusy(`Export à ${dpi} dpi (${slidePixels(dpi).toLocaleString("fr-FR")} px de côté)…`);
      const done = await exportRectified(folder, projector.id, destination, dpi);
      const mo = (done.bytes / 1_000_000).toLocaleString("fr-FR", { maximumFractionDigits: 1 });
      const s = done.seconds.toLocaleString("fr-FR", { maximumFractionDigits: 1 });
      setNotice(`Exporté : ${done.path} (${done.sizePx.toLocaleString("fr-FR")} px, ${mo} Mo, ${s} s)`);
    });

  // --- Poignées et clics sur la photo ----------------------------------------

  /** Position du pointeur dans la scène (pixels de photo en vue brute). */
  const scenePoint = (event: PointerEvent): { screen: Point; scene: Point } | null => {
    const scene = viewer.current?.scene();
    const canvas = stage.current?.querySelector(".rectification-main .viewer-canvas");
    if (!scene || !canvas) return null;
    const box = canvas.getBoundingClientRect();
    const screen = { x: event.clientX - box.left, y: event.clientY - box.top };
    return { screen, scene: screenToMm(scene.getView(), scene.getViewport(), screen) };
  };

  const clampToPhoto = (p: Point): Point =>
    photoSize
      ? { x: Math.min(Math.max(p.x, 0), photoSize.width), y: Math.min(Math.max(p.y, 0), photoSize.height) }
      : p;

  const onPointerDown = (event: PointerEvent) => {
    if (!editing || mode !== "brute" || !corners) return;
    const scene = viewer.current?.scene();
    const at = scenePoint(event);
    if (!scene || !at) return;
    const handles = corners.map((c) => mmToScreen(scene.getView(), scene.getViewport(), c));
    const hit = hitHandle(handles, at.screen, HANDLE_RADIUS);
    if (hit === null) return;
    // La poignée est saisie : la visionneuse ne doit pas se déplacer.
    event.stopPropagation();
    event.preventDefault();
    stage.current?.setPointerCapture(event.pointerId);
    setDragging(hit);
    setLoupeCenter(corners[hit]);
  };

  const onPointerMove = (event: PointerEvent) => {
    if (!editing || mode !== "brute") return;
    const at = scenePoint(event);
    if (!at) return;
    if (dragging !== null && corners) {
      const moved = [...corners] as Corners;
      moved[dragging] = clampToPhoto(at.scene);
      setCorners(moved);
      setLoupeCenter(moved[dragging]);
    } else {
      setLoupeCenter(at.scene);
    }
  };

  const onPointerUp = (event: PointerEvent) => {
    if (dragging === null) return;
    stage.current?.releasePointerCapture(event.pointerId);
    setDragging(null);
  };

  // --- Affichage ---------------------------------------------------------------

  const ready = busy === null;
  const summary = view ? summarize(view) : null;
  const projectName = project?.name ?? (engine ? "aucun" : "exemple sans moteur");

  let hint: string | null = null;
  if (editing) {
    hint =
      "Glisser chaque poignée sur un coin de la mire, dans n'importe quel ordre (la loupe aide à viser), puis « Détecter depuis les coins ». Le sens de la mire est retrouvé tout seul, et la grille affine ensuite.";
  }

  return (
    <main className="rectification">
      <header className="rectification-bar">
        <div className="rectification-group">
          <span className="rectification-label">Projet</span>
          <strong className="rectification-value" title={folder ?? undefined}>
            {projectName}
          </strong>
          <button type="button" onClick={openExisting} disabled={!engine || !ready}>
            Ouvrir…
          </button>
        </div>

        <div className="rectification-group">
          <span className="rectification-label">Projecteur</span>
          <select
            value={selected ?? ""}
            onChange={(event) => void choose(event.target.value)}
            disabled={!project || project.projectors.length === 0 || !ready}
          >
            {project?.projectors.length ? null : <option value="">aucun</option>}
            {project?.projectors.map((p) => (
              <option key={p.id} value={p.id}>
                {p.label} — {projectorStatus(p)}
              </option>
            ))}
          </select>
          <button type="button" onClick={newProjector} disabled={!folder || !ready}>
            Ajouter
          </button>
          <button type="button" onClick={importPhoto} disabled={!projector || !ready}>
            Importer la photo…
          </button>
        </div>

        <div className="rectification-group">
          <span className="rectification-label">Mire</span>
          <button type="button" onClick={detectAgain} disabled={!engine || !projector?.rawPhoto || !ready || editing}>
            Détecter
          </button>
          {!editing ? (
            <button type="button" onClick={startCorners} disabled={!photoSize || !ready}>
              Poser les coins
            </button>
          ) : (
            <>
              <button type="button" className="actif" onClick={applyCorners} disabled={!ready}>
                Détecter depuis les coins
              </button>
              <button type="button" onClick={cancelCorners} disabled={!ready}>
                Annuler
              </button>
            </>
          )}
        </div>

        <div className="rectification-group">
          <span className="rectification-label">Voir</span>
          <div className="rectification-switch" role="group" aria-label="Photo affichée">
            <button type="button" className={mode === "brute" ? "actif" : ""} onClick={() => setMode("brute")}>
              Photo brute
            </button>
            <button
              type="button"
              className={mode === "redressee" ? "actif" : ""}
              onClick={() => {
                cancelCorners();
                setMode("redressee");
              }}
              disabled={!rectifiedSource}
            >
              Photo redressée
            </button>
          </div>
          <button type="button" onClick={() => viewer.current?.fitAll()} disabled={layers.length === 0}>
            Tout voir
          </button>
        </div>

        <div className="rectification-group rectification-export">
          <span className="rectification-label">Export</span>
          <select value={dpi} onChange={(event) => setDpi(Number(event.target.value))} aria-label="Résolution">
            {EXPORT_DPI_CHOICES.map((d) => (
              <option key={d} value={d}>
                {d} dpi ({slidePixels(d).toLocaleString("fr-FR")} px)
              </option>
            ))}
          </select>
          <select value={format} onChange={(event) => setFormat(event.target.value as "tif" | "png")} aria-label="Format">
            <option value="tif">TIFF</option>
            <option value="png">PNG</option>
          </select>
          <button type="button" onClick={exportImage} disabled={!engine || !projector?.rectification || !ready}>
            Exporter…
          </button>
        </div>
      </header>

      <div
        className={`rectification-stage${editing ? " rectification-editing" : ""}${dragging !== null ? " rectification-dragging" : ""}`}
        ref={stage}
        onPointerDownCapture={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={() => dragging === null && setLoupeCenter(null)}
      >
        <Viewer ref={viewer} className="rectification-main" layers={layers} cursorUnit={mode === "brute" ? "px" : "mm"} />
        {editing && mode === "brute" && loupeCenter && rawSource && (
          <div className="rectification-loupe" aria-hidden="true">
            <Loupe source={rawSource} center={loupeCenter} zoom={LOUPE_ZOOM} size={LOUPE_SIZE} handles={corners ?? []} />
          </div>
        )}
        {layers.length === 0 && !busy && (
          <div className="rectification-message">
            {!engine ? (
              <p>
                Dans un simple navigateur, le moteur n'est pas là : ouvrir l'app pour redresser une photo.
                <br />
                <small>
                  Pour essayer l'écran ici, préparer l'exemple dans <code>tuiles-demo/</code> (voir{" "}
                  <code>docs/redressement.md</code>).
                </small>
              </p>
            ) : !folder ? (
              <p>Ouvrir un projet (ou en créer un dans l'écran Façade), puis ajouter un projecteur.</p>
            ) : !projector ? (
              <p>Ajouter un projecteur, puis importer la photo de sa mire.</p>
            ) : (
              <p>
                Importer la photo de la mire du projecteur {projector.label} : HEIC, JPEG, PNG ou TIFF.
                <br />
                <small>La mire est cherchée dès l'import.</small>
              </p>
            )}
          </div>
        )}
        {busy && <p className="rectification-message">{busy}</p>}
      </div>

      <footer className="rectification-foot">
        {error && <span className="rectification-error">{error}</span>}
        {!error && hint && <span className="rectification-hint">{hint}</span>}
        {!error && !hint && notice && <span className="rectification-ok">{notice}</span>}
        {!error && !hint && !notice && summary && (
          <span className={`rectification-quality-${summary.quality}`}>{summary.text}</span>
        )}
        {!error && !hint && !notice && view && (
          <span className="rectification-legend">
            <i className="pastille bon" /> &lt; 0,15 mm <i className="pastille moyen" /> &lt; 0,3 mm{" "}
            <i className="pastille mauvais" /> au-delà
          </span>
        )}
      </footer>
    </main>
  );
}

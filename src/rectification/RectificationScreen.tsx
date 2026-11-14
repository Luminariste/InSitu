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
  exportRectified,
  fetchProjectorView,
  hasEngine,
  importProjectorPhoto,
  openProject,
  openTiledImage,
  openTileFolder,
  projectFile,
  rectifyProjector,
  mirrorProjector,
  removeProjector,
  rotateProjector,
  renameProjector,
  type Project,
  type Projector,
  type RectificationView,
  type XY,
} from "../engine";
import { Viewer, nudged, placementForWidth, useArrowNudge, type LayerSpec, type Point, type TileSource, type ViewerHandle } from "../viewer";
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
import { ProjectMenu } from "../project/ProjectMenu";
import {
  Banner,
  Busy,
  Button,
  ConfirmDialog,
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
  ToolSeparator,
  ToolSpacer,
  Toolbar,
  useShortcuts,
} from "../ui";
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
  /** Dernière poignée saisie : les flèches la déplacent (⇧ : pas plus grand). */
  const [chosen, setChosen] = useState<number | null>(null);
  useEffect(() => {
    if (!editing) setChosen(null);
  }, [editing]);
  const [loupeCenter, setLoupeCenter] = useState<Point | null>(null);
  const [dpi, setDpi] = useState<number>(500);
  const [format, setFormat] = useState<"tif" | "png">("tif");
  /** Projecteur en cours de renommage, et le nom tapé. */
  const [renaming, setRenaming] = useState<string | null>(null);
  const [renameText, setRenameText] = useState("");
  /** Projecteur dont on demande la suppression (confirmation ouverte). */
  const [removing, setRemoving] = useState<string | null>(null);

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
    // Coins seuls : aucun croisement mesuré, rien à colorer.
    if (!view || view.gridUnread) return [];
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
      activeHandle: dragging ?? chosen,
    });
  }, [nodesInView, view, mode, editing, corners, dragging, chosen, setOverlay]);

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
      setError(
        `${message}. Glisser les quatre poignées sur les coins de la mire, puis « Détecter depuis les coins ». Si la grille ne se lit pas, la photo sera redressée par les coins seuls.`,
      );
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

  /** Miroir de la diapositive : gauche-droite ou haut-bas, à bascule. */
  const mirror = (axis: "horizontal" | "vertical") =>
    run("Miroir de la photo redressée…", async () => {
      if (!folder || !selected) return;
      const { project: next, view: mirrored } = await mirrorProjector(folder, selected, {
        horizontal: axis === "horizontal",
        vertical: axis === "vertical",
      });
      setProject(next);
      setView(mirrored);
      const p = next.projectors.find((q) => q.id === selected);
      if (p?.rectifiedPhoto) setRectifiedSource(await openTiledImage(projectFile(folder, p.rectifiedPhoto)));
      setMode("redressee");
    });

  /** Quart de tour de la diapositive, pour la mettre dans le sens de lecture (D-2.5-m). */
  const rotate = (turns: 1 | -1) =>
    run("Quart de tour de la photo redressée…", async () => {
      if (!folder || !selected) return;
      const { project: next, view: turned } = await rotateProjector(folder, selected, turns);
      setProject(next);
      setView(turned);
      const p = next.projectors.find((q) => q.id === selected);
      if (p?.rectifiedPhoto) setRectifiedSource(await openTiledImage(projectFile(folder, p.rectifiedPhoto)));
      setMode("redressee");
    });

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
    setChosen(hit);
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

  // Flèches : la poignée choisie bouge d'un point d'écran (dix avec ⇧).
  useArrowNudge(editing && mode === "brute" && chosen !== null, (step) => {
    const zoom = viewer.current?.scene()?.getView().zoom;
    if (chosen === null || !corners || !zoom) return false;
    const moved = [...corners] as Corners;
    moved[chosen] = clampToPhoto(nudged(corners[chosen], step, zoom));
    setCorners(moved);
    setLoupeCenter(moved[chosen]);
    return true;
  });

  // --- Affichage ---------------------------------------------------------------

  const ready = busy === null;
  const summary = view ? summarize(view) : null;

  let hint: string | null = null;
  if (editing) {
    hint =
      "Glisser chaque poignée sur un coin de la mire, dans n'importe quel ordre (la loupe aide à viser), puis « Détecter depuis les coins ». Une poignée cliquée se règle aussi aux flèches (⇧ : pas plus grand). Le sens de la mire est retrouvé tout seul, et la grille affine ensuite.";
  }

  const projectors = project?.projectors ?? [];
  const moveSelection = (step: -1 | 1) => {
    const at = projectors.findIndex((p) => p.id === selected);
    const next = projectors[Math.min(projectors.length - 1, Math.max(0, at + step))];
    if (next && next.id !== selected && ready) void choose(next.id);
  };
  const canExport = engine && !!projector?.rectification && ready;

  useShortcuts([
    { keys: "mod+i", run: () => void importPhoto(), enabled: !!projector && ready },
    { keys: "mod+e", run: () => void exportImage(), enabled: canExport },
    { keys: "mod+0", run: () => viewer.current?.fitAll(), enabled: layers.length > 0 },
    { keys: "escape", run: cancelCorners, enabled: editing },
    { keys: "backspace", run: () => selected && setRemoving(selected), enabled: engine && !!selected && ready && !editing },
  ]);

  const toolbar = (
    <Toolbar>
      <ProjectMenu
        name={project?.name ?? null}
        folder={folder}
        ready={ready}
        onOpen={(path) => void load(path)}
        onReload={() => folder && void load(folder, selected)}
        onError={setError}
      />
      <ToolSeparator />
      <Button icon="import" shortcut="mod+i" title="Importer la photo de la mire du projecteur choisi" onClick={() => void importPhoto()} disabled={!projector || !ready}>
        Importer la photo…
      </Button>
      <ToolGroup label="Mire">
        {!editing ? (
          <>
            <Button icon="target" onClick={() => void detectAgain()} disabled={!engine || !projector?.rawPhoto || !ready}>
              Détecter
            </Button>
            <Button icon="corners" onClick={startCorners} disabled={!photoSize || !ready}>
              Poser les coins
            </Button>
          </>
        ) : (
          <>
            <Button variant="primary" onClick={applyCorners} disabled={!ready}>
              Détecter depuis les coins
            </Button>
            <Button variant="quiet" onClick={cancelCorners} disabled={!ready} shortcut="escape" title="Abandonner les coins">
              Annuler
            </Button>
          </>
        )}
      </ToolGroup>
      <ToolSpacer />
      <Segmented
        label="Photo affichée"
        value={mode}
        onChange={(next) => {
          if (next === "redressee") cancelCorners();
          setMode(next);
        }}
        options={[
          { value: "brute", label: "Photo brute" },
          { value: "redressee", label: "Photo redressée", disabled: !rectifiedSource },
        ]}
      />
      <Button icon="fit" shortcut="mod+0" title="Voir toute la photo" onClick={() => viewer.current?.fitAll()} disabled={layers.length === 0}>
        Tout voir
      </Button>
      <ToolSeparator />
      <ToolGroup label="Miroir">
        <Button
          active={!!view?.mirror?.horizontal}
          onClick={() => void mirror("horizontal")}
          disabled={!engine || !view || !ready || editing}
          title="Échanger la gauche et la droite de la diapositive (photo redressée, export et points du recalage)"
        >
          ↔ Gauche-droite
        </Button>
        <Button
          active={!!view?.mirror?.vertical}
          onClick={() => void mirror("vertical")}
          disabled={!engine || !view || !ready || editing}
          title="Échanger le haut et le bas de la diapositive (photo redressée, export et points du recalage)"
        >
          ↕ Haut-bas
        </Button>
      </ToolGroup>
      <ToolGroup label="Tourner">
        <Button
          onClick={() => void rotate(-1)}
          disabled={!engine || !view || !ready || editing}
          title="Tourner la diapositive d'un quart de tour vers la gauche (photo redressée, export et points du recalage)"
        >
          ↺ 90° à gauche
        </Button>
        <Button
          onClick={() => void rotate(1)}
          disabled={!engine || !view || !ready || editing}
          title="Tourner la diapositive d'un quart de tour vers la droite (photo redressée, export et points du recalage)"
        >
          ↻ 90° à droite
        </Button>
      </ToolGroup>
      <ToolSeparator />
      <ToolGroup label="Export">
        <select className="ui-select" value={dpi} onChange={(event) => setDpi(Number(event.target.value))} aria-label="Résolution">
          {EXPORT_DPI_CHOICES.map((d) => (
            <option key={d} value={d}>
              {d} dpi ({slidePixels(d).toLocaleString("fr-FR")} px)
            </option>
          ))}
        </select>
        <select className="ui-select" value={format} onChange={(event) => setFormat(event.target.value as "tif" | "png")} aria-label="Format">
          <option value="tif">TIFF</option>
          <option value="png">PNG</option>
        </select>
        <Button icon="export" shortcut="mod+e" title="Exporter la photo redressée" onClick={() => void exportImage()} disabled={!canExport}>
          Exporter…
        </Button>
      </ToolGroup>
    </Toolbar>
  );

  const startRename = (p: Projector) => {
    setRenaming(p.id);
    setRenameText(p.label);
  };

  const commitRename = () =>
    run("Renommage du projecteur…", async () => {
      const id = renaming;
      setRenaming(null);
      if (!folder || !id) return;
      const before = projectors.find((p) => p.id === id);
      if (!before || renameText.trim() === before.label) return;
      setProject(await renameProjector(folder, id, renameText));
    });

  const confirmRemove = () =>
    run("Retrait du projecteur…", async () => {
      const id = removing;
      setRemoving(null);
      if (!folder || !id) return;
      const next = await removeProjector(folder, id);
      setProject(next);
      if (id === selected) {
        const other = next.projectors[0] ?? null;
        setSelected(other?.id ?? null);
        setMode("brute");
        await showProjector(folder, other);
      }
    });

  const removingProjector = projectors.find((p) => p.id === removing) ?? null;

  const sidebar = (
    <Panel
      side="left"
      title="Projecteurs"
      actions={<IconButton icon="plus" label="Ajouter un projecteur" size="sm" onClick={() => void newProjector()} disabled={!folder || !ready} />}
    >
      {projectors.length > 0 ? (
        <List label="Projecteurs" onMove={moveSelection}>
          {projectors.map((p) => (
            <ListRow
              key={p.id}
              selected={p.id === selected}
              onSelect={() => ready && p.id !== selected && renaming !== p.id && void choose(p.id)}
              onDoubleClick={engine && ready ? () => startRename(p) : undefined}
              leading={<Dot tone={p.rectification ? "success" : p.rawPhoto ? "warning" : undefined} />}
              title={
                renaming === p.id ? (
                  <input
                    className="ui-input ui-input-inline"
                    value={renameText}
                    autoFocus
                    aria-label="Nouveau nom du projecteur"
                    onChange={(event) => setRenameText(event.target.value)}
                    onClick={(event) => event.stopPropagation()}
                    onBlur={() => void commitRename()}
                    onKeyDown={(event) => {
                      event.stopPropagation();
                      if (event.key === "Enter") event.currentTarget.blur();
                      if (event.key === "Escape") setRenaming(null);
                    }}
                  />
                ) : (
                  p.label
                )
              }
              detail={projectorStatus(p)}
              actions={
                engine && renaming !== p.id ? (
                  <>
                    <IconButton icon="pencil" label={`Renommer ${p.label}`} size="sm" onClick={() => startRename(p)} disabled={!ready} />
                    <IconButton icon="trash" label={`Supprimer ${p.label}`} size="sm" onClick={() => setRemoving(p.id)} disabled={!ready} />
                  </>
                ) : undefined
              }
            />
          ))}
        </List>
      ) : (
        <p className="ui-list-empty">
          {!engine ? "Exemple sans moteur : un seul projecteur, sans nom." : !folder ? "Pas de projet ouvert." : "Aucun projecteur. Le bouton + en ajoute un."}
        </p>
      )}
      {removingProjector && (
        <ConfirmDialog
          title={`Supprimer le projecteur ${removingProjector.label} ?`}
          confirmLabel={`Supprimer ${removingProjector.label}`}
          onConfirm={() => void confirmRemove()}
          onCancel={() => setRemoving(null)}
        >
          <p>Son redressement et son recalage seront perdus.</p>
          <p>Ses photos restent dans le dossier du projet : on pourra les réimporter.</p>
        </ConfirmDialog>
      )}
    </Panel>
  );

  const status = (
    <StatusBar>
      {hint ? (
        <StatusText tone="info">{hint}</StatusText>
      ) : summary ? (
        <StatusText tone={summary.quality === "bon" ? "success" : summary.quality === "moyen" ? "warning" : "danger"}>{summary.text}</StatusText>
      ) : (
        <StatusText>{projector ? `Projecteur ${projector.label} : ${projectorStatus(projector)}` : "Aucun projecteur choisi"}</StatusText>
      )}
      {view && !hint && !view.gridUnread && (
        <span className="ui-legend push">
          <span>
            <Dot tone="success" /> &lt; 0,15 mm
          </span>
          <span>
            <Dot tone="warning" /> &lt; 0,3 mm
          </span>
          <span>
            <Dot tone="danger" /> au-delà
          </span>
        </span>
      )}
    </StatusBar>
  );

  return (
    <Screen
      className="rectification"
      toolbar={toolbar}
      banner={
        error ? (
          <Banner tone="danger" onClose={() => setError(null)}>
            {error}
          </Banner>
        ) : notice ? (
          <Banner tone="success" onClose={() => setNotice(null)}>
            {notice}
          </Banner>
        ) : null
      }
      sidebar={sidebar}
      status={status}
    >
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
        {layers.length === 0 &&
          !busy &&
          (!engine ? (
            <EmptyState icon="image" title="Pas de moteur dans ce navigateur">
              <p>Ouvrir l'app pour redresser une photo.</p>
              <small>
                Pour essayer l'écran ici, préparer l'exemple dans <code>tuiles-demo/</code> (voir <code>docs/redressement.md</code>).
              </small>
            </EmptyState>
          ) : !folder ? (
            <EmptyState icon="folder" title="Aucun projet ouvert">
              <p>Ouvrir un projet (menu Fichier du Mac, ⌘O), ou en créer un (⌘N).</p>
            </EmptyState>
          ) : !projector ? (
            <EmptyState
              icon="projector"
              title="Aucun projecteur"
              actions={
                <Button variant="primary" icon="plus" onClick={() => void newProjector()}>
                  Ajouter un projecteur
                </Button>
              }
            >
              <p>Chaque projecteur reçoit la photo de sa mire, prise en chambre noire.</p>
            </EmptyState>
          ) : (
            <EmptyState
              icon="image"
              title={`Pas encore de photo pour ${projector.label}`}
              actions={
                <Button variant="primary" icon="import" onClick={() => void importPhoto()}>
                  Importer la photo…
                </Button>
              }
            >
              <p>HEIC, JPEG, PNG ou TIFF. La mire est cherchée dès l'import.</p>
            </EmptyState>
          ))}
        {busy && <Busy>{busy}</Busy>}
      </div>
    </Screen>
  );
}

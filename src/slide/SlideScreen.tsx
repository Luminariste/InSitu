// Écran Diapositive (tâche 3.6) : la diapositive calculée d'un projecteur,
// posée sur sa photo redressée, pour vérifier que les dessins tombent sur
// l'architecture. On passe d'un projecteur à l'autre d'un clic ou avec les
// flèches ← → du clavier.
//
// Les deux images sont en mm de diapositive (carré de 280 mm) : la photo
// redressée par tuiles, la diapositive calculée (aperçu de 2 048 px fait par
// le moteur, même calcul que l'export) colorée par-dessus.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  cacheFileUrl,
  fetchSlideViews,
  hasEngine,
  openProject,
  openTiledImage,
  projectFile,
  type SlideView,
} from "../engine";
import { recalledProject } from "../project/current";
import { Viewer, placementForWidth, type LayerSpec, type TileSource, type ViewerHandle } from "../viewer";
import { demoSlideViews, demoImages } from "./demo";
import { SLIDE_MM, SLIDE_SQUARE, TINTS, drawingsLayer, nextProjector, viewProblem } from "./logic";
import { ProjectMenu } from "../project/ProjectMenu";
import {
  Banner,
  Busy,
  Button,
  Dot,
  EmptyState,
  List,
  ListRow,
  Panel,
  Screen,
  StatusBar,
  StatusText,
  ToolGroup,
  ToolSeparator,
  ToolSpacer,
  Toolbar,
  useShortcuts,
} from "../ui";
import "./slide.css";

export function SlideScreen() {
  const engine = hasEngine();
  const viewer = useRef<ViewerHandle>(null);
  const [folder, setFolder] = useState<string | null>(null);
  const [projectName, setProjectName] = useState<string | null>(null);
  const [views, setViews] = useState<SlideView[]>([]);
  const [current, setCurrent] = useState<string | null>(null);
  const [photos, setPhotos] = useState<Record<string, TileSource>>({});
  const [showDrawings, setShowDrawings] = useState(true);
  const [opacity, setOpacity] = useState(0.9);
  const [tint, setTint] = useState(TINTS[0].value);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  /** Dans le navigateur seul : photo et dessins inventés. */
  const [demo, setDemo] = useState<{ photo: string; drawings: Record<string, string> } | null>(null);

  const load = useCallback(async (path: string) => {
    setBusy("Calcul des diapositives…");
    setError(null);
    try {
      const project = await openProject(path);
      setFolder(path);
      setProjectName(project.name);
      const list = await fetchSlideViews(path);
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

  // Flèches gauche et droite : projecteur précédent ou suivant.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
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
    const out: LayerSpec[] = [];
    if (demo) {
      out.push({ id: "photo-demo", kind: "mesh-image", url: demo.photo, divisions: 1, points: SLIDE_SQUARE });
      out.push(...drawingsLayer(demo.drawings[view.projectorId] ?? null, showDrawings, opacity, tint));
      return out;
    }
    const photo = photos[view.projectorId];
    if (photo) {
      out.push({
        id: `photo:${view.projectorId}`,
        kind: "tiles",
        source: photo,
        placement: placementForWidth(photo.pyramid.width, SLIDE_MM),
      });
    }
    out.push(...drawingsLayer(view.preview ? cacheFileUrl(view.preview) : null, showDrawings, opacity, tint));
    return out;
  }, [view, photos, demo, showDrawings, opacity, tint]);

  // Cadrer la diapositive à l'ouverture.
  useEffect(() => {
    const id = window.setTimeout(() => viewer.current?.fitAll(), 0);
    return () => window.clearTimeout(id);
  }, [folder, demo]);

  const problem = viewProblem(view);

  const ready = busy === null;
  useShortcuts([{ keys: "mod+0", run: () => viewer.current?.fitAll(), enabled: view !== null }]);

  const toolbar = (
    <Toolbar>
      <ProjectMenu
        name={projectName}
        folder={folder}
        ready={ready}
        placeholder="Exemple sans moteur"
        onOpen={(path) => void load(path)}
        onReload={() => folder && void load(folder)}
        onError={setError}
      />
      <ToolSpacer />
      <ToolGroup label="Dessins">
        <label className="ui-check">
          <input type="checkbox" checked={showDrawings} onChange={(e) => setShowDrawings(e.target.checked)} />
          Montrer
        </label>
        <select className="ui-select" value={tint} onChange={(e) => setTint(Number(e.target.value))} aria-label="Couleur des dessins">
          {TINTS.map((t) => (
            <option key={t.value} value={t.value}>
              {t.label}
            </option>
          ))}
        </select>
        <input
          className="slide-opacity"
          type="range"
          min={0}
          max={1}
          step={0.05}
          value={opacity}
          onChange={(e) => setOpacity(Number(e.target.value))}
          aria-label="Opacité des dessins"
          title="Opacité des dessins"
        />
      </ToolGroup>
      <ToolSeparator />
      <Button icon="fit" shortcut="mod+0" title="Voir toute la diapositive" onClick={() => viewer.current?.fitAll()} disabled={!view}>
        Tout voir
      </Button>
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
        ) : null
      }
      sidebar={sidebar}
      status={
        <StatusBar>
          <StatusText>
            {view ? `${view.label} : diapositive calculée sur la photo redressée (280 mm)` : "Aucun projecteur choisi"}
          </StatusText>
          <StatusText push>← → ou ↑ ↓ : projecteur précédent ou suivant</StatusText>
        </StatusBar>
      }
    >
      <div className="slide-stage">
        <Viewer ref={viewer} layers={layers} cursorUnit="mm" />
        {engine && !folder && !busy && (
          <EmptyState icon="folder" title="Aucun projet ouvert">
            <p>Ouvrir un projet avec le menu Projet (⌘O), ou en créer un (⌘N).</p>
          </EmptyState>
        )}
        {busy && <Busy>{busy}</Busy>}
      </div>
    </Screen>
  );
}

// Écran Diapositive (tâche 3.6) : la diapositive calculée d'un projecteur,
// posée sur sa photo redressée, pour vérifier que les dessins tombent sur
// l'architecture. On passe d'un projecteur à l'autre d'un clic ou avec les
// flèches ← → du clavier.
//
// Les deux images sont en mm de diapositive (carré de 280 mm) : la photo
// redressée par tuiles, la diapositive calculée (aperçu de 2 048 px fait par
// le moteur, même calcul que l'export) colorée par-dessus.
//
// Un clic sur un dessin le choisit pour le retoucher dans cette diapositive
// seulement (tâche 4.6, `useSlideAdjust.ts`).

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  cacheFileUrl,
  chooseExportFolder,
  exportSlides,
  fetchSlideViews,
  hasEngine,
  openProject,
  openTiledImage,
  projectFile,
  type Placement,
  type Project,
  type SlideContent,
  type SlideView,
} from "../engine";
import { useSlideAdjust } from "./useSlideAdjust";
import { controlPlanSummary, exportSettingsOf, exportSummary } from "../facade/exportLogic";
import { GRIDS } from "../compose/CompositionToolbar";
import { recalledProject } from "../project/current";
import { Viewer, placementForWidth, type LayerSpec, type TileSource, type ViewerHandle } from "../viewer";
import { demoSlideViews, demoImages } from "./demo";
import {
  LIGHTBURN_LOOK,
  cutNotice,
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
  Segmented,
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
  const stage = useRef<HTMLDivElement>(null);
  const [placements, setPlacements] = useState<Placement[]>([]);
  const [folder, setFolder] = useState<string | null>(null);
  const [projectName, setProjectName] = useState<string | null>(null);
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
  /** Le réglage d'avant « Comme dans LightBurn », pour y revenir. */
  const beforeLightBurn = useRef<SlideLook | null>(null);
  const lightBurn = isLightBurnLook(look);
  const toggleLightBurn = () => {
    if (lightBurn) {
      setLook(beforeLightBurn.current ?? { ...look, photo: true });
      beforeLightBurn.current = null;
    } else {
      beforeLightBurn.current = look;
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
      const project = await openProject(path);
      setFolder(path);
      setProjectName(project.name);
      setPlacements(project.placements ?? []);
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

  // Une retouche enregistrée : le projet change, l'aperçu est recalculé.
  const onProject = useCallback(
    (project: Project) => {
      setPlacements(project.placements ?? []);
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
  const adjust = useSlideAdjust({
    engine,
    folder,
    view,
    placements,
    onProject,
    viewer,
    stage,
    onError: setError,
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
  drawingChosen.current = adjust.selected !== null;
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
    return out;
  }, [view, photos, demo, showDrawings, opacity, tint, look.photo, look.backdrop, adjust.layers, content.plan, lightBurn]);

  // Cadrer la diapositive à l'ouverture.
  useEffect(() => {
    const id = window.setTimeout(() => viewer.current?.fitAll(), 0);
    return () => window.clearTimeout(id);
  }, [folder, demo]);

  const problem = viewProblem(view);
  const cut = cutNotice(view);

  const ready = busy === null;
  // Exporter la seule diapositive affichée, avec les réglages d'export du
  // projet (ceux de l'onglet Export de la Façade, relus au moment d'exporter).
  const canExport = engine && folder !== null && ready && view !== null && view.preview !== null;
  // Le plan de contrôle (D-3.5-m) : il suffit que le projecteur soit recalé.
  const canControlPlan =
    engine && folder !== null && ready && view !== null && !(view.problem ?? "").startsWith("recalage incomplet");
  const exportThis = async (controlPlan = false) => {
    if (!folder || !view) return;
    setError(null);
    setNotice(null);
    try {
      const destination = await chooseExportFolder();
      if (!destination) return;
      setBusy(controlPlan ? `Export du plan de contrôle ${view.label}…` : `Export de la diapositive ${view.label}…`);
      const project = await openProject(folder);
      const settings = exportSettingsOf(project.export);
      const report = await exportSlides(folder, destination, settings, () => {}, [view.projectorId], controlPlan);
      setNotice(controlPlan ? controlPlanSummary(report, destination) : exportSummary(report, destination));
    } catch (failure) {
      setError(String(failure).replace(/^Error: /, ""));
    } finally {
      setBusy(null);
    }
  };

  useShortcuts([
    { keys: "mod+0", run: () => viewer.current?.fitAll(), enabled: view !== null },
    { keys: "mod+e", run: () => void exportThis(), enabled: canExport },
  ]);

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
      <ToolSeparator />
      <ToolGroup label="Photo">
        <label className="ui-check" title="Montrer la photo redressée sous les dessins">
          <input type="checkbox" checked={look.photo} onChange={(e) => setLook({ ...look, photo: e.target.checked })} />
          Montrer
        </label>
      </ToolGroup>
      <ToolGroup label="Fond">
        <Segmented
          label="Fond de la diapositive (vu sans la photo)"
          value={look.backdrop}
          onChange={(backdrop) => setLook(withBackdrop(look, backdrop))}
          options={SLIDE_BACKDROPS.map((b) => ({ value: b.value, label: b.label, title: b.title, disabled: look.photo }))}
        />
      </ToolGroup>
      <ToolSeparator />
      <ToolGroup label="Façade">
        <label
          className="ui-check"
          title="La fresque dans l'aperçu (comme le menu Affichage de la Façade). L'export la grave toujours."
        >
          <input type="checkbox" checked={content.fresco} disabled={!ready} onChange={(e) => setContent({ ...content, fresco: e.target.checked })} />
          Fresque
        </label>
        <label
          className="ui-check"
          title={
            lightBurn
              ? "Le plan n'est pas gravé : il ne se voit pas « comme dans LightBurn »"
              : "Le plan de la façade vu par ce projecteur, pour se repérer (comme le menu Affichage de la Façade)"
          }
        >
          <input
            type="checkbox"
            checked={content.plan}
            disabled={!ready || lightBurn}
            onChange={(e) => setContent({ ...content, plan: e.target.checked })}
          />
          Plan
        </label>
      </ToolGroup>
      <ToolGroup label="Dessins">
        <label className="ui-check">
          <input type="checkbox" checked={showDrawings} onChange={(e) => setLook({ ...look, showDrawings: e.target.checked })} />
          Montrer
        </label>
        <select className="ui-select" value={tint} onChange={(e) => setLook({ ...look, tint: Number(e.target.value) })} aria-label="Couleur des dessins">
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
          onChange={(e) => setLook({ ...look, opacity: Number(e.target.value) })}
          aria-label="Opacité des dessins"
          title="Opacité des dessins"
        />
      </ToolGroup>
      <ToolSeparator />
      {adjust.selected && (
        <>
          {/* Mêmes outils que la barre de la façade, pour cette diapositive seulement. */}
          <ToolGroup label="Dessin">
            <IconButton
              icon="rotateLeft"
              label="Tourner d'un quart de tour vers la gauche, dans cette diapositive seulement"
              onClick={() => adjust.rotateBy(-90)}
            />
            <IconButton
              icon="rotateRight"
              label="Tourner d'un quart de tour vers la droite, dans cette diapositive seulement"
              onClick={() => adjust.rotateBy(90)}
            />
          </ToolGroup>
          <ToolSeparator />
          <ToolGroup>
            <IconButton
              icon="warpCorners"
              label="Quatre coins à tirer, en perspective, dans cette diapositive seulement (remplace la retouche actuelle)"
              onClick={() => adjust.setGrid(1, 1)}
            />
            <MenuButton
              label="Tendre une grille de retouche, dans cette diapositive seulement"
              icon="warpGrid"
              iconOnly
              variant="quiet"
              items={GRIDS.map(([c, r]) => ({ label: `Grille ${c} × ${r}`, onSelect: () => adjust.setGrid(c, r) }))}
            />
          </ToolGroup>
          <ToolSeparator />
          <Button
            size="sm"
            icon="undo"
            onClick={() => adjust.setGrid(0, 0)}
            disabled={!adjust.adjustment}
            title="Retirer la retouche de cette diapositive : le dessin revient tel qu'il est calé sur la façade"
          >
            Comme sur la façade
          </Button>
          <ToolSeparator />
        </>
      )}
      <Button icon="fit" shortcut="mod+0" title="Voir toute la diapositive" onClick={() => viewer.current?.fitAll()} disabled={!view}>
        Tout voir
      </Button>
      <ToolSeparator />
      <Button
        icon="export"
        title={
          engine
            ? "Le plan de la façade vu par ce projecteur, à graver pour vérifier le recalage : projeté, il doit tomber sur la façade"
            : "Dans l'app seulement"
        }
        onClick={() => void exportThis(true)}
        disabled={!canControlPlan}
      >
        Plan de contrôle…
      </Button>
      <Button
        variant="primary"
        icon="export"
        shortcut="mod+e"
        title={
          engine
            ? "Le PNG de cette diapositive seulement, avec les réglages d'export du projet (onglet Export de la Façade)"
            : "Dans l'app seulement"
        }
        onClick={() => void exportThis()}
        disabled={!canExport}
      >
        Exporter cette diapositive…
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
        ) : notice ? (
          <Banner tone="success" onClose={() => setNotice(null)}>
            {notice}
          </Banner>
        ) : cut ? (
          <Banner tone="warning">{cut}</Banner>
        ) : null
      }
      sidebar={sidebar}
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
            {adjust.selected
              ? `${adjust.hint} · cliquer dans le vide : ← → changent de projecteur`
              : `${adjust.hint ? `${adjust.hint} · ` : ""}← → : projecteur précédent ou suivant`}
          </StatusText>
        </StatusBar>
      }
    >
      <div
        className="slide-stage"
        ref={stage}
        onPointerDownCapture={adjust.onPointerDown}
        onPointerMoveCapture={adjust.onPointerMove}
        onPointerUpCapture={adjust.onPointerUp}
      >
        <Viewer ref={viewer} layers={layers} cursorUnit="mm" />
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

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

  return (
    <main className="slide">
      <header className="slide-bar">
        <div className="slide-group">
          <span className="slide-label">Projet</span>
          <strong>{projectName ?? (engine ? "aucun" : "exemple")}</strong>
          {engine && (
            <button type="button" onClick={() => folder && void load(folder)} disabled={!folder || busy !== null}>
              Relire
            </button>
          )}
        </div>
        <nav className="slide-group slide-projectors" aria-label="Projecteurs">
          {views.map((v) => (
            <button
              type="button"
              key={v.projectorId}
              className={v.projectorId === current ? "actif" : v.preview ? "" : "slide-off"}
              onClick={() => setCurrent(v.projectorId)}
              title={v.problem ?? v.label}
            >
              {v.label}
            </button>
          ))}
        </nav>
        <div className="slide-group">
          <label>
            <input type="checkbox" checked={showDrawings} onChange={(e) => setShowDrawings(e.target.checked)} />
            Dessins
          </label>
          <select value={tint} onChange={(e) => setTint(Number(e.target.value))} aria-label="Couleur des dessins">
            {TINTS.map((t) => (
              <option key={t.value} value={t.value}>
                {t.label}
              </option>
            ))}
          </select>
          <input
            type="range"
            min={0}
            max={1}
            step={0.05}
            value={opacity}
            onChange={(e) => setOpacity(Number(e.target.value))}
            aria-label="Opacité des dessins"
          />
        </div>
        <div className="slide-group slide-fit">
          <button type="button" onClick={() => viewer.current?.fitAll()}>
            Tout voir
          </button>
        </div>
      </header>
      <div className="slide-stage">
        <Viewer ref={viewer} layers={layers} cursorUnit="mm" />
        {(busy || error || problem || (engine && !folder)) && (
          <p className={error ? "slide-message erreur" : "slide-message"}>
            {busy ?? error ?? problem ?? "Ouvrir un projet dans l'onglet Façade, puis revenir ici."}
          </p>
        )}
      </div>
      <footer className="slide-foot">
        {view
          ? `${view.label} : diapositive calculée sur la photo redressée (280 mm). Flèches ← → : projecteur précédent ou suivant.`
          : "Aucun projecteur."}
      </footer>
    </main>
  );
}

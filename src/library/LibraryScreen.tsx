// Écran Bibliothèque (tâche 4.1) : les dessins du projet, en vignettes.
// On importe un dossier entier (ses sous-dossiers deviennent les dossiers
// de la bibliothèque), on cherche par nom ou par dossier, et l'aperçu montre
// le dessin avec son fond noir rendu transparent, sur le fond qu'on choisit.
//
// Tout le travail (copie, lecture des formats, vignettes) est fait par le
// moteur (`../engine.ts`). Dans un simple navigateur (`npm run dev`), l'écran
// montre 200 dessins inventés, pour juger du parcours ; rien n'est importé.

import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import {
  cacheFileUrl,
  chooseDrawingFolder,
  fetchDrawingPreview,
  fetchDrawingVectors,
  fetchLibrary,
  hasEngine,
  importDrawingFolder,
  openProject,
  saveVectorSettings,
  vectorizeDrawings,
  type ImportProgress,
  type LibraryEntry,
  type Project,
  type VectorSettings,
  type VectorSummary,
} from "../engine";
import { recalledProject } from "../project/current";
import { countLabel, filterEntries, folderTree, formatSize, importSummary, progressLabel } from "./logic";
import { demoLibrary, demoVectors } from "./demo";
import { VectorCompare, type ShownVectors } from "./VectorCompare";
import { vectorLine } from "./vectorLogic";
import "./library.css";

/** Un dessin affiché : l'entrée du moteur et l'adresse de sa vignette. */
interface Shown extends LibraryEntry {
  url: string | null;
}

type Backdrop = "damier" | "noir" | "facade";

const BACKDROPS: { value: Backdrop; label: string }[] = [
  { value: "damier", label: "Damier" },
  { value: "noir", label: "Noir" },
  { value: "facade", label: "Pierre" },
];

export function LibraryScreen() {
  const engine = hasEngine();
  const [folder, setFolder] = useState<string | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [entries, setEntries] = useState<Shown[]>([]);
  const [query, setQuery] = useState("");
  const [currentFolder, setCurrentFolder] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [backdrop, setBackdrop] = useState<Backdrop>("damier");
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [vectors, setVectors] = useState<ShownVectors | null>(null);
  const [vectorError, setVectorError] = useState<string | null>(null);
  const [vectorBusy, setVectorBusy] = useState(false);
  const [comparing, setComparing] = useState(false);
  const [vectorProgress, setVectorProgress] = useState<ImportProgress | null>(null);
  const grid = useRef<HTMLDivElement>(null);

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

  const showEntries = useCallback((list: LibraryEntry[]) => {
    setEntries(list.map((e) => ({ ...e, url: e.thumbnail ? cacheFileUrl(e.thumbnail) : null })));
  }, []);

  const load = useCallback(
    (path: string) =>
      run("Ouverture de la bibliothèque…", async () => {
        const opened = await openProject(path);
        setFolder(path);
        setProject(opened);
        showEntries(await fetchLibrary(path));
      }),
    [run, showEntries],
  );

  useEffect(() => {
    if (engine) {
      const last = recalledProject();
      if (last) void load(last);
      return;
    }
    setEntries(demoLibrary(200));
  }, [engine, load]);

  const importFolder = () =>
    run(null, async () => {
      if (!folder) return;
      const source = await chooseDrawingFolder();
      if (!source) return;
      setMessage(null);
      setProgress({ done: 0, total: 0 });
      let added: string[] = [];
      try {
        const { project: updated, report } = await importDrawingFolder(folder, source, setProgress);
        setProject(updated);
        showEntries(await fetchLibrary(folder));
        setMessage(importSummary(report));
        const imported = report.folder.replace(/^bibliotheque\/?/, "");
        if (report.added.length > 0) {
          setCurrentFolder(imported || null);
          setQuery("");
        }
        added = report.added;
      } finally {
        setProgress(null);
      }
      // Vecteurs des nouveaux dessins (tâche 5.2), après l'import : la
      // bibliothèque se parcourt déjà pendant ce temps.
      if (added.length > 0) {
        setVectorProgress({ done: 0, total: added.length });
        vectorizeDrawings(folder, added, setVectorProgress)
          .then((result) => {
            if (result.failures.length > 0) {
              setError(`${result.failures.length} dessin(s) non vectorisé(s) : ${result.failures.map((f) => f.file).join(", ")}`);
            }
          })
          .catch((failure: unknown) => setError(String(failure).replace(/^Error: /, "")))
          .finally(() => setVectorProgress(null));
      }
    });

  // --- Filtres et sélection -------------------------------------------------

  const folders = useMemo(() => folderTree(entries), [entries]);
  const visible = useMemo(() => filterEntries(entries, query, currentFolder), [entries, query, currentFolder]);
  const chosen = entries.find((e) => e.id === selected) ?? null;

  // Aperçu du dessin choisi (fabriqué par le moteur à la première demande).
  useEffect(() => {
    setPreview(null);
    if (!chosen || chosen.problem) return;
    if (!engine || !folder) {
      setPreview(chosen.url);
      return;
    }
    let cancelled = false;
    fetchDrawingPreview(folder, chosen.id).then(
      (url) => !cancelled && setPreview(url),
      (failure: unknown) => !cancelled && setError(String(failure)),
    );
    return () => {
      cancelled = true;
    };
  }, [chosen, engine, folder]);

  // Vecteurs du dessin choisi (tâche 5.2) : déjà en cache après l'import ;
  // sinon le moteur les calcule (quelques dixièmes de seconde par dessin).
  const showVectors = useCallback((summary: VectorSummary, demo: boolean) => {
    const url = (request: string) => (demo ? request : cacheFileUrl(request));
    setVectors({
      ...summary,
      imageUrl: url(summary.image),
      filledUrl: url(summary.filledSvg),
      outlineUrl: url(summary.outlineSvg),
    });
  }, []);

  useEffect(() => {
    setVectors(null);
    setVectorError(null);
    if (!chosen || chosen.problem) return;
    if (!engine || !folder) {
      const index = entries.findIndex((e) => e.id === chosen.id);
      if (chosen.url) showVectors(demoVectors(index, chosen.url), true);
      return;
    }
    let cancelled = false;
    fetchDrawingVectors(folder, chosen.id).then(
      (summary) => !cancelled && showVectors(summary, false),
      (failure: unknown) => !cancelled && setVectorError(String(failure).replace(/^Error: /, "")),
    );
    return () => {
      cancelled = true;
    };
    // `entries` ne sert qu'au mode démonstration : pas de nouvelle demande
    // quand la liste change (réglages appliqués).
  }, [chosen?.id, chosen?.problem, engine, folder, showVectors]);

  const applySettings = (settings: VectorSettings | null) => {
    if (!engine || !folder || !chosen) return;
    const id = chosen.id;
    setVectorBusy(true);
    setVectorError(null);
    saveVectorSettings(folder, id, settings)
      .then(({ project: updated, vectors: summary }) => {
        setProject(updated);
        setEntries((list) => list.map((e) => (e.id === id ? { ...e, vector: settings } : e)));
        showVectors(summary, false);
      })
      .catch((failure: unknown) => setVectorError(String(failure).replace(/^Error: /, "")))
      .finally(() => setVectorBusy(false));
  };

  const onKey = (event: KeyboardEvent<HTMLDivElement>) => {
    if (visible.length === 0) return;
    const index = visible.findIndex((e) => e.id === selected);
    const columns = Math.max(
      1,
      Math.round((grid.current?.clientWidth ?? 1) / ((grid.current?.firstElementChild as HTMLElement | null)?.offsetWidth ?? 1)),
    );
    const step = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: columns, ArrowUp: -columns }[event.key];
    if (step === undefined) return;
    event.preventDefault();
    const next = index < 0 ? 0 : Math.min(visible.length - 1, Math.max(0, index + step));
    setSelected(visible[next].id);
    grid.current?.children[next]?.scrollIntoView({ block: "nearest" });
  };

  const ready = busy === null && progress === null;

  return (
    <div className="library">
      <header className="library-bar">
        <div className="library-group">
          <span className="library-label">Projet</span>
          <span className="library-value">{project?.name ?? (engine ? "aucun" : "exemple")}</span>
          {engine && (
            <button type="button" onClick={() => folder && void load(folder)} disabled={!folder || !ready}>
              Relire
            </button>
          )}
        </div>
        <button type="button" className="library-primary" onClick={() => void importFolder()} disabled={!engine || !folder || !ready}>
          Importer un dossier…
        </button>
        <input
          className="library-search"
          type="search"
          placeholder="Chercher (nom ou dossier)"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          aria-label="Chercher un dessin"
        />
        <div className="library-group">
          <span className="library-label">Fond</span>
          <select value={backdrop} onChange={(event) => setBackdrop(event.target.value as Backdrop)} aria-label="Fond">
            {BACKDROPS.map((b) => (
              <option key={b.value} value={b.value}>
                {b.label}
              </option>
            ))}
          </select>
        </div>
        <span className="library-count">
          {visible.length === entries.length ? countLabel(entries.length) : `${visible.length} sur ${countLabel(entries.length)}`}
        </span>
      </header>

      {progress && (
        <div className="library-progress" role="progressbar" aria-valuenow={progress.done} aria-valuemax={progress.total}>
          <div style={{ width: `${progress.total ? (100 * progress.done) / progress.total : 0}%` }} />
          <span>{progress.total ? progressLabel(progress.done, progress.total) : "Recherche des dessins…"}</span>
        </div>
      )}
      {vectorProgress && (
        <p className="library-message">
          Vectorisation des nouveaux dessins : {vectorProgress.done} sur {vectorProgress.total}
        </p>
      )}
      {(busy || message || error || (engine && !folder && !busy)) && (
        <p className={error ? "library-message erreur" : "library-message"}>
          {error ?? busy ?? message ?? "Ouvrir ou créer un projet dans l'onglet Façade, puis revenir ici."}
        </p>
      )}

      <div className="library-body">
        <nav className="library-folders" aria-label="Dossiers">
          <button type="button" className={currentFolder === null ? "actif" : ""} onClick={() => setCurrentFolder(null)}>
            <span>Tous les dessins</span>
            <span className="library-folder-count">{entries.length}</span>
          </button>
          {folders.map((row) => (
            <button
              type="button"
              key={row.path}
              className={currentFolder === row.path ? "actif" : ""}
              style={{ paddingLeft: 10 + 14 * row.depth }}
              onClick={() => setCurrentFolder(row.path)}
              title={row.path}
            >
              <span>{row.label}</span>
              <span className="library-folder-count">{row.count}</span>
            </button>
          ))}
        </nav>

        <div className={`library-grid fond-${backdrop}`} ref={grid} tabIndex={0} onKeyDown={onKey} aria-label="Dessins">
          {visible.map((entry) => (
            <button
              type="button"
              key={entry.id}
              className={entry.id === selected ? "library-card choisi" : "library-card"}
              onClick={() => setSelected(entry.id)}
              title={entry.folder ? `${entry.folder}/${entry.name}` : entry.name}
            >
              <span className="library-thumb">
                {entry.url ? (
                  <img src={entry.url} alt="" loading="lazy" decoding="async" draggable={false} />
                ) : (
                  <span className="library-missing">{entry.problem ? "introuvable" : "…"}</span>
                )}
              </span>
              <span className="library-name">{entry.name}</span>
            </button>
          ))}
          {entries.length > 0 && visible.length === 0 && <p className="library-empty">Aucun dessin ne correspond.</p>}
          {entries.length === 0 && folder && !busy && (
            <p className="library-empty">La bibliothèque est vide. « Importer un dossier… » y copie un dossier de dessins.</p>
          )}
        </div>

        <aside className="library-detail">
          {chosen ? (
            <>
              <div className={`library-preview fond-${backdrop}`}>
                {preview ? <img src={preview} alt={chosen.name} /> : <span>{chosen.problem ?? "Préparation de l'aperçu…"}</span>}
              </div>
              <h2>{chosen.name}</h2>
              <dl>
                <dt>Dossier</dt>
                <dd>{chosen.folder || "—"}</dd>
                <dt>Taille</dt>
                <dd>{formatSize(chosen)}</dd>
                <dt>Fichier</dt>
                <dd className="library-path">{chosen.source}</dd>
                {chosen.problem && (
                  <>
                    <dt>Problème</dt>
                    <dd className="erreur">{chosen.problem}</dd>
                  </>
                )}
              </dl>
              {!chosen.problem && (
                <section className="library-vectors">
                  <h3>Vecteurs</h3>
                  <p className={vectorError ? "erreur" : undefined}>
                    {vectorError ?? (vectors ? vectorLine(vectors) : "Vectorisation…")}
                    {vectors && !vectors.defaultSettings && " · réglages propres"}
                  </p>
                  <button type="button" onClick={() => setComparing(true)} disabled={!vectors}>
                    Comparer image et vecteurs…
                  </button>
                </section>
              )}
            </>
          ) : (
            <p className="library-hint">Cliquer un dessin pour le voir en grand. Les flèches du clavier passent d'un dessin à l'autre.</p>
          )}
        </aside>
      </div>

      {comparing && chosen && (
        <VectorCompare
          name={chosen.name}
          vectors={vectors}
          backdrop={backdrop}
          editable={engine}
          busy={vectorBusy}
          error={vectorError}
          onApply={applySettings}
          onClose={() => setComparing(false)}
        />
      )}
    </div>
  );
}

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
  removeDrawing,
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
import { ProjectMenu } from "../project/ProjectMenu";
import {
  Banner,
  Busy,
  Button,
  ConfirmDialog,
  EmptyState,
  Icon,
  List,
  ListRow,
  Panel,
  Progress,
  Screen,
  Section,
  Segmented,
  StatusBar,
  StatusText,
  ToolGroup,
  ToolSeparator,
  ToolSpacer,
  Toolbar,
  useShortcuts,
} from "../ui";
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
  const [removing, setRemoving] = useState<string | null>(null);
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

  // Retirer un dessin (D15-h) : avec confirmation ; ses placements sur la
  // façade partent avec lui, le fichier reste dans le projet.
  const removingEntry = entries.find((e) => e.id === removing) ?? null;
  const removingPlaced = removing ? (project?.placements ?? []).filter((p) => p.drawing === removing).length : 0;

  const confirmRemove = () =>
    run(null, async () => {
      if (!folder || !removingEntry) return;
      const entry = removingEntry;
      setRemoving(null);
      const index = visible.findIndex((e) => e.id === entry.id);
      const [updated, placed] = await removeDrawing(folder, entry.id);
      setProject(updated);
      setEntries((list) => list.filter((e) => e.id !== entry.id));
      const rest = visible.filter((e) => e.id !== entry.id);
      setSelected(rest.length ? rest[Math.min(index, rest.length - 1)].id : null);
      setMessage(
        placed > 0
          ? `« ${entry.name} » retiré de la bibliothèque, avec ${placed === 1 ? "son placement" : `ses ${placed} placements`} sur la façade.`
          : `« ${entry.name} » retiré de la bibliothèque.`,
      );
    });

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

  useShortcuts([
    { keys: "mod+i", run: () => void importFolder(), enabled: engine && !!folder && ready },
    { keys: "backspace", run: () => chosen && setRemoving(chosen.id), enabled: engine && !!folder && ready && !!chosen },
  ]);

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
      <Button icon="import" shortcut="mod+i" title="Copier un dossier de dessins dans le projet" onClick={() => void importFolder()} disabled={!engine || !folder || !ready}>
        Importer un dossier…
      </Button>
      <input
        className="ui-input ui-input-search library-search"
        type="search"
        placeholder="Chercher (nom ou dossier)"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        aria-label="Chercher un dessin"
      />
      <ToolSpacer />
      <ToolGroup label="Fond">
        <Segmented label="Fond des vignettes" value={backdrop} onChange={setBackdrop} options={BACKDROPS} />
      </ToolGroup>
    </Toolbar>
  );

  const banner = progress ? (
    <Progress
      done={progress.done}
      total={progress.total}
      label={progress.total ? progressLabel(progress.done, progress.total) : "Recherche des dessins…"}
    />
  ) : vectorProgress ? (
    <Progress
      done={vectorProgress.done}
      total={vectorProgress.total}
      label={`Vectorisation des nouveaux dessins : ${vectorProgress.done} sur ${vectorProgress.total}`}
    />
  ) : error ? (
    <Banner tone="danger" onClose={() => setError(null)}>
      {error}
    </Banner>
  ) : message ? (
    <Banner tone="success" onClose={() => setMessage(null)}>
      {message}
    </Banner>
  ) : null;

  const sidebar = (
    <Panel side="left" title="Dossiers">
      <List label="Dossiers">
        <ListRow
          selected={currentFolder === null}
          onSelect={() => setCurrentFolder(null)}
          leading={<Icon name="image" />}
          title="Tous les dessins"
          trailing={entries.length}
        />
        {folders.map((row) => (
          <div key={row.path} style={{ paddingLeft: 14 * row.depth }}>
            <ListRow
              selected={currentFolder === row.path}
              onSelect={() => setCurrentFolder(row.path)}
              leading={<Icon name="folder" />}
              title={row.label}
              trailing={row.count}
              tooltip={row.path}
            />
          </div>
        ))}
      </List>
    </Panel>
  );

  const inspector = (
    <Panel side="right" title="Dessin">
      {chosen ? (
        <>
          <div className={`library-preview fond-${backdrop}`}>
            {preview ? <img src={preview} alt={chosen.name} /> : <span>{chosen.problem ?? "Préparation de l'aperçu…"}</span>}
          </div>
          <h3 className="library-detail-name">{chosen.name}</h3>
          <dl className="ui-facts">
            <dt>Dossier</dt>
            <dd>{chosen.folder || "—"}</dd>
            <dt>Taille</dt>
            <dd>{formatSize(chosen)}</dd>
            <dt>Fichier</dt>
            <dd className="ui-selectable">{chosen.source}</dd>
            {chosen.problem && (
              <>
                <dt>Problème</dt>
                <dd className="library-problem">{chosen.problem}</dd>
              </>
            )}
          </dl>
          {!chosen.problem && (
            <Section title="Vecteurs">
              <p className={vectorError ? "library-vectors library-problem" : "library-vectors"}>
                {vectorError ?? (vectors ? vectorLine(vectors) : "Vectorisation…")}
                {vectors && !vectors.defaultSettings && " · réglages propres"}
              </p>
              <Button icon="search" onClick={() => setComparing(true)} disabled={!vectors}>
                Comparer image et vecteurs…
              </Button>
            </Section>
          )}
          {engine && folder && (
            <Section title="Retirer">
              <Button variant="danger" icon="trash" shortcut="backspace" onClick={() => setRemoving(chosen.id)} disabled={!ready}>
                Retirer de la bibliothèque…
              </Button>
            </Section>
          )}
        </>
      ) : (
        <p className="ui-list-empty">Cliquer un dessin pour le voir en grand. Les flèches du clavier passent d'un dessin à l'autre.</p>
      )}
    </Panel>
  );

  return (
    <Screen
      className="library"
      toolbar={toolbar}
      banner={banner}
      sidebar={sidebar}
      inspector={inspector}
      status={
        <StatusBar>
          <StatusText>
            {visible.length === entries.length ? countLabel(entries.length) : `${visible.length} sur ${countLabel(entries.length)}`}
          </StatusText>
          {!engine && <StatusText push>Exemple sans moteur : 200 dessins inventés</StatusText>}
        </StatusBar>
      }
    >
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
      </div>
      {entries.length > 0 && visible.length === 0 && (
        <EmptyState icon="search" title="Aucun dessin ne correspond">
          <p>Essayer un autre mot, ou choisir « Tous les dessins ».</p>
        </EmptyState>
      )}
      {engine && !folder && !busy && (
        <EmptyState icon="folder" title="Aucun projet ouvert">
          <p>Ouvrir un projet avec le menu Projet (⌘O), ou en créer un (⌘N).</p>
        </EmptyState>
      )}
      {entries.length === 0 && folder && !busy && (
        <EmptyState
          icon="image"
          title="La bibliothèque est vide"
          actions={
            <Button variant="primary" icon="import" onClick={() => void importFolder()} disabled={!ready}>
              Importer un dossier…
            </Button>
          }
        >
          <p>Les dessins d'un dossier (et de ses sous-dossiers) sont copiés dans le projet. PNG, BMP, TIFF, PSD aplati, SVG.</p>
        </EmptyState>
      )}
      {busy && <Busy>{busy}</Busy>}
      {removingEntry && (
        <ConfirmDialog
          title={`Retirer « ${removingEntry.name} » de la bibliothèque ?`}
          confirmLabel="Retirer"
          onConfirm={() => void confirmRemove()}
          onCancel={() => setRemoving(null)}
        >
          {removingPlaced > 0 && (
            <p>
              Ce dessin est posé {removingPlaced === 1 ? "une fois" : `${removingPlaced} fois`} sur la façade : {removingPlaced === 1 ? "ce placement sera retiré" : "ces placements seront retirés"} aussi.
            </p>
          )}
          <p>Le fichier reste dans le dossier du projet ; un nouvel import du même dossier le remet dans la bibliothèque.</p>
        </ConfirmDialog>
      )}
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
    </Screen>
  );
}

// Écran Bibliothèque (tâche 4.1) : les dessins du projet, en vignettes.
// On importe un dossier entier (ses sous-dossiers deviennent les dossiers
// de la bibliothèque), on cherche par nom ou par dossier, et l'aperçu montre
// le dessin avec son fond noir rendu transparent, sur le fond qu'on choisit.
//
// Tout le travail (copie, lecture des formats, vignettes) est fait par le
// moteur (`../engine.ts`). Dans un simple navigateur (`npm run dev`), l'écran
// montre 200 dessins inventés, pour juger du parcours ; rien n'est importé.

import { useCallback, useEffect, useMemo, useRef, useState, type DragEvent, type KeyboardEvent, type MouseEvent } from "react";
import {
  cacheFileUrl,
  chooseDrawingFolder,
  createLibraryFolder,
  fetchDrawingPreview,
  fetchDrawingVectors,
  fetchLibrary,
  fetchSilhouettes,
  hasEngine,
  importDrawingFolder,
  importDrawingFiles,
  importDroppedDrawings,
  chooseDrawingFiles,
  moveDrawings,
  openProject,
  removeDrawing,
  removeLibraryFolder,
  renameDrawing,
  renameLibraryFolder,
  saveRetouch,
  saveSilhouette,
  saveVectorSettings,
  vectorizeDrawings,
  type DrawingSilhouette,
  type ImportProgress,
  type ImportReport,
  type LibraryEntry,
  type Project,
  type RetouchStroke,
  type SilhouettePolygon,
  type VectorSettings,
  type VectorSummary,
} from "../engine";
import { recalledProject } from "../project/current";
import {
  childFolder,
  countLabel,
  draggedDrawings,
  filterEntries,
  folderTree,
  formatSize,
  importSummary,
  moveSummary,
  newFolderName,
  parentFolder,
  progressLabel,
} from "./logic";
import { treeClick } from "../compose/slideTree";
import { demoLibrary, demoSilhouette, demoVectors } from "./demo";
import { BACKDROPS, readBackdrop, saveBackdrop, type Backdrop } from "./backdrop";
import { ignoredNotice } from "./drop";
import { useFinderDrop } from "./useFinderDrop";
import { RetouchEditor } from "./RetouchEditor";
import { retouchLine } from "./retouchLogic";
import { SilhouetteEditor } from "./SilhouetteEditor";
import { silhouetteLine } from "./silhouetteLogic";
import { VectorCompare, type ShownVectors } from "./VectorCompare";
import { vectorLine } from "./vectorLogic";
import { ProjectMenu } from "../project/ProjectMenu";
import {
  Banner,
  Busy,
  DropVeil,
  Button,
  ConfirmDialog,
  ContextMenu,
  EmptyState,
  Icon,
  IconButton,
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
  type MenuEntry,
} from "../ui";
import "./library.css";

/** Type des données d'un glisser de vignettes vers un dossier (D-4.1-g). */
const DRAG_TYPE = "application/x-insitu-dessins";

/** Dessins en cours de glisser (la vue web ne rend pas toujours les données pendant le survol). */
let dragging: string[] | null = null;

/** Champ de nom en place : ↩ valide, Échap annule, quitter le champ valide (docs/design.md § 3). */
function NameField({ value, label, onDone }: { value: string; label: string; onDone: (name: string | null) => void }) {
  const [text, setText] = useState(value);
  const done = useRef(false);
  const finish = (name: string | null) => {
    if (done.current) return;
    done.current = true;
    onDone(name === null || name.trim() === "" || name.trim() === value ? null : name.trim());
  };
  return (
    <input
      className="ui-input library-name-field"
      autoFocus
      onFocus={(e) => e.currentTarget.select()}
      value={text}
      aria-label={label}
      onChange={(e) => setText(e.target.value)}
      onClick={(e) => e.stopPropagation()}
      onDoubleClick={(e) => e.stopPropagation()}
      onBlur={() => finish(text)}
      onKeyDown={(e) => {
        e.stopPropagation();
        if (e.key === "Enter") finish(text);
        if (e.key === "Escape") finish(null);
      }}
    />
  );
}

/** Un dessin affiché : l'entrée du moteur et l'adresse de sa vignette. */
interface Shown extends LibraryEntry {
  url: string | null;
}

export function LibraryScreen() {
  const engine = hasEngine();
  const [folder, setFolder] = useState<string | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [entries, setEntries] = useState<Shown[]>([]);
  const [query, setQuery] = useState("");
  const [currentFolder, setCurrentFolder] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  // Ranger la bibliothèque (D-4.1-g) : dessins choisis ensemble (⌘-clic,
  // ⇧-clic), dossier ou nom en cours de saisie, dossier survolé par un
  // glisser, menu du clic droit, dossiers vides du mode démonstration.
  const [marked, setMarked] = useState<Set<string>>(new Set());
  const anchor = useRef<string | null>(null);
  const [editingFolder, setEditingFolder] = useState<string | null>(null);
  const [editingName, setEditingName] = useState(false);
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; items: MenuEntry[] } | null>(null);
  const [demoFolders, setDemoFolders] = useState<string[]>([]);
  const [preview, setPreview] = useState<string | null>(null);
  // Fond derrière les dessins : le même ici et dans les éditeurs, retenu.
  const [backdrop, setBackdropState] = useState<Backdrop>(readBackdrop);
  const setBackdrop = (next: Backdrop) => {
    setBackdropState(next);
    saveBackdrop(next);
  };
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
  const [silhouette, setSilhouette] = useState<DrawingSilhouette | null>(null);
  const [silhouetteError, setSilhouetteError] = useState<string | null>(null);
  const [silhouetteBusy, setSilhouetteBusy] = useState(false);
  const [editingSilhouette, setEditingSilhouette] = useState(false);
  // Gomme et crayon (tâche 4.8) : l'éditeur, l'aperçu sans retouche, et
  // l'enregistrement en cours (un seul à la fois, le dernier état gagne).
  const [editingRetouch, setEditingRetouch] = useState(false);
  const [originalPreview, setOriginalPreview] = useState<string | null>(null);
  const [retouchSaving, setRetouchSaving] = useState(false);
  const [retouchError, setRetouchError] = useState<string | null>(null);
  const [demoRetouch, setDemoRetouch] = useState<Record<string, RetouchStroke[]>>({});
  /** Augmente à chaque retouche enregistrée : aperçu, vecteurs et silhouette se refont. */
  const [revision, setRevision] = useState(0);
  const retouchQueue = useRef<{ running: boolean; next: { id: string; strokes: RetouchStroke[] } | null }>({
    running: false,
    next: null,
  });
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

  // Un import de la bibliothèque : dossier, fichiers choisis, ou dépôt du
  // Finder (D-4.1-f). `start` demande ce qu'il faut importer et rend
  // l'import à lancer, ou rien si l'utilisateur renonce.
  const importWith = (
    start: () => Promise<((folder: string) => Promise<{ project: Project; report: ImportReport }>) | null>,
    notice: string | null = null,
  ) =>
    run(null, async () => {
      if (!folder) return;
      const job = await start();
      if (!job) return;
      setMessage(null);
      setProgress({ done: 0, total: 0 });
      let added: string[] = [];
      try {
        const { project: updated, report } = await job(folder);
        setProject(updated);
        showEntries(await fetchLibrary(folder));
        setMessage([importSummary(report), notice].filter(Boolean).join(" "));
        const imported = report.folder.replace(/^bibliotheque\/?/, "");
        if (report.added.length > 0) {
          setCurrentFolder(imported || null);
          setQuery("");
          if (report.added.length === 1) setSelected(report.added[0]);
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

  const importFolder = () =>
    importWith(async () => {
      const source = await chooseDrawingFolder();
      return source ? (f) => importDrawingFolder(f, source, setProgress) : null;
    });

  const importFiles = () =>
    importWith(async () => {
      const paths = await chooseDrawingFiles();
      return paths.length > 0 ? (f) => importDrawingFiles(f, paths, setProgress) : null;
    });

  // Dessins glissés depuis le Finder n'importe où sur l'écran.
  const dropping = useFinderDrop(engine && !!folder, (files, ignored) => {
    if (files.length === 0) {
      setError(ignoredNotice(Math.max(ignored, 1)));
      return;
    }
    void importWith(async () => (f) => importDroppedDrawings(f, files, setProgress), ignoredNotice(ignored));
  });

  // --- Filtres et sélection -------------------------------------------------

  const extraFolders = engine ? (project?.libraryFolders ?? []) : demoFolders;
  const folders = useMemo(() => folderTree(entries, extraFolders), [entries, extraFolders]);
  const visible = useMemo(() => filterEntries(entries, query, currentFolder), [entries, query, currentFolder]);
  const chosen = entries.find((e) => e.id === selected) ?? null;

  /** Choisit un seul dessin (ou aucun). */
  const choose = useCallback((id: string | null) => {
    setSelected(id);
    setMarked(new Set(id ? [id] : []));
    setEditingName(false);
    if (id) anchor.current = id;
  }, []);

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
    // `revision` : le dessin vient d'être retouché à la gomme ou au crayon.
  }, [chosen, engine, folder, revision]);

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
  }, [chosen?.id, chosen?.problem, engine, folder, showVectors, revision]);

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

  // Silhouette du dessin choisi (tâche 4.7) : retouchée, sinon détectée par
  // le moteur (une fraction de seconde, puis gardée en cache).
  useEffect(() => {
    setSilhouette(null);
    setSilhouetteError(null);
    if (!chosen || chosen.problem) return;
    if (!engine || !folder) {
      setSilhouette({ drawing: chosen.id, polygons: demoSilhouette(chosen.id), manual: false });
      return;
    }
    let cancelled = false;
    fetchSilhouettes(folder, [chosen.id]).then(
      ([found]) => !cancelled && setSilhouette(found ?? null),
      (failure: unknown) => !cancelled && setSilhouetteError(String(failure).replace(/^Error: /, "")),
    );
    return () => {
      cancelled = true;
    };
  }, [chosen?.id, chosen?.problem, engine, folder, revision]);

  // --- Gomme et crayon (tâche 4.8) --------------------------------------------

  const savedRetouch: RetouchStroke[] = chosen
    ? engine
      ? (project?.library?.find((d) => d.id === chosen.id)?.retouch ?? [])
      : (demoRetouch[chosen.id] ?? [])
    : [];

  const openRetouch = () => {
    if (!chosen) return;
    setRetouchError(null);
    setOriginalPreview(null);
    setEditingRetouch(true);
    if (!engine || !folder) {
      setOriginalPreview(chosen.url);
      return;
    }
    fetchDrawingPreview(folder, chosen.id, true).then(setOriginalPreview, (failure: unknown) =>
      setRetouchError(String(failure).replace(/^Error: /, "")),
    );
  };

  /** Enregistre les retouches d'un dessin ; pendant un enregistrement, seul le dernier état attend son tour. */
  const saveRetouchLater = (id: string, strokes: RetouchStroke[]) => {
    if (!engine || !folder) {
      setDemoRetouch((all) => ({ ...all, [id]: strokes }));
      return;
    }
    const queue = retouchQueue.current;
    queue.next = { id, strokes };
    if (queue.running) return;
    const drain = async () => {
      queue.running = true;
      setRetouchSaving(true);
      while (queue.next) {
        const job = queue.next;
        queue.next = null;
        try {
          const { project: updated, entry } = await saveRetouch(folder, job.id, job.strokes);
          setProject(updated);
          setEntries((list) =>
            list.map((e) => (e.id === entry.id ? { ...entry, url: entry.thumbnail ? cacheFileUrl(entry.thumbnail) : null } : e)),
          );
          setRetouchError(null);
        } catch (failure) {
          setRetouchError(String(failure).replace(/^Error: /, ""));
        }
      }
      queue.running = false;
      setRetouchSaving(false);
      setRevision((r) => r + 1);
    };
    void drain();
  };

  const applySilhouette = (polygons: SilhouettePolygon[] | null) => {
    if (!chosen) return;
    if (!engine || !folder) {
      // Navigateur seul : la retouche reste en mémoire, pour juger des gestes.
      setSilhouette({ drawing: chosen.id, polygons: polygons ?? demoSilhouette(chosen.id), manual: polygons !== null });
      return;
    }
    setSilhouetteBusy(true);
    setSilhouetteError(null);
    saveSilhouette(folder, chosen.id, polygons)
      .then(({ project: updated, silhouette: current }) => {
        setProject(updated);
        setSilhouette(current);
      })
      .catch((failure: unknown) => setSilhouetteError(String(failure).replace(/^Error: /, "")))
      .finally(() => setSilhouetteBusy(false));
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
      choose(rest.length ? rest[Math.min(index, rest.length - 1)].id : null);
      setMessage(
        placed > 0
          ? `« ${entry.name} » retiré de la bibliothèque, avec ${placed === 1 ? "son placement" : `ses ${placed} placements`} sur la façade.`
          : `« ${entry.name} » retiré de la bibliothèque.`,
      );
    });

  // --- Ranger : renommer, glisser dans un dossier (D-4.1-g) -------------------
  // Rien ne bouge sur le disque : seuls le nom et le dossier affichés
  // changent dans projet.json, et ⌘Z les remet.

  /** Applique un rangement : par le moteur, ou en mémoire dans le navigateur seul. */
  const organize = (engineTask: (path: string) => Promise<Project>, demoTask: () => void, done?: () => void) =>
    run(null, async () => {
      if (!engine || !folder) {
        demoTask();
        done?.();
        return;
      }
      const updated = await engineTask(folder);
      setProject(updated);
      showEntries(await fetchLibrary(folder));
      done?.();
    });

  const renameChosen = (name: string) => {
    if (!chosen) return;
    const id = chosen.id;
    void organize(
      (path) => renameDrawing(path, id, name),
      () => setEntries((list) => list.map((e) => (e.id === id ? { ...e, name } : e))),
    );
  };

  const moveTo = (ids: string[], target: string) => {
    const names = entries.filter((e) => ids.includes(e.id) && e.folder !== target).map((e) => e.name);
    if (names.length === 0) return;
    void organize(
      (path) => moveDrawings(path, ids, target),
      () => setEntries((list) => list.map((e) => (ids.includes(e.id) ? { ...e, folder: target } : e))),
      () => setMessage(moveSummary(names, target)),
    );
  };

  const addFolder = (parent: string | null) => {
    const name = newFolderName(
      folders.map((f) => f.path),
      parent,
    );
    const path = childFolder(parent, name);
    void organize(
      (project) => createLibraryFolder(project, path),
      () => setDemoFolders((list) => [...list, path]),
      () => setEditingFolder(path),
    );
  };

  const renameFolder = (path: string, name: string) => {
    const parent = parentFolder(path);
    const target = childFolder(parent || null, name);
    const moved = (f: string) => (f === path || f.startsWith(`${path}/`) ? target + f.slice(path.length) : f);
    void organize(
      (project) => renameLibraryFolder(project, path, name),
      () => {
        setEntries((list) => list.map((e) => ({ ...e, folder: moved(e.folder) })));
        setDemoFolders((list) => list.map(moved));
      },
      () => setCurrentFolder((current) => (current === null ? null : moved(current))),
    );
  };

  const removeFolder = (path: string) =>
    void organize(
      (project) => removeLibraryFolder(project, path),
      () => setDemoFolders((list) => list.filter((f) => f !== path && !f.startsWith(`${path}/`))),
      () => setCurrentFolder((current) => (current !== null && (current === path || current.startsWith(`${path}/`)) ? parentFolder(path) || null : current)),
    );

  const clickCard = (id: string) => (event: MouseEvent) => {
    const order = visible.map((e) => e.id);
    const before = new Set(marked);
    if (selected) before.add(selected);
    const next = treeClick(order, before, anchor.current, id, {
      toggle: event.metaKey || event.ctrlKey,
      range: event.shiftKey,
    });
    if (!event.shiftKey) anchor.current = id;
    setMarked(next);
    setEditingName(false);
    setSelected(next.has(id) ? id : (next.values().next().value ?? null));
  };

  const startDrag = (id: string) => (event: DragEvent) => {
    const ids = draggedDrawings(id, marked);
    dragging = ids;
    event.dataTransfer.setData(DRAG_TYPE, JSON.stringify(ids));
    event.dataTransfer.effectAllowed = "move";
  };

  const acceptDrop = (key: string) => (event: DragEvent) => {
    if (dragging === null && !event.dataTransfer.types.includes(DRAG_TYPE)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    if (dropTarget !== key) setDropTarget(key);
  };

  const dropOn = (target: string) => (event: DragEvent) => {
    let ids = dragging;
    try {
      const data = event.dataTransfer.getData(DRAG_TYPE);
      if (data) ids = JSON.parse(data) as string[];
    } catch {
      // Données illisibles : on garde celles notées au départ du glisser.
    }
    dragging = null;
    setDropTarget(null);
    if (!ids || ids.length === 0) return;
    event.preventDefault();
    event.stopPropagation();
    moveTo(ids, target);
  };

  const folderMenu = (row: { path: string; count: number }) => (event: MouseEvent) => {
    event.preventDefault();
    setMenu({
      x: event.clientX,
      y: event.clientY,
      items: [
        { label: "Renommer", icon: "pencil", onSelect: () => setEditingFolder(row.path) },
        { label: "Nouveau sous-dossier", icon: "plus", onSelect: () => addFolder(row.path) },
        { separator: true },
        {
          label: row.count > 0 ? "Retirer le dossier (le vider d'abord)" : "Retirer le dossier",
          icon: "trash",
          danger: true,
          disabled: row.count > 0,
          onSelect: () => removeFolder(row.path),
        },
      ],
    });
  };

  const cardMenu = (entry: Shown) => (event: MouseEvent) => {
    event.preventDefault();
    if (!marked.has(entry.id)) choose(entry.id);
    const ids = draggedDrawings(entry.id, marked.has(entry.id) ? marked : new Set([entry.id]));
    setMenu({
      x: event.clientX,
      y: event.clientY,
      items: [
        {
          label: "Renommer",
          icon: "pencil",
          disabled: ids.length > 1,
          onSelect: () => {
            choose(entry.id);
            setEditingName(true);
          },
        },
        { label: ids.length > 1 ? `Sortir ces ${ids.length} dessins de leur dossier` : "Sortir de son dossier", icon: "folder", onSelect: () => moveTo(ids, "") },
        { separator: true },
        {
          label: "Retirer de la bibliothèque…",
          icon: "trash",
          danger: true,
          disabled: !engine || !folder || ids.length > 1,
          onSelect: () => setRemoving(entry.id),
        },
      ],
    });
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
    choose(visible[next].id);
    grid.current?.children[next]?.scrollIntoView({ block: "nearest" });
  };

  const ready = busy === null && progress === null;

  // Pas de raccourci de la bibliothèque sous un éditeur plein écran.
  const overlay = editingSilhouette || editingRetouch || comparing;

  useShortcuts([
    { keys: "mod+i", run: () => void importFolder(), enabled: engine && !!folder && ready && !overlay },
    {
      keys: "backspace",
      run: () => chosen && setRemoving(chosen.id),
      enabled: engine && !!folder && ready && !!chosen && !overlay,
    },
  ]);

  const toolbar = (
    <Toolbar>
      <ProjectMenu
        name={project?.name ?? null}
        folder={folder}
        ready={ready}
        placeholder="Exemple sans moteur"
        shortcuts={!overlay}
        onOpen={(path) => void load(path)}
        onReload={() => folder && void load(folder)}
        onError={setError}
      />
      <ToolSeparator />
      <Button
        icon="import"
        title="Copier un ou plusieurs dessins dans le projet (ou les glisser depuis le Finder sur la fenêtre)"
        onClick={() => void importFiles()}
        disabled={!engine || !folder || !ready}
      >
        Importer des dessins…
      </Button>
      <Button icon="folder" shortcut="mod+i" title="Copier un dossier de dessins, et ses sous-dossiers, dans le projet" onClick={() => void importFolder()} disabled={!engine || !folder || !ready}>
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
    <Panel
      side="left"
      title="Dossiers"
      actions={
        <IconButton
          icon="plus"
          label={currentFolder ? `Nouveau dossier dans « ${currentFolder} »` : "Nouveau dossier"}
          onClick={() => addFolder(currentFolder)}
          disabled={!ready || (engine && !folder)}
        />
      }
    >
      <List label="Dossiers">
        <div
          className={dropTarget === "" ? "library-folder is-over" : "library-folder"}
          onDragOver={acceptDrop("")}
          onDragLeave={() => setDropTarget(null)}
          onDrop={dropOn("")}
        >
          <ListRow
            selected={currentFolder === null}
            onSelect={() => setCurrentFolder(null)}
            leading={<Icon name="image" />}
            title="Tous les dessins"
            trailing={entries.length}
            tooltip="Lâcher des dessins ici : ils sortent de leur dossier."
          />
        </div>
        {folders.map((row) => (
          <div
            key={row.path}
            className={dropTarget === row.path ? "library-folder is-over" : "library-folder"}
            style={{ paddingLeft: 14 * row.depth }}
            onDragOver={acceptDrop(row.path)}
            onDragLeave={() => setDropTarget(null)}
            onDrop={dropOn(row.path)}
            onContextMenu={folderMenu(row)}
          >
            <ListRow
              selected={currentFolder === row.path}
              onSelect={() => setCurrentFolder(row.path)}
              onDoubleClick={() => setEditingFolder(row.path)}
              leading={<Icon name="folder" />}
              title={
                editingFolder === row.path ? (
                  <NameField
                    value={row.label}
                    label="Nom du dossier"
                    onDone={(name) => {
                      setEditingFolder(null);
                      if (name !== null) renameFolder(row.path, name);
                    }}
                  />
                ) : (
                  row.label
                )
              }
              trailing={row.count}
              tooltip={`${row.path} — double-clic : renommer ; clic droit : menu ; lâcher des dessins ici pour les y ranger.`}
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
          {marked.size > 1 && (
            <p className="library-vectors">
              {countLabel(marked.size)} choisis : les glisser sur un dossier pour les y ranger.
            </p>
          )}
          {editingName ? (
            <NameField
              key={chosen.id}
              value={chosen.name}
              label="Nom du dessin"
              onDone={(name) => {
                setEditingName(false);
                if (name !== null) renameChosen(name);
              }}
            />
          ) : (
            <h3 className="library-detail-name" onDoubleClick={() => setEditingName(true)} title="Double-clic : renommer">
              {chosen.name}
              <IconButton icon="pencil" size="sm" label="Renommer le dessin" onClick={() => setEditingName(true)} />
            </h3>
          )}
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
          {!chosen.problem && (
            <Section title="Gomme et crayon">
              <p className={retouchError ? "library-vectors library-problem" : "library-vectors"}>
                {retouchError ?? retouchLine(savedRetouch)}
              </p>
              <Button icon="eraser" onClick={openRetouch}>
                Retoucher le dessin…
              </Button>
            </Section>
          )}
          {!chosen.problem && (
            <Section title="Silhouette">
              <p className={silhouetteError ? "library-vectors library-problem" : "library-vectors"}>
                {silhouetteError ?? (silhouette ? silhouetteLine(silhouette.polygons, silhouette.manual) : "Détection…")}
              </p>
              <Button icon="pencil" onClick={() => setEditingSilhouette(true)} disabled={!silhouette}>
                Retoucher la silhouette…
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
            className={entry.id === selected || marked.has(entry.id) ? "library-card choisi" : "library-card"}
            onClick={clickCard(entry.id)}
            onDoubleClick={() => {
              choose(entry.id);
              setEditingName(true);
            }}
            onContextMenu={cardMenu(entry)}
            draggable
            onDragStart={startDrag(entry.id)}
            onDragEnd={() => {
              dragging = null;
              setDropTarget(null);
            }}
            title={`${entry.folder ? `${entry.folder}/${entry.name}` : entry.name} — ⌘-clic : ajouter au choix ; double-clic : renommer ; glisser sur un dossier pour l'y ranger`}
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
          <p>Ouvrir un projet (menu Fichier du Mac, ⌘O), ou en créer un (⌘N).</p>
        </EmptyState>
      )}
      {entries.length === 0 && folder && !busy && (
        <EmptyState
          icon="image"
          title="La bibliothèque est vide"
          actions={
            <>
              <Button variant="primary" icon="import" onClick={() => void importFiles()} disabled={!ready}>
                Importer des dessins…
              </Button>
              <Button icon="folder" onClick={() => void importFolder()} disabled={!ready}>
                Importer un dossier…
              </Button>
            </>
          }
        >
          <p>
            Les dessins sont copiés dans le projet (un dossier avec ses sous-dossiers). PNG, BMP, TIFF, PSD aplati, SVG. On peut
            aussi les glisser depuis le Finder sur cette fenêtre.
          </p>
        </EmptyState>
      )}
      {busy && <Busy>{busy}</Busy>}
      {dropping && <DropVeil>Déposer pour importer dans la bibliothèque</DropVeil>}
      {menu && <ContextMenu x={menu.x} y={menu.y} items={menu.items} onClose={() => setMenu(null)} />}
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
      {editingSilhouette && chosen && (
        <SilhouetteEditor
          name={chosen.name}
          imageUrl={preview}
          width={chosen.width}
          height={chosen.height}
          silhouette={silhouette}
          backdrop={backdrop}
          onBackdrop={setBackdrop}
          editable
          busy={silhouetteBusy}
          error={silhouetteError}
          onSave={applySilhouette}
          onClose={() => setEditingSilhouette(false)}
        />
      )}
      {editingRetouch && chosen && (
        <RetouchEditor
          key={chosen.id}
          name={chosen.name}
          originalUrl={originalPreview}
          width={chosen.width}
          height={chosen.height}
          strokes={savedRetouch}
          backdrop={backdrop}
          onBackdrop={setBackdrop}
          editable
          saving={retouchSaving}
          error={retouchError}
          onChange={(strokes) => saveRetouchLater(chosen.id, strokes)}
          onClose={() => setEditingRetouch(false)}
        />
      )}
      {comparing && chosen && (
        <VectorCompare
          name={chosen.name}
          vectors={vectors}
          backdrop={backdrop}
          onBackdrop={setBackdrop}
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

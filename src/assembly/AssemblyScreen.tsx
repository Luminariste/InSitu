// Assemblage de photos de façade (tâche 6.4), ouvert depuis l'écran Façade.
// Quand aucun plan n'existe, plusieurs photos de la façade, prises de face
// et qui se recouvrent, sont recalées entre elles par des points communs :
// à gauche la photo à placer, à droite la photo sur laquelle elle s'appuie
// (la première photo sert de référence). Le moteur mesure l'écart de chaque
// point et signale ceux qui semblent mal placés. « Calculer l'assemblage »
// fabrique une seule image, qui devient la façade du projet.
//
// Depuis la tâche 6.6, « Raccorder automatiquement » laisse le moteur
// trouver lui-même les points communs et l'appui de chaque photo ; il le
// fait aussi tout seul après l'ajout des premières photos. Les points
// trouvés s'affichent et se corrigent comme des points cliqués.
//
// Tout le calcul et l'enregistrement passent par le moteur (`../engine.ts`).
// Dans un simple navigateur (`npm run dev`), l'écran montre deux photos
// d'exemple déjà tuilées dans `tuiles-demo/` si elles existent (voir
// `docs/assemblage.md`) : on peut y poser des points, rien n'est enregistré.

import { useCallback, useEffect, useMemo, useRef, useState, type PointerEvent, type RefObject } from "react";
import {
  addAssemblyPhoto,
  autoAssembly,
  chooseFacadePhotos,
  fetchAssembly,
  hasEngine,
  openTiledImage,
  openTileFolder,
  projectFile,
  removeAssemblyPhoto,
  renderAssembly,
  saveAssemblyPairs,
  type AssemblyPair,
  type AssemblyPhoto,
  type AssemblyReport,
  type AutoReport,
  type Project,
} from "../engine";
import { Viewer, type LayerSpec, type TileSource, type ViewerHandle } from "../viewer";
import { mmToScreen, screenToMm } from "../viewer/geometry";
import { hitHandle } from "../rectification/logic";
import { isClick } from "../facade/scale";
import { EMPTY_MARKERS, PairMarkersNode, type MarkersContent } from "../registration/markers";
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
  Property,
  Screen,
  Section,
  StatusBar,
  StatusText,
  ToolGroup,
  ToolSeparator,
  ToolSpacer,
  Toolbar,
  useShortcuts,
  type Tone,
} from "../ui";
import {
  GRAB_RADIUS_PX,
  NO_PENDING,
  autoMessage,
  click,
  formatPx,
  formatSize,
  hasManualPoints,
  hint,
  inside,
  movePoint,
  pairStatus,
  photoLine,
  possibleAnchors,
  removePair,
  summary,
  type Pending,
  type Side,
} from "./logic";
import "../registration/registration.css";
import "./assembly.css";

/** Les photos sont affichées en pixels : un « mm » de la visionneuse vaut un pixel. */
const PIXELS = { x: 0, y: 0, mmPerPixel: 1 };

/** Photos d'exemple du navigateur seul (tuilées dans `tuiles-demo/`). */
const DEMO_PHOTOS = ["assemblage-1", "assemblage-2"];

/** Installe un objet de repères dans la scène d'une visionneuse, dès qu'elle existe. */
function useMarkers(viewer: RefObject<ViewerHandle | null>, name: string) {
  const node = useRef<PairMarkersNode | null>(null);
  const content = useRef<MarkersContent>(EMPTY_MARKERS);
  useEffect(() => {
    let remove: (() => void) | null = null;
    let frame = 0;
    const attach = () => {
      const scene = viewer.current?.scene();
      if (!scene) {
        frame = requestAnimationFrame(attach);
        return;
      }
      const created = new PairMarkersNode(name);
      created.set(content.current);
      node.current = created;
      remove = scene.addNode(created);
    };
    attach();
    return () => {
      cancelAnimationFrame(frame);
      node.current = null;
      remove?.();
    };
  }, [viewer, name]);
  return useCallback(
    (next: MarkersContent) => {
      content.current = next;
      node.current?.set(next);
      viewer.current?.scene()?.invalidate();
    },
    [viewer],
  );
}

interface Props {
  /** Dossier du projet ; `null` dans le navigateur seul. */
  folder: string | null;
  project: Project | null;
  /** Fermeture ; `project` est le projet à jour s'il a changé. */
  onClose: (project: Project | null) => void;
  /** L'assemblage est calculé et devient la façade. */
  onRendered: (project: Project, message: string) => void;
}

export function AssemblyScreen({ folder, project: initial, onClose, onRendered }: Props) {
  const engine = hasEngine() && folder !== null;
  const hereViewer = useRef<ViewerHandle>(null);
  const thereViewer = useRef<ViewerHandle>(null);
  const herePane = useRef<HTMLDivElement>(null);
  const therePane = useRef<HTMLDivElement>(null);
  const pointerDown = useRef<{ x: number; y: number; t: number } | null>(null);
  const setHereMarkers = useMarkers(hereViewer, "photo");
  const setThereMarkers = useMarkers(thereViewer, "appui");

  const [project, setProject] = useState<Project | null>(initial);
  const [changed, setChanged] = useState(false);
  const [demoPhotos, setDemoPhotos] = useState<AssemblyPhoto[]>([]);
  const [report, setReport] = useState<AssemblyReport | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [pairs, setPairs] = useState<AssemblyPair[]>([]);
  const [pending, setPending] = useState<Pending>(NO_PENDING);
  const [selected, setSelected] = useState<number | null>(null);
  const [dragging, setDragging] = useState<{ side: Side; index: number; before: AssemblyPair[] } | null>(null);
  const [grabbable, setGrabbable] = useState(false);
  const [sources, setSources] = useState<Record<string, TileSource>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<{ kind: "remove" | "anchor" | "auto"; id: string } | null>(null);
  const [notice, setNotice] = useState<{ tone: "success" | "warning"; text: string } | null>(null);

  const photos = useMemo(() => (engine ? (project?.facadeAssembly?.photos ?? []) : demoPhotos), [engine, project, demoPhotos]);
  const names = useMemo(() => new Map(photos.map((p) => [p.id, p.name])), [photos]);
  const index = photos.findIndex((p) => p.id === selectedId);
  const photo = index >= 0 ? photos[index] : null;
  const isReference = index === 0;
  const anchor = photo && !isReference ? (photos.find((p) => p.id === photo.anchor) ?? null) : null;
  const ready = busy === null;

  const run = useCallback(async (label: string | null, task: () => Promise<void>) => {
    if (label) setBusy(label);
    setError(null);
    setNotice(null);
    try {
      await task();
    } catch (failure) {
      setError(String(failure).replace(/^Error: /, ""));
    } finally {
      if (label) setBusy(null);
    }
  }, []);

  const updated = (next: Project) => {
    setProject(next);
    setChanged(true);
  };

  // --- Chargement ------------------------------------------------------------

  useEffect(() => {
    if (engine && folder) {
      void run("Lecture de l'assemblage…", async () => {
        setReport(await fetchAssembly(folder));
      });
      return;
    }
    // Navigateur seul : deux photos d'exemple, s'il y en a.
    void (async () => {
      const found: AssemblyPhoto[] = [];
      const tiles: Record<string, TileSource> = {};
      for (const [i, name] of DEMO_PHOTOS.entries()) {
        const source = await openTileFolder(`/tuiles/${name}`);
        if (!source) continue;
        tiles[name] = source;
        found.push({
          id: name,
          name: `Photo ${i + 1}`,
          source: name,
          width: source.pyramid.width,
          height: source.pyramid.height,
          anchor: i === 0 ? null : DEMO_PHOTOS[0],
        });
      }
      setSources(tiles);
      setDemoPhotos(found);
    })();
  }, [engine, folder, run]);

  // Première photo à afficher : la première pas encore placée, sinon la deuxième.
  useEffect(() => {
    if (selectedId && photos.some((p) => p.id === selectedId)) return;
    const unplaced = photos.find((_, i) => i > 0 && !report?.photos[i]?.placed);
    setSelectedId(unplaced?.id ?? photos[1]?.id ?? photos[0]?.id ?? null);
  }, [photos, report, selectedId]);

  // Les points de la photo choisie.
  useEffect(() => {
    setPairs(photo?.pairs ?? []);
    setPending(NO_PENDING);
    setSelected(null);
    // Seulement quand on change de photo ou d'appui : pas à chaque enregistrement.
  }, [photo?.id, photo?.anchor]);

  // Tuiles de la photo choisie et de son appui, préparées à la demande.
  useEffect(() => {
    if (!engine || !folder) return;
    const wanted = [photo, anchor].filter((p): p is AssemblyPhoto => p !== null && !sources[p.id]);
    if (wanted.length === 0) return;
    void run("Préparation des photos…", async () => {
      const next: Record<string, TileSource> = {};
      for (const p of wanted) next[p.id] = await openTiledImage(projectFile(folder, p.source));
      setSources((known) => ({ ...known, ...next }));
    });
  }, [engine, folder, photo, anchor, sources, run]);

  const hereSource = photo ? (sources[photo.id] ?? null) : null;
  const thereSource = anchor ? (sources[anchor.id] ?? null) : null;

  const hereLayers = useMemo<LayerSpec[]>(
    () => (hereSource ? [{ id: "photo", kind: "tiles", source: hereSource, placement: PIXELS }] : []),
    [hereSource],
  );
  const thereLayers = useMemo<LayerSpec[]>(
    () => (thereSource ? [{ id: "appui", kind: "tiles", source: thereSource, placement: PIXELS }] : []),
    [thereSource],
  );

  useEffect(() => {
    const id = window.setTimeout(() => hereViewer.current?.fitAll(), 0);
    return () => window.clearTimeout(id);
  }, [hereSource]);
  useEffect(() => {
    const id = window.setTimeout(() => thereViewer.current?.fitAll(), 0);
    return () => window.clearTimeout(id);
  }, [thereSource]);

  // --- Repères ---------------------------------------------------------------

  const photoReport = index >= 0 ? report?.photos[index] : undefined;
  useEffect(() => {
    const statuses = pairs.map((_, i) => pairStatus(photoReport?.pairs[i]));
    setHereMarkers({ points: pairs.map((p, i) => ({ ...p.here, status: statuses[i] })), selected, pending: pending.here, footprint: null });
    setThereMarkers({ points: pairs.map((p, i) => ({ ...p.there, status: statuses[i] })), selected, pending: pending.there, footprint: null });
  }, [pairs, photoReport, pending, selected, setHereMarkers, setThereMarkers]);

  // --- Actions ---------------------------------------------------------------

  /** Enregistre les points (et l'appui) de la photo choisie, relit le diagnostic. */
  const commit = (next: AssemblyPair[], previous: AssemblyPair[] = pairs, anchorId: string | null = anchor?.id ?? null) => {
    setPairs(next);
    if (!engine || !folder || !photo || !anchorId) return;
    saveAssemblyPairs(folder, photo.id, anchorId, next).then(
      (saved) => {
        updated(saved.project);
        setReport(saved.report);
      },
      (failure: unknown) => {
        setPairs(previous);
        setError(String(failure).replace(/^Error: /, ""));
      },
    );
  };

  /** Montre le résultat du raccord automatique et la première photo raccordée. */
  const showAuto = (next: Project, done: AutoReport) => {
    updated(next);
    setReport(done.assembly);
    setNotice(autoMessage(done));
    const all = next.facadeAssembly?.photos ?? [];
    // La photo affichée garde son identifiant : ses points sont relus ici.
    const shown = all.find((p) => p.id === selectedId) ?? all[1] ?? null;
    if (shown) {
      setSelectedId(shown.id);
      setPairs(shown.pairs ?? []);
      setPending(NO_PENDING);
      setSelected(null);
    }
  };

  const LINKING = "Recherche des détails communs aux photos… (quelques secondes par photo)";

  const linkAutomatically = () =>
    run(LINKING, async () => {
      if (!folder) return;
      setConfirm(null);
      const { project: next, report: done } = await autoAssembly(folder);
      showAuto(next, done);
    });

  const addPhotos = () =>
    run(null, async () => {
      if (!folder) return;
      const paths = await chooseFacadePhotos();
      if (paths.length === 0) return;
      // Aucun point posé à la main : le raccord automatique se lance seul.
      const automatic = !hasManualPoints(photos);
      let first: string | null = null;
      let last: Project | null = null;
      for (const [i, path] of paths.entries()) {
        setBusy(`Copie de la photo ${i + 1} sur ${paths.length}…`);
        const added = await addAssemblyPhoto(folder, path, null);
        last = added.project;
        first ??= added.id;
      }
      setBusy(null);
      if (last) updated(last);
      const all = last?.facadeAssembly?.photos ?? [];
      if (automatic && all.length >= 2) {
        setBusy(LINKING);
        const { project: next, report: done } = await autoAssembly(folder);
        showAuto(next, done);
        return;
      }
      setReport(await fetchAssembly(folder));
      // La première nouvelle photo, si elle n'est pas la référence.
      const target = all.findIndex((p) => p.id === first);
      setSelectedId(target > 0 ? first : (all[1]?.id ?? first));
    }).finally(() => setBusy(null));

  const removePhoto = (id: string) =>
    run("Retrait de la photo…", async () => {
      if (!folder) return;
      updated(await removeAssemblyPhoto(folder, id));
      setReport(await fetchAssembly(folder));
      if (id === selectedId) setSelectedId(null);
    });

  const changeAnchor = (id: string) => {
    setConfirm(null);
    setPending(NO_PENDING);
    setSelected(null);
    // Les points de l'ancien appui ne valent rien sur le nouveau.
    commit([], pairs, id);
  };

  const render = () =>
    run("Calcul de l'image assemblée…", async () => {
      if (!folder) return;
      const { project: next, report: done } = await renderAssembly(folder);
      const size = formatSize(done.width, done.height);
      const message = done.keptScale
        ? `Assemblage calculé (${size}) : c'est la nouvelle façade, son échelle est gardée.`
        : `Assemblage calculé (${size}) : c'est la nouvelle façade. La mettre à l'échelle avec deux points.`;
      onRendered(next, message);
    });

  // --- Pointeur : poser, choisir, glisser des points ------------------------

  const sizeOf = (side: Side) => (side === "here" ? photo : anchor);

  const pointerAt = (side: Side, event: PointerEvent) => {
    const handle = side === "here" ? hereViewer.current : thereViewer.current;
    const pane = side === "here" ? herePane.current : therePane.current;
    const scene = handle?.scene();
    const canvas = pane?.querySelector(".viewer-canvas");
    if (!scene || !canvas) return null;
    const box = canvas.getBoundingClientRect();
    const screen = { x: event.clientX - box.left, y: event.clientY - box.top };
    const view = scene.getView();
    const viewport = scene.getViewport();
    return { screen, px: screenToMm(view, viewport, screen), toScreen: (p: { x: number; y: number }) => mmToScreen(view, viewport, p) };
  };

  const pointUnder = (side: Side, at: NonNullable<ReturnType<typeof pointerAt>>) =>
    hitHandle(
      pairs.map((pair) => at.toScreen(pair[side])),
      at.screen,
      GRAB_RADIUS_PX,
    );

  const onPointerDown = (side: Side) => (event: PointerEvent<HTMLDivElement>) => {
    pointerDown.current = { x: event.clientX, y: event.clientY, t: event.timeStamp };
    if (!ready) return;
    const at = pointerAt(side, event);
    const hit = at ? pointUnder(side, at) : null;
    if (hit === null) return;
    event.stopPropagation();
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    setDragging({ side, index: hit, before: pairs });
  };

  const onPointerMove = (side: Side) => (event: PointerEvent<HTMLDivElement>) => {
    const at = pointerAt(side, event);
    const size = sizeOf(side);
    if (!at || !size) return;
    if (dragging && dragging.side === side) {
      setPairs(movePoint(dragging.before, dragging.index, side, at.px, size));
      return;
    }
    setGrabbable(ready && pointUnder(side, at) !== null);
  };

  const onPointerUp = (side: Side) => (event: PointerEvent<HTMLDivElement>) => {
    const down = pointerDown.current;
    pointerDown.current = null;
    const up = { x: event.clientX, y: event.clientY, t: event.timeStamp };
    if (dragging) {
      event.currentTarget.releasePointerCapture(event.pointerId);
      const { index: grabbed, before } = dragging;
      setDragging(null);
      if (down && isClick(down, up)) {
        setPairs(before);
        setSelected(selected === grabbed ? null : grabbed);
        return;
      }
      setSelected(grabbed);
      commit(pairs, before);
      return;
    }
    if (!down || !ready || !anchor || !isClick(down, up)) return;
    const at = pointerAt(side, event)?.px;
    const size = sizeOf(side);
    if (!at || !size) return;
    if (!inside(at, size.width, size.height)) {
      setError("Ce point est hors de la photo : cliquer dans l'image.");
      return;
    }
    setError(null);
    const result = click(pairs, pending, selected, side, at);
    setPending(result.pending);
    if (result.changed) commit(result.pairs);
  };

  useShortcuts([
    {
      keys: "escape",
      run: () => {
        setPending(NO_PENDING);
        setSelected(null);
      },
      enabled: pending.here !== null || pending.there !== null || selected !== null,
    },
    {
      keys: "mod+0",
      run: () => {
        hereViewer.current?.fitAll();
        thereViewer.current?.fitAll();
      },
    },
    { keys: "backspace", run: () => selected !== null && commit(removePair(pairs, selected)), enabled: selected !== null && ready },
  ]);

  // --- Affichage -------------------------------------------------------------

  const tones: Record<string, Tone> = { bon: "success", moyen: "warning", aberrant: "danger", inconnu: "info" };
  const instruction = hint(pending, selected);
  const stageClass = `registration-stage${dragging ? " registration-dragging" : grabbable ? " registration-grab" : ""}`;
  const candidates = photo && !isReference ? possibleAnchors(photos, photo.id) : [];

  const toolbar = (
    <Toolbar label="Assemblage">
      <strong className="assembly-title">Assemblage de photos</strong>
      <ToolSeparator />
      <Button icon="plus" onClick={() => void addPhotos()} disabled={!engine || !ready} title="Photos de face de la façade, qui se recouvrent en partie">
        Ajouter des photos…
      </Button>
      <Button
        icon="link"
        onClick={() => (hasManualPoints(photos) ? setConfirm({ kind: "auto", id: photos[0]?.id ?? "" }) : void linkAutomatically())}
        disabled={!engine || !ready || photos.length < 2}
        title="L'app cherche elle-même les détails communs aux photos et pose les points"
      >
        Raccorder automatiquement
      </Button>
      <Button
        variant="primary"
        icon="check"
        onClick={() => void render()}
        disabled={!engine || !ready || !report?.ready || photos.length < 2}
        title="Fabrique une seule image, qui devient la façade du projet"
      >
        Calculer l'assemblage
      </Button>
      <ToolSpacer />
      <ToolGroup label="Voir">
        <Button icon="fit" shortcut="mod+0" onClick={() => {
          hereViewer.current?.fitAll();
          thereViewer.current?.fitAll();
        }} disabled={!hereSource}>
          Tout voir
        </Button>
      </ToolGroup>
      <ToolSeparator />
      <Button icon="close" onClick={() => onClose(changed ? project : null)}>
        Fermer
      </Button>
    </Toolbar>
  );

  const sidebar = (
    <Panel side="left" title="Photos">
      {photos.length > 0 ? (
        <List
          label="Photos"
          onMove={(step) => {
            const next = photos[Math.min(photos.length - 1, Math.max(0, index + step))];
            if (next) setSelectedId(next.id);
          }}
        >
          {photos.map((p, i) => {
            const line = photoLine(i, report?.photos[i], names);
            return (
              <ListRow
                key={p.id}
                selected={p.id === selectedId}
                onSelect={() => setSelectedId(p.id)}
                leading={<Dot tone={line.tone} />}
                title={p.name}
                detail={engine ? line.detail : i === 0 ? "référence" : "exemple"}
                actions={
                  engine && (
                    <IconButton icon="trash" label={`Retirer ${p.name}`} size="sm" onClick={() => setConfirm({ kind: "remove", id: p.id })} disabled={!ready} />
                  )
                }
              />
            );
          })}
        </List>
      ) : (
        <p className="ui-list-empty">{engine ? "Aucune photo. « Ajouter des photos… » : la première sert de référence." : "Exemple sans moteur."}</p>
      )}
    </Panel>
  );

  const inspector = (
    <Panel side="right" title={photo ? photo.name : "Photo"}>
      {photo && (
        <Section>
          <dl className="ui-facts">
            <dt>Taille</dt>
            <dd>{formatSize(photo.width, photo.height)}</dd>
            {photoReport?.meanResidualPx !== undefined && photoReport.meanResidualPx !== null && (
              <>
                <dt>Écart moyen</dt>
                <dd>{formatPx(photoReport.meanResidualPx)}</dd>
              </>
            )}
            {photoReport?.problem && (
              <>
                <dt>À faire</dt>
                <dd>{photoReport.problem}</dd>
              </>
            )}
          </dl>
        </Section>
      )}
      {photo && !isReference && (
        <Section title="Appui">
          <Property label="S'appuie sur" hint="La photo voisine avec laquelle elle partage le plus de détails.">
            <select
              className="ui-select"
              value={anchor?.id ?? ""}
              disabled={!engine || !ready}
              onChange={(event) => {
                const id = event.target.value;
                if (pairs.length > 0) setConfirm({ kind: "anchor", id });
                else changeAnchor(id);
              }}
              aria-label="Photo d'appui"
            >
              {!anchor && <option value="">—</option>}
              {candidates.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </Property>
        </Section>
      )}
      {photo && !isReference && (
        <Section title="Points communs">
          {pairs.length > 0 ? (
            <List
              label="Points communs"
              onMove={(step) => {
                setPending(NO_PENDING);
                setSelected(selected === null ? 0 : Math.min(pairs.length - 1, Math.max(0, selected + step)));
              }}
            >
              {pairs.map((_, i) => {
                const r = photoReport?.pairs[i];
                const status = pairStatus(r);
                return (
                  <ListRow
                    key={i}
                    selected={selected === i}
                    tone={status === "aberrant" ? "danger" : undefined}
                    onSelect={() => {
                      setPending(NO_PENDING);
                      setSelected(selected === i ? null : i);
                    }}
                    tooltip={r?.outlier ? "Point douteux : à vérifier, déplacer ou supprimer" : "Cliquer pour choisir ce point, puis le glisser pour le déplacer"}
                    leading={<Dot tone={tones[status]} />}
                    title={`Point ${i + 1}`}
                    detail={`écart ${formatPx(r?.residualPx)}`}
                    actions={
                      <IconButton icon="trash" label={`Supprimer le point ${i + 1}`} shortcut="backspace" size="sm" onClick={() => commit(removePair(pairs, i))} disabled={!ready} />
                    }
                  />
                );
              })}
            </List>
          ) : (
            <p className="ui-list-empty">« Raccorder automatiquement » les trouve. À la main : cliquer un détail sur la photo de gauche, puis le même sur celle de droite ; quatre points au moins, bien répartis.</p>
          )}
        </Section>
      )}
      {isReference && <p className="ui-list-empty">La première photo sert de référence : les autres viennent s'y raccorder, directement ou de proche en proche.</p>}
    </Panel>
  );

  const status = (
    <StatusBar>
      {instruction ? (
        <StatusText tone="info">{instruction} (Échap : annuler)</StatusText>
      ) : (
        <StatusText>{engine ? summary(report, photos.length) : "Exemple sans moteur : rien n'est enregistré."}</StatusText>
      )}
      <span className="ui-legend push">
        <span>
          <Dot tone="success" /> &lt; 3 px
        </span>
        <span>
          <Dot tone="warning" /> au-delà
        </span>
        <span>
          <Dot tone="danger" /> douteux
        </span>
        <span>
          <Dot tone="info" /> pas encore mesuré
        </span>
      </span>
    </StatusBar>
  );

  const confirmed = confirm ? photos.find((p) => p.id === confirm.id) : null;

  return (
    <div className="assembly-overlay" role="dialog" aria-label="Assemblage de photos de façade">
      <Screen
        className="assembly"
        toolbar={toolbar}
        banner={
          error ? (
            <Banner tone="danger" onClose={() => setError(null)}>
              {error}
            </Banner>
          ) : (
            notice && (
              <Banner tone={notice.tone} onClose={() => setNotice(null)}>
                {notice.text}
              </Banner>
            )
          )
        }
        sidebar={sidebar}
        inspector={inspector}
        status={status}
      >
        <div className="registration-views">
          <section className="registration-pane" ref={herePane}>
            <h2>À placer {photo ? `· ${photo.name}` : ""}</h2>
            <div className={stageClass} onPointerDownCapture={onPointerDown("here")} onPointerMove={onPointerMove("here")} onPointerUp={onPointerUp("here")}>
              <Viewer ref={hereViewer} layers={hereLayers} cursorUnit="px" />
            </div>
          </section>
          <section className="registration-pane" ref={therePane}>
            <h2>Photo d'appui {anchor ? `· ${anchor.name}` : ""}</h2>
            <div className={stageClass} onPointerDownCapture={onPointerDown("there")} onPointerMove={onPointerMove("there")} onPointerUp={onPointerUp("there")}>
              <Viewer ref={thereViewer} layers={thereLayers} cursorUnit="px" />
            </div>
          </section>
          {photos.length === 0 && !busy && (
            <EmptyState
              icon="image"
              title="Assembler des photos de la façade"
              actions={
                engine && (
                  <Button variant="primary" icon="plus" onClick={() => void addPhotos()}>
                    Ajouter des photos…
                  </Button>
                )
              }
            >
              <p>Sans plan d'architecte : des photos prises de face, qui se recouvrent d'un tiers environ, assemblées en une seule image. L'app trouve elle-même les détails communs.</p>
              {!engine && <small>Dans ce navigateur, tuiler deux photos dans <code>tuiles-demo/</code> (voir <code>docs/assemblage.md</code>).</small>}
            </EmptyState>
          )}
          {busy && <Busy>{busy}</Busy>}
        </div>
      </Screen>
      {confirm && confirmed && confirm.kind === "remove" && (
        <ConfirmDialog
          title={`Retirer « ${confirmed.name} » de l'assemblage ?`}
          confirmLabel="Retirer la photo"
          onCancel={() => setConfirm(null)}
          onConfirm={() => {
            setConfirm(null);
            void removePhoto(confirmed.id);
          }}
        >
          <p>Ses points communs sont perdus. Le fichier reste dans le dossier du projet.</p>
        </ConfirmDialog>
      )}
      {confirm && confirm.kind === "auto" && (
        <ConfirmDialog
          title="Raccorder automatiquement toutes les photos ?"
          confirmLabel="Raccorder automatiquement"
          onCancel={() => setConfirm(null)}
          onConfirm={() => void linkAutomatically()}
        >
          <p>Les points communs déjà posés sont remplacés par ceux que l'app trouve, et chaque photo peut changer de photo d'appui. Une photo sans détail commun avec les autres garde ses points.</p>
        </ConfirmDialog>
      )}
      {confirm && confirmed && confirm.kind === "anchor" && (
        <ConfirmDialog
          title={`S'appuyer sur « ${confirmed.name} » ?`}
          confirmLabel="Changer d'appui"
          onCancel={() => setConfirm(null)}
          onConfirm={() => changeAnchor(confirmed.id)}
        >
          <p>
            Les {pairs.length} points communs posés avec « {anchor?.name} » sont effacés : ils ne valent que pour cette photo-là.
          </p>
        </ConfirmDialog>
      )}
    </div>
  );
}

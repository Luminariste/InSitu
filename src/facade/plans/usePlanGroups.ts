// Plans ajoutés à l'espace de travail de la façade (D-PL-a à g) : leur
// affichage, leur sélection, le déplacement à la souris et le calage par
// deux points. Les calculs sont dans le moteur ; ce crochet ne fait que les
// appeler et montrer le résultat.

import { useCallback, useEffect, useMemo, useRef, useState, type PointerEvent, type RefObject } from "react";
import {
  addGroupViews,
  addPlanGroup,
  alignPlanGroup,
  choosePlanFile,
  inspectPlan,
  movePlanGroup,
  openPlanView,
  placePlanGroup,
  removePlan,
  renamePlan,
  rotatePlanGroup,
  setPlanHidden,
  type FacadeView,
  type PlanChoice,
  type PlanGroup,
  type PlanInfo,
  type Project,
} from "../../engine";
import { setViewerCursor, type LayerSpec, type Point, type ViewerHandle } from "../../viewer";
import { screenToMm } from "../../viewer/geometry";
import { isClick } from "../scale";
import { AlignMarkersNode } from "./AlignMarkersNode";
import {
  alignPrompt,
  dragOrigin,
  isShown,
  nextAlignStep,
  planGroupLayers,
  viewKey,
  type AlignStep,
  type GroupOverride,
} from "./planGroups";

/** Ce qu'on fait sur la toile avec le plan choisi. */
export type PlanMode = "off" | "move" | AlignStep;

/** Un fichier lu, en attente du choix de ses pages ou calques. */
export interface PendingPlanFile {
  path: string;
  info: PlanInfo;
  /** Ajout de vues à ce groupe ; absent : nouveau groupe. */
  group?: string;
}

interface Options {
  engine: boolean;
  folder: string | null;
  project: Project | null;
  onProject: (project: Project) => void;
  viewer: RefObject<ViewerHandle | null>;
  stage: RefObject<HTMLDivElement | null>;
  /** Plans en négatif (menu Affichage). */
  negative: boolean;
  /** Le plan de référence est à l'échelle : on peut poser des plans dessus. */
  scaled: boolean;
  onError: (message: string) => void;
  onNotice: (message: string) => void;
}

export interface PlanGroupsState {
  groups: PlanGroup[];
  /** Feuilles blanches, à mettre sous tous les plans. */
  sheets: LayerSpec[];
  /** Les vues montrées, au-dessus du plan de référence. */
  layers: LayerSpec[];
  selected: PlanGroup | null;
  select: (id: string | null) => void;
  mode: PlanMode;
  setMode: (mode: PlanMode) => void;
  busy: string | null;
  pending: PendingPlanFile | null;
  cancelPending: () => void;
  /** Consigne du geste en cours, pour la barre d'état. */
  hint: string | null;
  /** Le premier geste du calage a commencé : les points posés (1, 1′, 2, 2′). */
  alignPoints: Point[];
  actions: {
    chooseFile: (group?: string) => Promise<void>;
    confirmPending: (choices: PlanChoice[]) => Promise<void>;
    place: (id: string, origin: Point, mmPerUnit: number) => Promise<void>;
    rotate: (id: string, turns: number) => Promise<void>;
    setHidden: (group: string, plan: string | null, hidden: boolean) => Promise<void>;
    rename: (group: string, plan: string | null, name: string) => Promise<void>;
    remove: (group: string, plan: string | null) => Promise<void>;
    move: (group: string, to: number) => Promise<void>;
  };
  onPointerDown: (event: PointerEvent<HTMLDivElement>) => boolean;
  onPointerMove: (event: PointerEvent<HTMLDivElement>) => boolean;
  onPointerUp: (event: PointerEvent<HTMLDivElement>) => boolean;
}

export function usePlanGroups(options: Options): PlanGroupsState {
  const { engine, folder, project, onProject, viewer, stage, negative, scaled, onError, onNotice } = options;
  const groups = useMemo(() => project?.planGroups ?? [], [project?.planGroups]);
  const [views, setViews] = useState<Map<string, FacadeView>>(() => new Map());
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [mode, setModeState] = useState<PlanMode>("off");
  const [busy, setBusy] = useState<string | null>(null);
  const [pending, setPending] = useState<PendingPlanFile | null>(null);
  const [override, setOverride] = useState<GroupOverride | null>(null);
  const [alignPoints, setAlignPoints] = useState<Point[]>([]);
  const drag = useRef<{ pointerId: number; start: Point; origin: Point; mmPerUnit: number; group: string } | null>(null);
  const down = useRef<{ x: number; y: number; t: number } | null>(null);
  const markers = useRef<AlignMarkersNode | null>(null);

  const selected = groups.find((g) => g.id === selectedId) ?? null;

  // Un projet rouvert (autre projet, ⌘Z…) : la sélection disparue est oubliée.
  useEffect(() => {
    if (selectedId && !groups.some((g) => g.id === selectedId)) setSelectedId(null);
  }, [groups, selectedId]);

  const setMode = useCallback((next: PlanMode) => {
    setModeState(next);
    setAlignPoints([]);
    setOverride(null);
    drag.current = null;
  }, []);

  // Le plan choisi change : on sort du geste en cours.
  useEffect(() => setMode("off"), [selectedId, setMode]);

  // Affichages à préparer : les vues montrées dont on n'a pas encore les tuiles.
  const wanted = useMemo(
    () => groups.flatMap((g) => g.plans.filter((p) => isShown(g, p)).map((p) => ({ group: g, plan: p, key: viewKey(g, p) }))),
    [groups],
  );
  const loading = useRef(new Set<string>());
  useEffect(() => {
    if (!engine || !folder) return;
    for (const { group, plan, key } of wanted) {
      if (views.has(key) || loading.current.has(key)) continue;
      loading.current.add(key);
      openPlanView(folder, group.id, plan.id)
        .then((view) => setViews((current) => new Map(current).set(key, view)))
        .catch((failure: unknown) => onError(`Plan « ${plan.name} » : ${String(failure).replace(/^Error: /, "")}`))
        .finally(() => loading.current.delete(key));
    }
  }, [engine, folder, wanted, views, onError]);

  // Un autre projet : les affichages préparés ne valent plus.
  useEffect(() => {
    setViews(new Map());
    setSelectedId(null);
  }, [folder]);

  const { sheets, plans: layers } = useMemo(
    () => planGroupLayers(groups, views, { negative, override }),
    [groups, views, negative, override],
  );

  // Repères du calage, posés sur la visionneuse quand elle existe.
  useEffect(() => {
    let remove: (() => void) | null = null;
    let frame = 0;
    const attach = () => {
      const scene = viewer.current?.scene();
      if (!scene) {
        frame = requestAnimationFrame(attach);
        return;
      }
      const node = new AlignMarkersNode();
      markers.current = node;
      remove = scene.addNode(node);
    };
    attach();
    return () => {
      cancelAnimationFrame(frame);
      markers.current = null;
      remove?.();
    };
  }, [viewer]);
  useEffect(() => {
    markers.current?.setPoints(alignPoints);
    viewer.current?.scene()?.invalidate();
  }, [alignPoints, viewer]);

  // Curseur : une main pour déplacer, une croix pour viser.
  useEffect(() => {
    setViewerCursor(stage.current, mode === "move" ? "move" : mode === "off" ? "" : "crosshair");
    return () => setViewerCursor(stage.current, "");
  }, [mode, stage]);

  const run = useCallback(
    async (label: string, task: () => Promise<void>) => {
      if (!folder) return;
      setBusy(label);
      try {
        await task();
      } catch (failure) {
        onError(String(failure).replace(/^Error: /, ""));
      } finally {
        setBusy(null);
      }
    },
    [folder, onError],
  );

  const update = useCallback(
    (label: string, call: (folder: string) => Promise<Project>) =>
      run(label, async () => {
        onProject(await call(folder!));
      }),
    [run, folder, onProject],
  );

  const actions: PlanGroupsState["actions"] = {
    chooseFile: (group) =>
      run("Lecture du plan…", async () => {
        const path = await choosePlanFile(group ? "Ajouter des pages ou des calques" : "Ajouter un plan");
        if (!path) return;
        const info = await inspectPlan(path);
        const many = (info.layers?.length ?? 0) > 0 || (info.kind === "pdf" && (info.pageCount ?? 1) > 1);
        if (many || group) {
          setPending({ path, info, group });
          return;
        }
        const next = await addPlanGroup(folder!, path, [{}]);
        onProject(next);
        setSelectedId(next.planGroups?.at(-1)?.id ?? null);
      }),
    confirmPending: async (choices) => {
      const current = pending;
      if (!current) return;
      setPending(null);
      await run("Ajout du plan…", async () => {
        if (current.group) {
          onProject(await addGroupViews(folder!, current.group, choices));
          return;
        }
        const next = await addPlanGroup(folder!, current.path, choices);
        onProject(next);
        setSelectedId(next.planGroups?.at(-1)?.id ?? null);
      });
    },
    place: (id, origin, mmPerUnit) => update("Pose du plan…", (f) => placePlanGroup(f, id, origin, mmPerUnit)),
    rotate: (id, turns) => update("Rotation du plan…", (f) => rotatePlanGroup(f, id, turns)),
    setHidden: (group, plan, hidden) => update(hidden ? "Plan caché…" : "Plan montré…", (f) => setPlanHidden(f, group, plan, hidden)),
    rename: (group, plan, name) => update("Nouveau nom…", (f) => renamePlan(f, group, plan, name)),
    remove: (group, plan) => update("Retrait du plan…", (f) => removePlan(f, group, plan)),
    move: (group, to) => update("Ordre des plans…", (f) => movePlanGroup(f, group, to)),
  };

  // --- Gestes sur la toile ----------------------------------------------------

  const pointerMm = (event: PointerEvent<HTMLDivElement>): Point | null => {
    const scene = viewer.current?.scene();
    const canvas = stage.current?.querySelector(".viewer-canvas");
    if (!scene || !canvas) return null;
    const box = canvas.getBoundingClientRect();
    return screenToMm(scene.getView(), scene.getViewport(), { x: event.clientX - box.left, y: event.clientY - box.top });
  };

  const onPointerDown = (event: PointerEvent<HTMLDivElement>): boolean => {
    if (mode === "off" || !selected || event.button !== 0) return false;
    if (mode !== "move") {
      // Calage : un clic pose un point ; glisser déplace la vue comme d'habitude.
      down.current = { x: event.clientX, y: event.clientY, t: event.timeStamp };
      return false;
    }
    const at = pointerMm(event);
    if (!at) return false;
    event.stopPropagation();
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    // La vue ne se recadre plus toute seule pendant le geste : sinon elle
    // suivrait le plan déplacé et le pointeur glisserait sous lui.
    const scene = viewer.current?.scene();
    if (scene) scene.setView(scene.getView());
    drag.current = { pointerId: event.pointerId, start: at, origin: selected.origin, mmPerUnit: selected.mmPerUnit, group: selected.id };
    return true;
  };

  const onPointerMove = (event: PointerEvent<HTMLDivElement>): boolean => {
    const current = drag.current;
    if (!current || current.pointerId !== event.pointerId) return false;
    const at = pointerMm(event);
    if (!at) return true;
    setOverride({ group: current.group, origin: dragOrigin(current.origin, current.start, at), mmPerUnit: current.mmPerUnit });
    return true;
  };

  const onPointerUp = (event: PointerEvent<HTMLDivElement>): boolean => {
    const current = drag.current;
    if (current && current.pointerId === event.pointerId) {
      drag.current = null;
      if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
      const at = pointerMm(event);
      const origin = at ? dragOrigin(current.origin, current.start, at) : current.origin;
      if (origin.x !== current.origin.x || origin.y !== current.origin.y) {
        void actions.place(current.group, origin, current.mmPerUnit).finally(() => setOverride(null));
      } else setOverride(null);
      return true;
    }
    if (mode === "off" || mode === "move" || !selected) return false;
    const start = down.current;
    down.current = null;
    if (!start || !isClick(start, { x: event.clientX, y: event.clientY, t: event.timeStamp })) return false;
    const at = pointerMm(event);
    if (!at) return false;
    const points = [...alignPoints, at];
    const next = nextAlignStep(mode);
    if (next) {
      setAlignPoints(points);
      setModeState(next);
      return true;
    }
    const group = selected.id;
    setAlignPoints(points);
    void run("Calage du plan…", async () => {
      const result = await alignPlanGroup(folder!, group, [points[0], points[2]], [points[1], points[3]]);
      onProject(result.project);
      const turn = Math.abs(result.turnDegrees);
      onNotice(
        turn >= 0.5
          ? `Plan calé. Les deux segments diffèrent de ${turn.toLocaleString("fr-FR", { maximumFractionDigits: 1 })}° : un plan ne tourne que par quarts de tour, cet écart reste.`
          : "Plan calé sur la façade par deux points.",
      );
    }).finally(() => setMode("off"));
    return true;
  };

  const hint =
    mode === "move"
      ? "Glisser pour déplacer le plan choisi (Échap : terminer)"
      : mode !== "off"
        ? `${alignPrompt(mode)} (Échap : annuler)`
        : null;

  return {
    groups,
    sheets: scaled ? sheets : [],
    layers: scaled ? layers : [],
    selected,
    select: setSelectedId,
    mode,
    setMode,
    busy,
    pending,
    cancelPending: () => setPending(null),
    hint,
    alignPoints,
    actions,
    onPointerDown,
    onPointerMove,
    onPointerUp,
  };
}

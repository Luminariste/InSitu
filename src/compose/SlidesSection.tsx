// Rubrique « Diapositives » du panneau de gauche de l'écran Façade : les
// dessins posés rangés par diapositive, comme les calques d'Illustrator
// (demande de Benjamin, 12 novembre 2026). Le dessin du haut d'un groupe est
// devant. Glisser une ligne sur le titre d'une autre diapositive l'y affecte
// à la main ; la glisser sur un dessin la passe juste devant lui. Comme dans
// Illustrator : clic, ⌘-clic et ⇧-clic choisissent, l'œil masque, le cadenas
// verrouille, le clic droit ouvre un menu (grouper, ordre, diapositive…).

import { useRef, useState, type DragEvent, type MouseEvent } from "react";
import { cssColor, projectorColor } from "../facade/overview";
import { ContextMenu, Icon, IconButton, Section, type MenuEntry } from "../ui";
import type { Placement } from "./placement";
import { treeClick, type SlideGroup, type TreeDrop } from "./slideTree";
import type { CompositionActions } from "./useComposition";

const ROW_DRAG_TYPE = "application/x-insitu-placement";

/** Ligne en cours de glisser (la vue web ne rend pas toujours les données pendant le survol). */
let dragging: string | null = null;

interface Props {
  groups: SlideGroup[];
  selected: Set<string>;
  /** Nom et vignette du dessin d'un dessin posé. */
  describe: (p: Placement) => { name: string; url?: string | null };
  actions: CompositionActions;
  /** Pourquoi l'arborescence n'est pas disponible ; `null` si elle l'est. */
  blocked: string | null;
  /**
   * Diapositives masquées (Façade seulement) : un œil sur la ligne de la
   * diapositive cache ses dessins à l'écran, sans toucher au projet ni à
   * l'export. Absent : pas d'œil.
   */
  hiddenSlides?: ReadonlySet<string>;
  onToggleSlide?: (projector: string) => void;
}

export function SlidesSection({ groups, selected, describe, actions, blocked, hiddenSlides, onToggleSlide }: Props) {
  const [closed, setClosed] = useState<Set<string>>(new Set());
  /** Cible survolée : `g:<projecteur>` ou `r:<dessin>`. */
  const [over, setOver] = useState<string | null>(null);
  /** Dernière ligne cliquée, point de départ du ⇧-clic. */
  const anchor = useRef<string | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; row: Placement } | null>(null);
  if (blocked) return null;
  if (groups.length === 0) return null;

  const order = groups.flatMap((g) => g.rows.map((r) => r.placement.id));
  const click = (id: string) => (event: MouseEvent) => {
    const next = treeClick(order, selected, anchor.current, id, {
      toggle: event.metaKey || event.ctrlKey,
      range: event.shiftKey,
    });
    if (!event.shiftKey) anchor.current = id;
    actions.selectInTree(next);
  };
  const openMenu = (row: Placement) => (event: MouseEvent) => {
    event.preventDefault();
    if (!selected.has(row.id)) {
      anchor.current = row.id;
      actions.selectInTree([row.id]);
    }
    setMenu({ x: event.clientX, y: event.clientY, row });
  };

  const accept = (key: string) => (event: DragEvent) => {
    if (dragging === null && !event.dataTransfer.types.includes(ROW_DRAG_TYPE)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    if (over !== key) setOver(key);
  };
  const release = (drop: TreeDrop) => (event: DragEvent) => {
    const id = event.dataTransfer.getData(ROW_DRAG_TYPE) || dragging;
    dragging = null;
    setOver(null);
    if (!id) return;
    event.preventDefault();
    event.stopPropagation();
    actions.dropInTree(id, drop);
  };
  const toggle = (key: string) =>
    setClosed((before) => {
      const next = new Set(before);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  return (
    <Section title="Diapositives">
      <div
        className="compose-tree"
        role="tree"
        aria-label="Dessins par diapositive"
        onDragLeave={(e) => e.currentTarget === e.target && setOver(null)}
      >
        {groups.map((g) => {
          const key = g.projector ?? "aucune";
          const open = !closed.has(key);
          const droppable = g.projector !== null;
          const color = g.colorIndex === null ? null : cssColor(projectorColor(g.colorIndex));
          const slideHidden = g.projector !== null && (hiddenSlides?.has(g.projector) ?? false);
          return (
            <div
              key={key}
              role="treeitem"
              aria-expanded={open}
              className={`compose-tree-group${slideHidden ? " is-slide-hidden" : ""}`}
            >
              <div
                className={`compose-tree-head${over === `g:${key}` ? " is-over" : ""}`}
                onClick={() => toggle(key)}
                onDragOver={droppable ? accept(`g:${key}`) : undefined}
                onDrop={droppable ? release({ projector: g.projector! }) : undefined}
                title={
                  droppable
                    ? "Glisser un dessin ici pour l'envoyer dans cette diapositive"
                    : "Dessins qui ne tombent dans aucune emprise"
                }
              >
                <Icon name={open ? "chevronDown" : "chevronRight"} />
                <span className="ui-swatch" style={{ background: color ?? "transparent" }} />
                <span className="compose-tree-label">{g.label}</span>
                <span className="compose-tree-count">{g.rows.length}</span>
                {onToggleSlide && g.projector !== null && (
                  <IconButton
                    icon={slideHidden ? "eyeOff" : "eye"}
                    label={
                      slideHidden
                        ? "Afficher les dessins de cette diapositive"
                        : "Masquer les dessins de cette diapositive (à l'écran seulement, l'export les garde)"
                    }
                    size="sm"
                    onClick={(e) => {
                      e.stopPropagation();
                      onToggleSlide(g.projector!);
                    }}
                  />
                )}
              </div>
              {open && (
                <div role="group">
                  {g.rows.length === 0 && <p className="compose-tree-empty">Aucun dessin</p>}
                  {g.rows.map((r) => {
                    const { name, url } = describe(r.placement);
                    const id = r.placement.id;
                    const hidden = r.placement.hidden === true;
                    const locked = r.placement.locked === true;
                    const notes = [
                      r.manual ? "choisie à la main" : null,
                      r.whole ? null : g.projector ? "à cheval" : null,
                    ].filter(Boolean);
                    if (r.placement.group) notes.unshift("groupé");
                    return (
                      <div
                        key={id}
                        role="treeitem"
                        aria-selected={selected.has(id)}
                        className={`compose-tree-row${selected.has(id) ? " is-selected" : ""}${over === `r:${id}` ? " is-over" : ""}${r.whole ? "" : " is-cut"}${hidden ? " is-hidden" : ""}${locked ? " is-locked" : ""}`}
                        draggable
                        onDragStart={(e) => {
                          dragging = id;
                          e.dataTransfer.setData(ROW_DRAG_TYPE, id);
                          e.dataTransfer.effectAllowed = "move";
                        }}
                        onDragEnd={() => {
                          dragging = null;
                          setOver(null);
                        }}
                        onDragOver={droppable ? accept(`r:${id}`) : undefined}
                        onDrop={droppable ? release({ projector: g.projector!, before: id }) : undefined}
                        onClick={click(id)}
                        onContextMenu={openMenu(r.placement)}
                        title="Clic : choisir. ⌘-clic : ajouter ou retirer. ⇧-clic : choisir jusqu'ici. Clic droit : menu. Glisser : sur une autre diapositive, ou au-dessus d'un dessin pour le passer devant."
                      >
                        {url ? <img src={url} alt="" draggable={false} /> : <span className="compose-tree-thumb" />}
                        <span className="compose-tree-label">{name}</span>
                        {notes.length > 0 && <span className="compose-tree-note">{notes.join(", ")}</span>}
                        <span className="compose-tree-flags" onClick={(e) => e.stopPropagation()}>
                          <IconButton
                            icon={hidden ? "eyeOff" : "eye"}
                            size="sm"
                            label={hidden ? "Afficher ce dessin" : "Masquer ce dessin"}
                            className={hidden ? "is-on" : undefined}
                            onClick={() => actions.setHidden([id], !hidden)}
                          />
                          <IconButton
                            icon={locked ? "lock" : "lockOpen"}
                            size="sm"
                            label={locked ? "Déverrouiller ce dessin" : "Verrouiller ce dessin"}
                            className={locked ? "is-on" : undefined}
                            onClick={() => actions.setLocked([id], !locked)}
                          />
                        </span>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
      </div>
      {menu && (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          items={menuItems(menu.row, selected, groups, actions)}
          onClose={() => setMenu(null)}
        />
      )}
    </Section>
  );
}

/**
 * Le menu au clic droit d'une ligne. Il porte sur la sélection si la ligne
 * en fait partie, sinon sur la ligne seule (qu'un clic droit choisit). Un
 * dessin masqué ou verrouillé ne se choisit pas : seuls l'œil et le cadenas
 * s'appliquent alors.
 */
function menuItems(
  row: Placement,
  selected: Set<string>,
  groups: SlideGroup[],
  actions: CompositionActions,
): MenuEntry[] {
  const all = groups.flatMap((g) => g.rows.map((r) => r.placement));
  const targets = selected.has(row.id) ? all.filter((p) => selected.has(p.id)) : [row];
  const ids = targets.map((p) => p.id);
  const free = !row.hidden && !row.locked;
  const hidden = targets.every((p) => p.hidden);
  const locked = targets.every((p) => p.locked);
  const slides = groups.filter((g) => g.projector !== null);
  return [
    { label: "Grouper", shortcut: "mod+g", onSelect: actions.group, disabled: !free || targets.length < 2 },
    {
      label: "Dégrouper",
      shortcut: "mod+shift+g",
      onSelect: actions.ungroup,
      disabled: !free || !targets.some((p) => p.group),
    },
    { separator: true },
    {
      label: hidden ? "Afficher" : "Masquer",
      icon: hidden ? "eye" : "eyeOff",
      onSelect: () => actions.setHidden(ids, !hidden),
    },
    {
      label: locked ? "Déverrouiller" : "Verrouiller",
      icon: locked ? "lockOpen" : "lock",
      onSelect: () => actions.setLocked(ids, !locked),
    },
    { separator: true },
    { label: "Premier plan", shortcut: "mod+shift+]", onSelect: () => actions.arrange("front"), disabled: !free },
    { label: "En avant", shortcut: "mod+]", onSelect: () => actions.arrange("forward"), disabled: !free },
    { label: "En arrière", shortcut: "mod+[", onSelect: () => actions.arrange("backward"), disabled: !free },
    { label: "Arrière-plan", shortcut: "mod+shift+[", onSelect: () => actions.arrange("back"), disabled: !free },
    ...(slides.length > 0
      ? ([
          { separator: true },
          { heading: "Envoyer vers la diapositive" },
          { label: "Automatique", onSelect: () => actions.assignTo(null), disabled: !free },
          ...slides.map((g) => ({ label: g.label, onSelect: () => actions.assignTo(g.projector), disabled: !free })),
        ] as MenuEntry[])
      : []),
    { separator: true },
    { label: "Supprimer", icon: "trash", danger: true, onSelect: actions.remove, disabled: !free },
  ];
}

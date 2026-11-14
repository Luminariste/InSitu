// Composants communs de l'interface (docs/design.md). Un écran se compose
// de ces pièces ; il n'invente pas son propre bouton, son propre panneau ni
// sa propre barre. Les styles sont dans `ui.css`, les valeurs dans
// `tokens.css`.

import {
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
} from "react";
import { Icon, type IconName } from "./icons";
import { formatShortcut, isMac, isTextEntry, matchesShortcut, type ShortcutSpec } from "./shortcuts";

function cx(...names: (string | false | null | undefined)[]): string {
  return names.filter(Boolean).join(" ");
}

/** Bulle d'aide : le texte, suivi du raccourci s'il y en a un. */
function tooltip(text: string | undefined, shortcut: ShortcutSpec | undefined): string | undefined {
  if (!shortcut) return text;
  return `${text ?? ""} (${formatShortcut(shortcut, isMac())})`.trim();
}

// --- Boutons -------------------------------------------------------------------

export type ButtonVariant = "default" | "primary" | "quiet" | "danger";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** default : action ordinaire ; primary : l'action principale de la zone (une seule) ;
   *  quiet : action secondaire sans cadre ; danger : suppression. */
  variant?: ButtonVariant;
  icon?: IconName;
  /** Raccourci affiché dans la bulle d'aide (le déclencher reste à `useShortcuts`). */
  shortcut?: ShortcutSpec;
  /** Bouton à bascule enfoncé (mode en cours). */
  active?: boolean;
  size?: "md" | "sm";
}

export function Button({ variant = "default", icon, shortcut, active, size = "md", className, title, children, ...rest }: ButtonProps) {
  return (
    <button
      type="button"
      {...rest}
      title={tooltip(title, shortcut)}
      aria-pressed={active === undefined ? undefined : active}
      className={cx("ui-button", `ui-button-${variant}`, size === "sm" && "ui-button-sm", active && "is-active", !children && "ui-button-icon", className)}
    >
      {icon && <Icon name={icon} />}
      {children && <span className="ui-button-label">{children}</span>}
    </button>
  );
}

export interface IconButtonProps extends Omit<ButtonProps, "children" | "icon"> {
  icon: IconName;
  /** Ce que fait le bouton : lu par la bulle d'aide et par VoiceOver. */
  label: string;
}

export function IconButton({ icon, label, variant = "quiet", shortcut, ...rest }: IconButtonProps) {
  return <Button {...rest} variant={variant} icon={icon} title={tooltip(label, shortcut)} aria-label={label} />;
}

// --- Choix exclusif ---------------------------------------------------------------

export interface SegmentOption<T extends string> {
  value: T;
  label: ReactNode;
  disabled?: boolean;
  title?: string;
  shortcut?: ShortcutSpec;
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  label,
  size = "md",
}: {
  value: T;
  options: SegmentOption<T>[];
  onChange: (value: T) => void;
  label: string;
  size?: "md" | "lg";
}) {
  return (
    <div className={cx("ui-segmented", size === "lg" && "ui-segmented-lg")} role="radiogroup" aria-label={label}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={o.value === value}
          className={o.value === value ? "is-selected" : undefined}
          disabled={o.disabled}
          title={tooltip(o.title, o.shortcut)}
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

// --- Barre d'outils ------------------------------------------------------------------

/** Barre d'outils d'un écran : de gauche à droite, le projet, les actions
 *  de l'étape, puis (à droite, après `ToolSpacer`) l'affichage. */
export function Toolbar({ children, label = "Outils" }: { children: ReactNode; label?: string }) {
  return (
    <header className="ui-toolbar" role="toolbar" aria-label={label}>
      {children}
    </header>
  );
}

/** Groupe d'outils liés. Le libellé, facultatif, nomme le groupe quand les boutons ne suffisent pas. */
export function ToolGroup({ label, children, className }: { label?: string; children: ReactNode; className?: string }) {
  return (
    <div className={cx("ui-toolgroup", className)} role="group" aria-label={label}>
      {label && <span className="ui-toolgroup-label">{label}</span>}
      {children}
    </div>
  );
}

export function ToolSeparator() {
  return <span className="ui-toolseparator" role="separator" aria-orientation="vertical" />;
}

export function ToolSpacer() {
  return <span className="ui-toolspacer" />;
}

// --- Gabarit d'écran ------------------------------------------------------------------

/**
 * Gabarit commun : barre d'outils, bannière, [panneau des objets] zone de
 * travail [inspecteur], barre d'état. Les panneaux sont facultatifs.
 */
export function Screen({
  toolbar,
  banner,
  sidebar,
  inspector,
  status,
  children,
  className,
}: {
  toolbar: ReactNode;
  banner?: ReactNode;
  sidebar?: ReactNode;
  inspector?: ReactNode;
  status?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <main className={cx("ui-screen", className)}>
      {toolbar}
      {banner ? <div className="ui-screen-banner">{banner}</div> : <div />}
      <div className={cx("ui-screen-body", sidebar ? "has-sidebar" : null, inspector ? "has-inspector" : null)}>
        {sidebar}
        <div className="ui-workspace">{children}</div>
        {inspector}
      </div>
      {status ?? <div />}
    </main>
  );
}

// --- Panneaux --------------------------------------------------------------------------

/** Panneau latéral. À gauche (`side="left"`) : ce qu'il y a (objets, dossiers).
 *  À droite : les réglages de ce qui est choisi (inspecteur). */
export function Panel({
  title,
  actions,
  side,
  children,
  footer,
  className,
}: {
  title: string;
  actions?: ReactNode;
  side: "left" | "right";
  children: ReactNode;
  footer?: ReactNode;
  className?: string;
}) {
  return (
    <aside className={cx("ui-panel", side === "left" ? "ui-sidebar" : "ui-inspector", className)} aria-label={title}>
      <div className="ui-panel-head">
        <h2>{title}</h2>
        {actions && <div className="ui-panel-actions">{actions}</div>}
      </div>
      <div className="ui-panel-body">{children}</div>
      {footer && <div className="ui-panel-foot">{footer}</div>}
    </aside>
  );
}

// --- Onglets d'un panneau ----------------------------------------------------------------

export interface TabSpec<T extends string> {
  value: T;
  label: string;
  icon: IconName;
  /** Bulle d'aide : ce que l'onglet règle. */
  title?: string;
  /** Pastille : un nombre (sélection) ou un point coloré (à faire). */
  badge?: number | Tone;
  disabled?: boolean;
}

/** Panneau de propriétés en onglets (D15-i) : une rangée d'onglets icône et
 *  libellé à la place du titre, un seul contenu visible. Tous les contenus
 *  restent montés (cachés), pour garder leurs raccourcis et leur saisie. */
export function TabbedPanel<T extends string>({
  label,
  side,
  tabs,
  value,
  onChange,
  panes,
  className,
}: {
  label: string;
  side: "left" | "right";
  tabs: TabSpec<T>[];
  value: T;
  onChange: (value: T) => void;
  panes: Record<T, ReactNode>;
  className?: string;
}) {
  const id = useId();
  const onKey = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
    if (!step) return;
    event.preventDefault();
    const usable = tabs.filter((t) => !t.disabled);
    const index = usable.findIndex((t) => t.value === value);
    const next = usable[(index + step + usable.length) % usable.length];
    onChange(next.value);
    document.getElementById(`${id}-tab-${next.value}`)?.focus();
  };
  return (
    <aside className={cx("ui-panel", "ui-tabbed", side === "left" ? "ui-sidebar" : "ui-inspector", className)} aria-label={label}>
      <div className="ui-tabs" role="tablist" aria-label={label} onKeyDown={onKey}>
        {tabs.map((t) => {
          const selected = t.value === value;
          return (
            <button
              key={t.value}
              id={`${id}-tab-${t.value}`}
              type="button"
              role="tab"
              aria-selected={selected}
              aria-controls={`${id}-pane-${t.value}`}
              tabIndex={selected ? 0 : -1}
              className={selected ? "is-selected" : undefined}
              disabled={t.disabled}
              title={t.title ?? t.label}
              onClick={() => onChange(t.value)}
            >
              <span className="ui-tab-icon">
                <Icon name={t.icon} />
                {typeof t.badge === "number" && t.badge > 0 && <span className="ui-tab-count">{t.badge}</span>}
                {typeof t.badge === "string" && <span className={cx("ui-tab-dot", `tone-${t.badge}`)} />}
              </span>
              <span className="ui-tab-label">{t.label}</span>
            </button>
          );
        })}
      </div>
      {tabs.map((t) => (
        <div
          key={t.value}
          id={`${id}-pane-${t.value}`}
          role="tabpanel"
          aria-labelledby={`${id}-tab-${t.value}`}
          className="ui-panel-body"
          hidden={t.value !== value}
        >
          {panes[t.value]}
        </div>
      ))}
    </aside>
  );
}

/** Rubrique d'un panneau : un titre en petites capitales, un contenu. */
export function Section({ title, actions, children, className }: { title?: string; actions?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <section className={cx("ui-section", className)}>
      {(title || actions) && (
        <div className="ui-section-head">
          {title && <h3>{title}</h3>}
          {actions && <div className="ui-panel-actions">{actions}</div>}
        </div>
      )}
      {children}
    </section>
  );
}

/** Ligne de propriété d'un inspecteur : libellé à gauche, valeur ou contrôle à droite. */
export function Property({ label, children, hint }: { label: string; children: ReactNode; hint?: ReactNode }) {
  return (
    <div className="ui-property">
      <span className="ui-property-label">{label}</span>
      <div className="ui-property-value">{children}</div>
      {hint && <p className="ui-property-hint">{hint}</p>}
    </div>
  );
}

// --- Listes : choisir, ajouter, modifier, supprimer ---------------------------------------

/** Liste d'objets. ↑ et ↓ passent d'un objet à l'autre quand elle a le focus. */
export function List({
  label,
  children,
  onMove,
}: {
  label: string;
  children: ReactNode;
  /** Appelé avec -1 ou +1 quand on presse ↑ ou ↓. */
  onMove?: (step: -1 | 1) => void;
}) {
  const onKeyDown = (event: ReactKeyboardEvent) => {
    if (!onMove) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      onMove(event.key === "ArrowDown" ? 1 : -1);
    }
  };
  return (
    <div className="ui-list" role="listbox" aria-label={label} tabIndex={0} onKeyDown={onKeyDown}>
      {children}
    </div>
  );
}

export function ListRow({
  selected,
  onSelect,
  leading,
  title,
  detail,
  trailing,
  actions,
  tone,
  dimmed,
  tooltip: help,
  onDoubleClick,
}: {
  selected?: boolean;
  onSelect?: () => void;
  leading?: ReactNode;
  title: ReactNode;
  detail?: ReactNode;
  /** Valeur à droite (nombre, mesure). */
  trailing?: ReactNode;
  /** Boutons de la ligne (modifier, supprimer) : visibles au survol et sur la ligne choisie. */
  actions?: ReactNode;
  tone?: Tone;
  dimmed?: boolean;
  tooltip?: string;
  /** Double-clic : renommer, selon docs/design.md § 3. */
  onDoubleClick?: () => void;
}) {
  return (
    <div
      role="option"
      onDoubleClick={onDoubleClick}
      aria-selected={selected ?? false}
      className={cx("ui-row", selected && "is-selected", dimmed && "is-dimmed", tone && `tone-${tone}`)}
      onClick={onSelect}
      title={help}
    >
      {leading && <span className="ui-row-leading">{leading}</span>}
      <span className="ui-row-text">
        <span className="ui-row-title">{title}</span>
        {detail && <span className="ui-row-detail">{detail}</span>}
      </span>
      {trailing !== undefined && <span className="ui-row-trailing">{trailing}</span>}
      {actions && (
        <span className="ui-row-actions" onClick={(event) => event.stopPropagation()}>
          {actions}
        </span>
      )}
    </div>
  );
}

// --- États, messages, erreurs ---------------------------------------------------------

export type Tone = "neutral" | "info" | "success" | "warning" | "danger";

/** Pastille de couleur d'état (bon, moyen, douteux…). */
export function Dot({ tone, color, label }: { tone?: Tone; color?: string; label?: string }) {
  return <span className={cx("ui-dot", tone && `tone-${tone}`)} style={color ? { background: color } : undefined} aria-label={label} role={label ? "img" : undefined} />;
}

/** Zone de travail vide : ce qui manque, et le bouton qui le règle. */
export function EmptyState({ icon, title, children, actions }: { icon?: IconName; title: string; children?: ReactNode; actions?: ReactNode }) {
  return (
    <div className="ui-empty" role="status">
      {icon && (
        <span className="ui-empty-icon">
          <Icon name={icon} size={28} />
        </span>
      )}
      <h2>{title}</h2>
      {children && <div className="ui-empty-text">{children}</div>}
      {actions && <div className="ui-empty-actions">{actions}</div>}
    </div>
  );
}

/** Travail en cours : posé au centre de la zone de travail, sans la bloquer. */
export function Busy({ children }: { children: ReactNode }) {
  return (
    <div className="ui-busy" role="status" aria-live="polite">
      <span className="ui-spinner" aria-hidden="true" />
      <span>{children}</span>
    </div>
  );
}

/**
 * Voile sur la toile pendant qu'on glisse des fichiers du Finder au-dessus
 * de la fenêtre : dit ce que le lâcher va faire (D-4.1-f). Ne capte pas la
 * souris.
 */
export function DropVeil({ children }: { children: ReactNode }) {
  return (
    <div className="ui-drop-veil" role="status" aria-live="polite">
      <span className="ui-drop-veil-label">
        <Icon name="import" />
        {children}
      </span>
    </div>
  );
}

const TONE_ICON: Record<Tone, IconName> = {
  neutral: "info",
  info: "info",
  success: "check",
  warning: "warning",
  danger: "error",
};

/** Message d'une ligne sous la barre d'outils : résultat d'une action, avertissement, erreur. */
export function Banner({ tone = "info", children, actions, onClose }: { tone?: Tone; children: ReactNode; actions?: ReactNode; onClose?: () => void }) {
  return (
    <div className={cx("ui-banner", `tone-${tone}`)} role={tone === "danger" ? "alert" : "status"}>
      <Icon name={TONE_ICON[tone]} />
      <div className="ui-banner-text">{children}</div>
      {actions}
      {onClose && <IconButton icon="close" label="Fermer ce message" size="sm" onClick={onClose} />}
    </div>
  );
}

/** Barre d'état en bas de l'écran : l'état du travail, en un coup d'œil. */
export function StatusBar({ children }: { children?: ReactNode }) {
  return (
    <footer className="ui-statusbar" role="status">
      {children}
    </footer>
  );
}

export function StatusText({ tone, children, push }: { tone?: Tone; children: ReactNode; push?: boolean }) {
  return <span className={cx("ui-status-text", tone && `tone-${tone}`, push && "push")}>{children}</span>;
}

/** Barre de progression d'un long travail (import, export). */
export function Progress({ done, total, label }: { done: number; total: number; label: ReactNode }) {
  const share = total > 0 ? Math.min(1, done / total) : 0;
  return (
    <div className="ui-progress" role="progressbar" aria-valuenow={done} aria-valuemax={total}>
      <div className="ui-progress-track">
        <div className={cx("ui-progress-fill", total === 0 && "is-indeterminate")} style={{ width: total > 0 ? `${share * 100}%` : undefined }} />
      </div>
      <span>{label}</span>
    </div>
  );
}

export function Kbd({ keys }: { keys: ShortcutSpec }) {
  return <kbd className="ui-kbd">{formatShortcut(keys, isMac())}</kbd>;
}

/** Champ avec son libellé au-dessus. */
export function Field({ label, children, hint }: { label: string; children: ReactNode; hint?: ReactNode }) {
  return (
    <label className="ui-field">
      <span className="ui-field-label">{label}</span>
      {children}
      {hint && <span className="ui-field-hint">{hint}</span>}
    </label>
  );
}

// --- Menus --------------------------------------------------------------------------------

export type MenuEntry =
  | {
      label: string;
      onSelect: () => void;
      shortcut?: ShortcutSpec;
      disabled?: boolean;
      danger?: boolean;
      checked?: boolean;
      icon?: IconName;
    }
  | { separator: true }
  | { heading: string };

/** Bouton qui ouvre un menu déroulant. Échap ou un clic ailleurs le referme. */
export function MenuButton({
  label,
  icon,
  items,
  variant = "default",
  align = "start",
  disabled,
  title,
  className,
  iconOnly,
}: {
  label: ReactNode;
  icon?: IconName;
  items: MenuEntry[];
  variant?: ButtonVariant;
  align?: "start" | "end";
  disabled?: boolean;
  title?: string;
  className?: string;
  /** N'affiche que l'icône ; `label` devient la bulle d'aide. */
  iconOnly?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const id = useId();

  useEffect(() => {
    if (!open) return;
    const close = (event: MouseEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener("mousedown", close);
    // Le premier choix reçoit le focus, pour le clavier.
    list.current?.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
    return () => window.removeEventListener("mousedown", close);
  }, [open]);

  const onKeyDown = (event: ReactKeyboardEvent) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      setOpen(false);
      root.current?.querySelector<HTMLButtonElement>(".ui-menu-trigger")?.focus();
      return;
    }
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    const buttons = [...(list.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next = (at + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
    buttons[next]?.focus();
  };

  return (
    <div className="ui-menu-root" ref={root} onKeyDown={onKeyDown}>
      <button
        type="button"
        className={cx("ui-button", `ui-button-${variant}`, "ui-menu-trigger", iconOnly && "ui-button-icon", open && "is-active", className)}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={id}
        aria-label={iconOnly && typeof label === "string" ? label : undefined}
        title={title ?? (iconOnly && typeof label === "string" ? label : undefined)}
        disabled={disabled}
        onClick={() => setOpen(!open)}
      >
        {icon && <Icon name={icon} />}
        {!iconOnly && <span className="ui-button-label">{label}</span>}
        {!iconOnly && <Icon name="chevronDown" size={12} className="ui-menu-chevron" />}
      </button>
      {open && (
        <div className={cx("ui-menu", align === "end" && "align-end")} role="menu" id={id} ref={list}>
          {items.map((item, i) => {
            if ("separator" in item) return <div key={i} className="ui-menu-separator" role="separator" />;
            if ("heading" in item)
              return (
                <div key={i} className="ui-menu-heading">
                  {item.heading}
                </div>
              );
            return (
              <button
                key={i}
                type="button"
                role={item.checked === undefined ? "menuitem" : "menuitemradio"}
                aria-checked={item.checked}
                className={cx("ui-menu-item", item.danger && "is-danger")}
                disabled={item.disabled}
                onClick={() => {
                  setOpen(false);
                  item.onSelect();
                }}
              >
                <span className="ui-menu-check">{item.checked ? <Icon name="check" size={12} /> : item.icon ? <Icon name={item.icon} size={14} /> : null}</span>
                <span className="ui-menu-label">{item.label}</span>
                {item.shortcut && <span className="ui-menu-shortcut">{formatShortcut(item.shortcut, isMac())}</span>}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

/**
 * Menu au clic droit, posé au point (`x`, `y`) de la fenêtre. Échap, un clic
 * ailleurs ou un défilement le referme ; mêmes entrées que `MenuButton`.
 */
export function ContextMenu({ x, y, items, onClose }: { x: number; y: number; items: MenuEntry[]; onClose: () => void }) {
  const list = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState({ x, y });
  useEffect(() => {
    const close = (event: Event) => {
      if (!list.current?.contains(event.target as Node)) onClose();
    };
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("mousedown", close);
    window.addEventListener("wheel", close, { passive: true });
    window.addEventListener("keydown", key, true);
    // Reste dans la fenêtre : le menu remonte ou part à gauche s'il dépasse.
    const box = list.current?.getBoundingClientRect();
    if (box) setAt({ x: Math.max(4, Math.min(x, window.innerWidth - box.width - 4)), y: Math.max(4, Math.min(y, window.innerHeight - box.height - 4)) });
    list.current?.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("wheel", close);
      window.removeEventListener("keydown", key, true);
    };
  }, [x, y, onClose]);
  const onKeyDown = (event: ReactKeyboardEvent) => {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    const buttons = [...(list.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
    const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
    buttons[(i + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length]?.focus();
  };
  return (
    <div className="ui-menu ui-context-menu" role="menu" ref={list} style={{ left: at.x, top: at.y }} onKeyDown={onKeyDown} onContextMenu={(e) => e.preventDefault()}>
      {items.map((item, i) => {
        if ("separator" in item) return <div key={i} className="ui-menu-separator" role="separator" />;
        if ("heading" in item)
          return (
            <div key={i} className="ui-menu-heading">
              {item.heading}
            </div>
          );
        return (
          <button
            key={i}
            type="button"
            role={item.checked === undefined ? "menuitem" : "menuitemradio"}
            aria-checked={item.checked}
            className={cx("ui-menu-item", item.danger && "is-danger")}
            disabled={item.disabled}
            onClick={() => {
              onClose();
              item.onSelect();
            }}
          >
            <span className="ui-menu-check">{item.checked ? <Icon name="check" size={12} /> : item.icon ? <Icon name={item.icon} size={14} /> : null}</span>
            <span className="ui-menu-label">{item.label}</span>
            {item.shortcut && <span className="ui-menu-shortcut">{formatShortcut(item.shortcut, isMac())}</span>}
          </button>
        );
      })}
    </div>
  );
}

// --- Dialogues ----------------------------------------------------------------------------

/** Fenêtre de dialogue posée sur l'écran. Échap ferme ; ↩ valide l'action principale. */
export function Dialog({
  title,
  children,
  actions,
  onClose,
  wide,
  size,
}: {
  title: string;
  children: ReactNode;
  actions?: ReactNode;
  onClose: () => void;
  wide?: boolean;
  /** « large » : fenêtre qui montre une image et une liste côte à côte (aperçu d'import). */
  size?: "large";
}) {
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const target = box.current?.querySelector<HTMLElement>(".ui-button-primary, .ui-button-danger, input, select, button") ?? box.current;
    target?.focus();
    return () => previous?.focus?.();
  }, []);
  return (
    <div className="ui-scrim" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <div
        className={cx("ui-dialog", wide && "is-wide", size === "large" && "is-large")}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        ref={box}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.stopPropagation();
            onClose();
          }
        }}
      >
        <h2>{title}</h2>
        <div className="ui-dialog-body">{children}</div>
        {actions && <div className="ui-dialog-actions">{actions}</div>}
      </div>
    </div>
  );
}

/** Confirmation d'une suppression qui ne s'annule pas. Le bouton dit ce qu'il fait. */
export function ConfirmDialog({
  title,
  children,
  confirmLabel,
  onConfirm,
  onCancel,
}: {
  title: string;
  children: ReactNode;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <Dialog
      title={title}
      onClose={onCancel}
      actions={
        <>
          <Button onClick={onCancel}>Annuler</Button>
          <Button variant="danger" onClick={onConfirm}>
            {confirmLabel}
          </Button>
        </>
      }
    >
      {children}
    </Dialog>
  );
}

// --- Raccourcis clavier ---------------------------------------------------------------------

/** Raccourcis branchés en ce moment, pour la barre de menus du Mac. */
const registry = new Set<{ current: ShortcutBinding[] }>();

/**
 * Déclenche l'action du raccourci `keys` comme si la touche avait été
 * pressée. Sert à la barre de menus du Mac : quand un menu porte le
 * raccourci, la touche n'arrive pas jusqu'à l'interface. Renvoie faux si
 * aucune action n'est disponible.
 */
export function runShortcut(keys: ShortcutSpec): boolean {
  if (document.querySelector(".ui-scrim") && keys !== "mod+/") return false;
  for (const bindings of registry) {
    for (const b of bindings.current) {
      if (b.keys === keys && b.enabled !== false) {
        b.run();
        return true;
      }
    }
  }
  return false;
}

export interface ShortcutBinding {
  keys: ShortcutSpec;
  run: () => void;
  /** Faux : le raccourci est ignoré (action indisponible). */
  enabled?: boolean;
}

/**
 * Branche des raccourcis clavier tant que le composant est affiché. Sans ⌘,
 * un raccourci ne se déclenche pas pendant la saisie dans un champ, ni quand
 * un dialogue est ouvert.
 */
export function useShortcuts(bindings: ShortcutBinding[]): void {
  const current = useRef(bindings);
  current.current = bindings;
  useEffect(() => {
    registry.add(current);
    return () => {
      registry.delete(current);
    };
  }, []);
  const onKey = useCallback((event: KeyboardEvent) => {
    if (event.defaultPrevented) return;
    const mac = isMac();
    const withCommand = mac ? event.metaKey : event.ctrlKey;
    if (!withCommand && isTextEntry(event.target as HTMLElement | null)) return;
    if (document.querySelector(".ui-scrim")) return;
    for (const b of current.current) {
      if (b.enabled === false || !matchesShortcut(b.keys, event, mac)) continue;
      event.preventDefault();
      b.run();
      return;
    }
  }, []);
  useEffect(() => {
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onKey]);
}

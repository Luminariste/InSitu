// Déplacer un point choisi aux flèches du clavier (demande de Benjamin,
// D15-p) : un point d'écran par appui, dix avec ⇧, quel que soit le
// zoom. Le pas suit donc la vue, comme l'œil : zoomer donne un réglage plus
// fin. Logique pure, testée sans écran (`nudge.test.ts`), et un crochet qui
// écoute les flèches tant qu'un point est choisi.

import { useEffect, useRef } from "react";
import { isTextEntry } from "../ui/shortcuts";
import type { Point } from "./geometry";

/** Pas d'un appui sur une flèche, en points d'écran. */
export const NUDGE_PX = 1;
/** Pas avec ⇧. */
export const NUDGE_SHIFT_PX = 10;
/**
 * Délai après le dernier appui avant d'enregistrer : une flèche tenue
 * enfoncée ne demande pas un calcul au moteur à chaque répétition.
 */
export const NUDGE_SETTLE_MS = 400;

export interface ArrowInput {
  key: string;
  shiftKey: boolean;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
}

/** Décalage d'écran demandé par une frappe, ou rien si ce n'est pas une flèche seule (⇧ permis). */
export function arrowStep(event: ArrowInput): Point | null {
  if (event.metaKey || event.ctrlKey || event.altKey) return null;
  const s = event.shiftKey ? NUDGE_SHIFT_PX : NUDGE_PX;
  switch (event.key) {
    case "ArrowLeft":
      return { x: -s, y: 0 };
    case "ArrowRight":
      return { x: s, y: 0 };
    case "ArrowUp":
      return { x: 0, y: -s };
    case "ArrowDown":
      return { x: 0, y: s };
    default:
      return null;
  }
}

/** Le point décalé de `screen` points d'écran, à ce zoom (points d'écran par unité). */
export function nudged(p: Point, screen: Point, zoom: number): Point {
  return { x: p.x + screen.x / zoom, y: p.y + screen.y / zoom };
}

/**
 * Regroupe une rafale d'appuis : `run` reçoit l'état d'avant le premier
 * appui, et n'est appelé qu'une fois la rafale finie.
 */
export class NudgeSettle<T> {
  private timer: ReturnType<typeof setTimeout> | null = null;
  private before: { value: T } | null = null;
  private pending: ((before: T) => void) | null = null;

  constructor(private readonly ms = NUDGE_SETTLE_MS) {}

  push(before: T, run: (before: T) => void): void {
    if (!this.before) this.before = { value: before };
    this.pending = run;
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => this.flush(), this.ms);
  }

  /** Enregistre tout de suite ce qui attend (avant un autre geste). */
  flush(): void {
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
    const before = this.before;
    const run = this.pending;
    this.before = null;
    this.pending = null;
    if (before && run) run(before.value);
  }

  /** Oublie ce qui attend, sans l'enregistrer. */
  cancel(): void {
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
    this.before = null;
    this.pending = null;
  }

  get busy(): boolean {
    return this.before !== null;
  }
}

/**
 * Écoute les flèches tant que `enabled` : `onStep` reçoit le décalage
 * d'écran et rend vrai s'il a déplacé quelque chose. Rien pendant une
 * saisie dans un champ.
 */
export function useArrowNudge(enabled: boolean, onStep: (screen: Point) => boolean): void {
  const handler = useRef(onStep);
  handler.current = onStep;
  useEffect(() => {
    if (!enabled) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || isTextEntry(event.target as HTMLElement | null)) return;
      const step = arrowStep(event);
      if (!step) return;
      if (handler.current(step)) event.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [enabled]);
}

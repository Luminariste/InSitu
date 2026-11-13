import { afterEach, describe, expect, it, vi } from "vitest";
import { NUDGE_PX, NUDGE_SHIFT_PX, NudgeSettle, arrowStep, nudged } from "./nudge";

const key = (k: string, mods: Partial<{ shiftKey: boolean; metaKey: boolean; ctrlKey: boolean; altKey: boolean }> = {}) => ({
  key: k,
  shiftKey: false,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  ...mods,
});

describe("déplacer un point aux flèches", () => {
  afterEach(() => vi.useRealTimers());

  it("une flèche donne un point d'écran, dix avec ⇧", () => {
    expect(arrowStep(key("ArrowLeft"))).toEqual({ x: -NUDGE_PX, y: 0 });
    expect(arrowStep(key("ArrowDown"))).toEqual({ x: 0, y: NUDGE_PX });
    expect(arrowStep(key("ArrowRight", { shiftKey: true }))).toEqual({ x: NUDGE_SHIFT_PX, y: 0 });
    expect(arrowStep(key("ArrowUp", { shiftKey: true }))).toEqual({ x: 0, y: -NUDGE_SHIFT_PX });
    expect(NUDGE_SHIFT_PX).toBeGreaterThan(NUDGE_PX);
  });

  it("⌘, ⌃ ou ⌥ avec une flèche ne déplacent pas le point", () => {
    expect(arrowStep(key("ArrowLeft", { metaKey: true }))).toBeNull();
    expect(arrowStep(key("ArrowLeft", { ctrlKey: true }))).toBeNull();
    expect(arrowStep(key("ArrowLeft", { altKey: true }))).toBeNull();
    expect(arrowStep(key("a"))).toBeNull();
  });

  it("le pas suit le zoom : un point d'écran vaut moins de mm quand on zoome", () => {
    expect(nudged({ x: 10, y: 20 }, { x: 1, y: 0 }, 2)).toEqual({ x: 10.5, y: 20 });
    expect(nudged({ x: 10, y: 20 }, { x: 0, y: -10 }, 0.5)).toEqual({ x: 10, y: 0 });
  });

  it("une rafale d'appuis s'enregistre une fois, depuis l'état d'avant le premier appui", () => {
    vi.useFakeTimers();
    const settle = new NudgeSettle<number>(400);
    const saved: [number, number][] = [];
    settle.push(0, (before) => saved.push([before, 1]));
    vi.advanceTimersByTime(200);
    settle.push(1, (before) => saved.push([before, 2]));
    settle.push(2, (before) => saved.push([before, 3]));
    expect(saved).toEqual([]);
    expect(settle.busy).toBe(true);
    vi.advanceTimersByTime(400);
    expect(saved).toEqual([[0, 3]]);
    expect(settle.busy).toBe(false);
  });

  it("un autre geste enregistre tout de suite ce qui attend", () => {
    vi.useFakeTimers();
    const settle = new NudgeSettle<string>(400);
    const saved: string[] = [];
    settle.push("avant", (before) => saved.push(before));
    settle.flush();
    expect(saved).toEqual(["avant"]);
    vi.advanceTimersByTime(1000);
    expect(saved).toEqual(["avant"]);
  });
});

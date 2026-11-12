import { describe, expect, it } from "vitest";
import { FrameMeter, formatFrameStats, GESTURE_TAIL_MS, type FrameStats } from "./frameMeter";

/**
 * Rejoue un geste comme le fait la scène : des mouvements arrivent aux
 * instants `moves` (en ms), l'écran se rafraîchit toutes les `periodMs`, et
 * pendant le geste chaque rafraîchissement donne une image. `skipped` liste
 * les rafraîchissements manqués (image en retard).
 */
function replay(moves: number[], periodMs: number, skipped: number[] = []): FrameStats[] {
  const reports: FrameStats[] = [];
  const meter = new FrameMeter((stats) => reports.push(stats));
  let activeUntil = -Infinity;
  let pending = false;
  const end = moves[moves.length - 1] + GESTURE_TAIL_MS + 5 * periodMs;
  let next = 0;
  for (let tick = 0, time = 0; time <= end; tick += 1, time = tick * periodMs) {
    while (next < moves.length && moves[next] <= time) {
      activeUntil = moves[next] + GESTURE_TAIL_MS;
      pending = true;
      next += 1;
    }
    if (!pending || skipped.includes(tick)) continue;
    const active = time < activeUntil;
    meter.frame(time, 1, active);
    pending = active;
  }
  return reports;
}

describe("mesure de fluidité d'un geste", () => {
  it("mesure le rythme de l'écran, pas celui des mouvements", () => {
    // Un mouvement toutes les 40 ms pendant une demi-seconde : c'est le cas
    // qu'avait mesuré Benjamin (39 ms en moyenne). L'écran tourne à 60 Hz.
    const moves = Array.from({ length: 13 }, (_, i) => i * 40);
    const [stats] = replay(moves, 1000 / 60);
    expect(stats.meanMs).toBeCloseTo(1000 / 60, 6);
    expect(stats.worstMs).toBeCloseTo(1000 / 60, 6);
    expect(stats.slowFrames).toBe(0);
    // 480 ms de mouvements + 180 ms de prolongation, à 60 images par seconde.
    expect(stats.frames).toBeGreaterThanOrEqual(38);
  });

  it("suit un écran à 120 Hz", () => {
    const [stats] = replay([0, 50, 100, 150, 200], 1000 / 120);
    expect(stats.meanMs).toBeCloseTo(1000 / 120, 6);
    expect(stats.slowFrames).toBe(0);
  });

  it("compte les images en retard", () => {
    const moves = Array.from({ length: 20 }, (_, i) => i * 16);
    // Deux rafraîchissements manqués de suite : une image à 50 ms.
    const [stats] = replay(moves, 1000 / 60, [10, 11]);
    expect(stats.slowFrames).toBe(1);
    expect(stats.worstMs).toBeCloseTo(50, 6);
  });

  it("ne rapporte qu'une mesure par geste, et une par geste séparé", () => {
    const reports = replay([0, 30, 60, 90, 1000, 1030, 1060], 1000 / 60);
    expect(reports).toHaveLength(2);
  });

  it("ignore un geste trop court pour être mesuré", () => {
    const reports: FrameStats[] = [];
    const meter = new FrameMeter((stats) => reports.push(stats));
    for (let i = 0; i < 4; i += 1) meter.frame(i * 16, 1, true);
    meter.frame(100, 1, false);
    expect(reports).toHaveLength(0);
  });

  it("ne compte pas l'attente avant le geste", () => {
    const reports: FrameStats[] = [];
    const meter = new FrameMeter((stats) => reports.push(stats));
    meter.frame(0, 1, false); // image hors geste (chargement d'une tuile)
    for (let i = 0; i < 10; i += 1) meter.frame(5000 + i * 16, 2, true);
    meter.frame(5200, 1, false);
    expect(reports).toEqual([{ frames: 9, meanMs: 16, worstMs: 16, slowFrames: 0, meanWorkMs: 2 }]);
  });

  it("s'affiche en français, avec le nombre d'images par seconde", () => {
    const text = formatFrameStats({ frames: 40, meanMs: 16.7, worstMs: 33.4, slowFrames: 1, meanWorkMs: 1.5 });
    expect(text).toBe("40 images, 60 par seconde (16,7 ms en moyenne, 33,4 ms au pire), 1 en retard, calcul : 1,5 ms");
  });
});

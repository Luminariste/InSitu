// Mesure de fluidité d'un geste dans la visionneuse. Logique pure, sans
// PixiJS ni navigateur, pour être vérifiée par des tests.

/** Mesure de fluidité d'un geste (suite d'images dessinées sans pause). */
export interface FrameStats {
  /** Nombre d'écarts mesurés entre deux images pendant le geste. */
  frames: number;
  /** Durée moyenne entre deux images, en ms (16,7 ms = 60 images par seconde). */
  meanMs: number;
  /** Pire durée entre deux images, en ms. */
  worstMs: number;
  /** Nombre d'images arrivées plus de 25 ms après la précédente (saccades visibles). */
  slowFrames: number;
  /** Temps de calcul moyen d'une image par l'interface, en ms. */
  meanWorkMs: number;
}

/**
 * Après le dernier mouvement, la scène continue d'être redessinée pendant ce
 * délai : la mesure porte ainsi sur le rythme réel de l'écran, pas sur le
 * rythme auquel les mouvements arrivent.
 */
export const GESTURE_TAIL_MS = 180;
/** Au-delà de cet écart entre deux images, l'image compte comme en retard. */
export const SLOW_FRAME_MS = 25;
/** En dessous de ce nombre d'écarts mesurés, le geste n'est pas rapporté. */
export const MIN_GESTURE_FRAMES = 5;

/**
 * Compte les images d'un geste. La scène l'appelle à chaque image dessinée,
 * en disant si un geste est en cours (un mouvement reçu il y a moins de
 * `GESTURE_TAIL_MS`). Pendant un geste, la scène redessine à chaque
 * rafraîchissement de l'écran : l'écart entre deux images successives mesure
 * donc la fluidité (16,7 ms à 60 Hz, 8,3 ms à 120 Hz), et non le rythme des
 * mouvements de la main.
 */
export class FrameMeter {
  private lastFrameAt = 0;
  private inGesture = false;
  private totals = { frames: 0, total: 0, worst: 0, slow: 0, work: 0 };

  constructor(private readonly onGestureEnd: (stats: FrameStats) => void) {}

  /**
   * `time` : horodatage de l'image (celui de `requestAnimationFrame`), en ms.
   * `workMs` : temps passé à la préparer. `active` : un geste est en cours.
   */
  frame(time: number, workMs: number, active: boolean): void {
    const g = this.totals;
    if (active) {
      // La première image d'un geste sert de point de départ : l'écart avec
      // l'image précédente (dessinée avant le geste) ne veut rien dire.
      if (this.inGesture) {
        const gap = time - this.lastFrameAt;
        g.frames += 1;
        g.total += gap;
        g.worst = Math.max(g.worst, gap);
        g.work += workMs;
        if (gap > SLOW_FRAME_MS) g.slow += 1;
      }
      this.inGesture = true;
      this.lastFrameAt = time;
      return;
    }
    if (!this.inGesture) return;
    this.inGesture = false;
    this.totals = { frames: 0, total: 0, worst: 0, slow: 0, work: 0 };
    if (g.frames < MIN_GESTURE_FRAMES) return;
    this.onGestureEnd({
      frames: g.frames,
      meanMs: g.total / g.frames,
      worstMs: g.worst,
      slowFrames: g.slow,
      meanWorkMs: g.work / g.frames,
    });
  }
}

const decimal = new Intl.NumberFormat("fr-FR", { minimumFractionDigits: 1, maximumFractionDigits: 1 });

/** Texte de la barre d'état : « 60 images, 60 par seconde (16,7 ms…) ». */
export function formatFrameStats(stats: FrameStats): string {
  const slow = stats.slowFrames === 0 ? "aucune saccade" : `${stats.slowFrames} en retard`;
  const perSecond = Math.round(1000 / stats.meanMs);
  return (
    `${stats.frames} images, ${perSecond} par seconde ` +
    `(${decimal.format(stats.meanMs)} ms en moyenne, ${decimal.format(stats.worstMs)} ms au pire), ` +
    `${slow}, calcul : ${decimal.format(stats.meanWorkMs)} ms`
  );
}

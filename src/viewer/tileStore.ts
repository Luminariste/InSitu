// Réserve de tuiles : chargement à la demande, annulation de ce qui n'est
// plus utile, et mémoire bornée.
//
// Ce fichier ne connaît ni WebGL ni le réseau : on lui fournit la fonction
// qui charge une tuile et celle qui la libère. Il est donc testable tel quel
// (voir tileStore.test.ts).

export interface TileRequest {
  /** Adresse de la tuile. */
  url: string;
  /** Plus le nombre est petit, plus la tuile est chargée tôt. */
  priority: number;
}

export interface TileStoreOptions<T> {
  /** Nombre de tuiles gardées en mémoire au-delà de celles qui sont affichées. */
  capacity: number;
  /** Nombre de chargements simultanés. */
  concurrency: number;
  load: (url: string, signal: AbortSignal) => Promise<T>;
  dispose: (tile: T) => void;
  /** Appelée quand une tuile vient d'arriver (ou d'échouer) : il faut redessiner. */
  onChange: () => void;
}

export interface TileStoreStatus {
  loaded: number;
  loading: number;
  failed: number;
}

interface Entry<T> {
  tile: T;
  /** Numéro de la dernière image qui s'en est servie. */
  lastUsed: number;
}

export class TileStore<T> {
  private readonly options: TileStoreOptions<T>;
  private readonly entries = new Map<string, Entry<T>>();
  private readonly inFlight = new Map<string, AbortController>();
  private readonly failed = new Set<string>();
  private queue: Array<{ key: string; request: TileRequest }> = [];
  private clock = 0;
  private destroyed = false;

  constructor(options: TileStoreOptions<T>) {
    this.options = options;
  }

  /** La tuile si elle est en mémoire. La lire la marque comme utilisée. */
  get(key: string): T | undefined {
    const entry = this.entries.get(key);
    if (!entry) return undefined;
    entry.lastUsed = this.clock;
    return entry.tile;
  }

  has(key: string): boolean {
    return this.entries.has(key);
  }

  /**
   * À appeler une fois par image, après avoir lu avec `get` toutes les tuiles
   * affichées. `wanted` liste les tuiles que l'image voudrait montrer : celles
   * qui manquent sont chargées par ordre de priorité, les chargements devenus
   * inutiles sont annulés, et les tuiles les plus anciennement utilisées sont
   * libérées au-delà de la capacité.
   */
  sync(wanted: Map<string, TileRequest>): void {
    if (this.destroyed) return;

    for (const [key, controller] of this.inFlight) {
      if (!wanted.has(key)) {
        controller.abort();
        this.inFlight.delete(key);
      }
    }

    this.queue = [];
    for (const [key, request] of wanted) {
      const entry = this.entries.get(key);
      if (entry) entry.lastUsed = this.clock;
      if (entry || this.inFlight.has(key) || this.failed.has(key)) continue;
      this.queue.push({ key, request });
    }
    this.queue.sort((a, b) => a.request.priority - b.request.priority);
    this.startLoads();

    this.evict(wanted);
    this.clock += 1;
  }

  status(): TileStoreStatus {
    return {
      loaded: this.entries.size,
      loading: this.inFlight.size + this.queue.length,
      failed: this.failed.size,
    };
  }

  /** Oublie les échecs : les tuiles concernées seront redemandées. */
  retryFailed(): void {
    this.failed.clear();
  }

  destroy(): void {
    this.destroyed = true;
    for (const controller of this.inFlight.values()) controller.abort();
    this.inFlight.clear();
    this.queue = [];
    for (const entry of this.entries.values()) this.options.dispose(entry.tile);
    this.entries.clear();
  }

  private startLoads(): void {
    while (this.inFlight.size < this.options.concurrency && this.queue.length > 0) {
      const { key, request } = this.queue.shift()!;
      const controller = new AbortController();
      this.inFlight.set(key, controller);
      this.options.load(request.url, controller.signal).then(
        (tile) => {
          // Annulée entre-temps, ou réserve détruite : la tuile ne sert plus.
          if (this.inFlight.get(key) !== controller) {
            this.options.dispose(tile);
            return;
          }
          this.inFlight.delete(key);
          this.entries.set(key, { tile, lastUsed: this.clock });
          this.startLoads();
          this.options.onChange();
        },
        () => {
          if (this.inFlight.get(key) !== controller) return;
          this.inFlight.delete(key);
          this.failed.add(key);
          this.startLoads();
          this.options.onChange();
        },
      );
    }
  }

  private evict(wanted: Map<string, TileRequest>): void {
    // Sont intouchables : les tuiles lues pendant cette image (elles sont à
    // l'écran) et celles que l'image demande.
    const removable: Array<[string, Entry<T>]> = [];
    for (const item of this.entries) {
      if (item[1].lastUsed !== this.clock && !wanted.has(item[0])) removable.push(item);
    }
    let excess = removable.length - this.options.capacity;
    if (excess <= 0) return;
    removable.sort((a, b) => a[1].lastUsed - b[1].lastUsed);
    for (const [key, entry] of removable) {
      if (excess <= 0) break;
      this.entries.delete(key);
      this.options.dispose(entry.tile);
      excess -= 1;
    }
  }
}

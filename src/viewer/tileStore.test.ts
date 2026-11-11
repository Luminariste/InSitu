import { describe, expect, it } from "vitest";
import { TileStore, type TileRequest } from "./tileStore";

/** Chargeur factice : chaque chargement attend qu'on le termine à la main. */
function harness(capacity: number, concurrency: number) {
  const pending = new Map<string, { resolve: (tile: string) => void; reject: (error: Error) => void; signal: AbortSignal }>();
  const started: string[] = [];
  const disposed: string[] = [];
  let changes = 0;
  const store = new TileStore<string>({
    capacity,
    concurrency,
    load: (url, signal) =>
      new Promise<string>((resolve, reject) => {
        started.push(url);
        pending.set(url, { resolve, reject, signal });
      }),
    dispose: (tile) => disposed.push(tile),
    onChange: () => {
      changes += 1;
    },
  });
  const finish = async (url: string) => {
    pending.get(url)!.resolve(`tuile:${url}`);
    await Promise.resolve();
  };
  const fail = async (url: string) => {
    pending.get(url)!.reject(new Error("illisible"));
    await Promise.resolve();
  };
  return { store, pending, started, disposed, finish, fail, changes: () => changes };
}

function want(...urls: string[]): Map<string, TileRequest> {
  return new Map(urls.map((url, index) => [url, { url, priority: index }]));
}

describe("réserve de tuiles", () => {
  it("charge par ordre de priorité, sans dépasser le nombre de chargements simultanés", async () => {
    const h = harness(10, 2);
    h.store.sync(
      new Map([
        ["loin", { url: "loin", priority: 5 }],
        ["centre", { url: "centre", priority: 0 }],
        ["proche", { url: "proche", priority: 1 }],
      ]),
    );
    expect(h.started).toEqual(["centre", "proche"]);
    expect(h.store.status()).toEqual({ loaded: 0, loading: 3, failed: 0 });

    await h.finish("centre");
    expect(h.started).toEqual(["centre", "proche", "loin"]);
    expect(h.store.get("centre")).toBe("tuile:centre");
    expect(h.changes()).toBe(1);
  });

  it("ne redemande pas une tuile déjà là ou déjà en route", async () => {
    const h = harness(10, 4);
    h.store.sync(want("a", "b"));
    h.store.sync(want("a", "b"));
    await h.finish("a");
    h.store.sync(want("a", "b"));
    expect(h.started).toEqual(["a", "b"]);
  });

  it("annule les chargements devenus inutiles et libère ce qui arrive trop tard", async () => {
    const h = harness(10, 4);
    h.store.sync(want("a", "b"));
    h.store.sync(want("b"));
    expect(h.pending.get("a")!.signal.aborted).toBe(true);
    expect(h.pending.get("b")!.signal.aborted).toBe(false);
    // Le chargement annulé aboutit quand même : la tuile est libérée aussitôt.
    await h.finish("a");
    expect(h.store.has("a")).toBe(false);
    expect(h.disposed).toEqual(["tuile:a"]);
  });

  it("garde les tuiles affichées et libère les plus anciennes au-delà de la capacité", async () => {
    const h = harness(2, 8);
    h.store.sync(want("a", "b", "c", "d", "e"));
    for (const url of ["a", "b", "c", "d", "e"]) await h.finish(url);
    h.store.sync(want("a", "b", "c", "d", "e"));

    // Image suivante : seule « e » est demandée, « a » est lue (affichée en
    // attendant). Les trois autres ne servent plus : on en garde deux.
    h.store.get("a");
    h.store.sync(want("e"));
    expect(h.store.has("a")).toBe(true);
    expect(h.store.has("e")).toBe(true);
    expect(h.disposed).toHaveLength(1);
    expect(h.store.status().loaded).toBe(4);

    // Plus rien n'est demandé ni lu : la capacité s'applique à tout.
    h.store.sync(want());
    expect(h.store.status().loaded).toBe(2);
    // « a » et « e » ont servi en dernier : ce sont elles qui restent.
    expect(h.store.has("a")).toBe(true);
    expect(h.store.has("e")).toBe(true);
  });

  it("retient les échecs sans insister, jusqu'à ce qu'on le lui demande", async () => {
    const h = harness(10, 4);
    h.store.sync(want("a"));
    await h.fail("a");
    await Promise.resolve();
    expect(h.store.status()).toEqual({ loaded: 0, loading: 0, failed: 1 });
    h.store.sync(want("a"));
    expect(h.started).toEqual(["a"]);
    h.store.retryFailed();
    h.store.sync(want("a"));
    expect(h.started).toEqual(["a", "a"]);
  });

  it("libère tout à la destruction", async () => {
    const h = harness(10, 4);
    h.store.sync(want("a", "b"));
    await h.finish("a");
    h.store.destroy();
    expect(h.disposed).toEqual(["tuile:a"]);
    expect(h.pending.get("b")!.signal.aborted).toBe(true);
    await h.finish("b");
    expect(h.disposed).toEqual(["tuile:a", "tuile:b"]);
  });
});

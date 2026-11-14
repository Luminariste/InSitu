import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { asStrings, keep, oneOf, projectKey, recall } from "./remember";

describe("mémoire de l'interface", () => {
  const store = new Map<string, string>();
  let previous: unknown;
  beforeEach(() => {
    store.clear();
    previous = (globalThis as { localStorage?: unknown }).localStorage;
    (globalThis as { localStorage?: unknown }).localStorage = {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
      removeItem: (k: string) => void store.delete(k),
    };
  });
  afterEach(() => {
    (globalThis as { localStorage?: unknown }).localStorage = previous;
  });

  it("relit ce qui a été retenu, et la valeur d'ouverture sinon", () => {
    expect(recall("x", asStrings, [])).toEqual([]);
    keep("x", ["K", "A"]);
    expect(recall("x", asStrings, [])).toEqual(["K", "A"]);
    keep("x", null);
    expect(recall("x", asStrings, ["z"])).toEqual(["z"]);
  });

  it("ignore une valeur abîmée ou inattendue", () => {
    store.set("ecran", "{abîmé");
    expect(recall("ecran", oneOf(["facade", "diapositive"] as const), "facade")).toBe("facade");
    keep("ecran", "inconnu");
    expect(recall("ecran", oneOf(["facade", "diapositive"] as const), "facade")).toBe("facade");
    keep("ecran", "diapositive");
    expect(recall("ecran", oneOf(["facade", "diapositive"] as const), "facade")).toBe("diapositive");
  });

  it("sépare les réglages de chaque projet", () => {
    keep(projectKey("p", "/epinal"), "K");
    keep(projectKey("p", "/trevarez"), "B");
    expect(recall(projectKey("p", "/epinal"), (r) => (typeof r === "string" ? r : undefined), "")).toBe("K");
    expect(recall(projectKey("p", "/trevarez"), (r) => (typeof r === "string" ? r : undefined), "")).toBe("B");
    expect(projectKey("p", null)).toBe("p");
  });
});

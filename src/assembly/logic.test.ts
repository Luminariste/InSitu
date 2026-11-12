import { describe, expect, it } from "vitest";
import type { AssemblyPhoto, AssemblyPhotoReport, AutoReport } from "../engine";
import { NO_PENDING, click, formatPx, hint, movePoint, pairStatus, photoLine, possibleAnchors, removePair, summary, autoMessage, hasManualPoints } from "./logic";

const photo = (id: string, anchor: string | null): AssemblyPhoto => ({ id, name: id, source: `facade/assemblage/${id}.jpg`, width: 100, height: 80, anchor });

const report = (over: Partial<AssemblyPhotoReport>): AssemblyPhotoReport => ({
  id: "b",
  name: "b",
  width: 100,
  height: 80,
  anchor: "a",
  pairs: [],
  placed: false,
  corners: null,
  meanResidualPx: null,
  problem: null,
  ...over,
});

describe("pose des points en deux clics", () => {
  it("ajoute la paire quand les deux côtés sont posés, dans n'importe quel ordre", () => {
    const first = click([], NO_PENDING, null, "there", { x: 5, y: 6 });
    expect(first.changed).toBe(false);
    expect(first.pending.there).toEqual({ x: 5, y: 6 });
    const second = click(first.pairs, first.pending, null, "here", { x: 1, y: 2 });
    expect(second.changed).toBe(true);
    expect(second.pairs).toEqual([{ here: { x: 1, y: 2 }, there: { x: 5, y: 6 } }]);
    expect(second.pending).toEqual(NO_PENDING);
  });

  it("déplace le point choisi au lieu d'en ajouter un", () => {
    const pairs = [{ here: { x: 1, y: 1 }, there: { x: 2, y: 2 } }];
    const result = click(pairs, NO_PENDING, 0, "here", { x: 9, y: 9 });
    expect(result.pairs).toEqual([{ here: { x: 9, y: 9 }, there: { x: 2, y: 2 } }]);
  });

  it("garde un point glissé dans sa photo et retire une paire", () => {
    const pairs = [
      { here: { x: 1, y: 1 }, there: { x: 2, y: 2 } },
      { here: { x: 3, y: 3 }, there: { x: 4, y: 4 } },
    ];
    expect(movePoint(pairs, 1, "there", { x: -5, y: 500 }, { width: 100, height: 80 })[1].there).toEqual({ x: 0, y: 80 });
    expect(removePair(pairs, 0)).toEqual([pairs[1]]);
  });
});

describe("états et textes", () => {
  it("colore un point selon son écart", () => {
    expect(pairStatus(undefined)).toBe("inconnu");
    expect(pairStatus({ residualPx: null, outlier: false })).toBe("inconnu");
    expect(pairStatus({ residualPx: 1.2, outlier: false })).toBe("bon");
    expect(pairStatus({ residualPx: 8, outlier: false })).toBe("moyen");
    expect(pairStatus({ residualPx: 1, outlier: true })).toBe("aberrant");
    expect(formatPx(1.25)).toBe("1,3 px");
  });

  it("décrit chaque photo de la liste", () => {
    const names = new Map([["a", "gauche"]]);
    expect(photoLine(0, undefined, names).detail).toBe("référence");
    expect(photoLine(1, report({ pairs: [{ residualPx: null, outlier: false }] }), names).detail).toBe("sur gauche · encore 3 points");
    const placed = photoLine(
      1,
      report({ placed: true, meanResidualPx: 0.8, pairs: Array.from({ length: 6 }, (_, i) => ({ residualPx: 1, outlier: i === 2 })) }),
      names,
    );
    expect(placed).toEqual({ tone: "danger", detail: "sur gauche · 6 points · 0,8 px · 1 douteux" });
  });

  it("n'offre pas d'appui qui ferait une boucle", () => {
    const photos = [photo("a", null), photo("b", "a"), photo("c", "b"), photo("d", "a")];
    expect(possibleAnchors(photos, "b").map((p) => p.id)).toEqual(["a", "d"]);
    expect(possibleAnchors(photos, "d").map((p) => p.id)).toEqual(["a", "b", "c"]);
  });

  it("résume l'assemblage et guide la pose", () => {
    expect(summary(null, 0)).toMatch(/première sert de référence/);
    const ready = { ready: true, photos: [report({ id: "a", placed: true }), report({ placed: true })] };
    expect(summary(ready, 2)).toMatch(/Toutes les photos sont placées/);
    expect(summary({ ready: false, photos: [report({ id: "a", placed: true }), report({})] }, 2)).toMatch(/^1 photo à placer/);
    expect(hint({ here: { x: 0, y: 0 }, there: null }, null)).toMatch(/photo d'appui/);
    expect(hint(NO_PENDING, null)).toBeNull();
  });
});

describe("raccord automatique (tâche 6.6)", () => {
  const auto = (over: Partial<AutoReport>): AutoReport => ({
    photos: [
      { id: "a", name: "a", anchor: null, matches: 0, pairs: 0, linked: true },
      { id: "b", name: "b", anchor: "a", matches: 120, pairs: 24, linked: true },
      { id: "c", name: "c", anchor: "b", matches: 90, pairs: 24, linked: true },
    ],
    unlinked: [],
    seconds: 3.4,
    assembly: { photos: [report({ id: "a" }), report({ id: "b" }), report({ id: "c" })], ready: true },
    ...over,
  });

  it("annonce les photos raccordées", () => {
    expect(autoMessage(auto({}))).toEqual({
      tone: "success",
      text: "2 photos raccordées automatiquement (3 s). Vérifier les points, puis « Calculer l'assemblage ».",
    });
  });

  it("nomme les photos sans détail commun", () => {
    const r = auto({ unlinked: ["c"] });
    r.photos[2] = { ...r.photos[2], linked: false, anchor: null, pairs: 0 };
    expect(autoMessage(r)).toEqual({
      tone: "warning",
      text: "1 photo raccordée automatiquement (3 s). Aucun détail commun trouvé pour « c » : poser ses points à la main, ou la retirer.",
    });
  });

  it("signale les points douteux", () => {
    const r = auto({});
    r.assembly.photos[1] = report({ pairs: [{ residualPx: 9, outlier: true }] });
    expect(autoMessage(r).text).toContain("1 point douteux");
  });

  it("ne demande confirmation que si des points ont déjà été posés", () => {
    expect(hasManualPoints([photo("a", null), photo("b", "a")])).toBe(false);
    expect(hasManualPoints([photo("a", null), { ...photo("b", "a"), pairs: [{ here: { x: 1, y: 1 }, there: { x: 2, y: 2 } }] }])).toBe(true);
  });
});

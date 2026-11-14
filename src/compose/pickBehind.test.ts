// Choisir un dessin parmi des dessins superposés (demande de Benjamin du
// 14 novembre) : un clic ne prend que ce qu'on voit d'un dessin (sa
// silhouette), et ⌘-clic répété descend d'un dessin à chaque clic.
import { describe, expect, it } from "vitest";
import { hitsAt, hitTest, nextBehind, touches, unitKey, type Placement, type Silhouette } from "./placement";
import { drawingsAt, slidePoint } from "../slide/adjust";
import type { SlideDrawing } from "../engine";
import { commandDown, formatShortcut } from "../ui/shortcuts";

function placed(id: string, x: number, y: number, extra: Partial<Placement> = {}): Placement {
  return { id, drawing: id, center: { x, y }, width: 200, height: 200, rotation: 0, ...extra };
}

/** Un losange au milieu du dessin : les coins du cadre sont vides. */
const diamond: Silhouette = [
  [
    [0.5, 0],
    [1, 0.5],
    [0.5, 1],
    [0, 0.5],
  ],
];

describe("clic sur ce qu'on voit du dessin", () => {
  it("le vide du cadre laisse passer le clic vers le dessin du dessous", () => {
    const back = placed("dessous", 0, 0);
    const front = placed("dessus", 0, 0);
    const silhouettes = new Map([["dessus", diamond]]);
    const of = (d: string) => silhouettes.get(d) ?? null;
    // Au centre : le trait du dessin de devant.
    expect(hitTest([back, front], { x: 0, y: 0 }, of)?.id).toBe("dessus");
    // Dans le coin du cadre, hors du losange : celui du dessous.
    expect(hitTest([back, front], { x: -90, y: -90 }, of)?.id).toBe("dessous");
    // Sans silhouette connue : le cadre entier, comme avant.
    expect(hitTest([back, front], { x: -90, y: -90 })?.id).toBe("dessus");
  });

  it("la silhouette suit la rotation, le miroir et la déformation du dessin", () => {
    // Un triangle dans la moitié gauche du dessin.
    const left: Silhouette = [
      [
        [0, 0],
        [0.5, 0.5],
        [0, 1],
      ],
    ];
    const p = placed("a", 0, 0);
    expect(touches(p, { x: -80, y: 0 }, left)).toBe(true);
    expect(touches(p, { x: 80, y: 0 }, left)).toBe(false);
    const mirrored = { ...p, mirrored: true };
    expect(touches(mirrored, { x: 80, y: 0 }, left)).toBe(true);
    expect(touches(mirrored, { x: -80, y: 0 }, left)).toBe(false);
    const turned = { ...p, rotation: 90 };
    expect(touches(turned, { x: 0, y: -80 }, left)).toBe(true);
    expect(touches(turned, { x: 0, y: 80 }, left)).toBe(false);
    // Une silhouette vide (rien de détecté) : le cadre.
    expect(touches(p, { x: 80, y: 0 }, [])).toBe(true);
    // Hors du cadre : jamais.
    expect(touches(p, { x: 150, y: 0 }, left)).toBe(false);
  });
});

describe("⌘-clic répété : le dessin suivant en dessous", () => {
  const a = placed("a", 0, 0);
  const b = placed("b", 10, 0);
  const c = placed("c", 20, 0);
  const all = [a, b, c]; // a au fond, c devant
  const at = { x: 10, y: 0 };

  it("descend d'un dessin à chaque clic puis revient devant", () => {
    const hits = hitsAt(all, at);
    expect(hits.map((p) => p.id)).toEqual(["c", "b", "a"]);
    let chosen = new Set<string>();
    const order: string[] = [];
    for (let k = 0; k < 4; k++) {
      const next = nextBehind(hits, unitKey, (p) => chosen.has(p.id))!;
      order.push(next.id);
      chosen = new Set([next.id]);
    }
    expect(order).toEqual(["c", "b", "a", "c"]);
  });

  it("un dessin choisi ailleurs ne compte pas : on part de devant", () => {
    const far = placed("loin", 1000, 0);
    const next = nextBehind(hitsAt([...all, far], at), unitKey, (p) => p.id === "loin");
    expect(next?.id).toBe("c");
  });

  it("un groupe compte pour un", () => {
    const grouped = [a, { ...b, group: "g" }, { ...c, group: "g" }];
    const hits = hitsAt(grouped, at);
    const fromGroup = nextBehind(hits, unitKey, (p) => p.id === "c");
    expect(fromGroup?.id).toBe("a");
    const fromA = nextBehind(hits, unitKey, (p) => p.id === "a");
    expect(fromA?.id).toBe("c");
  });

  it("rien sous le point : rien", () => {
    expect(nextBehind(hitsAt(all, { x: 500, y: 500 }), unitKey, () => false)).toBeNull();
  });

  it("⌘ sur Mac, Ctrl ailleurs ; affiché « ⌘-clic »", () => {
    expect(commandDown({ metaKey: true, ctrlKey: false }, true)).toBe(true);
    expect(commandDown({ metaKey: false, ctrlKey: true }, true)).toBe(false);
    expect(commandDown({ metaKey: false, ctrlKey: true }, false)).toBe(true);
    expect(formatShortcut("mod+clic")).toBe("⌘-clic");
    expect(formatShortcut("mod+clic", false)).toBe("Ctrl-clic");
  });
});

describe("même chose dans la vue Diapositive", () => {
  function drawing(placement: string, x: number, y: number, w: number, h: number): SlideDrawing {
    const n = 4;
    const points = [];
    for (let j = 0; j <= n; j++) for (let i = 0; i <= n; i++) points.push({ x: x + (w * i) / n, y: y + (h * j) / n });
    return { placement, drawing: placement, divisions: n, points };
  }
  const none = () => null;

  it("la silhouette se place dans la maille du dessin", () => {
    const d = drawing("a", 10, 20, 100, 50);
    const p = slidePoint(d, null, 0.3, 0.7);
    expect(p.x).toBeCloseTo(40, 9);
    expect(p.y).toBeCloseTo(55, 9);
  });

  it("le vide d'un dessin laisse passer le clic, ⌘-clic descend", () => {
    const back = drawing("dessous", 0, 0, 100, 100);
    const front = drawing("dessus", 0, 0, 100, 100);
    const of = (id: string) => (id === "dessus" ? diamond : null);
    expect(drawingsAt([back, front], none, { x: 50, y: 50 }, of).map((d) => d.placement)).toEqual([
      "dessus",
      "dessous",
    ]);
    expect(drawingsAt([back, front], none, { x: 5, y: 5 }, of).map((d) => d.placement)).toEqual(["dessous"]);
    const hits = drawingsAt([back, front], none, { x: 50, y: 50 }, of);
    const key = (d: SlideDrawing) => d.placement;
    expect(nextBehind(hits, key, (d) => d.placement === "dessus")?.placement).toBe("dessous");
    expect(nextBehind(hits, key, (d) => d.placement === "dessous")?.placement).toBe("dessus");
  });
});

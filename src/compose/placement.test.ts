import { describe, expect, it } from "vitest";
import {
  align,
  arrange,
  bounds,
  constrainMove,
  contains,
  copyOf,
  corners,
  distribute,
  dropped,
  duplicate,
  expandToGroups,
  flip,
  group,
  hitTest,
  inMarquee,
  normalizeAngle,
  rotate,
  scale,
  translate,
  ungroup,
  type Placement,
} from "./placement";

function placed(id: string, x: number, y: number, extra: Partial<Placement> = {}): Placement {
  return { id, drawing: "merle", center: { x, y }, width: 200, height: 100, rotation: 0, ...extra };
}

const ids = (...list: string[]) => new Set(list);
let counter = 0;
const makeId = () => `n${++counter}`;

function near(a: { x: number; y: number }, x: number, y: number) {
  expect(a.x).toBeCloseTo(x, 9);
  expect(a.y).toBeCloseTo(y, 9);
}

describe("dessins posés", () => {
  it("place les coins comme le moteur (taille, rotation horaire, miroir)", () => {
    const p = placed("a", 1000, 500);
    near(corners(p)[0], 900, 450);
    near(corners({ ...p, rotation: 90 })[0], 1050, 400);
    near(corners({ ...p, mirrored: true })[0], 1100, 450);
    expect(contains({ ...p, rotation: 90 }, { x: 1000, y: 590 })).toBe(true);
    expect(contains({ ...p, rotation: 90 }, { x: 1090, y: 500 })).toBe(false);
  });

  it("choisit le dessin le plus en avant, et tout son groupe", () => {
    const list = [placed("a", 0, 0, { group: "g" }), placed("b", 50, 0), placed("c", 1000, 0, { group: "g" })];
    expect(hitTest(list, { x: 40, y: 0 })?.id).toBe("b");
    expect(hitTest(list, { x: 500, y: 500 })).toBeNull();
    expect([...expandToGroups(list, ["a"])].sort()).toEqual(["a", "c"]);
    expect([...inMarquee(list, { x: 900, y: -10, width: 20, height: 20 })].sort()).toEqual(["a", "c"]);
  });

  it("déplace, tourne et agrandit la sélection seulement", () => {
    const list = [placed("a", 0, 0), placed("b", 100, 0)];
    const moved = translate(list, ids("a"), 10, -5);
    near(moved[0].center, 10, -5);
    expect(moved[1]).toBe(list[1]);

    const turned = rotate(list, ids("a", "b"), 90, { x: 0, y: 0 });
    near(turned[1].center, 0, 100);
    expect(turned[1].rotation).toBe(90);
    expect(normalizeAngle(270)).toBe(-90);

    const bigger = scale(list, ids("a", "b"), 2, { x: 0, y: 0 });
    near(bigger[1].center, 200, 0);
    expect([bigger[1].width, bigger[1].height]).toEqual([400, 200]);
    // Jamais en dessous d'un millimètre.
    expect(scale(list, ids("a"), 0, { x: 0, y: 0 })[0].height).toBeCloseTo(1, 9);
  });

  it("une symétrie retourne le dessin et le fait passer de l'autre côté du cadre", () => {
    const list = [placed("a", 0, 0, { rotation: 30 }), placed("b", 1000, 0)];
    const h = flip(list, ids("a", "b"), "horizontal");
    // Les deux dessins échangent leur place autour du même axe.
    expect(h[0].center.x + list[0].center.x).toBeCloseTo(h[1].center.x + list[1].center.x);
    expect(h[0].mirrored).toBe(true);
    expect(h[0].rotation).toBe(-30);
    // Le coin haut-gauche du fichier est désormais à droite.
    expect(corners(h[1])[0].x).toBeGreaterThan(h[1].center.x);

    const v = flip([placed("a", 0, 0)], ids("a"), "vertical");
    expect(v[0].mirrored).toBe(true);
    expect(v[0].rotation).toBe(180);
    // Haut et bas échangés, gauche et droite conservées.
    near(corners(v[0])[0], -100, 50);
    // Deux symétries ramènent au point de départ.
    const back = flip(v, ids("a"), "vertical");
    near(corners(back[0])[0], -100, -50);
  });

  it("une copie reçoit de nouveaux identifiants, un nouveau groupe, et passe devant", () => {
    const list = [placed("a", 0, 0, { group: "g" }), placed("b", 10, 0, { group: "g" }), placed("c", 99, 0)];
    const { placements, copies } = duplicate(list, ids("a", "b"), { x: 100, y: 0 }, makeId);
    expect(placements).toHaveLength(5);
    const [x, y] = placements.slice(3);
    expect(copies).toEqual(ids(x.id, y.id));
    expect(x.group).toBe(y.group);
    expect(x.group).not.toBe("g");
    near(x.center, 100, 0);
    expect(copyOf([placed("d", 0, 0)], { x: 0, y: 0 }, makeId)[0].group).toBeNull();
  });

  it("groupe, dégroupe et range l'ordre d'empilement", () => {
    const list = [placed("a", 0, 0), placed("b", 0, 0), placed("c", 0, 0), placed("d", 0, 0)];
    const grouped = group(list, ids("a", "c"), () => "g1");
    expect(grouped.map((p) => p.group ?? null)).toEqual(["g1", null, "g1", null]);
    expect(ungroup(grouped, ids("a", "c")).every((p) => !p.group)).toBe(true);
    expect(group(list, ids("a"))).toBe(list);

    const order = (l: Placement[]) => l.map((p) => p.id).join("");
    expect(order(arrange(list, ids("b"), "front"))).toBe("acdb");
    expect(order(arrange(list, ids("c"), "back"))).toBe("cabd");
    expect(order(arrange(list, ids("a", "b"), "forward"))).toBe("cabd");
    expect(order(arrange(list, ids("c", "d"), "backward"))).toBe("acdb");
  });

  it("aligne et répartit comme le panneau Alignement, un groupe d'un bloc", () => {
    const list = [
      placed("a", 0, 0),
      placed("b", 500, 300),
      placed("c", 2000, 120, { group: "g" }),
      placed("d", 2300, 120, { group: "g" }),
    ];
    const left = align(list, ids("a", "b", "c", "d"), "left");
    expect(bounds([left[1]])!.x).toBeCloseTo(-100);
    // Le groupe garde sa forme : 300 mm entre ses deux dessins.
    expect(left[3].center.x - left[2].center.x).toBeCloseTo(300);
    expect(bounds([left[2]])!.x).toBeCloseTo(-100);

    const top = align(list, ids("a", "b"), "bottom");
    expect(top[0].center.y).toBeCloseTo(300);

    const spread = distribute(
      [placed("a", 0, 0), placed("b", 300, 0), placed("c", 1000, 0)],
      ids("a", "b", "c"),
      "horizontal",
    );
    // Écarts égaux : (1100 - (-100) - 600) / 2 = 300 mm.
    expect(spread[1].center.x).toBeCloseTo(500);
    expect(distribute(list, ids("a", "b"), "vertical")).toBe(list);
  });

  it("contraint un déplacement avec Maj, et pose un dessin lâché aux proportions du fichier", () => {
    near(constrainMove(10, 2), 10, 0);
    near(constrainMove(1, 30), 0, 30);
    const d = constrainMove(10, 9);
    expect(d.x).toBeCloseTo(d.y);
    const p = dropped({ id: "merle", width: 2000, height: 1000 }, { x: 5, y: 6 }, 8000, () => "x");
    expect(p).toMatchObject({ id: "x", drawing: "merle", width: 2000, height: 1000, center: { x: 5, y: 6 } });
  });
});

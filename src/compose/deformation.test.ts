import { describe, expect, it } from "vitest";
import cases from "./deformation-cases.json";
import { identity, moveNode, nodePoints, sameDeformation, type Deformation } from "./deformation";
import { applyGesture, nodeAt } from "./gesture";
import {
  contains,
  deform,
  deformationFrameOf,
  facadePoint,
  flip,
  outline,
  rotate,
  scale,
  sharedDeformation,
  translate,
  type Placement,
} from "./placement";

const close = (a: { x: number; y: number }, b: { x: number; y: number }, eps = 1e-9) =>
  expect(Math.hypot(a.x - b.x, a.y - b.y)).toBeLessThan(eps);

function placed(extra: Partial<Placement> = {}): Placement {
  return { id: "p", drawing: "merle", center: { x: 1000, y: 500 }, width: 400, height: 200, rotation: 0, ...extra };
}

function bent(p: Placement): Placement {
  let d: Deformation = identity({ center: p.center, width: p.width, height: p.height, rotation: p.rotation }, 3, 2);
  d = moveNode(d, 5, { x: 1010, y: 470 });
  d = moveNode(d, 0, { x: 790, y: 410 });
  return { ...p, deformation: d };
}

describe("déformation libre", () => {
  it("l'aperçu tombe exactement où le moteur exporte (cas calculés par le moteur)", () => {
    for (const c of cases.cases) {
      const p = c.placement as unknown as Placement;
      for (const s of c.samples) close(facadePoint(p, s.u, s.v), s);
    }
  });

  it("une grille au repos ne change rien", () => {
    const p = placed({ rotation: 30, mirrored: true });
    const d = identity({ center: p.center, width: p.width, height: p.height, rotation: 30 }, 4, 3);
    for (const [u, v] of [
      [0, 0],
      [0.3, 0.7],
      [1, 1],
    ])
      close(facadePoint({ ...p, deformation: d }, u, v), facadePoint(p, u, v));
  });

  it("le nœud déplacé va où on le pose", () => {
    const p = bent(placed());
    close(nodePoints(p.deformation!)[5], { x: 1010, y: 470 });
    // Le coin haut-gauche du dessin suit le nœud 0.
    close(facadePoint(p, 0, 0), { x: 790, y: 410 });
  });

  it("déplacer, tourner, agrandir ou retourner un dessin déformé emporte sa déformation", () => {
    const p = bent(placed({ rotation: 20 }));
    const ids = new Set(["p"]);
    const samples = [
      [0, 0],
      [0.4, 0.3],
      [1, 0.8],
    ];
    const moved = translate([p], ids, 50, -20)[0];
    for (const [u, v] of samples) {
      const a = facadePoint(p, u, v);
      close(facadePoint(moved, u, v), { x: a.x + 50, y: a.y - 20 }, 1e-6);
    }
    const pivot = { x: 900, y: 600 };
    const turned = rotate([p], ids, 35, pivot)[0];
    const r = (35 * Math.PI) / 180;
    for (const [u, v] of samples) {
      const a = facadePoint(p, u, v);
      const dx = a.x - pivot.x;
      const dy = a.y - pivot.y;
      close(
        facadePoint(turned, u, v),
        { x: pivot.x + dx * Math.cos(r) - dy * Math.sin(r), y: pivot.y + dx * Math.sin(r) + dy * Math.cos(r) },
        1e-6,
      );
    }
    const grown = scale([p], ids, 1.5, pivot)[0];
    for (const [u, v] of samples) {
      const a = facadePoint(p, u, v);
      close(facadePoint(grown, u, v), { x: pivot.x + (a.x - pivot.x) * 1.5, y: pivot.y + (a.y - pivot.y) * 1.5 }, 1e-6);
    }
    for (const axis of ["horizontal", "vertical"] as const) {
      const flipped = flip([p], ids, axis)[0];
      // Le contour retourné est le miroir du contour d'origine.
      const before = outline(p);
      const after = outline(flipped);
      const xs = (pts: { x: number }[]) => pts.map((q) => q.x);
      const ys = (pts: { y: number }[]) => pts.map((q) => q.y);
      if (axis === "horizontal") {
        expect(Math.min(...xs(after))).toBeCloseTo(Math.min(...xs(before)), 6);
        expect(Math.max(...ys(after))).toBeCloseTo(Math.max(...ys(before)), 6);
      } else {
        expect(Math.min(...ys(after))).toBeCloseTo(Math.min(...ys(before)), 6);
        expect(Math.max(...xs(after))).toBeCloseTo(Math.max(...xs(before)), 6);
      }
      // Chaque point du dessin va au miroir de sa place (le retournement
      // bascule « miroir » : un même point du dessin passe de l'autre côté).
      for (const [u, v] of samples) {
        const a = facadePoint(p, u, v);
        const b = facadePoint(flipped, u, v);
        const box = { x: Math.min(...xs(before)), y: Math.min(...ys(before)) };
        const width = Math.max(...xs(before)) - box.x;
        const height = Math.max(...ys(before)) - box.y;
        if (axis === "horizontal") close(b, { x: 2 * (box.x + width / 2) - a.x, y: a.y }, 1e-6);
        else close(b, { x: a.x, y: 2 * (box.y + height / 2) - a.y }, 1e-6);
      }
    }
  });

  it("on choisit un dessin déformé dans sa forme déformée", () => {
    const p = bent(placed());
    expect(contains(p, { x: 1000, y: 500 })).toBe(true);
    expect(contains(p, { x: 790 + 2, y: 410 + 3 })).toBe(true); // coin tiré au-dehors
    expect(contains(placed(), { x: 792, y: 413 })).toBe(false);
  });

  it("deux dessins d'un groupe déformé d'un bloc partagent la même grille", () => {
    const d = identity({ center: { x: 0, y: 0 }, width: 10, height: 10, rotation: 0 }, 2, 2);
    expect(sameDeformation(d, { ...d })).toBe(true);
    expect(sameDeformation(d, moveNode(d, 4, { x: 1, y: 1 }))).toBe(false);
    expect(sameDeformation(null, d)).toBe(false);
  });

  it("une grille tendue sur un groupe le déforme d'un bloc, comme le moteur", () => {
    const a = placed({ id: "a", center: { x: 0, y: 0 }, width: 100, height: 50 });
    const b = placed({ id: "b", center: { x: 300, y: 100 }, width: 100, height: 50, rotation: 90 });
    // Même cadre que `placement::deformation_frame` : le rectangle droit des deux dessins.
    expect(deformationFrameOf([a, b])).toEqual({ center: { x: 137.5, y: 62.5 }, width: 375, height: 175, rotation: 0 });
    const ids = new Set(["a", "b"]);
    const both = deform([a, b], ids, 3, 2);
    const shared = sharedDeformation(both);
    expect(shared?.columns).toBe(3);
    expect(shared?.nodes.length).toBe(12);
    // Au repos, rien ne bouge.
    close(facadePoint(both[1], 0.2, 0.9), facadePoint(b, 0.2, 0.9));

    // Tirer un nœud déforme les deux dessins ensemble, sans saut à la prise.
    const nodes = nodePoints(shared!);
    const start = { x: nodes[0].x + 2, y: nodes[0].y + 1 };
    const step = applyGesture(
      { kind: "node", start, before: both, ids, deformation: shared!, index: 0, origin: nodes[0] },
      { x: start.x - 30, y: start.y - 10 },
      { shift: false, alt: false },
    );
    const after = sharedDeformation(step.placements);
    expect(after).not.toBeNull();
    close(nodePoints(after!)[0], { x: nodes[0].x - 30, y: nodes[0].y - 10 }, 1e-9);
    // Retirer la déformation redresse les deux.
    const straight = deform(step.placements, ids, 0, 0);
    expect(straight.every((p) => !p.deformation)).toBe(true);
    expect(sharedDeformation(straight)).toBeNull();
  });

  it("un nœud se prend à quelques pixels près, le plus proche d'abord", () => {
    const nodes = [
      { x: 10, y: 10 },
      { x: 14, y: 10 },
      { x: 100, y: 100 },
    ];
    expect(nodeAt(nodes, { x: 13, y: 11 })).toBe(1);
    expect(nodeAt(nodes, { x: 50, y: 50 })).toBeNull();
  });
});

describe("déformation affine ou tirée (D-4.2-q)", () => {
  it("une grille au repos ou étirée reste affine ; un coin tiré ne l'est plus", async () => {
    const { identity, isAffine, moveNode, stretchDeformation } = await import("./deformation");
    const rest = identity({ center: { x: 0, y: 0 }, width: 200, height: 100, rotation: 30 }, 1, 1);
    expect(isAffine(rest)).toBe(true);
    // Dessin tourné de 30° puis étiré en x par la poignée d'un côté (D-4.2-l).
    expect(isAffine(stretchDeformation(rest, 1.5, 1, Math.sqrt(1.5), { x: -100, y: 0 }))).toBe(true);
    expect(isAffine(identity(rest.frame, 4, 3))).toBe(true);
    expect(isAffine(moveNode(rest, 2, { x: 300, y: 300 }))).toBe(false);
  });
});

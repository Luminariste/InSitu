import { describe, expect, it } from "vitest";
import type { FacadeOverview, RegistrationReport } from "../engine";
import {
  NO_PENDING,
  SLIDE_MM,
  click,
  facadePhotoLayer,
  formatFacadeCm,
  formatSlideMm,
  hint,
  movePoint,
  onSlide,
  pairStatus,
  removePair,
  summary,
  cornerName,
  ensureGuides,
  cornerPairs,
  demoTrapezeMesh,
  facadeGrab,
  initialSide,
  locateOnSlide,
  meshCenter,
  pairTitle,
  translateFacade,
  followPins,
  isGuide,
  pinFit,
  pinFitText,
  followConstraints,
  constraintsText,
  poseIsSound,
} from "./logic";

const p = (x: number, y: number) => ({ x, y });
type XY = { x: number; y: number };

describe("pose des points communs", () => {
  it("ajoute une paire quand les deux côtés sont posés, dans n'importe quel ordre", () => {
    let state = click([], NO_PENDING, null, "slide", p(10, 20));
    expect(state.changed).toBe(false);
    expect(state.pending.slide).toEqual(p(10, 20));
    state = click(state.pairs, state.pending, null, "facade", p(1000, 2000));
    expect(state.changed).toBe(true);
    expect(state.pairs).toEqual([{ slide: p(10, 20), facade: p(1000, 2000) }]);
    expect(state.pending).toEqual(NO_PENDING);

    const other = click(state.pairs, NO_PENDING, null, "facade", p(5, 5));
    const done = click(other.pairs, other.pending, null, "slide", p(1, 1));
    expect(done.pairs[1]).toEqual({ slide: p(1, 1), facade: p(5, 5) });
  });

  it("un second clic du même côté remplace le point en attente", () => {
    const a = click([], NO_PENDING, null, "slide", p(1, 1));
    const b = click(a.pairs, a.pending, null, "slide", p(2, 2));
    expect(b.pending.slide).toEqual(p(2, 2));
    expect(b.pairs).toHaveLength(0);
  });

  it("déplace le point du côté cliqué de la paire sélectionnée", () => {
    const pairs = [
      { slide: p(1, 1), facade: p(10, 10) },
      { slide: p(2, 2), facade: p(20, 20) },
    ];
    const moved = click(pairs, NO_PENDING, 1, "facade", p(25, 21));
    expect(moved.changed).toBe(true);
    expect(moved.pairs[1]).toEqual({ slide: p(2, 2), facade: p(25, 21) });
    expect(moved.pairs[0]).toBe(pairs[0]);
    expect(pairs[1].facade).toEqual(p(20, 20)); // pas de modification en place
  });

  it("retire une paire", () => {
    const pairs = [
      { slide: p(1, 1), facade: p(10, 10) },
      { slide: p(2, 2), facade: p(20, 20) },
    ];
    expect(removePair(pairs, 0)).toEqual([pairs[1]]);
  });

  it("refuse un point hors du carré de 280 mm", () => {
    expect(onSlide(p(0, 280))).toBe(true);
    expect(onSlide(p(-0.1, 10))).toBe(false);
    expect(onSlide(p(10, 280.5))).toBe(false);
  });
});

describe("textes", () => {
  it("écarts en mm sur la diapositive et en cm sur la façade", () => {
    expect(formatSlideMm(0.345)).toBe("0,35 mm");
    expect(formatFacadeCm(24)).toBe("2,4 cm");
    expect(formatSlideMm(null)).toBe("—");
    expect(formatFacadeCm(undefined)).toBe("—");
  });

  it("état d'un point", () => {
    expect(pairStatus(undefined)).toBe("inconnu");
    expect(pairStatus({ slideResidualMm: null, facadeResidualMm: null, outlier: false })).toBe("inconnu");
    expect(pairStatus({ slideResidualMm: 0.4, facadeResidualMm: 12, outlier: false })).toBe("bon");
    expect(pairStatus({ slideResidualMm: 1.6, facadeResidualMm: 50, outlier: false })).toBe("moyen");
    expect(pairStatus({ slideResidualMm: 6, facadeResidualMm: 200, outlier: true })).toBe("aberrant");
  });

  it("résumé selon l'avancement", () => {
    expect(summary(null, 1)).toMatch(/^Encore 3 points/);
    expect(summary(null, 3)).toMatch(/^Encore 1 point pour/);
    const base: RegistrationReport = {
      projectorId: "x",
      stage: "trapeze",
      pairs: [],
      rmsSlideMm: null,
      rmsFacadeMm: null,
      trapezeRmsSlideMm: 0,
      footprint: [],
    };
    expect(summary(base, 4)).toMatch(/^Trapèze calculé \(4 points/);
    const corrected: RegistrationReport = {
      ...base,
      stage: "corrected",
      rmsSlideMm: 0.31,
      rmsFacadeMm: 9,
      pairs: [
        { slideResidualMm: 0.3, facadeResidualMm: 9, outlier: false },
        { slideResidualMm: 7, facadeResidualMm: 210, outlier: true },
      ],
    };
    expect(summary(corrected, 8)).toBe(
      "Relief corrigé sur 8 points : écart moyen 0,31 mm sur la diapositive, 0,9 cm sur la façade ; 1 point douteux à vérifier (en rouge).",
    );
  });

  it("consigne de pose", () => {
    expect(hint(NO_PENDING, null)).toBeNull();
    expect(hint({ slide: p(1, 1), facade: null }, null)).toMatch(/sur la façade/);
    expect(hint({ slide: null, facade: p(1, 1) }, null)).toMatch(/sur la photo/);
    expect(hint(NO_PENDING, 2)).toMatch(/^Point 3/);
    // Les flèches déplacent le point du côté cliqué en dernier (D15-p).
    expect(hint(NO_PENDING, 2)).toMatch(/point de la façade/);
    expect(hint(NO_PENDING, 2, "slide")).toMatch(/point de la photo/);
  });
});

describe("glisser un point", () => {
  const pairs = [
    { slide: { x: 10, y: 10 }, facade: { x: 100, y: 100 } },
    { slide: { x: 20, y: 20 }, facade: { x: 200, y: 200 } },
  ];

  it("déplace le seul point voulu, du seul côté voulu", () => {
    const moved = movePoint(pairs, 1, "facade", { x: 250, y: 260 });
    expect(moved[1]).toEqual({ slide: { x: 20, y: 20 }, facade: { x: 250, y: 260 } });
    expect(moved[0]).toBe(pairs[0]);
    expect(pairs[1].facade).toEqual({ x: 200, y: 200 });
  });

  it("garde un point de diapositive dans le carré", () => {
    expect(movePoint(pairs, 0, "slide", { x: -5, y: 300 })[0].slide).toEqual({ x: 0, y: SLIDE_MM });
  });
});

describe("photo recalée sur la façade", () => {
  const square = [p(0, 0), p(100, 0), p(0, 100), p(100, 100)];
  const overview = {
    projectors: [
      { projectorId: "a", mesh: { divisions: 1, facade: square }, photoPreview: "apercus/a.png" },
      { projectorId: "b", mesh: null, photoPreview: null },
    ],
    overlaps: [],
  } as unknown as FacadeOverview;

  it("pose la photo du projecteur choisi sur son maillage, à l'opacité choisie", () => {
    const layer = facadePhotoLayer(overview, "a", true, 0.6, (r) => `cache://${r}`);
    expect(layer?.kind).toBe("mesh-image");
    expect(layer?.kind === "mesh-image" && layer.url).toBe("cache://apercus/a.png");
    expect(layer?.kind === "mesh-image" && layer.points).toEqual(square);
    expect(layer?.opacity).toBe(0.6);
    expect(layer?.visible).toBe(true);
    expect(facadePhotoLayer(overview, "a", false, 0.6, (r) => r)?.visible).toBe(false);
  });

  it("rien tant que le projecteur n'est pas recalé", () => {
    expect(facadePhotoLayer(overview, "b", true, 0.6, (r) => r)).toBeNull();
    expect(facadePhotoLayer(overview, null, true, 0.6, (r) => r)).toBeNull();
    expect(facadePhotoLayer(null, "a", true, 0.6, (r) => r)).toBeNull();
  });
});

describe("pose directe sur la façade (D-3.2-h)", () => {
  // Diapositive posée agrandie 10 fois, coin haut-gauche en (1000, 500), légèrement en trapèze.
  const pairs = cornerPairs(p(2400, 1900), 2800);
  const skewed = pairs.map((q, i) => (i === 2 ? { ...q, facade: p(q.facade.x + 300, q.facade.y + 200) } : q));

  it("pose les quatre coins de la diapositive en carré autour d'un centre", () => {
    expect(pairs.map((q) => q.slide)).toEqual([p(0, 0), p(280, 0), p(280, 280), p(0, 280)]);
    expect(pairs.map((q) => q.facade)).toEqual([p(1000, 500), p(3800, 500), p(3800, 3300), p(1000, 3300)]);
    expect(pairs.map((_, i) => pairTitle(pairs, i))).toEqual([
      "Coin haut-gauche (guide)",
      "Coin haut-droit (guide)",
      "Coin bas-droit (guide)",
      "Coin bas-gauche (guide)",
    ]);
    expect(cornerName({ slide: p(140, 0), facade: p(0, 0) })).toBeNull();
    expect(pairTitle([...pairs, { slide: p(10, 10), facade: p(0, 0) }], 4)).toBe("Point 5");
  });

  it("prend la taille des emprises déjà recalées, sinon 40 % de la vue", () => {
    expect(initialSide({ x: 0, y: 0, width: 10000, height: 5000 }, [])).toBe(2000);
    const square = [p(0, 0), p(3000, 0), p(3000, 3000), p(0, 3000), p(0, 0)];
    expect(initialSide({ x: 0, y: 0, width: 10000, height: 5000 }, [square])).toBeCloseTo(3000, 9);
  });

  it("déplace toute la photo sans toucher à la diapositive", () => {
    const moved = translateFacade(pairs, 100, -50);
    expect(moved[0]).toEqual({ slide: p(0, 0), facade: p(1100, 450) });
  });

  it("retrouve sur la diapositive le détail attrapé sur la façade, comme il est affiché", () => {
    const mesh = demoTrapezeMesh(skewed)!;
    expect(mesh.facade).toHaveLength(17 * 17);
    // Un nœud du maillage : retrouvé exactement.
    const k = 5 * 17 + 3;
    const back = locateOnSlide(mesh, mesh.facade[k])!;
    expect(back.x).toBeCloseTo(3 * 17.5, 9);
    expect(back.y).toBeCloseTo(5 * 17.5, 9);
    // Le carré non déformé : un point quelconque, à l'échelle 10.
    const flat = demoTrapezeMesh(pairs)!;
    const inside = locateOnSlide(flat, p(1000 + 1234, 500 + 567))!;
    expect(inside.x).toBeCloseTo(123.4, 6);
    expect(inside.y).toBeCloseTo(56.7, 6);
    expect(locateOnSlide(flat, p(900, 400))).toBeNull();
    expect(meshCenter(flat)).toEqual(p(2400, 1900));
  });

  it("choisit entre point, poignée du centre, épingle et déplacement de la vue", () => {
    const mesh = demoTrapezeMesh(pairs)!;
    const base = { pointHit: null, centerDistancePx: 100, mesh, at: p(1500, 1000), photoShown: true, spaceHeld: false };
    expect(facadeGrab({ ...base, pointHit: 2 })).toEqual({ kind: "point", index: 2 });
    expect(facadeGrab({ ...base, centerDistancePx: 5 })).toEqual({ kind: "move" });
    const pin = facadeGrab(base);
    expect(pin.kind).toBe("pin");
    if (pin.kind === "pin") {
      expect(pin.slide.x).toBeCloseTo(50, 6);
      expect(pin.slide.y).toBeCloseTo(50, 6);
    }
    expect(facadeGrab({ ...base, at: p(0, 0) })).toEqual({ kind: "none" });
    expect(facadeGrab({ ...base, photoShown: false })).toEqual({ kind: "none" });
    expect(facadeGrab({ ...base, pointHit: 2, spaceHeld: true })).toEqual({ kind: "none" });
  });
});

describe("points sûrs libres (D-3.2-j)", () => {
  // Pose de départ : carré de 2 800 mm centré sur (5 000, 5 000), soit 10 mm de façade par mm de diapositive.
  const guides = cornerPairs(p(5000, 5000), 2800);
  const onPose = (pairs: ReturnType<typeof cornerPairs>, s: { x: number; y: number }) => {
    // Maillage d'un point par mm de diapositive, par l'homographie des coins guides.
    const m = demoTrapezeMesh(pairs.filter(isGuide), 280)!;
    return m.facade[Math.round(s.y) * 281 + Math.round(s.x)];
  };

  it("1 point : la photo glisse, sans tourner ni changer de taille", () => {
    const out = followPins([...guides, { slide: p(70, 70), facade: p(4000, 4500) }]);
    // (70, 70) était en (4 300, 4 300) : tout glisse de (-300, +200).
    out.slice(0, 4).forEach((g, i) => {
      expect(g.facade.x).toBeCloseTo(guides[i].facade.x - 300, 6);
      expect(g.facade.y).toBeCloseTo(guides[i].facade.y + 200, 6);
    });
    expect(out[4]).toEqual({ slide: p(70, 70), facade: p(4000, 4500) });
  });

  it("2 points : la photo tourne et change de taille pour passer par les deux", () => {
    const pins = [
      { slide: p(100, 140), facade: p(5000, 6000) },
      { slide: p(200, 140), facade: p(5000, 8000) },
    ];
    const out = followPins([...guides, ...pins]);
    for (const pin of pins) {
      const at = onPose(out, pin.slide);
      expect(at.x).toBeCloseTo(pin.facade.x, 3);
      expect(at.y).toBeCloseTo(pin.facade.y, 3);
    }
    // Toujours un carré : côtés égaux et angle droit.
    const [a, b, , d] = out.map((q) => q.facade);
    expect(Math.hypot(b.x - a.x, b.y - a.y)).toBeCloseTo(Math.hypot(d.x - a.x, d.y - a.y), 6);
  });

  it("3 points alignés : pas de déformation, la forme reste celle du guide", () => {
    const slides = [p(40, 100), p(140, 102), p(240, 104)];
    expect(pinFit(slides)).toBe("similitude");
    const out = followPins([...guides, ...slides.map((s, k) => ({ slide: s, facade: p(4000 + 1000 * k, 4200) }))]);
    const [a, b, c, d] = out.map((q) => q.facade);
    const side = (u: { x: number; y: number }, v: { x: number; y: number }) => Math.hypot(v.x - u.x, v.y - u.y);
    expect(side(a, b)).toBeCloseTo(side(b, c), 6);
    expect(side(c, d)).toBeCloseTo(side(d, a), 6);
  });

  it("4 points bien répartis : la forme vient des seuls points sûrs", () => {
    // Points d'une vraie pose en trapèze (homographie connue).
    const truth = [
      { slide: p(0, 0), facade: p(1000, 1000) },
      { slide: p(280, 0), facade: p(4000, 1200) },
      { slide: p(280, 280), facade: p(3800, 4100) },
      { slide: p(0, 280), facade: p(900, 3900) },
    ];
    const at = (s: { x: number; y: number }) => {
      const m = demoTrapezeMesh(truth, 280)!;
      return m.facade[Math.round(s.y) * 281 + Math.round(s.x)];
    };
    const slides = [p(50, 60), p(230, 40), p(220, 240), p(60, 220), p(140, 150)];
    expect(pinFit(slides)).toBe("libre");
    const out = followPins([...guides, ...slides.map((s) => ({ slide: s, facade: at(s) }))]);
    out.slice(0, 4).forEach((g, i) => {
      expect(g.facade.x).toBeCloseTo(truth[i].facade.x, 3);
      expect(g.facade.y).toBeCloseTo(truth[i].facade.y, 3);
    });
    expect(pinFitText(out)).toContain("5 points sûrs bien répartis");
  });

  it("sans point sûr, ou sans les quatre coins, rien ne bouge", () => {
    expect(followPins(guides)).toEqual(guides);
    const loose = [{ slide: p(10, 10), facade: p(0, 0) }, { slide: p(100, 10), facade: p(900, 0) }];
    expect(followPins(loose)).toEqual(loose);
    expect(pinFit([p(10, 10)])).toBe("glisse");
    expect(pinFit([p(20, 20), p(250, 30), p(140, 250)])).toBe("affine");
  });
});

describe("lignes communes (D-3.2-k)", () => {
  const guides = cornerPairs(p(5000, 5000), 2800);
  // Vraie pose : un trapèze (homographie connue), diapositive → façade.
  const truth = [
    { slide: p(0, 0), facade: p(1000, 1000) },
    { slide: p(280, 0), facade: p(4000, 1200) },
    { slide: p(280, 280), facade: p(3800, 4100) },
    { slide: p(0, 280), facade: p(900, 3900) },
  ];
  const at = (s: { x: number; y: number }) => demoTrapezeMesh(truth, 280)!.facade[Math.round(s.y) * 281 + Math.round(s.x)];
  /** Une ligne vraie : deux points de la photo, et deux autres points de la même droite sur la façade. */
  const line = (a: { x: number; y: number }, b: { x: number; y: number }) => {
    const fa = at(a);
    const fb = at(b);
    // Sur la façade, d'autres points de la même droite (on ne trace pas les mêmes bouts).
    const f0 = { x: fa.x + 0.3 * (fb.x - fa.x), y: fa.y + 0.3 * (fb.y - fa.y) };
    const f1 = { x: fa.x + 1.4 * (fb.x - fa.x), y: fa.y + 1.4 * (fb.y - fa.y) };
    return { slide: [a, b] as [typeof a, typeof b], facade: [f0, f1] as [typeof a, typeof b] };
  };

  it("2 horizontales et 2 verticales retrouvent le trapèze", () => {
    const lines = [line(p(20, 60), p(260, 60)), line(p(20, 220), p(260, 220)), line(p(40, 20), p(40, 260)), line(p(230, 20), p(230, 260))];
    const out = followConstraints(guides, lines);
    out.forEach((g, i) => {
      expect(g.facade.x).toBeCloseTo(truth[i].facade.x, 0);
      expect(g.facade.y).toBeCloseTo(truth[i].facade.y, 0);
    });
    expect(constraintsText(out, lines)).toContain("assez pour décider");
  });

  it("une corniche seule : la photo s'y aligne, le reste vient des guides", () => {
    // Corniche de la photo à y = 140 ; sur le plan, une horizontale à y = 4 400.
    const cornice = { slide: [p(10, 140), p(270, 140)] as [ReturnType<typeof p>, ReturnType<typeof p>], facade: [p(0, 4400), p(9000, 4400)] as [ReturnType<typeof p>, ReturnType<typeof p>] };
    const out = followConstraints(guides, [cornice]);
    const m = demoTrapezeMesh(out, 280)!;
    const left = m.facade[140 * 281 + 10];
    const right = m.facade[140 * 281 + 270];
    expect(left.y).toBeCloseTo(4400, 2);
    expect(right.y).toBeCloseTo(4400, 2);
    // Pas de déformation ni de changement de taille : juste une glisse.
    expect(out[1].facade.x - out[0].facade.x).toBeCloseTo(2800, 3);
    expect(out[0].facade.x).toBeCloseTo(guides[0].facade.x, 3);
  });

  it("des lignes et un point sûr se combinent", () => {
    // 3 lignes (une corniche, deux bords) et 1 point : 8 équations. (Deux
    // lignes qui se croisent et deux points ne suffisent jamais : une
    // homologie de centre leur croisement et d'axe la droite des deux
    // points les respecte tous.)
    const lines = [line(p(20, 60), p(260, 60)), line(p(40, 20), p(40, 260)), line(p(230, 20), p(230, 260))];
    const pins = [p(200, 200)].map((s) => ({ slide: s, facade: at(s) }));
    const out = followConstraints([...guides, ...pins], lines);
    out.slice(0, 4).forEach((g, i) => {
      expect(g.facade.x).toBeCloseTo(truth[i].facade.x, 0);
      expect(g.facade.y).toBeCloseTo(truth[i].facade.y, 0);
    });
  });

  it("sans ligne, c'est le calcul des points sûrs", () => {
    const pins = [{ slide: p(70, 70), facade: p(4000, 4500) }];
    expect(followConstraints([...guides, ...pins], [])).toEqual(followPins([...guides, ...pins]));
  });
});

describe("coins guides remis avant de caler par lignes (D-3.2-k)", () => {
  // Points posés un à un, comme sur le projecteur A de Trévarez : aucun coin.
  const pins = [
    { slide: { x: 50.17, y: 199.58 }, facade: { x: 27671.8, y: 16203.8 } },
    { slide: { x: 52.79, y: 176.39 }, facade: { x: 27679.1, y: 15367.7 } },
    { slide: { x: 236.99, y: 175.08 }, facade: { x: 34180.2, y: 15861.3 } },
    { slide: { x: 15.04, y: 157.56 }, facade: { x: 26351.2, y: 14714.0 } },
    { slide: { x: 135.23, y: 182.42 }, facade: { x: 30992.7, y: 15864.5 } },
    { slide: { x: 222.5, y: 223.73 }, facade: { x: 33586.8, y: 17359.9 } },
  ];

  it("des points sans coin : 4 coins ajoutés, les points gardés", () => {
    const out = ensureGuides(pins);
    expect(out.filter(isGuide)).toHaveLength(4);
    expect(out.filter((q) => !isGuide(q))).toEqual(pins);
    // Une ligne compte ensuite : la photo bouge pour la suivre.
    const line = { slide: [{ x: 20, y: 120 }, { x: 260, y: 120 }] as [XY, XY], facade: [{ x: 26000, y: 14000 }, { x: 35000, y: 14000 }] as [XY, XY] };
    const moved = followConstraints(out, [line]);
    expect(moved).not.toEqual(out);
    expect(moved.filter(isGuide)).toHaveLength(4);
  });

  it("un coin déplacé sur la photo (D de Trévarez) : 4 coins à nouveau", () => {
    const d = [
      { slide: { x: 0, y: 0 }, facade: { x: 35151, y: 14120 } },
      { slide: { x: 280, y: 0 }, facade: { x: 44968, y: 13797 } },
      { slide: { x: 280, y: 280 }, facade: { x: 44987, y: 21146 } },
      { slide: { x: 2.35, y: 277.57 }, facade: { x: 35474, y: 21108 } },
    ];
    const out = ensureGuides(d);
    expect(out.filter(isGuide)).toHaveLength(4);
    expect(out).toHaveLength(5);
    // Les 4 coins tombent sur la pose actuelle : les coins qui y étaient ne bougent pas.
    expect(out[0].facade.x).toBeCloseTo(35151, 3);
    expect(out[2].facade.y).toBeCloseTo(21146, 3);
  });

  it("4 coins déjà là, ou moins de 4 points : rien ne change", () => {
    const g = cornerPairs(p(5000, 5000), 2800);
    expect(ensureGuides(g)).toBe(g);
    expect(ensureGuides(pins.slice(0, 3))).toEqual(pins.slice(0, 3));
  });
});


describe("points sûrs qui ne renversent jamais la photo (D-3.2-m)", () => {
  const guides = cornerPairs(p(5000, 5000), 2800);
  // Les 4 points de Benjamin sur A (Trévarez, 14 novembre), posés sans coins guides.
  const benjaminA = [
    { slide: p(236.99, 175.08), facade: p(34180.15, 15861.35) },
    { slide: p(10.88, 175.27), facade: p(26289.19, 15181.95) },
    { slide: p(222.5, 223.73), facade: p(33586.77, 17359.94) },
    { slide: p(55.37, 203.39), facade: p(28132.61, 16368.42) },
  ];

  it("4 points groupés dans le bas, deux inversés : la forme libre croiserait, la photo se penche seulement", () => {
    const slides = [p(40, 200), p(240, 200), p(40, 260), p(240, 260)];
    expect(pinFit(slides)).toBe("libre");
    // Les deux du bas sont lâchés l'un à la place de l'autre : un trapèze exact par les 4 serait croisé.
    const facades = [p(3800, 5600), p(6200, 5600), p(6200, 6200), p(3800, 6200)];
    const out = followPins([...guides, ...slides.map((s, k) => ({ slide: s, facade: facades[k] }))]);
    expect(out).not.toEqual([...guides, ...slides.map((s, k) => ({ slide: s, facade: facades[k] }))]);
    expect(poseIsSound(out)).toBe(true);
    // Forme penchée : les côtés opposés restent parallèles.
    const [a, b, c, d] = out.slice(0, 4).map((q) => q.facade);
    expect(b.x - a.x).toBeCloseTo(c.x - d.x, 6);
    expect(b.y - a.y).toBeCloseTo(c.y - d.y, 6);
  });

  it("des points posés un à un retrouvent les coins guides, sans rien croiser", () => {
    const out = ensureGuides(benjaminA);
    expect(out.filter(isGuide)).toHaveLength(4);
    expect(out.filter((q) => !isGuide(q))).toEqual(benjaminA);
    expect(poseIsSound(out)).toBe(true);
    // Une fois les coins revenus, déplacer un point garde la photo saine.
    const moved = benjaminA.map((q, i) => (i === 2 ? { ...q, facade: p(q.facade.x - 3000, q.facade.y + 2500) } : q));
    expect(poseIsSound(followPins(ensureGuides(moved)))).toBe(true);
  });

  it("4 points sans coins qui donneraient un trapèze croisé : coins d'une forme penchée", () => {
    const crossed = [
      { slide: p(40, 200), facade: p(3800, 5600) },
      { slide: p(240, 200), facade: p(6200, 5600) },
      { slide: p(40, 260), facade: p(6200, 6200) },
      { slide: p(240, 260), facade: p(3800, 6200) },
    ];
    const out = ensureGuides(crossed);
    expect(out.filter(isGuide)).toHaveLength(4);
    expect(poseIsSound(out)).toBe(true);
  });

  it("une pose ordinaire est dite saine, une pose retournée ne l'est pas", () => {
    expect(poseIsSound(guides)).toBe(true);
    const flipped = guides.map((g, i) => (i === 1 ? { ...g, facade: guides[2].facade } : i === 2 ? { ...g, facade: guides[1].facade } : g));
    expect(poseIsSound(flipped)).toBe(false);
  });
});

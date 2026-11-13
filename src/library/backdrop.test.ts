import { describe, expect, it } from "vitest";
import { BACKDROPS, parseBackdrop } from "./backdrop";

describe("fond des dessins", () => {
  it("offre noir, damier et pierre", () => {
    expect(BACKDROPS.map((b) => b.label)).toEqual(["Noir", "Damier", "Pierre"]);
  });

  it("relit le choix enregistré", () => {
    expect(parseBackdrop("noir")).toBe("noir");
    expect(parseBackdrop("facade")).toBe("facade");
  });

  it("revient au damier pour un choix inconnu ou absent", () => {
    expect(parseBackdrop(null)).toBe("damier");
    expect(parseBackdrop("bleu")).toBe("damier");
  });
});

import { describe, expect, it } from "vitest";
import { normalizeTurns, rotationLabel } from "./rotation";

describe("orientation du plan", () => {
  it("ramène les quarts de tour entre 0 et 3", () => {
    expect(normalizeTurns(undefined)).toBe(0);
    expect(normalizeTurns(4)).toBe(0);
    expect(normalizeTurns(-1)).toBe(3);
    expect(normalizeTurns(5)).toBe(1);
  });

  it("dit l'orientation en mots", () => {
    expect(rotationLabel(undefined)).toBe("Comme dans le fichier");
    expect(rotationLabel(1)).toBe("Tourné d'un quart de tour à droite");
    expect(rotationLabel(2)).toBe("Tourné d'un demi-tour");
    expect(rotationLabel(3)).toBe("Tourné d'un quart de tour à gauche");
  });
});

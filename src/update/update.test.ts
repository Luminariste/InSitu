import { describe, expect, it } from "vitest";
import { checkFailureMessage, failureMessage, isDownloaded, progressLabel, progressShare, upToDateMessage } from "./update";

const MB = 1024 * 1024;

describe("avancement de la mise à jour", () => {
  it("dit la taille téléchargée sur la taille totale, en Mo", () => {
    expect(progressLabel({ done: 12.34 * MB, total: 45.6 * MB })).toMatch(/^12,3\s?Mo sur 45,6\s?Mo$/);
  });

  it("se contente de la taille téléchargée quand le total est inconnu", () => {
    expect(progressLabel({ done: 3 * MB, total: null })).toMatch(/^3,0\s?Mo$/);
    expect(progressShare({ done: 3 * MB, total: null })).toBeNull();
  });

  it("borne la part entre 0 et 1", () => {
    expect(progressShare({ done: 50, total: 100 })).toBe(0.5);
    expect(progressShare({ done: 150, total: 100 })).toBe(1);
    expect(isDownloaded({ done: 100, total: 100 })).toBe(true);
    expect(isDownloaded({ done: 99, total: 100 })).toBe(false);
  });
});

describe("échec de la mise à jour", () => {
  it("dit simplement qu'internet manque, et que l'app reste en place", () => {
    const message = failureMessage(new Error("error sending request for url"));
    expect(message).toContain("connexion à GitHub impossible");
    expect(message).toContain("L'app actuelle reste en place");
  });

  it("garde le détail technique d'une autre erreur", () => {
    const message = failureMessage("signature invalide");
    expect(message).toContain("n'a pas pu être installée");
    expect(message).toContain("signature invalide");
  });
});

describe("recherche de mise à jour demandée à la main", () => {
  it("dit que l'app est à jour, avec son numéro", () => {
    expect(upToDateMessage("0.1.45")).toBe("InSitu 0.1.45 est la dernière version publiée.");
    expect(upToDateMessage(null)).toBe("InSitu est la dernière version publiée.");
  });

  it("dit simplement qu'Internet manque", () => {
    expect(checkFailureMessage(new Error("error sending request for url"))).toMatch(/^Impossible de joindre GitHub/);
    expect(checkFailureMessage("signature invalide")).toMatch(/^La recherche de mise à jour n'a pas abouti\. Détail : signature invalide$/);
  });
});

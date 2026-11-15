import { describe, expect, it } from "vitest";
import type { ConflictDifference } from "../engine";
import { allChoices, copyTitle, groupBySection, holderName, initialChoices, minutesAgo, shortTime } from "./conflictLogic";

const diff = (key: string, section: string, suggested: ConflictDifference["suggested"]): ConflictDifference => ({
  key,
  section,
  label: key,
  detail: "position",
  suggested,
});

describe("fenêtre « Deux versions du projet »", () => {
  const differences = [
    diff("projectors/b", "Projecteurs", null),
    diff("placements/p1", "Dessins posés", "copy"),
    diff("placements/p2", "Dessins posés", "current"),
  ];

  it("part des suggestions, sinon de la version ouverte", () => {
    expect(initialChoices(differences)).toEqual({ "projectors/b": "current", "placements/p1": "copy", "placements/p2": "current" });
  });

  it("peut tout prendre d'un côté", () => {
    expect(Object.values(allChoices(differences, "copy"))).toEqual(["copy", "copy", "copy"]);
  });

  it("range les différences par rubrique", () => {
    expect(groupBySection(differences).map((g) => [g.section, g.items.length])).toEqual([
      ["Projecteurs", 1],
      ["Dessins posés", 2],
    ]);
  });

  it("montre le nom de la copie sans « projet ( ).json »", () => {
    expect(copyTitle("projet (Copie en conflit de Benjamin Nesme 2026-11-15).json")).toBe("Copie en conflit de Benjamin Nesme 2026-11-15");
  });
});

describe("avertissements du verrou", () => {
  const now = new Date(2026, 10, 15, 12, 30);

  it("dit depuis quand l'autre Mac n'a pas donné signe de vie", () => {
    expect(minutesAgo(new Date(2026, 10, 15, 12, 27).toISOString(), now)).toBe("il y a 3 min");
    expect(minutesAgo(new Date(2026, 10, 15, 10, 25).toISOString(), now)).toBe("il y a 2 h 05");
    expect(minutesAgo(now.toISOString(), now)).toBe("à l'instant");
  });

  it("écrit les dates courtes", () => {
    expect(shortTime(new Date(2026, 10, 15, 9, 5).toISOString(), now)).toBe("aujourd'hui à 09:05");
    expect(shortTime(new Date(2026, 10, 14, 18, 0).toISOString(), now)).toBe("le 14/11 à 18:00");
    expect(shortTime(null, now)).toBe("date inconnue");
  });

  it("nomme la personne et son Mac", () => {
    const holder = { sessionId: "", machine: "iMac-de-Marc.local", user: "marc", openedAt: "", refreshedAt: "", appVersion: "" };
    expect(holderName(holder)).toBe("marc sur « iMac-de-Marc »");
  });
});

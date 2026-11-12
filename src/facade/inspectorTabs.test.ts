import { describe, expect, it } from "vitest";
import { contextTab, type TabContext } from "./inspectorTabs";

const idle: TabContext = { hasPlan: true, picking: false, selection: 0 };

describe("onglet de l'inspecteur selon le contexte", () => {
  it("choisir un dessin ouvre Outils", () => {
    expect(contextTab(idle, { ...idle, selection: 1 }, "export")).toBe("outils");
  });

  it("un onglet choisi à la main reste tant que rien ne change", () => {
    expect(contextTab({ ...idle, selection: 2 }, { ...idle, selection: 3 }, "calques")).toBe("calques");
    expect(contextTab(idle, idle, "fresque")).toBe("fresque");
  });

  it("poser A et B ouvre Mise en plan", () => {
    expect(contextTab(idle, { ...idle, picking: true }, "outils")).toBe("echelle");
  });

  it("sans plan, on revient à Plan", () => {
    expect(contextTab(idle, { ...idle, hasPlan: false }, "export")).toBe("plan");
    expect(contextTab(null, { ...idle, hasPlan: false }, "export")).toBe("plan");
  });

  it("au premier affichage, l'onglet retenu est gardé s'il y a un plan", () => {
    expect(contextTab(null, idle, "calques")).toBe("calques");
  });
});

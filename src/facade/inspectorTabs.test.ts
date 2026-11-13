import { describe, expect, it } from "vitest";
import { contextTab, type TabContext } from "./inspectorTabs";

const idle: TabContext = { hasPlan: true, picking: false, selection: 0 };

describe("onglet de l'inspecteur selon le contexte", () => {
  it("choisir un dessin ne change plus d'onglet : ses réglages sont en haut (D-4.2-p)", () => {
    expect(contextTab(idle, { ...idle, selection: 1 }, "export")).toBe("export");
  });

  it("un onglet choisi à la main reste tant que rien ne change", () => {
    expect(contextTab({ ...idle, selection: 2 }, { ...idle, selection: 3 }, "calques")).toBe("calques");
    expect(contextTab(idle, idle, "export")).toBe("export");
  });

  it("poser A et B ouvre Plan", () => {
    expect(contextTab(idle, { ...idle, picking: true }, "dessins")).toBe("plan");
  });

  it("sans plan, on revient à Plan", () => {
    expect(contextTab(idle, { ...idle, hasPlan: false }, "export")).toBe("plan");
    expect(contextTab(null, { ...idle, hasPlan: false }, "export")).toBe("plan");
  });

  it("au premier affichage, l'onglet retenu est gardé s'il y a un plan", () => {
    expect(contextTab(null, idle, "calques")).toBe("calques");
  });
});

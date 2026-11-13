import { describe, expect, it } from "vitest";
import { drawingTree } from "./drawingTree";

const d = (id: string, folder: string, name = id) => ({ id, name, folder });

describe("dessins classés par dossier (D-4.2-o)", () => {
  const list = [
    d("murene", "Poissons", "Murène"),
    d("diodon", "Poissons", "Diodon"),
    d("raie2", "Poissons/Raies", "Raie 10"),
    d("raie1", "Poissons/Raies", "Raie 2"),
    d("algue", "Algues"),
    d("seul", ""),
  ];

  it("range chaque dessin dans son dossier, comme la Bibliothèque", () => {
    const root = drawingTree(list);
    expect(root.count).toBe(6);
    expect(root.drawings.map((x) => x.id)).toEqual(["seul"]);
    expect(root.children.map((c) => [c.label, c.count])).toEqual([
      ["Algues", 1],
      ["Poissons", 4],
    ]);
    const poissons = root.children[1];
    expect(poissons.drawings.map((x) => x.name)).toEqual(["Diodon", "Murène"]);
    expect(poissons.children[0]).toMatchObject({ path: "Poissons/Raies", label: "Raies", depth: 1, count: 2 });
    // Tri naturel : « Raie 2 » avant « Raie 10 ».
    expect(poissons.children[0].drawings.map((x) => x.name)).toEqual(["Raie 2", "Raie 10"]);
  });

  it("la recherche porte sur le nom et le dossier, et retire les dossiers vides", () => {
    const root = drawingTree(list, "raie");
    expect(root.count).toBe(2);
    expect(root.children.map((c) => c.label)).toEqual(["Poissons"]);
    expect(root.children[0].drawings).toEqual([]);
    expect(drawingTree(list, "poissons mur").count).toBe(1);
    expect(drawingTree(list, "rien").children).toEqual([]);
  });
});

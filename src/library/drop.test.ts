import { describe, expect, it } from "vitest";
import { asciiJson, carriesFiles, ignoredNotice, isDrawingName } from "./drop";

describe("dépôt depuis le Finder", () => {
  it("garde les dessins, écarte le reste et les fichiers cachés", () => {
    expect(isDrawingName("merle.png")).toBe(true);
    expect(isDrawingName("Oiseaux/Grands/héron.TIF")).toBe(true);
    expect(isDrawingName("Oiseaux/aigle.svg")).toBe(true);
    expect(isDrawingName("notes.txt")).toBe(false);
    expect(isDrawingName(".DS_Store")).toBe(false);
    expect(isDrawingName("Oiseaux/._merle.png")).toBe(false);
    expect(isDrawingName(".cache/merle.png")).toBe(false);
    expect(isDrawingName("png")).toBe(false);
  });
  it("reconnaît un glisser du Finder", () => {
    expect(carriesFiles(["Files"])).toBe(true);
    expect(carriesFiles(["application/x-insitu-dessin", "text/plain"])).toBe(false);
    expect(carriesFiles(["application/x-insitu-dessins", "Files"])).toBe(false);
  });
  it("écrit l'en-tête en ASCII, relu à l'identique", () => {
    const value = { depot: "abc-1", chemin: "Grands/héron été.tif" };
    const text = asciiJson(value);
    expect(/^[\x20-\x7e]*$/.test(text)).toBe(true);
    expect(JSON.parse(text)).toEqual(value);
  });
  it("dit ce qui est laissé de côté", () => {
    expect(ignoredNotice(0)).toBeNull();
    expect(ignoredNotice(1)).toContain("Un fichier");
    expect(ignoredNotice(3)).toContain("3 fichiers");
  });
});

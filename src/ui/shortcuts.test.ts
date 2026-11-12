import { describe, expect, it } from "vitest";
import { formatShortcut, isTextEntry, matchesShortcut } from "./shortcuts";

const key = (k: string, extra: Partial<Parameters<typeof matchesShortcut>[1]> = {}) => ({
  key: k,
  metaKey: false,
  ctrlKey: false,
  shiftKey: false,
  altKey: false,
  ...extra,
});

describe("raccourcis", () => {
  it("s'affichent dans l'ordre du Mac", () => {
    expect(formatShortcut("mod+n")).toBe("⌘N");
    expect(formatShortcut("mod+shift+e")).toBe("⇧⌘E");
    expect(formatShortcut("alt+mod+0")).toBe("⌥⌘0");
    expect(formatShortcut("backspace")).toBe("⌫");
    expect(formatShortcut("mod+n", false)).toBe("Ctrl+N");
  });

  it("reconnaissent la frappe exacte", () => {
    expect(matchesShortcut("mod+n", key("n", { metaKey: true, code: "KeyN" }))).toBe(true);
    expect(matchesShortcut("mod+n", key("n", { code: "KeyN" }))).toBe(false);
    expect(matchesShortcut("mod+n", key("N", { metaKey: true, shiftKey: true, code: "KeyN" }))).toBe(false);
    expect(matchesShortcut("mod+1", key("1", { metaKey: true, code: "Digit1" }))).toBe(true);
    expect(matchesShortcut("mod+0", key("0", { ctrlKey: true, code: "Digit0" }), false)).toBe(true);
  });

  it("lisent la position de la touche quand ⌥ change le caractère", () => {
    expect(matchesShortcut("alt+mod+n", key("˜", { metaKey: true, altKey: true, code: "KeyN" }))).toBe(true);
  });

  it("acceptent ⌫ et ⌦ pour supprimer", () => {
    expect(matchesShortcut("backspace", key("Backspace"))).toBe(true);
    expect(matchesShortcut("backspace", key("Delete"))).toBe(true);
  });

  it("ne volent pas la frappe d'un champ de texte", () => {
    expect(isTextEntry({ tagName: "INPUT", type: "text" })).toBe(true);
    expect(isTextEntry({ tagName: "INPUT", type: "search" })).toBe(true);
    expect(isTextEntry({ tagName: "INPUT", type: "checkbox" })).toBe(false);
    expect(isTextEntry({ tagName: "TEXTAREA" })).toBe(true);
    expect(isTextEntry({ tagName: "BUTTON" })).toBe(false);
  });
});

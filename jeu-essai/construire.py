#!/usr/bin/env python3
"""Reconstruit un jeu d'essai à partir des fichiers d'origine.

Usage : python3 jeu-essai/construire.py jeu-essai/epinal <dossier des originaux>

Les fichiers d'origine ne sont pas dans le dépôt (ils restent dans le Dropbox
de Luminariste). Ce script lit `<jeu>/sources.json`, la liste tenue à la main
des fichiers et de leur rôle, et produit pour chacun :

- `references/…` : la version pleine résolution versée au dépôt, selon le
  champ `reference` de sources.json :
    "identique"         copie exacte de l'original ;
    "jpeg-92"           même taille en pixels, JPEG qualité 92 ;
    "gris-8-bits"       même taille en pixels, PNG en niveaux de gris ;
    "lightburn-allege"  projet LightBurn sans ses images incorporées ;
    null                rien (l'original est seulement décrit).
- `reduits/…` : une version divisée par `reduction`, pour les tests rapides.
- `manifeste.json` : tailles, empreintes et dimensions de l'ensemble.

Aucune rotation EXIF n'est appliquée : une coordonnée de l'original se
retrouve dans le réduit en la divisant par `echelle`. Les images produites
n'ont plus d'EXIF (donc plus de position GPS). Dépendance : Pillow.
"""
import hashlib
import json
import re
import shutil
import sys
from pathlib import Path

from PIL import Image

Image.MAX_IMAGE_PIXELS = None
IMAGE_SUFFIXES = (".jpg", ".png", ".tiff")


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def describe(path: Path, rel: str) -> dict:
    info = {"chemin": rel, "octets": path.stat().st_size, "sha256": sha256(path)}
    if path.suffix.lower() in IMAGE_SUFFIXES:
        info["largeur"], info["hauteur"] = Image.open(path).size
    return info


def make_reference(kind: str, original: Path, root: Path, fichier: str) -> dict:
    rel = Path("references") / fichier
    if kind == "jpeg-92":
        rel = rel.with_suffix(".jpg")
    elif kind == "gris-8-bits":
        rel = rel.with_suffix(".png")
    dst = root / rel
    dst.parent.mkdir(parents=True, exist_ok=True)
    if kind == "identique":
        shutil.copyfile(original, dst)
    elif kind == "jpeg-92":
        Image.open(original).convert("RGB").save(dst, quality=92)
    elif kind == "gris-8-bits":
        Image.open(original).convert("L").save(dst, optimize=True)
    elif kind == "lightburn-allege":
        text = original.read_text(encoding="utf-8", errors="replace")
        text = re.sub(r'(Source|Data)="[^"]{200,}"', r'\1=""', text)
        dst.write_text(text, encoding="utf-8")
    else:
        raise ValueError(f"type de référence inconnu : {kind}")
    info = describe(dst, rel.as_posix())
    info["fidelite"] = kind
    return info


def make_reduced(original: Path, root: Path, fichier: str, scale: int, role: str) -> dict:
    ext = ".jpg" if original.suffix.lower() == ".jpg" else ".png"
    rel = (Path("reduits") / fichier).with_suffix(ext)
    dst = root / rel
    dst.parent.mkdir(parents=True, exist_ok=True)
    im = Image.open(original)
    size = (round(im.width / scale), round(im.height / scale))
    grayscale = role in ("engraving_export", "target")
    im = im.convert("L" if grayscale else "RGB").resize(size, Image.LANCZOS)
    if ext == ".jpg":
        im.save(dst, quality=85)
    else:
        im.save(dst, optimize=True)
    info = describe(dst, rel.as_posix())
    info["echelle"] = scale
    return info


def main(root: Path, originals: Path):
    sources = json.loads((root / "sources.json").read_text(encoding="utf-8"))
    for generated in ("references", "reduits"):
        shutil.rmtree(root / generated, ignore_errors=True)
    entries = []
    for item in sources["fichiers"]:
        original = originals / item["fichier"]
        entry = {
            "id": item["id"],
            "role": item["role"],
            "projecteur": item.get("projecteur"),
            "original": describe(original, item["nomDorigine"]),
        }
        if original.suffix.lower() == ".jpg":
            exif = Image.open(original).getexif()
            entry["original"]["orientationExif"] = exif.get(274, 1)
            if exif.get(306):
                entry["original"]["priseDeVue"] = exif.get(306)
        if item.get("note"):
            entry["note"] = item["note"]
        if item.get("reference"):
            entry["reference"] = make_reference(item["reference"], original, root, item["fichier"])
        if item.get("reduction"):
            entry["reduit"] = make_reduced(
                original, root, item["fichier"], item["reduction"], item["role"]
            )
        entries.append(entry)
    manifest = {
        "nom": sources["nom"],
        "description": sources["description"],
        "projecteurs": sources["projecteurs"],
        "fichiers": entries,
    }
    (root / "manifeste.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(f"{len(entries)} fichiers décrits dans {root / 'manifeste.json'}")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(Path(sys.argv[1]), Path(sys.argv[2]))

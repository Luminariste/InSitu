"""Fabrique la photo HEIC d'essai à partir de la photo brute A d'Épinal.

Tâche 2.5 : le moteur lit le HEIC des iPhone. Les photos d'Épinal ont été
livrées en JPEG ; pour tester la lecture du HEIC sur une vraie mire, on
réencode `a-brut.jpg` en HEIC avec un encodeur indépendant du moteur
(libheif + x265, par le paquet Python `pillow-heif`).

Les pixels gardent la disposition du JPEG (image couchée, sans rotation) :
les croisements de la mire doivent tomber au même endroit dans les deux
fichiers, aux pertes de compression près.

Usage : python3 jeu-essai/heic/fabriquer.py
Dépendances : pip install pillow pillow-heif
"""

from pathlib import Path

import pillow_heif
from PIL import Image

ICI = Path(__file__).resolve().parent
SOURCE = ICI.parent / "epinal" / "references" / "photos-brutes" / "a-brut.jpg"
SORTIE = ICI / "a-brut.heic"
QUALITE = 40


def main() -> None:
    image = Image.open(SOURCE)
    image.load()
    # On retire l'EXIF : il dirait « orientation 8 » et certains lecteurs
    # tourneraient l'image ; le moteur, lui, ne regarde que le conteneur.
    rgb = Image.frombytes("RGB", image.size, image.convert("RGB").tobytes())
    heif = pillow_heif.from_pillow(rgb)
    heif.save(SORTIE, quality=QUALITE, chroma=420)
    print(f"{SORTIE.name} : {rgb.size[0]} × {rgb.size[1]} px, {SORTIE.stat().st_size} octets"
          f" (libheif {pillow_heif.libheif_version()}, qualité {QUALITE})")


if __name__ == "__main__":
    main()

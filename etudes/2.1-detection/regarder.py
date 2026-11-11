"""Fabrique une planche-contact des photos brutes et des recadrages pleine
résolution, pour regarder ce que contiennent vraiment les photos.

    python3 etudes/2.1-detection/regarder.py <dossier de sortie>
"""
import os
import sys

import cv2
import numpy as np

ICI = os.path.dirname(os.path.abspath(__file__))
BRUTES = os.path.join(ICI, "..", "..", "jeu-essai", "epinal", "references", "photos-brutes")

# Recadrages (x, y) de 700 px dans la photo a-brut, pixels tels qu'enregistrés.
RECADRAGES = {
    "hg": (500, 300),
    "centre": (1300, 1300),
    "ciel": (1900, 400),
    "bd": (2000, 2000),
    "bg": (550, 2000),
    "hd": (2100, 300),
}


def lire(nom):
    """Lit une photo sans appliquer l'orientation EXIF (pixels tels qu'enregistrés)."""
    return cv2.imread(os.path.join(BRUTES, nom), cv2.IMREAD_COLOR | cv2.IMREAD_IGNORE_ORIENTATION)


def main(sortie):
    os.makedirs(sortie, exist_ok=True)
    noms = sorted(os.listdir(BRUTES))
    vignettes = []
    for nom in noms:
        petite = cv2.resize(lire(nom), (672, 504), interpolation=cv2.INTER_AREA)
        cv2.putText(petite, nom, (10, 30), 0, 1, (0, 255, 255), 2)
        vignettes.append(petite)
    planche = np.vstack([np.hstack(vignettes[:4]), np.hstack(vignettes[4:])])
    cv2.imwrite(os.path.join(sortie, "planche.jpg"), planche)
    for nom in noms:
        image = lire(nom)
        base = nom.split(".")[0]
        for cle, (x, y) in RECADRAGES.items():
            cv2.imwrite(os.path.join(sortie, f"{base}_{cle}.png"), image[y:y + 700, x:x + 700])


if __name__ == "__main__":
    main(sys.argv[1])

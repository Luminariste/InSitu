"""Images d'illustration du rapport : les mires candidates côte à côte et une
photo simulée par niveau de difficulté.

    python3 etudes/2.1-detection/figures.py <dossier de sortie>
"""
import os
import sys

import cv2
import numpy as np

import simulateur as S
from mires import dessiner


def main(sortie):
    os.makedirs(sortie, exist_ok=True)
    blanc = np.full((560, 10, 3), 255, np.uint8)
    vues = []
    for nom in ("actuelle", "reperes", "charuco"):
        vues += [cv2.resize(dessiner(nom)["image"], (560, 560), interpolation=cv2.INTER_AREA), blanc]
    cv2.imwrite(os.path.join(sortie, "mires-candidates.png"), cv2.cvtColor(np.hstack(vues[:-1]), cv2.COLOR_BGR2GRAY))
    # La mire proposée, plus grande, en couleur (cercle rouge).
    cv2.imwrite(os.path.join(sortie, "mire-proposee.png"), cv2.resize(dessiner("reperes")["image"], (900, 900), interpolation=cv2.INTER_AREA),
                [cv2.IMWRITE_PNG_COMPRESSION, 9])
    vues = []
    for niveau in S.NIVEAUX:
        alea = np.random.default_rng(100 + niveau)
        photo = S.Scene(S.tirer(alea, niveau)).rendre("reperes")
        photo = cv2.resize(photo, (640, 640), interpolation=cv2.INTER_AREA)
        cv2.putText(photo, f"niveau {niveau}", (14, 34), cv2.FONT_HERSHEY_SIMPLEX, 1.0, (0, 255, 255), 2, cv2.LINE_AA)
        vues.append(photo)
    planche = np.vstack([np.hstack(vues[:2]), np.hstack(vues[2:])])
    cv2.imwrite(os.path.join(sortie, "simulation-niveaux.jpg"), planche, [cv2.IMWRITE_JPEG_QUALITY, 80])


if __name__ == "__main__":
    main(sys.argv[1])

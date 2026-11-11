"""Compare le rendu du simulateur (niveau 1, mire actuelle) aux vraies photos
d'Épinal : contraste et largeur apparente des traits, écart au trapèze, et
une image côte à côte.

    python3 etudes/2.1-detection/calibrer.py <dossier de sortie>
"""
import json
import os
import sys

import cv2
import numpy as np

import detecteurs as D
import simulateur as S
from mires import CENTRE
from photos_reelles import REF, contraste_traits


class Verite:
    def __init__(self, geo):
        self.geo = geo

    def predire(self, X):
        return self.geo.mm_vers_px(np.asarray(X, np.float64).reshape(-1, 2))


def main(sortie):
    os.makedirs(sortie, exist_ok=True)
    m = S.mire("actuelle")
    prof, larg, trap, bruit = [], [], [], []
    vignette = None
    for k in range(6):
        alea = np.random.default_rng(900 + k)
        scene = S.Scene(S.tirer(alea, 1))
        bgr = scene.rendre("actuelle")
        gris = D.en_gris(bgr)
        res = D.amorce_actuelle(bgr, gris, m["valides"])
        if not res["ok"]:
            continue
        p, l = contraste_traits(gris, Verite(scene.geo), res["mm"])
        prof.append(np.median(p))
        larg.append(np.median(l) * S.ECHELLE)
        H = D.homographie(res["mm"], res["px"])
        trap.append(np.mean(np.linalg.norm(D.projeter(H, res["mm"]) - res["px"], axis=1)) / D.echelle_locale(H))
        if vignette is None:
            c = scene.geo.mm_vers_px(np.array([[CENTRE + 45, CENTRE + 45]]))[0].astype(int)
            vignette = bgr[c[1] - 300:c[1] + 300, c[0] - 300:c[0] + 300]
    bilan = {
        "simule_niveau_1": {"contraste_trait": round(float(np.median(prof)), 3), "largeur_trait_px": round(float(np.median(larg)), 2),
                            "ecart_au_trapeze_moyen_mm": round(float(np.median(trap)), 3)},
        "reel_epinal": "voir photos-reelles.json : contraste 0,40 à 0,46 ; largeur 2,4 à 2,7 px ; écart au trapèze 0,33 à 0,48 mm",
    }
    print(json.dumps(bilan, ensure_ascii=False, indent=1))
    with open(os.path.join(sortie, "calibrage.json"), "w") as f:
        json.dump(bilan, f, ensure_ascii=False, indent=1)
    reel = cv2.imread(os.path.join(REF, "photos-brutes", "a-brut.jpg"), cv2.IMREAD_COLOR | cv2.IMREAD_IGNORE_ORIENTATION)
    reel = reel[1500:2100, 1500:2100]
    cote = np.hstack([reel, np.full((600, 12, 3), 255, np.uint8), vignette])
    cv2.imwrite(os.path.join(sortie, "reel-contre-simule.jpg"), cote, [cv2.IMWRITE_JPEG_QUALITY, 82])


if __name__ == "__main__":
    main(sys.argv[1])

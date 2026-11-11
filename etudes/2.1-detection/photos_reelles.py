"""Détection de la mire ACTUELLE sur les 8 vraies photos d'Épinal, et mesure
de l'écart du redressement manuel (photos redressées dans Millumin).

    python3 etudes/2.1-detection/photos_reelles.py <dossier de sortie>

Écrit `photos-reelles.json` et deux images de contrôle.
"""
import json
import os
import sys

import cv2
import numpy as np

import detecteurs as D
from mires import CENTRE, COTE, dessiner

ICI = os.path.dirname(os.path.abspath(__file__))
REF = os.path.join(ICI, "..", "..", "jeu-essai", "epinal", "references")


def stats(v):
    v = np.asarray(v)
    return {"moyenne": round(float(v.mean()), 3), "p95": round(float(np.quantile(v, 0.95)), 3), "max": round(float(v.max()), 3)}


def contraste_traits(gris, modele, mm):
    """Profondeur relative et largeur à mi-hauteur des traits fins, mesurées
    au milieu des segments entre deux nœuds voisins."""
    prof, larg = [], []
    decal = np.arange(-1.5, 1.501, 0.02)
    for x, y in mm[::3]:
        if x + 10 > COTE:
            continue
        pts = np.stack([np.full(len(decal), x + 5.0), y + decal], 1)
        lignes = [D._echantillonner(gris, modele.predire(pts + [dx, 0])) for dx in (-1.5, -0.75, 0, 0.75, 1.5)]
        v = np.mean(lignes, 0)
        fond = np.median(np.r_[v[:25], v[-25:]])
        i = v[50:101].argmin() + 50
        creux = fond - v[i]
        if fond < 5 or creux <= 0:
            continue
        prof.append(creux / fond)
        sous = v < fond - creux / 2
        g, d = i, i
        while g > 0 and sous[g]:
            g -= 1
        while d < len(v) - 1 and sous[d]:
            d += 1
        larg.append((d - g) * 0.02)
    return prof, larg


def main(sortie):
    os.makedirs(sortie, exist_ok=True)
    mire = dessiner("actuelle")
    valides = mire["valides"]
    bilan = {"photos_brutes": {}, "photos_redressees": {}}
    brutes = os.path.join(REF, "photos-brutes")
    modeles = {}
    for nom in sorted(os.listdir(brutes)):
        bgr = cv2.imread(os.path.join(brutes, nom), cv2.IMREAD_COLOR | cv2.IMREAD_IGNORE_ORIENTATION)
        gris = D.en_gris(bgr)
        res = D.amorce_actuelle(bgr, gris, valides)
        ligne = {"detectee": bool(res["ok"]), **res["info"]}
        if res["ok"]:
            mm, px = res["mm"], res["px"]
            H = D.homographie(mm, px)
            ech = D.echelle_locale(H)
            trapeze = np.linalg.norm(D.projeter(H, mm) - px, axis=1) / ech
            # Validation croisée : un nœud sur quatre est prédit par les autres.
            alea = np.random.default_rng(1)
            tirage = alea.permutation(len(mm))
            croise = np.zeros(len(mm))
            for k in range(4):
                test = tirage[k::4]
                reste = np.setdiff1d(tirage, test)
                mod = D.Modele(mm[reste], px[reste])
                croise[test] = np.linalg.norm(mod.predire(mm[test]) - px[test], axis=1) / ech
            modele = D.Modele(mm, px)
            prof, larg = contraste_traits(gris, modele, mm)
            # Luminosité du papier au centre des cases (0 à 255).
            c = np.arange(5.0, COTE, 10.0)
            centres = np.stack(np.meshgrid(c, c), 2).reshape(-1, 2)
            niveaux = D._echantillonner(cv2.blur(gris, (9, 9)), D.ModeleTourne(modele, res["quarts"]).predire(centres))
            ligne.update({
                "papier_part_saturee": round(float((niveaux > 245).mean()), 3),
                "papier_part_sombre": round(float((niveaux < 50).mean()), 3),
                "papier_gris_p5_p50_p95": [int(np.quantile(niveaux, q)) for q in (0.05, 0.5, 0.95)],
                "noeuds": int(len(mm)), "noeuds_possibles": int(valides.sum()),
                "echelle_px_par_mm": round(ech, 2), "quarts_de_tour": res["quarts"],
                "ecart_au_trapeze_mm": stats(trapeze),
                "ecart_validation_croisee_mm": stats(croise),
                "contraste_trait_median": round(float(np.median(prof)), 3),
                "largeur_trait_px_mediane": round(float(np.median(larg)) * ech, 2),
            })
            modeles[nom] = (mm, px, res["quarts"], bgr.shape)
            if nom == "a-brut.jpg":
                vue = bgr.copy()
                for (x, y), e in zip(px, trapeze):
                    coul = (0, 255, 0) if e < 0.5 else (0, 200, 255) if e < 1.0 else (0, 0, 255)
                    cv2.circle(vue, (int(x), int(y)), 9, coul, 3)
                cv2.imwrite(os.path.join(sortie, "reel-a-brut-noeuds.jpg"),
                            cv2.resize(vue, None, fx=0.4, fy=0.4, interpolation=cv2.INTER_AREA), [cv2.IMWRITE_JPEG_QUALITY, 80])
        bilan["photos_brutes"][nom] = ligne
        print(nom, json.dumps(ligne, ensure_ascii=False))

    # Photos redressées à la main : où tombent les croisements, par rapport
    # à leur place théorique ?
    red = os.path.join(REF, "photos-redressees")
    for nom in sorted(os.listdir(red)):
        bgr = cv2.imread(os.path.join(red, nom))
        gris = D.en_gris(bgr)
        s = bgr.shape[0] / COTE
        H0 = np.diag([s, s, 1.0])
        res = D._finir(gris, valides, np.zeros((0, 2)), np.zeros((0, 2)), H0, np.ones_like(valides))
        ligne = {"detectee": bool(res["ok"]), **res["info"]}
        if res["ok"]:
            mm, px = res["mm"], res["px"]
            ecart = np.linalg.norm(px / s - mm, axis=1)
            H = D.homographie(mm, px)
            hors_trapeze = np.linalg.norm(D.projeter(H, mm) - px, axis=1) / s
            ligne.update({"noeuds": int(len(mm)), "ecart_a_la_grille_ideale_mm": stats(ecart),
                          "dont_non_rattrapable_par_4_coins_mm": stats(hors_trapeze)})
            if nom == "a.jpg":
                vue = cv2.resize(bgr, (1100, 1100), interpolation=cv2.INTER_AREA)
                k = 1100 / COTE
                for m, e in zip(mm, px / s - mm):
                    p = (int(m[0] * k), int(m[1] * k))
                    q = (int((m[0] + 8 * e[0]) * k), int((m[1] + 8 * e[1]) * k))
                    cv2.arrowedLine(vue, p, q, (0, 0, 255), 2, cv2.LINE_AA, tipLength=0.3)
                cv2.imwrite(os.path.join(sortie, "reel-a-redressee-ecarts.jpg"), vue, [cv2.IMWRITE_JPEG_QUALITY, 80])
        bilan["photos_redressees"][nom] = ligne
        print(nom, json.dumps(ligne, ensure_ascii=False))

    with open(os.path.join(sortie, "photos-reelles.json"), "w") as f:
        json.dump(bilan, f, ensure_ascii=False, indent=1)


if __name__ == "__main__":
    main(sys.argv[1])

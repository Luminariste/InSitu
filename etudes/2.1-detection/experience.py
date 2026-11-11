"""Expérience de l'étude 2.1 : chaque candidat est essayé sur les mêmes scènes
simulées ; on mesure le taux de détection et la précision en mm sur la mire.

    python3 etudes/2.1-detection/experience.py <fichier.jsonl> [n_niveaux] [n_balayages] [balayages]

Par défaut 30 scènes par niveau de difficulté et 10 par point de balayage,
soit 440 scènes × 4 candidats. Compter 35 à 60 minutes sur 2 cœurs.
Le dernier argument (noms séparés par des virgules) ne refait que certains
balayages.
Le fichier de sortie contient une ligne JSON par (scène, candidat).
"""
import json
import multiprocessing
import sys
import time
import zlib

import cv2
import numpy as np

import detecteurs as D
import simulateur as S
from mires import CANDIDATS, COTE

BALAYAGES = {
    "flou": [2.0, 3.0, 4.5, 6.0],
    "bouge": [4.0, 8.0, 14.0, 20.0],
    "bruit": [5.0, 10.0, 20.0, 30.0],
    "biais": [25.0, 40.0, 50.0],
    "sombre": [0.25, 0.5, 0.7, 0.85],
    "cache": [0.15, 0.3, 0.5],
    "gondole": [3.0, 5.0, 8.0],
    # Mêmes défauts, mais la zone sombre ou cachée est vraiment noire (0,3 %
    # de lumière au lieu de 5 à 7 %).
    "sombre-noir": [0.25, 0.5, 0.7, 0.85],
    "cache-noir": [0.15, 0.3, 0.5],
}

# Points de contrôle : les centres des cases, jamais utilisés par la détection.
_c = np.arange(5.0, COTE, 10.0)
CONTROLE = np.stack(np.meshgrid(_c, _c), 2).reshape(-1, 2)


def erreurs(geo, modele, pres_de=None):
    """Erreur en mm sur la mire aux points de contrôle visibles dans la photo.
    Avec `pres_de` (points mesurés, mm), on ne garde que les points de
    contrôle situés à moins de 8 mm d'un point mesuré : la zone « couverte »."""
    vrai = geo.mm_vers_px(CONTROLE)
    dans = (vrai[:, 0] > 0) & (vrai[:, 0] < S.TAILLE) & (vrai[:, 1] > 0) & (vrai[:, 1] < S.TAILLE)
    pred = modele.predire(CONTROLE[dans])
    retour, _ = geo.px_vers_mm(pred)
    e = np.linalg.norm(retour - CONTROLE[dans], axis=1)
    sortie = {"moy": round(float(e.mean()), 4), "p95": round(float(np.quantile(e, 0.95)), 4), "max": round(float(e.max()), 4)}
    if pres_de is not None:
        d = np.linalg.norm(CONTROLE[dans][:, None, :] - pres_de[None, :, :], axis=2).min(1)
        c = d < 8.0
        sortie["couverture"] = round(float(c.sum() / len(CONTROLE)), 3)
        if c.any():
            sortie["moy_couvert"] = round(float(e[c].mean()), 4)
            sortie["p95_couvert"] = round(float(np.quantile(e[c], 0.95)), 4)
    return sortie


def essayer(scene, nom):
    m = S.mire(nom)
    bgr = scene.rendre(nom)
    gris = D.en_gris(bgr)
    t0 = time.time()
    if nom in ("actuelle", "epaisse"):
        res = D.amorce_actuelle(bgr, gris, m["valides"])
    elif nom == "reperes":
        res = D.amorce_reperes(bgr, gris, m["valides"], m["reperes"])
    else:
        res = D.amorce_charuco(bgr, gris, m["valides"], m["selles"])
    ligne = {"candidat": nom, "ok": bool(res["ok"]), "duree": round(time.time() - t0, 2), "info": res["info"]}
    geo = scene.geo
    if res["ok"]:
        q = res["quarts"]
        ligne["noeuds"] = int(len(res["mm"]))
        tourne = D.ModeleTourne(None, (4 - q) % 4)
        vraies_mm = tourne.tourner(res["mm"])       # nœuds dans le repère réel de la mire
        mesure, _ = geo.px_vers_mm(res["px"])
        e = np.linalg.norm(mesure - vraies_mm, axis=1)
        ligne["noeud_mediane"] = round(float(np.median(e)), 4)
        ligne["noeuds_faux"] = int((e > 0.5).sum())
        modele = D.Modele(res["mm"], res["px"])
        ligne["grille"] = erreurs(geo, D.ModeleTourne(modele, q), vraies_mm)
        ligne["trapeze"] = erreurs(geo, D.ModeleTourne(D.Modele(res["mm"], res["px"], lisse=False), q))
    # Repères codés ou points de selle seuls, sans la grille.
    ap = res.get("appuis")
    if ap is not None and len(ap[0]) >= 4:
        ligne["appuis"] = int(len(ap[0]))
        ligne["appuis_trapeze"] = erreurs(geo, D.Modele(ap[0], ap[1], lisse=False, robuste_px=30.0))
        if len(ap[0]) >= 16:
            ligne["appuis_grille"] = erreurs(geo, D.Modele(ap[0], ap[1], sigma=40.0, robuste_px=30.0))
    return ligne


def traiter(tache):
    condition, niveau, forces, k = tache
    cv2.setNumThreads(1)
    graine = zlib.crc32(f"{condition}-{k}".encode())
    alea = np.random.default_rng(graine)
    p = S.tirer(alea, niveau, **forces)
    scene = S.Scene(p)
    lignes = []
    for nom in CANDIDATS:
        try:
            ligne = essayer(scene, nom)
        except Exception as e:  # une panne du prototype compte comme un échec
            ligne = {"candidat": nom, "ok": False, "info": {"cause": f"erreur : {e}"}}
        ligne.update(condition=condition, scene=k,
                     params={c: round(p[c], 3) for c in ("biais", "gondole", "flou", "bouge", "bruit", "sombre", "cache")})
        lignes.append(ligne)
    return lignes


def main():
    sortie = sys.argv[1]
    n_niv = int(sys.argv[2]) if len(sys.argv) > 2 else 30
    n_bal = int(sys.argv[3]) if len(sys.argv) > 3 else 10
    seuls = sys.argv[4].split(",") if len(sys.argv) > 4 else None   # ne refaire que certains balayages
    taches = [] if seuls else [(f"niveau-{n}", n, {}, k) for n in S.NIVEAUX for k in range(n_niv)]
    for facteur, valeurs in BALAYAGES.items():
        if seuls and facteur not in seuls:
            continue
        for v in valeurs:
            forces = {facteur.replace("-noir", ""): v}
            if facteur.endswith("-noir"):
                forces["noir"] = 1.0
            taches += [(f"{facteur}={v:g}", 1, forces, k) for k in range(n_bal)]
    debut = time.time()
    with multiprocessing.Pool(2) as bassin, open(sortie, "w") as f:
        for i, lignes in enumerate(bassin.imap_unordered(traiter, taches)):
            for ligne in lignes:
                f.write(json.dumps(ligne, ensure_ascii=False) + "\n")
            f.flush()
            if i % 20 == 0:
                print(f"{i + 1}/{len(taches)} scènes, {time.time() - debut:.0f} s", flush=True)


if __name__ == "__main__":
    main()

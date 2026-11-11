"""Essai « semi-réel » : on incruste les 16 repères codés de la mire B dans
les 8 vraies photos d'Épinal, à l'endroit où ils seraient imprimés, puis on
cherche à les relire.

L'incrustation multiplie la photo par la réflectance de l'encre (en lumière
linéaire), après un flou égal à celui mesuré sur les traits réels. Le bruit,
le traitement de l'iPhone, l'image de façade et l'ondulation du papier sont
ceux des vraies photos ; seule l'encre des repères est simulée.

    python3 etudes/2.1-detection/reperes_sur_reel.py <dossier de sortie> [binaire essai-reperes]
"""
import json
import os
import subprocess
import sys

import cv2
import numpy as np

import detecteurs as D
from mires import COTE, MARGE, PXMM, dessiner
from photos_reelles import REF


def carte_inverse(modele, forme, pas=16):
    """px -> mm, par inversion du modèle mm -> px sur une grille lâche."""
    h, w = forme
    v, u = np.mgrid[0:h + pas:pas, 0:w + pas:pas].astype(np.float64)
    px = np.stack([u.ravel(), v.ravel()], 1)
    Hinv = np.linalg.inv(modele.H)
    mm = D.projeter(Hinv, px)
    for _ in range(3):
        dans = (mm[:, 0] > -30) & (mm[:, 0] < COTE + 30) & (mm[:, 1] > -30) & (mm[:, 1] < COTE + 30)
        p, J = modele.jacobien(mm[dans])
        mm[dans] -= np.linalg.solve(J, (p - px[dans])[:, :, None])[:, :, 0]
    cx = cv2.resize(mm[:, 0].reshape(u.shape).astype(np.float32), None, fx=pas, fy=pas, interpolation=cv2.INTER_LINEAR)
    cy = cv2.resize(mm[:, 1].reshape(u.shape).astype(np.float32), None, fx=pas, fy=pas, interpolation=cv2.INTER_LINEAR)
    d = (pas - 1) // 2      # cv2.resize aligne les centres de pixels
    return cx[d:d + h, d:d + w], cy[d:d + h, d:d + w]


def main():
    sortie = sys.argv[1]
    binaire = sys.argv[2] if len(sys.argv) > 2 else None
    os.makedirs(sortie, exist_ok=True)
    actuelle = dessiner("actuelle")
    mire_b = dessiner("reperes")
    # Calque des repères seuls : blanc partout sauf dans les blocs de 20 mm.
    calque = np.full(mire_b["image"].shape[:2], 255, np.uint8)
    for coins in mire_b["reperes"].values():
        a = ((coins[0] - 2.0 + MARGE) * PXMM).astype(int)
        b = ((coins[2] + 2.0 + MARGE) * PXMM).astype(int)
        calque[a[1]:b[1], a[0]:b[0]] = cv2.cvtColor(mire_b["image"][a[1]:b[1], a[0]:b[0]], cv2.COLOR_BGR2GRAY)
    brutes = os.path.join(REF, "photos-brutes")
    bilan = {}
    for nom in sorted(os.listdir(brutes)):
        bgr = cv2.imread(os.path.join(brutes, nom), cv2.IMREAD_COLOR | cv2.IMREAD_IGNORE_ORIENTATION)
        res = D.amorce_actuelle(bgr, D.en_gris(bgr), actuelle["valides"])
        if not res["ok"]:
            continue
        modele = D.Modele(res["mm"], res["px"])
        tourne = D.ModeleTourne(modele, res["quarts"])
        # La carte inverse est calculée dans le repère détecté, puis tournée.
        cx, cy = carte_inverse(modele, bgr.shape[:2])
        q = (4 - res["quarts"]) % 4
        mm = D.ModeleTourne(None, q).tourner(np.stack([cx.ravel(), cy.ravel()], 1)).astype(np.float32)
        sx = ((mm[:, 0] + MARGE) * PXMM).reshape(cx.shape)
        sy = ((mm[:, 1] + MARGE) * PXMM).reshape(cx.shape)
        ech = D.echelle_locale(modele.H)
        doux = cv2.GaussianBlur(calque, (0, 0), 0.45 * PXMM / ech)
        encre = cv2.remap(doux, sx, sy, cv2.INTER_LINEAR, borderMode=cv2.BORDER_CONSTANT, borderValue=255).astype(np.float32) / 255.0
        encre = cv2.GaussianBlur(encre, (0, 0), 1.0)                 # flou mesuré sur les traits réels
        lin = (bgr.astype(np.float32) / 255.0) ** 2.2 * (0.06 + 0.94 * encre)[..., None]
        photo = (np.clip(lin, 0, 1) ** (1 / 2.2) * 255 + 0.5).astype(np.uint8)
        gris8 = cv2.cvtColor(photo, cv2.COLOR_BGR2GRAY)
        trouves = {"opencv": D.detecter_reperes(gris8, mire_b["reperes"])}
        if binaire:
            chemin = os.path.join(sortie, "tmp-reperes.png")
            cv2.imwrite(chemin, gris8)
            lu = json.loads(subprocess.run([binaire, chemin], capture_output=True, text=True, check=True).stdout)
            trouves["rust"] = {r["id"]: np.array(r["coins"]) for r in lu["reperes"]}
            os.remove(chemin)
        ligne = {}
        for outil, t in trouves.items():
            ecarts = []
            for ident, coins in t.items():
                attendu = tourne.predire(mire_b["reperes"][ident])
                d = np.linalg.norm(coins[:, None, :] - attendu[None, :, :], axis=2).min(1) / ech
                ecarts.extend(d.tolist())
            ligne[outil] = {"reperes_sur_16": len(t), "coin_mediane_mm": round(float(np.median(ecarts)), 3) if ecarts else None,
                            "coin_max_mm": round(float(np.max(ecarts)), 3) if ecarts else None}
        bilan[nom] = ligne
        print(nom, json.dumps(ligne), flush=True)
        if nom == "b-brut.jpg":
            vue = photo.copy()
            for coins in trouves["opencv"].values():
                cv2.polylines(vue, [coins.astype(np.int32)], True, (0, 255, 0), 5)
            cv2.imwrite(os.path.join(sortie, "reel-b-brut-reperes-incrustes.jpg"),
                        cv2.resize(vue, None, fx=0.4, fy=0.4, interpolation=cv2.INTER_AREA), [cv2.IMWRITE_JPEG_QUALITY, 80])
    with open(os.path.join(sortie, "reperes-sur-reel.json"), "w") as f:
        json.dump(bilan, f, indent=1)


if __name__ == "__main__":
    main()

"""Relit les 16 repères de la mire générée par le moteur (tâche 2.2).

Contrôle fait une fois, hors des tests automatiques : on rend le PDF de la
mire en image, on fabrique une photo simulée en biais et floue, et on y
cherche les repères avec OpenCV et, si on lui donne le binaire de l'étude 2.1,
avec la crate Rust `kornia-apriltag`. Les deux lecteurs sont indépendants du
moteur : s'ils relisent les 16 numéros à leur place, les codes recopiés dans
`crates/insitu-core/src/target/apriltag.rs` sont les bons.

    python3 etudes/2.2-mire/verifier_reperes.py <dossier de travail> [binaire essai-reperes]

Demande `pdftoppm` (poppler) et `opencv-contrib-python-headless`.
"""
import json
import os
import subprocess
import sys

import cv2
import numpy as np

FEUILLE = 297.0
MARGE = (FEUILLE - 280.0) / 2


def lancer(*args):
    return subprocess.run(args, capture_output=True, text=True, check=True).stdout


def main():
    dossier = sys.argv[1]
    binaire = sys.argv[2] if len(sys.argv) > 2 else None
    os.makedirs(dossier, exist_ok=True)
    pdf = os.path.join(dossier, "mire-carre.pdf")
    lancer("cargo", "run", "-q", "-p", "insitu-cli", "--", "target", "pdf", pdf, "--page", "carre")
    lancer("pdftoppm", "-r", "300", "-gray", "-png", "-singlefile", pdf, os.path.join(dossier, "rendu"))
    info = json.loads(lancer("cargo", "run", "-q", "-p", "insitu-cli", "--", "target", "info"))
    attendu = {m["id"]: np.array(m["corners"]) for m in info["markers"]}

    rendu = cv2.imread(os.path.join(dossier, "rendu.png"), cv2.IMREAD_GRAYSCALE)
    h, w = rendu.shape
    px_mm = w / FEUILLE
    # Photo simulée : prise en biais, flou de 2 px, réduite à environ 8 px/mm.
    biais = cv2.getPerspectiveTransform(
        np.float32([[0, 0], [w, 0], [w, h], [0, h]]),
        np.float32([[300, 200], [w - 100, 350], [w - 350, h - 150], [150, h - 300]]),
    )
    photo = cv2.warpPerspective(rendu, biais, (w, h), borderValue=255)
    photo = cv2.GaussianBlur(photo, (0, 0), 2.0)
    photo = cv2.resize(photo, (w * 2 // 3, h * 2 // 3), interpolation=cv2.INTER_AREA)
    cv2.imwrite(os.path.join(dossier, "photo-simulee.png"), photo)

    cas = [
        ("rendu 300 dpi", "rendu.png", np.eye(3), 1.0),
        ("photo simulée en biais et floue", "photo-simulee.png", biais, 2 / 3),
    ]
    detecteur = cv2.aruco.ArucoDetector(cv2.aruco.getPredefinedDictionary(cv2.aruco.DICT_APRILTAG_36h11))
    ok = True

    def ecart(coins_px, ident, matrice, echelle):
        p = (np.asarray(coins_px, np.float64) / echelle).reshape(-1, 1, 2)
        p = cv2.perspectiveTransform(p, np.linalg.inv(matrice)).reshape(4, 2)
        return float(np.abs(p / px_mm - MARGE - attendu[ident]).max())

    for nom, fichier, matrice, echelle in cas:
        chemin = os.path.join(dossier, fichier)
        image = cv2.imread(chemin, cv2.IMREAD_GRAYSCALE)
        coins, ids, _ = detecteur.detectMarkers(image)
        ids = [] if ids is None else ids.flatten().tolist()
        ecarts = [ecart(c.reshape(4, 2), i, matrice, echelle) for c, i in zip(coins, ids)]
        lus = sorted(ids)
        ok &= lus == list(range(16))
        print(f"{nom} — OpenCV : {len(lus)} repères {lus}, écart max d'un coin {max(ecarts):.2f} mm")
        if binaire:
            sortie = json.loads(lancer(binaire, chemin).splitlines()[0])
            reperes = sortie["reperes"]
            lus = sorted(r["id"] for r in reperes)
            erreurs = sum(r["hamming"] for r in reperes)
            ecarts = [ecart(r["coins"], r["id"], matrice, echelle) for r in reperes]
            ok &= lus == list(range(16))
            print(f"{nom} — kornia-apriltag : {len(lus)} repères {lus}, {erreurs} bit(s) corrigé(s), "
                  f"écart max d'un coin {max(ecarts):.2f} mm")
    print("Tous les repères relus." if ok else "ÉCHEC : repères manquants ou mal numérotés.")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()

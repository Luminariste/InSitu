"""Compare, sur des photos simulées de la mire à repères codés, le détecteur
AprilTag d'OpenCV et un détecteur en Rust pur (`essai-rust`, crate
kornia-apriltag).

    cargo build --release --manifest-path etudes/2.1-detection/essai-rust/Cargo.toml
    python3 etudes/2.1-detection/comparer_rust.py <dossier de travail> <binaire essai-reperes> [n par niveau]

Pour chaque niveau : nombre de repères trouvés (sur 16) et erreur des coins
en mm, par rapport à la vérité terrain du simulateur.
"""
import json
import os
import subprocess
import sys

import cv2
import numpy as np

import detecteurs as D
import simulateur as S


def main():
    travail, binaire = sys.argv[1], sys.argv[2]
    n = int(sys.argv[3]) if len(sys.argv) > 3 else 10
    os.makedirs(travail, exist_ok=True)
    m = S.mire("reperes")
    bilan = {}
    for niveau in S.NIVEAUX:
        compte = {"opencv": [], "rust": []}
        erreur = {"opencv": [], "rust": []}
        faux = {"opencv": 0, "rust": 0}
        duree = []
        for k in range(n):
            alea = np.random.default_rng(5000 + 100 * niveau + k)
            scene = S.Scene(S.tirer(alea, niveau))
            bgr = scene.rendre("reperes")
            chemin = os.path.join(travail, f"reperes-n{niveau}-{k}.png")
            cv2.imwrite(chemin, cv2.cvtColor(bgr, cv2.COLOR_BGR2GRAY))
            gris8 = cv2.cvtColor(bgr, cv2.COLOR_BGR2GRAY)
            trouves = {"opencv": D.detecter_reperes(gris8, m["reperes"])}
            sortie = json.loads(subprocess.run([binaire, chemin], capture_output=True, text=True, check=True).stdout)
            duree.append(sortie["ms"])
            trouves["rust"] = {r["id"]: np.array(r["coins"]) for r in sortie["reperes"]}
            for outil, t in trouves.items():
                bons = 0
                for ident, coins in t.items():
                    if ident not in m["reperes"]:
                        faux[outil] += 1
                        continue
                    retour, _ = scene.geo.px_vers_mm(coins)
                    # L'ordre des coins dépend de l'outil : on apparie au plus proche.
                    d = np.linalg.norm(retour[:, None, :] - m["reperes"][ident][None, :, :], axis=2).min(1)
                    if d.max() > 3.0:
                        faux[outil] += 1
                        continue
                    bons += 1
                    erreur[outil].extend(d.tolist())
                compte[outil].append(bons)
            os.remove(chemin)
        bilan[niveau] = {
            outil: {
                "reperes_moyens_sur_16": round(float(np.mean(compte[outil])), 1),
                "photos_avec_au_moins_un": int(np.sum(np.array(compte[outil]) > 0)),
                "photos": n,
                "coin_mediane_mm": round(float(np.median(erreur[outil])), 3) if erreur[outil] else None,
                "coin_p95_mm": round(float(np.quantile(erreur[outil], 0.95)), 3) if erreur[outil] else None,
                "faux": faux[outil],
            } for outil in ("opencv", "rust")
        }
        bilan[niveau]["rust"]["ms_median"] = float(np.median(duree))
        print(niveau, json.dumps(bilan[niveau]), flush=True)
    with open(os.path.join(travail, "comparaison-rust.json"), "w") as f:
        json.dump(bilan, f, indent=1)


if __name__ == "__main__":
    main()

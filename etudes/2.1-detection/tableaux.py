"""Résume le fichier produit par experience.py en tableaux Markdown.

    python3 etudes/2.1-detection/tableaux.py <fichier.jsonl> [resume.json]

Définitions :
- détectée : le détecteur a rendu un résultat ;
- juste    : détectée, et l'erreur au 95e centile dans la zone couverte par
             des points mesurés est inférieure ou égale à 0,5 mm ;
- médiocre : détectée, erreur entre 0,5 et 2 mm ;
- fausse   : détectée, erreur supérieure à 2 mm (erreur silencieuse grave :
             grille décalée d'une case, mire tournée d'un quart de tour) ;
- couverture : part de la mire située à moins de 8 mm d'un point mesuré.
"""
import json
import sys
from collections import defaultdict

import numpy as np

from mires import CANDIDATS

SEUIL = 0.5
ETIQUETTE = {"actuelle": "A", "epaisse": "A+", "reperes": "B", "charuco": "C"}


def med(v):
    v = [x for x in v if x is not None]
    return float(np.median(v)) if v else None


def fmt(x, d=2):
    return "—" if x is None else f"{x:.{d}f}".replace(".", ",")


def resumer(lignes):
    n = len(lignes)
    ok = [l for l in lignes if l["ok"]]
    justes = [l for l in ok if l["grille"].get("p95_couvert", 99) <= SEUIL]
    r = {
        "n": n,
        "detectees": len(ok),
        "justes": len(justes),
        "mediocres": sum(1 for l in ok if SEUIL < l["grille"].get("p95_couvert", 99) <= 2.0),
        "fausses": sum(1 for l in ok if l["grille"].get("p95_couvert", 99) > 2.0),
        "noeuds": med([l["noeuds"] for l in justes]),
        "couverture": med([l["grille"]["couverture"] for l in justes]),
        "grille_moy": med([l["grille"]["moy_couvert"] for l in justes]),
        "grille_p95": med([l["grille"]["p95_couvert"] for l in justes]),
        "grille_p95_pire": max([l["grille"]["p95_couvert"] for l in justes], default=None),
        "feuille_moy": med([l["grille"]["moy"] for l in justes]),
        "feuille_p95": med([l["grille"]["p95"] for l in justes]),
        "trapeze_moy": med([l["trapeze"]["moy"] for l in justes]),
        "trapeze_p95": med([l["trapeze"]["p95"] for l in justes]),
        "noeuds_faux": int(sum(l.get("noeuds_faux", 0) for l in ok)),
        "noeud_mediane": med([l.get("noeud_mediane") for l in justes]),
        "avec_appuis": sum(1 for l in lignes if l.get("appuis", 0) >= 4),
        "appuis": med([l.get("appuis") for l in lignes if l.get("appuis")]),
        "appuis_trapeze_moy": med([l["appuis_trapeze"]["moy"] for l in lignes if "appuis_trapeze" in l]),
        "appuis_trapeze_p95": med([l["appuis_trapeze"]["p95"] for l in lignes if "appuis_trapeze" in l]),
        "appuis_grille_moy": med([l["appuis_grille"]["moy"] for l in lignes if "appuis_grille" in l]),
        "appuis_grille_p95": med([l["appuis_grille"]["p95"] for l in lignes if "appuis_grille" in l]),
        "causes": {},
    }
    for l in lignes:
        if not l["ok"]:
            c = l["info"].get("cause", "?")
            c = "trop peu de nœuds" if "nœuds seulement" in c else c
            r["causes"][c] = r["causes"].get(c, 0) + 1
    return r


def main():
    par = defaultdict(list)
    conditions = []
    for ligne in open(sys.argv[1]):
        l = json.loads(ligne)
        par[(l["condition"], l["candidat"])].append(l)
        if l["condition"] not in conditions:
            conditions.append(l["condition"])
    resume = {c: {k: resumer(par[(c, k)]) for k in CANDIDATS} for c in conditions}

    niveaux = sorted(c for c in conditions if c.startswith("niveau"))
    print("### Par niveau de difficulté\n")
    print("| Niveau | Candidat | Détectées | Justes | Médiocres | Fausses | Points | Couverture | Erreur moy. | Erreur p95 | Pire p95 | Trapèze seul, moy. / p95 |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    for c in niveaux:
        for k in CANDIDATS:
            r = resume[c][k]
            print(f"| {c[-1]} | {ETIQUETTE[k]} | {r['detectees']}/{r['n']} | {r['justes']}/{r['n']} | {r['mediocres']} | {r['fausses']} | {fmt(r['noeuds'], 0)} | "
                  f"{fmt(None if r['couverture'] is None else 100 * r['couverture'], 0)} % | {fmt(r['grille_moy'])} | {fmt(r['grille_p95'])} | "
                  f"{fmt(r['grille_p95_pire'])} | {fmt(r['trapeze_moy'])} / {fmt(r['trapeze_p95'])} |")

    print("\n### Repères codés ou points de selle seuls (sans la grille)\n")
    print("| Niveau | Candidat | Photos avec repères | Points d'appui | Trapèze, moy. / p95 | Correction lissée, moy. / p95 |")
    print("| --- | --- | --- | --- | --- | --- |")
    for c in niveaux:
        for k in ("reperes", "charuco"):
            r = resume[c][k]
            print(f"| {c[-1]} | {ETIQUETTE[k]} | {r['avec_appuis']}/{r['n']} | {fmt(r['appuis'], 0)} | {fmt(r['appuis_trapeze_moy'])} / {fmt(r['appuis_trapeze_p95'])} | "
                  f"{fmt(r['appuis_grille_moy'])} / {fmt(r['appuis_grille_p95'])} |")

    print("\n### Un seul défaut à la fois (base : niveau 1)\n")
    print("Chaque case : photos justes / photos, puis erreur p95 médiane en mm.\n")
    print("| Défaut | Valeur | A | A+ | B | C |")
    print("| --- | --- | --- | --- | --- | --- |")
    for c in conditions:
        if c.startswith("niveau"):
            continue
        facteur, valeur = c.split("=")
        cases = []
        for k in CANDIDATS:
            r = resume[c][k]
            cases.append(f"{r['justes']}/{r['n']} · {fmt(r['grille_p95'])}" + (f" · {r['mediocres']} médiocre(s)" if r["mediocres"] else "")
                         + (f" · {r['fausses']} fausse(s)" if r["fausses"] else ""))
        print(f"| {facteur} | {valeur.replace('.', ',')} | " + " | ".join(cases) + " |")

    print("\n### Causes d'échec (toutes conditions)\n")
    for k in CANDIDATS:
        total = defaultdict(int)
        for c in conditions:
            for cause, nb in resume[c][k]["causes"].items():
                total[cause] += nb
        print(f"- {ETIQUETTE[k]} : " + ", ".join(f"{cause} ({nb})" for cause, nb in sorted(total.items(), key=lambda x: -x[1])))
    faux = {ETIQUETTE[k]: sum(resume[c][k]["noeuds_faux"] for c in conditions) for k in CANDIDATS}
    print(f"\nPoints mesurés à plus de 0,5 mm de leur vraie place, toutes photos détectées : {faux}")

    if len(sys.argv) > 2:
        with open(sys.argv[2], "w") as f:
            json.dump(resume, f, ensure_ascii=False, indent=1)


if __name__ == "__main__":
    main()

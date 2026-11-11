"""Dessin des mires candidates de l'étude 2.1.

Une mire est décrite en millimètres dans le repère de la diapositive
(0 à 280 mm, origine en haut à gauche, y vers le bas) et dessinée sur une
feuille de 297 mm (A3 recoupé), ce qui laisse 8,5 mm de marge blanche,
comme sur les photos d'Épinal.

    python3 etudes/2.1-detection/mires.py <dossier de sortie>

écrit un aperçu PNG de chaque candidat.
"""
import os
import sys

import cv2
import numpy as np

COTE = 280.0        # côté de la mire, mm
FEUILLE = 297.0     # côté de la feuille, mm
MARGE = (FEUILLE - COTE) / 2
PAS = 10.0          # pas de la grille, mm
N = 29              # nombre de lignes par direction
PXMM = 16           # résolution de dessin, px par mm
CENTRE = COTE / 2

# Blocs de 20 mm (2 × 2 cases) recevant un repère codé de 16 mm : coin haut
# gauche du bloc, en mm. 12 en couronne, 4 à l'intérieur, hors croix et cercle.
POSITIONS_BLOCS = [10, 70, 190, 250]
TAILLE_BLOC = 20.0
TAILLE_REPERE = 16.0
DICO_REPERES = cv2.aruco.DICT_APRILTAG_36h11

# Candidat C : damier ChArUco de 28 × 28 cases de 10 mm, dont on ne garde que
# la couronne extérieure, large de 2 cases.
DICO_CHARUCO = cv2.aruco.DICT_4X4_1000
LARGEUR_COURONNE = 2


def planche_charuco():
    dico = cv2.aruco.getPredefinedDictionary(DICO_CHARUCO)
    return cv2.aruco.CharucoBoard((28, 28), 10.0, 7.0, dico)


def _px(mm):
    """Millimètres de mire vers pixels de la feuille dessinée."""
    return (np.asarray(mm, dtype=np.float64) + MARGE) * PXMM


def _poly(img, pts_mm, couleur):
    pts = np.round(_px(pts_mm) * 16).astype(np.int32)
    cv2.fillPoly(img, [pts], couleur, lineType=cv2.LINE_AA, shift=4)


def _segment(img, a, b, largeur, couleur=(0, 0, 0)):
    a = np.asarray(a, float)
    b = np.asarray(b, float)
    d = b - a
    n = np.array([-d[1], d[0]]) / np.hypot(*d) * largeur / 2
    _poly(img, [a + n, b + n, b - n, a - n], couleur)


def _cercle(img, centre, rayon, largeur, couleur=(0, 0, 0)):
    t = np.linspace(0, 2 * np.pi, 721)
    ext = np.stack([centre[0] + (rayon + largeur / 2) * np.cos(t), centre[1] + (rayon + largeur / 2) * np.sin(t)], 1)
    inte = np.stack([centre[0] + (rayon - largeur / 2) * np.cos(t), centre[1] + (rayon - largeur / 2) * np.sin(t)], 1)
    pts = [np.round(_px(p) * 16).astype(np.int32) for p in (ext, inte[::-1])]
    cv2.fillPoly(img, [np.concatenate(pts)], couleur, lineType=cv2.LINE_AA, shift=4)


ROUGE = (40, 30, 230)  # BGR : réflectance de l'encre rouge


def dessiner(nom):
    """Renvoie la description d'un candidat.

    `image` : réflectance BGR 8 bits de la feuille, à PXMM px/mm.
    `valides` : nœuds (j, i) de la grille où deux traits fins se croisent.
    `reperes` : {identifiant: 4 coins en mm} pour les repères codés.
    `selles` : {identifiant ChArUco: (x, y) en mm} pour les points de selle.
    """
    trait = {"actuelle": 0.2}.get(nom, 0.35)
    avec_reperes = nom == "reperes"
    avec_couronne = nom == "charuco"
    petits_cercles = nom in ("actuelle", "epaisse")

    cote_px = int(round(FEUILLE * PXMM))
    img = np.full((cote_px, cote_px, 3), 255, np.uint8)
    valides = np.ones((N, N), bool)

    # Grille fine.
    for k in range(N):
        _segment(img, (k * PAS, 0), (k * PAS, COTE), trait)
        _segment(img, (0, k * PAS), (COTE, k * PAS), trait)
    # Croix centrale épaisse et cercle rouge.
    _segment(img, (CENTRE, 0), (CENTRE, COTE), 0.76)
    _segment(img, (0, CENTRE), (COTE, CENTRE), 0.76)
    _cercle(img, (CENTRE, CENTRE), 60.0, 0.9, ROUGE)
    # Disque central blanc, son cercle et ses quatre tirets.
    cv2.circle(img, tuple(np.round(_px((CENTRE, CENTRE)) * 16).astype(int)), int(30 * PXMM * 16), (255, 255, 255), -1, cv2.LINE_AA, 4)
    _cercle(img, (CENTRE, CENTRE), 30.0, max(trait, 0.15))
    for sx in (-1, 1):
        for sy in (-1, 1):
            u = np.array([sx, sy]) / np.sqrt(2)
            _segment(img, CENTRE + 14.4 * u, CENTRE + 30 * u, max(trait, 0.15))
    if petits_cercles:
        for k in range(6):
            _cercle(img, (15 + 20 * k, 265 - 20 * k), 5.0, max(trait, 0.12))
        cv2.putText(img, "Mire 280 x 280 mm", tuple(_px((2, 278.5)).astype(int)), cv2.FONT_HERSHEY_SIMPLEX, 0.11 * PXMM, (0, 0, 0), max(1, PXMM // 8), cv2.LINE_AA)

    jj, ii = np.mgrid[0:N, 0:N]
    rayon = np.hypot(ii * PAS - CENTRE, jj * PAS - CENTRE)
    valides &= rayon > 34.0

    reperes = {}
    if avec_reperes:
        dico = cv2.aruco.getPredefinedDictionary(DICO_REPERES)
        ident = 0
        for y0 in POSITIONS_BLOCS:
            for x0 in POSITIONS_BLOCS:
                a = _px((x0, y0)).astype(int)
                b = _px((x0 + TAILLE_BLOC, y0 + TAILLE_BLOC)).astype(int)
                e = int(trait * PXMM)  # on garde le trait du pourtour du bloc
                img[a[1] + e:b[1] - e, a[0] + e:b[0] - e] = 255
                cote = int(TAILLE_REPERE * PXMM)
                motif = cv2.aruco.generateImageMarker(dico, ident, cote)
                d = int((TAILLE_BLOC - TAILLE_REPERE) / 2 * PXMM)
                img[a[1] + d:a[1] + d + cote, a[0] + d:a[0] + d + cote] = motif[:, :, None]
                m = (TAILLE_BLOC - TAILLE_REPERE) / 2
                reperes[ident] = np.array([[x0 + m, y0 + m], [x0 + TAILLE_BLOC - m, y0 + m],
                                           [x0 + TAILLE_BLOC - m, y0 + TAILLE_BLOC - m], [x0 + m, y0 + TAILLE_BLOC - m]])
                # Les 9 nœuds du bloc sont écartés : celui du centre a disparu,
                # ceux du pourtour sont trop près du repère pour être mesurés.
                j0, i0 = int(y0 / PAS), int(x0 / PAS)
                valides[j0:j0 + 3, i0:i0 + 3] = False
                ident += 1

    selles = {}
    if avec_couronne:
        planche = planche_charuco()
        damier = planche.generateImage((int(COTE * PXMM), int(COTE * PXMM)))
        c = LARGEUR_COURONNE * int(PAS * PXMM)
        masque = np.ones(damier.shape, bool)
        masque[c:-c, c:-c] = False
        o = int(round(MARGE * PXMM))
        zone = img[o:o + damier.shape[0], o:o + damier.shape[1]]
        zone[masque] = damier[masque][:, None]
        coins = planche.getChessboardCorners()[:, :2]
        for ident, (x, y) in enumerate(coins):
            # Vrais points de selle : sur la ligne médiane de la couronne.
            sur_x = x in (10.0, 270.0) and 10.0 <= y <= 270.0
            sur_y = y in (10.0, 270.0) and 10.0 <= x <= 270.0
            if sur_x or sur_y:
                selles[ident] = (float(x), float(y))
        valides[:LARGEUR_COURONNE + 1, :] = False
        valides[-LARGEUR_COURONNE - 1:, :] = False
        valides[:, :LARGEUR_COURONNE + 1] = False
        valides[:, -LARGEUR_COURONNE - 1:] = False

    return {"nom": nom, "image": img, "valides": valides, "reperes": reperes, "selles": selles, "trait": trait}


CANDIDATS = {
    "actuelle": "A — mire actuelle (traits de 0,2 mm de largeur apparente, aucun repère codé)",
    "epaisse": "A+ — mire actuelle, traits de 0,35 mm",
    "reperes": "B — grille 0,35 mm + 16 repères codés AprilTag de 16 mm",
    "charuco": "C — grille 0,35 mm + couronne ChArUco de 20 mm",
}

if __name__ == "__main__":
    sortie = sys.argv[1]
    os.makedirs(sortie, exist_ok=True)
    for nom in CANDIDATS:
        petit = cv2.resize(dessiner(nom)["image"], (1188, 1188), interpolation=cv2.INTER_AREA)
        cv2.imwrite(os.path.join(sortie, f"mire-{nom}.png"), petit)

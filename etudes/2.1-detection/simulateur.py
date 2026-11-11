"""Simulateur de photos de mire en chambre noire (étude 2.1).

Une photo simulée = une mire candidate (encre sur papier) éclairée par une
image de façade, vue par un téléphone tenu à la main : biais, papier qui
gondole, distorsion d'objectif, flou de mise au point et de bougé, bruit,
vignettage, zones sombres de la façade, tissu ou main devant l'objectif.

La vérité terrain (mm de mire -> px de photo) est connue exactement.

L'image de façade vient des photos directes du portail (`x-mur`), pas des
photos redressées : ces dernières contiennent déjà l'ancienne grille.
"""
import os

import cv2
import numpy as np

from mires import CENTRE, COTE, FEUILLE, MARGE, PXMM, dessiner

ICI = os.path.dirname(os.path.abspath(__file__))
REDUITS = os.path.join(ICI, "..", "..", "jeu-essai", "epinal", "reduits", "photos-brutes")
FACADES = ["a-mur.jpg", "b-mur.jpg", "c-mur.jpg", "d-mur.jpg"]

TAILLE = 2600            # côté de la photo simulée, px
ECHELLE = 6.7            # px par mm au centre, mesuré sur les photos d'Épinal
FOCALE = 2688.0          # px : objectif 24 mm (équivalent) sur 4 032 px de large

# Plages de tirage par niveau de difficulté. Le niveau 1 est réglé pour
# ressembler aux photos d'Épinal.
NIVEAUX = {
    1: dict(biais=(3, 12), gondole=(0.6, 1.5), flou=(0.8, 1.1), bouge=(0, 1.5), bruit=(1.0, 2.0), sombre=(0, 0), cache=(0, 0)),
    2: dict(biais=(10, 20), gondole=(1.5, 2.5), flou=(1.2, 1.6), bouge=(2, 4), bruit=(3, 5), sombre=(0.10, 0.20), cache=(0.03, 0.08)),
    3: dict(biais=(18, 28), gondole=(2.5, 3.5), flou=(1.6, 2.2), bouge=(4, 7), bruit=(5, 8), sombre=(0.20, 0.30), cache=(0.08, 0.15)),
    4: dict(biais=(28, 38), gondole=(3.5, 4.5), flou=(2.2, 3.0), bouge=(7, 11), bruit=(8, 12), sombre=(0.35, 0.45), cache=(0.15, 0.25)),
}

_cache_mires = {}
_cache_facades = {}


def mire(nom):
    """Mire dessinée, légèrement adoucie pour être rééchantillonnée sans moiré."""
    if nom not in _cache_mires:
        m = dessiner(nom)
        m["douce"] = cv2.GaussianBlur(m["image"], (0, 0), 0.45 * PXMM / ECHELLE)
        _cache_mires[nom] = m
    return _cache_mires[nom]


def facade(k):
    if k not in _cache_facades:
        _cache_facades[k] = cv2.imread(os.path.join(REDUITS, FACADES[k]))
    return _cache_facades[k]


def tirer(alea, niveau, **forces):
    """Tire les paramètres d'une scène. `forces` impose certaines valeurs."""
    plages = NIVEAUX[niveau]
    p = {k: float(alea.uniform(*v)) for k, v in plages.items()}
    p.update(forces)
    p.update(
        azimut=float(alea.uniform(0, 360)),                    # direction du biais
        rotation=float(alea.integers(0, 4) * 90 + alea.uniform(-8, 8)),
        decentrage=alea.uniform(-60, 60, 2).tolist(),
        k1=float(alea.uniform(-0.012, 0.012)),
        ondes=[dict(lam=float(alea.uniform(90, 300)), cap=float(alea.uniform(0, np.pi)), phase=float(alea.uniform(0, 2 * np.pi)),
                    dir=float(alea.uniform(0, 2 * np.pi)), poids=float(alea.uniform(0.4, 1.0))) for _ in range(3)],
        facade=int(alea.integers(0, len(FACADES))),
        facade_echelle=float(alea.uniform(0.45, 0.95)),
        facade_rotation=float(alea.integers(0, 4) * 90 + alea.uniform(-15, 15)),
        facade_centre=alea.uniform(0.3, 0.7, 2).tolist(),
        facade_miroir=bool(alea.integers(0, 2)),
        voile=float(alea.uniform(0.30, 0.45)),
        exposition=float(alea.uniform(1.0, 1.35)),
        vignette=float(alea.uniform(0.0, 0.35)),
        bouge_angle=float(alea.uniform(0, np.pi)),
        sombre_centres=alea.uniform(0, COTE, (3, 2)).tolist(),
        cache_angle=float(alea.uniform(0, 2 * np.pi)),
        graine_bruit=int(alea.integers(0, 2 ** 31)),
    )
    return p


class Geometrie:
    """Correspondance px de photo -> mm de mire, et son inverse (vérité)."""

    def __init__(self, p):
        self.p = p
        a, az, rot = np.deg2rad(p["biais"]), np.deg2rad(p["azimut"]), np.deg2rad(p["rotation"])
        axe = np.array([np.cos(az), np.sin(az), 0.0])
        K = np.array([[0, -axe[2], axe[1]], [axe[2], 0, -axe[0]], [-axe[1], axe[0], 0]])
        Rb = np.eye(3) + np.sin(a) * K + (1 - np.cos(a)) * K @ K
        Rz = np.array([[np.cos(rot), -np.sin(rot), 0], [np.sin(rot), np.cos(rot), 0], [0, 0, 1]])
        R = Rb @ Rz
        z = FOCALE / ECHELLE
        t = np.array([p["decentrage"][0] / FOCALE * z, p["decentrage"][1] / FOCALE * z, z])
        # mm centrés -> coordonnées normalisées de l'objectif
        Hc = np.stack([R[:, 0], R[:, 1], t], 1)
        self.Hinv = np.linalg.inv(Hc)
        self.Hc = Hc
        amp = 0.0
        # Normalisation de l'ondulation : amplitude maximale = p["gondole"] mm.
        g = np.mgrid[0:COTE + 1:10, 0:COTE + 1:10].reshape(2, -1).T.astype(float)
        self._ech_onde = 1.0
        d = self._onde(g)
        self._ech_onde = p["gondole"] / (np.linalg.norm(d, axis=1).max() + 1e-9)

    def _onde(self, plat):
        d = np.zeros_like(plat)
        for o in self.p["ondes"]:
            arg = 2 * np.pi * (plat[:, 0] * np.cos(o["cap"]) + plat[:, 1] * np.sin(o["cap"])) / o["lam"] + o["phase"]
            s = o["poids"] * np.sin(arg)
            d[:, 0] += s * np.cos(o["dir"])
            d[:, 1] += s * np.sin(o["dir"])
        return d * self._ech_onde

    def px_vers_plat(self, px):
        """px -> position sur le plan de la vitre (mm), là où tombe la lumière."""
        xn = (px[:, 0] - TAILLE / 2) / FOCALE
        yn = (px[:, 1] - TAILLE / 2) / FOCALE
        f = 1 + self.p["k1"] * (xn * xn + yn * yn) * 4
        q = np.stack([xn * f, yn * f, np.ones_like(xn)], 1) @ self.Hinv.T
        return q[:, :2] / q[:, 2:3] + CENTRE

    def px_vers_mm(self, px):
        """px -> point de la feuille (mm de mire) : le papier a glissé."""
        plat = self.px_vers_plat(px)
        return plat + self._onde(plat), plat

    def mm_vers_px(self, mm):
        """Vérité terrain : inversion numérique de px_vers_mm."""
        mm = np.asarray(mm, np.float64)
        q = np.c_[mm - CENTRE, np.ones(len(mm))] @ self.Hc.T
        px = q[:, :2] / q[:, 2:3] * FOCALE + TAILLE / 2
        for _ in range(8):
            f0, _ = self.px_vers_mm(px)
            fx, _ = self.px_vers_mm(px + [0.5, 0])
            fy, _ = self.px_vers_mm(px + [0, 0.5])
            J = np.stack([(fx - f0) / 0.5, (fy - f0) / 0.5], 2)
            px = px - np.linalg.solve(J, (f0 - mm)[:, :, None])[:, :, 0]
        return px


def _cartes(geo):
    """Cartes px -> mm (feuille et plan), calculées sur une grille lâche puis
    agrandies : les fonctions sont très lisses."""
    pas = 4
    n = TAILLE // pas + 1
    v, u = np.mgrid[0:n, 0:n].astype(np.float64) * pas
    mm, plat = geo.px_vers_mm(np.stack([u.ravel(), v.ravel()], 1))
    sortie = []
    for c in (mm, plat):
        cx = cv2.resize(c[:, 0].reshape(n, n).astype(np.float32), (n * pas, n * pas), interpolation=cv2.INTER_LINEAR)
        cy = cv2.resize(c[:, 1].reshape(n, n).astype(np.float32), (n * pas, n * pas), interpolation=cv2.INTER_LINEAR)
        # cv2.resize aligne les centres de pixels : on corrige le demi-pas.
        M = np.float32([[1, 0, (pas - 1) / 2], [0, 1, (pas - 1) / 2]])
        cx = cv2.warpAffine(cx, M, (TAILLE, TAILLE), flags=cv2.INTER_LINEAR | cv2.WARP_INVERSE_MAP, borderMode=cv2.BORDER_REPLICATE)
        cy = cv2.warpAffine(cy, M, (TAILLE, TAILLE), flags=cv2.INTER_LINEAR | cv2.WARP_INVERSE_MAP, borderMode=cv2.BORDER_REPLICATE)
        sortie.append((cx, cy))
    return sortie


class Scene:
    """Tout ce qui ne dépend pas du dessin de la mire : géométrie, lumière."""

    def __init__(self, p):
        self.p = p
        self.geo = Geometrie(p)
        (self.mx, self.my), (px_, py_) = _cartes(self.geo)
        # Lumière de la façade sur le plan de la vitre.
        img = facade(p["facade"])
        if p["facade_miroir"]:
            img = img[:, ::-1]
        h, w = img.shape[:2]
        s = p["facade_echelle"] * min(h, w) / COTE
        a = np.deg2rad(p["facade_rotation"])
        dx, dy = px_ - CENTRE, py_ - CENTRE
        fx = (p["facade_centre"][0] * w + s * (np.cos(a) * dx - np.sin(a) * dy)).astype(np.float32)
        fy = (p["facade_centre"][1] * h + s * (np.sin(a) * dx + np.cos(a) * dy)).astype(np.float32)
        lum = cv2.remap(np.ascontiguousarray(img), fx, fy, cv2.INTER_LINEAR, borderMode=cv2.BORDER_REFLECT).astype(np.float32) / 255.0
        lum = (p["voile"] + (1 - p["voile"]) * lum) ** 2.2 * p["exposition"]
        r2 = (dx * dx + dy * dy) / 200.0 ** 2
        lum *= np.clip(1 - p["vignette"] * r2, 0.05, 1)[..., None]
        # Zones sombres de la façade (nuit, parties non éclairées).
        if p["sombre"] > 0:
            champ = np.zeros_like(dx)
            for cx, cy in p["sombre_centres"]:
                champ += np.exp(-((px_ - cx) ** 2 + (py_ - cy) ** 2) / (2 * 70.0 ** 2))
            ech = champ[::8, ::8][(np.abs(dx[::8, ::8]) < CENTRE) & (np.abs(dy[::8, ::8]) < CENTRE)]
            seuil = np.quantile(ech, 1 - p["sombre"])
            reste = 0.003 if p.get("noir") else 0.05      # lumière qui reste dans la zone sombre
            lum *= (1 - (1 - reste) / (1 + np.exp(-(champ - seuil) / 0.03)))[..., None]
        # Hors de la feuille : plateau du projecteur, puis tissu.
        hors = np.maximum(np.abs(dx), np.abs(dy))
        self.plateau = (hors > FEUILLE / 2)
        lum[hors > 185] *= 0.1
        self.lum = lum
        # Tissu ou main flous devant l'objectif (repère de la photo).
        self.masque = None
        if p["cache"] > 0:
            v, u = np.mgrid[0:TAILLE, 0:TAILLE].astype(np.float32)
            n = np.array([np.cos(p["cache_angle"]), np.sin(p["cache_angle"])], np.float32)
            proj = (u - TAILLE / 2) * n[0] + (v - TAILLE / 2) * n[1]
            dans = (np.abs(dx) < CENTRE) & (np.abs(dy) < CENTRE)
            seuil = np.quantile(proj[dans][::16], 1 - p["cache"])
            reste = 0.003 if p.get("noir") else 0.07
            self.masque = 1 - (1 - reste) / (1 + np.exp(-(proj - seuil) / 45.0))

    def rendre(self, nom_mire):
        p = self.p
        m = mire(nom_mire)
        sx = (self.mx + MARGE) * PXMM
        sy = (self.my + MARGE) * PXMM
        encre = cv2.remap(m["douce"], sx, sy, cv2.INTER_LINEAR, borderMode=cv2.BORDER_CONSTANT, borderValue=(38, 38, 38)).astype(np.float32) / 255.0
        img = self.lum * (0.06 + 0.94 * encre)
        if self.masque is not None:
            img = img * self.masque[..., None] + 0.004
        if p["flou"] > 0:
            img = cv2.GaussianBlur(img, (0, 0), p["flou"])
        if p["bouge"] > 0.5:
            L = int(np.ceil(p["bouge"])) | 1
            noyau = np.zeros((L + 2, L + 2), np.float32)
            c = (L + 1) / 2
            d = p["bouge"] / 2
            a = p["bouge_angle"]
            cv2.line(noyau, (int(round((c - d * np.cos(a)) * 16)), int(round((c - d * np.sin(a)) * 16))),
                     (int(round((c + d * np.cos(a)) * 16)), int(round((c + d * np.sin(a)) * 16))), 1.0, 1, cv2.LINE_AA, 4)
            img = cv2.filter2D(img, -1, noyau / noyau.sum())
        alea = np.random.default_rng(p["graine_bruit"])
        s = p["bruit"] / 100.0
        # Bruit de photons (dépend du signal) + bruit de lecture, réglé pour
        # valoir `bruit` % au gris moyen.
        ecart = s * np.sqrt(np.clip(img, 0, None) / 0.2 + 0.01)
        img = img + alea.standard_normal(img.shape, np.float32) * ecart
        img8 = (np.clip(img, 0, 1) ** (1 / 2.2) * 255 + 0.5).astype(np.uint8)
        _, tampon = cv2.imencode(".jpg", img8, [cv2.IMWRITE_JPEG_QUALITY, 90])
        return cv2.imdecode(tampon, cv2.IMREAD_COLOR)


if __name__ == "__main__":
    import sys
    sortie = sys.argv[1]
    os.makedirs(sortie, exist_ok=True)
    for niveau in (1, 2, 3, 4):
        alea = np.random.default_rng(100 + niveau)
        sc = Scene(tirer(alea, niveau))
        for nom in ("actuelle", "reperes", "charuco"):
            cv2.imwrite(os.path.join(sortie, f"simu-n{niveau}-{nom}.jpg"), sc.rendre(nom), [cv2.IMWRITE_JPEG_QUALITY, 88])

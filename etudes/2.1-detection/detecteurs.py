"""Détecteurs de l'étude 2.1 (prototypes Python, OpenCV + NumPy).

Trois façons de trouver la mire dans la photo (« amorce »), puis une étape
commune : la mesure des croisements de la grille, de proche en proche.

- amorce_actuelle : cercle rouge + croix épaisse (mire actuelle, sans code) ;
- amorce_reperes  : repères codés AprilTag ;
- amorce_charuco  : couronne ChArUco (points de selle).

Tout est exprimé comme une correspondance mm de mire -> px de photo.
"""
import cv2
import numpy as np

from mires import CENTRE, DICO_REPERES, N, PAS, planche_charuco

# ---------------------------------------------------------------- géométrie


def projeter(H, mm):
    mm = np.asarray(mm, np.float64)
    q = np.c_[mm, np.ones(len(mm))] @ H.T
    return q[:, :2] / q[:, 2:3]


def homographie(mm, px, robuste_px=None):
    mm = np.asarray(mm, np.float64)
    px = np.asarray(px, np.float64)
    if robuste_px is not None and len(mm) >= 8:
        H, _ = cv2.findHomography(mm, px, cv2.RANSAC, robuste_px)
        if H is not None:
            ecart = np.linalg.norm(projeter(H, mm) - px, axis=1)
            bons = ecart < robuste_px
            if bons.sum() >= 4:
                H2, _ = cv2.findHomography(mm[bons], px[bons], 0)
                return H2 if H2 is not None else H
    H, _ = cv2.findHomography(mm, px, 0)
    if H is None:
        raise ValueError("points alignés ou confondus : pas de trapèze")
    return H


class Modele:
    """Correspondance mm -> px : trapèze + correction locale lissée.

    La correction est une régression affine locale des écarts au trapèze
    (poids gaussiens de largeur `sigma` mm). C'est le modèle « grille » de
    l'étude ; avec `lisse=False` il ne reste que le trapèze (4 coins).
    """

    def __init__(self, mm, px, sigma=12.0, lisse=True, robuste_px=None, H=None):
        self.mm = np.asarray(mm, np.float64).reshape(-1, 2)
        self.px = np.asarray(px, np.float64).reshape(-1, 2)
        self.sigma = sigma
        self.H = H if H is not None else homographie(self.mm, self.px, robuste_px)
        self.lisse = lisse and len(self.mm) >= 6
        if len(self.mm):
            self.res = self.px - projeter(self.H, self.mm)

    def predire(self, X):
        X = np.asarray(X, np.float64).reshape(-1, 2)
        base = projeter(self.H, X)
        if not self.lisse:
            return base
        sortie = np.empty_like(base)
        for d in range(0, len(X), 256):
            x = X[d:d + 256]
            dx = self.mm[None, :, 0] - x[:, None, 0]
            dy = self.mm[None, :, 1] - x[:, None, 1]
            w = np.exp(-(dx * dx + dy * dy) / (2 * self.sigma ** 2)) + 1e-5
            phi = np.stack([np.ones_like(dx), dx, dy], 2)            # m, n, 3
            A = np.einsum("mn,mni,mnj->mij", w, phi, phi)
            frein = 0.05 * w.sum(1) * self.sigma ** 2
            A[:, 1, 1] += frein
            A[:, 2, 2] += frein
            b = np.einsum("mn,mni,nk->mik", w, phi, self.res)
            sortie[d:d + 256] = base[d:d + 256] + np.linalg.solve(A, b)[:, 0, :]
        return sortie

    def jacobien(self, X, h=0.5):
        X = np.asarray(X, np.float64).reshape(-1, 2)
        n = len(X)
        tous = self.predire(np.r_[X, X + [h, 0], X + [0, h]])
        p = tous[:n]
        J = np.stack([(tous[n:2 * n] - p) / h, (tous[2 * n:] - p) / h], 2)   # n, 2, 2
        return p, J


class ModeleTourne:
    """Modèle composé avec un quart de tour de la mire autour de son centre."""

    def __init__(self, modele, quarts):
        self.modele = modele
        self.quarts = quarts % 4

    def tourner(self, X):
        X = np.asarray(X, np.float64).reshape(-1, 2) - CENTRE
        for _ in range(self.quarts):
            X = np.stack([-X[:, 1], X[:, 0]], 1)
        return X + CENTRE

    def predire(self, X):
        return self.modele.predire(self.tourner(X))


# ------------------------------------------------ mesure des croisements

DEMI = 4.0     # demi-fenêtre de recherche, mm
ECH = 0.1      # pas d'échantillonnage, mm
_OFFS = np.arange(-DEMI, DEMI + 1e-9, ECH)
ECHELLES = (1.5, 3.0, 6.0)   # largeurs de filtre, en échantillons


def _noyau(sigma):
    r = int(3 * sigma * 3) + 1
    x = np.arange(-r, r + 1)
    g1 = np.exp(-x * x / (2 * sigma ** 2))
    g3 = np.exp(-x * x / (2 * (3 * sigma) ** 2))
    return g3 / g3.sum() - g1 / g1.sum()


_NOYAUX = [_noyau(s) for s in ECHELLES]


def _trait_dans_profil(prof, seuil_snr, seuil_unicite):
    """Cherche un trait sombre dans des profils (n, m). Renvoie décalage en mm,
    rapport signal/bruit, validité."""
    n, m = prof.shape
    q = prof / (prof.mean(1, keepdims=True) + 1e-6)
    meilleur = np.zeros(n)
    snr = np.zeros(n)
    ok = np.zeros(n, bool)
    for noyau in _NOYAUX:
        r = len(noyau) // 2
        qq = np.pad(q, ((0, 0), (r, r)), mode="reflect")
        rep = np.stack([np.convolve(l, noyau, mode="valid") for l in qq])
        bord = 5
        coeur = rep[:, bord:m - bord]
        k = coeur.argmax(1) + bord
        lignes = np.arange(n)
        pic = rep[lignes, k]
        bruit = 1.4826 * np.median(np.abs(rep - np.median(rep, 1, keepdims=True)), 1) + 1e-4
        # Deuxième pic, à plus de 1,2 mm du premier.
        masque = np.abs(np.arange(m)[None, :] - k[:, None]) > 12
        masque[:, :bord] = False
        masque[:, m - bord:] = False
        second = np.where(masque, rep, -1).max(1)
        a = rep[lignes, k - 1]
        c = rep[lignes, np.minimum(k + 1, m - 1)]
        denom = a - 2 * pic + c
        fin = np.where(np.abs(denom) > 1e-9, 0.5 * (a - c) / denom, 0.0)
        s = pic / bruit
        valide = (s > seuil_snr) & (second < seuil_unicite * pic) & (pic > 0.004)
        mieux = valide & (s > snr)
        meilleur[mieux] = (_OFFS[0] + (k + np.clip(fin, -1, 1)) * ECH)[mieux]
        snr[mieux] = s[mieux]
        ok |= valide
    return meilleur, snr, ok


def mesurer_noeuds(gris, centres, jac, seuil_snr=5.0, seuil_unicite=0.7):
    """Mesure le croisement de deux traits près de chaque centre prédit.

    `centres` : (n, 2) px ; `jac` : (n, 2, 2) px par mm. Renvoie la position
    mesurée en px, un score, la validité.
    """
    n = len(centres)
    m = len(_OFFS)
    h, w = gris.shape
    dx, dy = np.meshgrid(_OFFS, _OFFS)
    pos = np.zeros((n, 2))
    score = np.zeros(n)
    ok = np.zeros(n, bool)
    for d in range(0, n, 300):
        c = centres[d:d + 300]
        J = jac[d:d + 300]
        X = c[:, 0, None, None] + J[:, 0, 0, None, None] * dx + J[:, 0, 1, None, None] * dy
        Y = c[:, 1, None, None] + J[:, 1, 0, None, None] * dx + J[:, 1, 1, None, None] * dy
        k = len(c)
        pat = cv2.remap(gris, X.reshape(k * m, m).astype(np.float32), Y.reshape(k * m, m).astype(np.float32),
                        cv2.INTER_LINEAR, borderMode=cv2.BORDER_REPLICATE).reshape(k, m, m)
        oy, sy, vy = _trait_dans_profil(pat.mean(2), seuil_snr, seuil_unicite)   # trait horizontal
        ox, sx, vx = _trait_dans_profil(pat.mean(1), seuil_snr, seuil_unicite)   # trait vertical
        dedans = (c[:, 0] > 4) & (c[:, 0] < w - 5) & (c[:, 1] > 4) & (c[:, 1] < h - 5)
        off = np.stack([ox, oy], 1)
        pos[d:d + 300] = c + np.einsum("nij,nj->ni", J, off)
        score[d:d + 300] = np.minimum(sx, sy)
        ok[d:d + 300] = vx & vy & dedans & (np.abs(off).max(1) < DEMI - 0.4)
    return pos, score, ok


def _noeuds_mm():
    jj, ii = np.mgrid[0:N, 0:N]
    return np.stack([ii * PAS, jj * PAS], 2).astype(np.float64)   # [j, i] -> (x, y)


def croissance(gris, valides, sup_mm, sup_px, H0=None, germes=None):
    """Mesure les croisements de proche en proche à partir d'appuis connus.

    `sup_mm`, `sup_px` : appuis de départ (coins de repères, points de selle).
    `H0`, `germes` : à défaut d'appuis, un trapèze approché et les nœuds par
    lesquels commencer. Renvoie les nœuds acceptés (mm, px, score).
    """
    noeuds = _noeuds_mm()
    sup_mm = [np.asarray(sup_mm, np.float64).reshape(-1, 2)]
    sup_px = [np.asarray(sup_px, np.float64).reshape(-1, 2)]
    accepte = np.zeros((N, N), bool)
    essais = np.zeros((N, N), int)
    acc_mm, acc_px, acc_sc = [], [], []
    portee = 15.0
    premiere = germes is not None
    while True:
        smm = np.concatenate(sup_mm)
        spx = np.concatenate(sup_px)
        libres = valides & ~accepte & (essais < 2)
        if premiere:
            cand = libres & germes
            modele = Modele(np.zeros((0, 2)), np.zeros((0, 2)), lisse=False, H=H0)
        else:
            if len(smm) < 4:
                break
            plat = noeuds.reshape(-1, 2)
            dmin = np.full(len(plat), np.inf)
            for d in range(0, len(smm), 500):
                dmin = np.minimum(dmin, np.linalg.norm(plat[:, None, :] - smm[None, d:d + 500, :], axis=2).min(1))
            cand = libres & (dmin.reshape(N, N) <= portee)
            modele = Modele(smm, spx, sigma=15.0, robuste_px=40.0 if len(smm) >= 16 else None)
        idx = np.argwhere(cand)
        nouveaux = 0
        if len(idx):
            X = noeuds[idx[:, 0], idx[:, 1]]
            p, J = modele.jacobien(X)
            pos, sc, ok = mesurer_noeuds(gris, p, J)
            essais[idx[:, 0], idx[:, 1]] += 1
            bons = idx[ok]
            accepte[bons[:, 0], bons[:, 1]] = True
            if ok.any():
                sup_mm.append(X[ok])
                sup_px.append(pos[ok])
                acc_mm.append(X[ok])
                acc_px.append(pos[ok])
                acc_sc.append(sc[ok])
            nouveaux = int(ok.sum())
        if premiere:
            premiere = False
            if nouveaux < 4:
                break
            continue
        if nouveaux == 0:
            if portee >= 35.0:
                break
            portee += 10.0
        else:
            portee = 15.0
    if not acc_mm:
        return np.zeros((0, 2)), np.zeros((0, 2)), np.zeros(0)
    return np.concatenate(acc_mm), np.concatenate(acc_px), np.concatenate(acc_sc)


def nettoyer(mm, px, echelle):
    """Écarte les nœuds incohérents : loin du trapèze d'ensemble, ou en
    désaccord avec leurs voisins. `echelle` : px par mm."""
    if len(mm) < 8:
        return np.ones(len(mm), bool)
    garde = np.ones(len(mm), bool)
    H = homographie(mm, px, robuste_px=3.0 * echelle)
    garde &= np.linalg.norm(projeter(H, mm) - px, axis=1) < 6.0 * echelle
    for _ in range(2):
        if garde.sum() < 8:
            break
        modele = Modele(mm[garde], px[garde], sigma=12.0)
        ecart = np.linalg.norm(modele.predire(mm) - px, axis=1)
        seuil = max(0.35 * echelle, 4 * 1.4826 * np.median(ecart[garde]))
        garde &= ecart < seuil
    return garde


def echelle_locale(H):
    p = projeter(H, [[CENTRE, CENTRE], [CENTRE + 1, CENTRE], [CENTRE, CENTRE + 1]])
    return float(np.sqrt(abs(np.linalg.det(np.stack([p[1] - p[0], p[2] - p[0]])))))


def _resultat(ok, mm=None, px=None, info=None, quarts=0, appuis=None):
    return {"ok": ok, "mm": mm, "px": px, "info": info or {}, "quarts": quarts, "appuis": appuis}


def _finir(gris, valides, sup_mm, sup_px, H0=None, germes=None, mini=40, info=None):
    mm, px, sc = croissance(gris, valides, sup_mm, sup_px, H0, germes)
    info = dict(info or {})
    if len(mm) < mini:
        info["cause"] = f"{len(mm)} nœuds seulement"
        return _resultat(False, info=info, appuis=(sup_mm, sup_px))
    H = homographie(mm, px, robuste_px=30.0)
    garde = nettoyer(mm, px, echelle_locale(H))
    info["noeuds_bruts"] = len(mm)
    if garde.sum() < mini:
        info["cause"] = "nœuds incohérents"
        return _resultat(False, info=info, appuis=(sup_mm, sup_px))
    return _resultat(True, mm[garde], px[garde], info, appuis=(sup_mm, sup_px))


# ------------------------------------------------------- amorce : mire actuelle


def _ellipse_rouge(bgr):
    """Cherche le cercle rouge (rayon 60 mm) à demi-résolution. Renvoie
    l'ellipse (centre, demi-axes, angle) en px pleine résolution, ou None."""
    demi = cv2.resize(bgr, None, fx=0.5, fy=0.5, interpolation=cv2.INTER_AREA).astype(np.float32)
    demi = cv2.GaussianBlur(demi, (0, 0), 1.3)      # calme le bruit de couleur
    b, g, r = cv2.split(demi)
    rougeur = (r - 0.5 * (g + b)) / (r + g + b + 120.0)
    rougeur *= np.minimum(1.0, (r + g + b) / 150.0)   # rien à chercher dans le noir
    noyau = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (13, 13))
    relief = cv2.morphologyEx(rougeur, cv2.MORPH_TOPHAT, noyau)
    seuil = max(0.03, float(np.quantile(relief[::2, ::2], 0.9975)))   # le cercle : environ 0,2 % des pixels
    ys, xs = np.nonzero(relief > seuil)
    if len(xs) < 80:
        return None
    pts = np.stack([xs, ys], 1).astype(np.float32)
    alea = np.random.default_rng(0)
    if len(pts) > 4000:
        pts = pts[alea.choice(len(pts), 4000, replace=False)]
    h, w = rougeur.shape
    meilleur, meilleure_note = None, 0
    for _ in range(400):
        ech = pts[alea.choice(len(pts), 6, replace=False)]
        try:
            (cx, cy), (d1, d2), ang = cv2.fitEllipse(ech)
        except cv2.error:
            continue
        a, b2 = d1 / 2, d2 / 2
        if not (0.04 * w < min(a, b2) and max(a, b2) < 0.45 * w and min(a, b2) / max(a, b2) > 0.55):
            continue
        t = np.deg2rad(ang)
        dx, dy = pts[:, 0] - cx, pts[:, 1] - cy
        u = dx * np.cos(t) + dy * np.sin(t)
        v = -dx * np.sin(t) + dy * np.cos(t)
        dist = np.abs(np.hypot(u / a, v / b2) - 1) * min(a, b2)
        dans = dist < 2.0
        secteurs = len(np.unique((np.degrees(np.arctan2(v[dans] / b2, u[dans] / a)) // 10).astype(int)))
        note = secteurs * 1000 + dans.sum()
        if secteurs >= 18 and dans.sum() > 0.2 * len(pts) and note > meilleure_note:
            meilleure_note, meilleur = note, dans
    if meilleur is None:
        return None
    (cx, cy), (d1, d2), ang = cv2.fitEllipse(pts[meilleur])
    return (2 * cx + 0.5, 2 * cy + 0.5), (d1, d2), ang, int(meilleure_note // 1000)


def _echantillonner(img, pts):
    carte = np.asarray(pts, np.float32).reshape(1, -1, 2)
    return cv2.remap(img, carte[..., 0], carte[..., 1], cv2.INTER_LINEAR, borderMode=cv2.BORDER_CONSTANT, borderValue=0)[0]


def amorce_actuelle(bgr, gris, valides, cercles=True):
    """Mire sans repère codé : cercle rouge, croix épaisse, puis grille.
    L'orientation (à un quart de tour près) vient des six petits cercles."""
    ell = _ellipse_rouge(bgr)
    if ell is None:
        return _resultat(False, info={"cause": "cercle rouge introuvable"})
    (cx, cy), (a, b), ang, secteurs = ell
    t = np.deg2rad(ang)
    R = np.array([[np.cos(t), -np.sin(t)], [np.sin(t), np.cos(t)]])
    P = R @ np.diag([a, b]) / 60.0          # mm (repère tourné inconnu) -> px
    c = np.array([cx, cy])
    # Relief des traits sombres épais.
    noyau = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (21, 21))
    relief = cv2.morphologyEx(gris, cv2.MORPH_BLACKHAT, noyau)
    relief = cv2.GaussianBlur(relief, (0, 0), 2.5)
    # Angle de la croix, à 90° près.
    rayons = np.arange(66.0, 126.0, 1.0)
    angles = np.deg2rad(np.arange(0, 90, 0.5))
    notes = np.zeros(len(angles))
    for k, th in enumerate(angles):
        dirs = np.stack([np.cos(th + np.arange(4) * np.pi / 2), np.sin(th + np.arange(4) * np.pi / 2)], 1)
        pts = c + (rayons[None, :, None] * dirs[:, None, :]) @ P.T
        notes[k] = _echantillonner(relief, pts.reshape(-1, 2)).mean()
    th0 = angles[notes.argmax()]
    # Points sur chaque bras, puis deux droites.
    decal = np.arange(-6.0, 6.01, 0.1)
    droites = []
    for k in range(2):
        pts_droite = []
        for bras in (k, k + 2):
            u = np.array([np.cos(th0 + bras * np.pi / 2), np.sin(th0 + bras * np.pi / 2)])
            nrm = np.array([-u[1], u[0]])
            for r in np.arange(66.0, 132.0, 3.0):
                ligne = c + (r * u[None, :] + decal[:, None] * nrm[None, :]) @ P.T
                prof = _echantillonner(relief, ligne)
                i = prof.argmax()
                if prof[i] > 2.5 * (np.median(prof) + 0.5):
                    pts_droite.append(ligne[i])
        if len(pts_droite) < 5:
            return _resultat(False, info={"cause": "croix introuvable"})
        vx, vy, x0, y0 = cv2.fitLine(np.array(pts_droite, np.float32), cv2.DIST_HUBER, 0, 0.01, 0.01).ravel()
        droites.append((np.array([x0, y0]), np.array([vx, vy])))
    # Intersections droite / ellipse : images des points à ±60 mm du centre.
    Pinv = np.linalg.inv(P)
    coins_px, coins_mm = [], []
    for k, (p0, v) in enumerate(droites):
        q0 = Pinv @ (p0 - c)
        qv = Pinv @ v
        A, B, C = qv @ qv, 2 * q0 @ qv, q0 @ q0 - 3600.0
        disc = B * B - 4 * A * C
        if disc <= 0:
            return _resultat(False, info={"cause": "croix hors du cercle"})
        u = np.array([np.cos(th0 + k * np.pi / 2), np.sin(th0 + k * np.pi / 2)])
        for s in ((-B + np.sqrt(disc)) / (2 * A), (-B - np.sqrt(disc)) / (2 * A)):
            q = q0 + s * qv
            signe = 1.0 if q @ u > 0 else -1.0
            coins_px.append(c + P @ q)
            axe = np.array([1.0, 0.0]) if k == 0 else np.array([0.0, 1.0])
            coins_mm.append(CENTRE + signe * 60.0 * axe)
    H0 = cv2.getPerspectiveTransform(np.array(coins_mm, np.float32), np.array(coins_px, np.float32))
    noeuds = _noeuds_mm()
    rayon = np.hypot(noeuds[..., 0] - CENTRE, noeuds[..., 1] - CENTRE)
    res = _finir(gris, valides, np.zeros((0, 2)), np.zeros((0, 2)), H0, rayon < 82.0, info={"secteurs": secteurs})
    if not res["ok"] or not cercles:
        return res
    # Orientation : où sont les six petits cercles ?
    modele = Modele(res["mm"], res["px"])
    ang_c = np.deg2rad(np.r_[np.arange(20, 71, 10), np.arange(110, 161, 10), np.arange(200, 251, 10), np.arange(290, 341, 10)])
    ang_t = np.deg2rad(np.arange(45, 360, 90))
    notes = []
    for q in range(4):
        tourne = ModeleTourne(modele, q)
        total = []
        for k in range(6):
            ctr = np.array([15.0 + 20 * k, 265.0 - 20 * k])
            bord = ctr + 5.0 * np.stack([np.cos(ang_c), np.sin(ang_c)], 1)
            temoin = np.r_[ctr + 3.4 * np.stack([np.cos(ang_c), np.sin(ang_c)], 1),
                           ctr + 6.4 * np.stack([np.cos(ang_t), np.sin(ang_t)], 1)]
            vb = _echantillonner(gris, tourne.predire(bord))
            vt = _echantillonner(gris, tourne.predire(temoin))
            total.append((vt.mean() - vb.mean()) / (vt.mean() + 1.0))
        notes.append(float(np.median(total)))
    ordre = np.argsort(notes)[::-1]
    res["quarts"] = int(ordre[0])
    res["info"]["orientation_notes"] = [round(x, 4) for x in notes]
    res["info"]["orientation_marge"] = round(notes[ordre[0]] - notes[ordre[1]], 4)
    if notes[ordre[0]] - notes[ordre[1]] < 0.04:
        # Les petits cercles ne tranchent pas : on préfère le dire.
        res["ok"] = False
        res["info"]["cause"] = "orientation incertaine"
    return res


# ------------------------------------------------------- amorce : repères codés

_DETECTEUR_REPERES = None


def _passes(gris8):
    yield gris8
    yield cv2.GaussianBlur(gris8, (0, 0), 1.5)
    yield cv2.GaussianBlur(gris8, (0, 0), 3.0)


def detecter_reperes(gris8, reperes):
    """Renvoie {identifiant: 4 coins px} pour les repères reconnus et cohérents."""
    global _DETECTEUR_REPERES
    if _DETECTEUR_REPERES is None:
        par = cv2.aruco.DetectorParameters()
        par.cornerRefinementMethod = cv2.aruco.CORNER_REFINE_SUBPIX
        _DETECTEUR_REPERES = cv2.aruco.ArucoDetector(cv2.aruco.getPredefinedDictionary(DICO_REPERES), par)
    trouves = {}
    # Trois passes, de plus en plus floutées : la passe nette donne les coins
    # les plus précis, les passes floutées résistent au bruit et au bougé.
    for image in _passes(gris8):
        coins, ids, _ = _DETECTEUR_REPERES.detectMarkers(image)
        if ids is not None:
            for c, i in zip(coins, ids.ravel()):
                if int(i) in reperes and int(i) not in trouves:
                    trouves[int(i)] = c.reshape(4, 2).astype(np.float64)
    if len(trouves) >= 2:
        mm = np.concatenate([reperes[i] for i in trouves])
        px = np.concatenate([trouves[i] for i in trouves])
        H, _ = cv2.findHomography(mm, px, cv2.RANSAC, 30.0)
        if H is not None:
            for i in list(trouves):
                if np.linalg.norm(projeter(H, reperes[i]) - trouves[i], axis=1).max() > 80.0:
                    del trouves[i]
    return trouves


def amorce_reperes(bgr, gris, valides, reperes):
    gris8 = np.clip(gris, 0, 255).astype(np.uint8)
    trouves = detecter_reperes(gris8, reperes)
    info = {"reperes": len(trouves)}
    if not trouves:
        info["cause"] = "aucun repère codé reconnu"
        return _resultat(False, info=info)
    mm = np.concatenate([reperes[i] for i in trouves])
    px = np.concatenate([trouves[i] for i in trouves])
    return _finir(gris, valides, mm, px, info=info)


# ------------------------------------------------------- amorce : ChArUco

_DETECTEUR_CHARUCO = None


def detecter_selles(gris8, selles):
    global _DETECTEUR_CHARUCO
    if _DETECTEUR_CHARUCO is None:
        _DETECTEUR_CHARUCO = cv2.aruco.CharucoDetector(planche_charuco())
    mm, px, vus = [], [], set()
    for image in _passes(gris8):
        coins, ids, _, _ = _DETECTEUR_CHARUCO.detectBoard(image)
        if ids is not None:
            for c, i in zip(coins.reshape(-1, 2), ids.ravel()):
                if int(i) in selles and int(i) not in vus:
                    vus.add(int(i))
                    mm.append(selles[int(i)])
                    px.append(c)
    mm = np.array(mm, np.float64).reshape(-1, 2)
    px = np.array(px, np.float64).reshape(-1, 2)
    if len(mm) >= 8:
        H, _ = cv2.findHomography(mm, px, cv2.RANSAC, 30.0)
        if H is not None:
            bons = np.linalg.norm(projeter(H, mm) - px, axis=1) < 60.0
            mm, px = mm[bons], px[bons]
    return mm, px


def amorce_charuco(bgr, gris, valides, selles):
    gris8 = np.clip(gris, 0, 255).astype(np.uint8)
    mm, px = detecter_selles(gris8, selles)
    info = {"selles": len(mm)}
    if len(mm) < 4:
        info["cause"] = "moins de 4 points de selle"
        return _resultat(False, info=info)
    return _finir(gris, valides, mm, px, info=info)


def en_gris(bgr):
    return cv2.cvtColor(bgr, cv2.COLOR_BGR2GRAY).astype(np.float32)

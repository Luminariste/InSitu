#!/usr/bin/env python3
"""Étude du pli : un projecteur qui éclaire deux surfaces à angle (mur et
avant-toit, ou mur et plafond), recalé par points sur un déroulé.

Le déroulé met chaque surface à plat, l'une à côté de l'autre, le long de
l'arête commune. La correspondance diapositive -> déroulé est alors faite de
deux trapèzes exacts (un par surface), recollés le long de l'arête avec une
cassure. Le moteur la modélise par UN trapèze plus une correction lisse
(spline en plaque mince) : ce script mesure ce que cette correction laisse
comme écart, selon le nombre de points cliqués et leur place, et le compare
à un recalage par surface (un trapèze de chaque côté de l'arête).

Modèle reproduit du moteur (crates/insitu-core/src/geometry/warp.rs) :
trapèze aux moindres carrés, puis spline en plaque mince sur les écarts,
lissage nul. Usage : python3 etudes/pli/simuler.py   (numpy seulement).
"""
import numpy as np

import sys
rng = np.random.default_rng(1)
NOISE = 0.0 if "--sans-bruit" in sys.argv else 1.0

# ---------------------------------------------------------------- scène 3D
# Repère monde en mètres : X le long du mur, Y vers le haut, Z en profondeur.
# Mur : plan Z = 6, de Y = 0 à Y = H. Avant-toit (dessous) : plan Y = H, de
# Z = 6 vers Z = 6 - P (il avance vers le projecteur).
H, P = 4.0, 3.0
# Projecteur au sol devant le mur, visant l'arête (comme sur la photo A LUM).
C = np.array([0.0, 1.2, 0.5])
target = np.array([0.0, H - 0.2, 5.0])
SLIDE = 280.0          # mm
FOCAL = 280.0 * 1.05   # mm : champ d'environ 53°, l'image couvre ~5 m


def camera():
    fwd = target - C
    fwd /= np.linalg.norm(fwd)
    right = np.cross([0, 1, 0], fwd)
    right /= np.linalg.norm(right)
    up = np.cross(fwd, right)
    return fwd, right, up


FWD, RIGHT, UP = camera()


def ray(slide_pts):
    """Rayon issu du projecteur pour un point de la diapositive (mm,
    origine en haut à gauche, y vers le bas)."""
    u = slide_pts[:, 0] - SLIDE / 2
    v = SLIDE / 2 - slide_pts[:, 1]
    return FWD[None] * FOCAL + RIGHT[None] * u[:, None] + UP[None] * v[:, None]


def to_unrolled(slide_pts):
    """Point de la diapositive -> point du déroulé (mm). Dans le déroulé,
    l'arête est la ligne y = 0 ; le mur est en dessous (y > 0, vers le sol),
    l'avant-toit au-dessus (y < 0), rabattu comme sur le plan de Trévarez.
    Renvoie aussi la surface touchée (0 mur, 1 avant-toit)."""
    d = ray(slide_pts)
    out = np.full((len(d), 2), np.nan)
    surf = np.full(len(d), -1)
    t_wall = (6.0 - C[2]) / d[:, 2]
    hit_wall = C[None] + t_wall[:, None] * d
    t_roof = (H - C[1]) / d[:, 1]
    hit_roof = C[None] + t_roof[:, None] * d
    wall_ok = (t_wall > 0) & (hit_wall[:, 1] <= H) & (hit_wall[:, 1] >= 0)
    roof_ok = (t_roof > 0) & (hit_roof[:, 2] <= 6.0) & (hit_roof[:, 2] >= 6.0 - P)
    # La première surface rencontrée.
    use_roof = roof_ok & (~wall_ok | (t_roof < t_wall))
    use_wall = wall_ok & ~use_roof
    out[use_wall, 0] = hit_wall[use_wall, 0] * 1000
    out[use_wall, 1] = (H - hit_wall[use_wall, 1]) * 1000
    out[use_roof, 0] = hit_roof[use_roof, 0] * 1000
    out[use_roof, 1] = -(6.0 - hit_roof[use_roof, 2]) * 1000
    surf[use_wall] = 0
    surf[use_roof] = 1
    return out, surf


# ------------------------------------------------------- modèle du moteur
def homography(src, dst):
    a = []
    for (x, y), (u, v) in zip(src, dst):
        a.append([x, y, 1, 0, 0, 0, -u * x, -u * y, -u])
        a.append([0, 0, 0, x, y, 1, -v * x, -v * y, -v])
    _, _, vt = np.linalg.svd(np.array(a))
    return vt[-1].reshape(3, 3)


def apply_h(h, pts):
    p = np.c_[pts, np.ones(len(pts))] @ h.T
    return p[:, :2] / p[:, 2:3]


def fit_h(src, dst):
    # Normalisation des deux côtés, comme le moteur.
    def norm(p):
        c = p.mean(0)
        s = np.sqrt(2) / np.mean(np.linalg.norm(p - c, axis=1))
        return np.array([[s, 0, -s * c[0]], [0, s, -s * c[1]], [0, 0, 1]])
    ts, td = norm(src), norm(dst)
    hn = homography(apply_h(ts, src), apply_h(td, dst))
    return np.linalg.inv(td) @ hn @ ts


def tps_kernel(r2):
    with np.errstate(divide="ignore", invalid="ignore"):
        k = 0.5 * r2 * np.log(r2)
    return np.nan_to_num(k)


class Warp:
    """Trapèze + spline en plaque mince sur les écarts (lissage nul)."""

    def __init__(self, src, dst):
        self.h = fit_h(src, dst)
        self.src = src
        if len(src) > 4:
            res = dst - apply_h(self.h, src)
            n = len(src)
            k = tps_kernel(((src[:, None] - src[None]) ** 2).sum(-1))
            p = np.c_[np.ones(n), src]
            a = np.zeros((n + 3, n + 3))
            a[:n, :n], a[:n, n:], a[n:, :n] = k, p, p.T
            b = np.zeros((n + 3, 2))
            b[:n] = res
            self.coef = np.linalg.solve(a, b)
        else:
            self.coef = None

    def __call__(self, pts):
        out = apply_h(self.h, pts)
        if self.coef is not None:
            n = len(self.src)
            k = tps_kernel(((pts[:, None] - self.src[None]) ** 2).sum(-1))
            out = out + k @ self.coef[:n] + np.c_[np.ones(len(pts)), pts] @ self.coef[n:]
        return out


# ------------------------------------------------------------- expériences
grid = np.stack(np.meshgrid(np.linspace(2, 278, 70), np.linspace(2, 278, 70)), -1).reshape(-1, 2)
truth, surf = to_unrolled(grid)
ok = surf >= 0
grid, truth, surf = grid[ok], truth[ok], surf[ok]
print(f"Couverture : {100 * np.mean(surf == 1):.0f} % de la diapositive sur l'avant-toit, "
      f"{100 * np.mean(surf == 0):.0f} % sur le mur.")
span = truth.max(0) - truth.min(0)
print(f"Emprise sur le déroulé : {span[0] / 1000:.1f} m × {span[1] / 1000:.1f} m")


def pick(n, near_fold=0):
    """n points « cliqués » au hasard sur des détails d'architecture, dont
    near_fold à moins de 15 cm de l'arête de chaque côté ; bruit de clic :
    0,3 mm sur la diapositive, 2 cm sur le déroulé."""
    idx = rng.choice(len(grid), n - near_fold, replace=False)
    close = np.where(np.abs(truth[:, 1]) < 150)[0]
    idx = np.r_[idx, rng.choice(close, near_fold, replace=False)] if near_fold else idx
    s = grid[idx] + NOISE * rng.normal(0, 0.3, (len(idx), 2))
    t = truth[idx] + NOISE * rng.normal(0, 20, (len(idx), 2))
    return s, t, surf[idx]


def report(name, pred):
    err = np.linalg.norm(pred - truth, axis=1) / 10  # cm
    band = np.abs(truth[:, 1]) < 500
    return (f"{name:<44} moy {err.mean():5.1f} cm   p95 {np.percentile(err, 95):5.1f} cm   "
            f"max {err.max():5.1f} cm   à 50 cm de l'arête : p95 {np.percentile(err[band], 95):5.1f} cm")


def per_surface(s, t, sf):
    """Recalage par surface : un trapèze par côté (4 points au moins de
    chaque côté), chaque point de la grille passe par le trapèze de sa
    surface (supposée connue : l'utilisateur tracerait l'arête)."""
    pred = np.zeros_like(truth)
    for k in (0, 1):
        m = sf == k
        if m.sum() < 4:
            return None
        w = Warp(s[m], t[m])
        pred[surf == k] = w(grid[surf == k])
    return pred


trials = 30
for n, nf in [(4, 0), (8, 0), (12, 0), (12, 4), (20, 0), (20, 6), (40, 10)]:
    rows_one, rows_two = [], []
    for _ in range(trials):
        s, t, sf = pick(n, nf)
        one = Warp(s, t)(grid)
        rows_one.append(np.linalg.norm(one - truth, axis=1) / 10)
        two = per_surface(s, t, sf)
        if two is not None:
            rows_two.append(np.linalg.norm(two - truth, axis=1) / 10)
    def summary(rows):
        if not rows:
            return "   (pas 4 points de chaque côté)"
        e = np.array(rows)
        return (f"p95 médian {np.median(np.percentile(e, 95, axis=1)):5.1f} cm, "
                f"max médian {np.median(e.max(1)):5.1f} cm")
    print(f"{n:2d} points ({nf:2d} près de l'arête) | moteur actuel : {summary(rows_one)} | "
          f"par surface : {summary(rows_two)}")

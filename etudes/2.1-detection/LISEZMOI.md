# Étude 2.1 — scripts d'expérience

Ces scripts ont produit les chiffres de `docs/etudes/2.1-detection-mire.md`.
Ce sont des prototypes d'étude en Python : ils ne font pas partie du moteur
et ne sont lancés par aucun test.

## Installation

```
pip install --break-system-packages opencv-contrib-python-headless numpy pillow
```

Versions utilisées : Python 3.13, OpenCV 5.0, NumPy 2. Tout se lance depuis la
racine du dépôt. Les scripts lisent le jeu d'essai d'Épinal (`jeu-essai/`).

## Les scripts

| Script | Ce qu'il fait | Durée (2 cœurs) |
| --- | --- | --- |
| `regarder.py <dossier>` | Planche des 8 photos brutes et recadrages pleine résolution, pour regarder les photos | 10 s |
| `mires.py <dossier>` | Dessine les mires candidates (A actuelle, A+ traits épais, B repères codés, C couronne ChArUco) | 10 s |
| `photos_reelles.py <dossier>` | Détecte la mire actuelle sur les 8 vraies photos, mesure l'écart du redressement manuel | 2 min |
| `calibrer.py <dossier>` | Compare le rendu simulé aux vraies photos (contraste des traits, écart au trapèze) | 1 min |
| `experience.py <fichier.jsonl> [n_niveaux] [n_balayages]` | L'expérience : 440 scènes simulées × 4 candidats | 40 min |
| `tableaux.py <fichier.jsonl> [resume.json]` | Tableaux Markdown du rapport à partir du fichier précédent | 1 s |
| `figures.py <dossier>` | Images d'illustration | 30 s |
| `comparer_rust.py <dossier> <binaire> [n]` | Compare le détecteur AprilTag d'OpenCV à un détecteur en Rust pur | 3 min |
| `reperes_sur_reel.py <dossier> [binaire]` | Incruste les 16 repères de la mire B dans les 8 vraies photos et tente de les relire | 1 min |

Modules communs : `mires.py` (dessin), `simulateur.py` (photo simulée et
vérité terrain), `detecteurs.py` (les trois amorces et la mesure des
croisements de la grille).

## Tout refaire

```
cd etudes/2.1-detection
python3 photos_reelles.py resultats
python3 calibrer.py resultats
python3 figures.py resultats
python3 experience.py /tmp/resultats.jsonl
python3 tableaux.py /tmp/resultats.jsonl resultats/resume.json > resultats/tableaux.md

# Essai du détecteur en Rust pur (hors de l'espace de travail du dépôt) :
cargo build --release --manifest-path essai-rust/Cargo.toml --target-dir /tmp/cible-essai
python3 comparer_rust.py resultats /tmp/cible-essai/release/essai-reperes
python3 reperes_sur_reel.py resultats /tmp/cible-essai/release/essai-reperes
```

Les tirages sont reproductibles : les graines sont fixées par le nom de la
condition et le numéro de la scène.

`/tmp/resultats.jsonl` pèse quelques centaines de Ko et n'est pas versé ; seul
son résumé (`resultats/resume.json`, `resultats/tableaux.md`) l'est.

## Contenu de `resultats/`

- `photos-reelles.json`, `calibrage.json`, `comparaison-rust.json`,
  `reperes-sur-reel.json`, `resume.json`, `tableaux.md` : les chiffres bruts.
- Sept images réduites d'illustration (1,4 Mo au total).

## Ce que ces prototypes ne sont pas

- Pas optimisés, pas robustes aux entrées invalides, pas testés unitairement.
- La détection des repères codés et des points de selle s'appuie sur OpenCV
  (`cv2.aruco`), qui ne sera pas dans le moteur. La mesure des croisements de
  grille (`mesurer_noeuds`, `croissance`, `Modele`) est écrite à la main, avec
  NumPy seul, et sert de modèle à l'écriture en Rust.

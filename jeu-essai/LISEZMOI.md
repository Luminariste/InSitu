# Jeu d'essai

Des fichiers réels de Luminariste, pour que le moteur soit testé sur ce qu'il
devra vraiment traiter. Un dossier par projet : Épinal (le premier) et Trévarez (le deuxième).

## Épinal, novembre 2025

Un portail éclairé par quatre rétroprojecteurs, A à D, dont les emprises se
recouvrent. Prises de vue des 24 et 25 novembre 2025 à l'iPhone 16 Pro.

```
epinal/
  sources.json     Liste des fichiers et de leur rôle, tenue à la main
  manifeste.json   Produit par construire.py : tailles, empreintes, dimensions
  references/      Versions pleine résolution, parfois allégées (voir tableau)
  reduits/         Versions réduites, pour les tests rapides
```

Les fichiers d'origine ne sont pas dans le dépôt : ils restent dans le Dropbox
de Luminariste (dossier « 2025-12 - Epinal - InSitu / 05 - Projection »).
`manifeste.json` garde leur nom, leur taille et leur empreinte, pour pouvoir
vérifier qu'on parle bien du même fichier.

| Famille | Contenu | Taille d'origine | Référence | Réduit |
| --- | --- | --- | --- | --- |
| `photos-brutes/x-brut` | Photo de la mire en chambre noire, une par projecteur | 4 032 × 3 024 | copie exacte | ÷ 4 |
| `photos-brutes/img_52…` | Autres prises en chambre noire, non retenues | 4 032 × 3 024 | copie exacte | ÷ 4 |
| `photos-brutes/x-lum` | Façade de nuit, projecteur allumé avec la mire : repères larges pour situer la diapositive | 5 712 × 4 284 | — | ÷ 2 |
| `photos-brutes/x-mur` | Photo directe de la façade, de jour | 5 712 × 4 284 | — | ÷ 2 |
| `photos-brutes/b-grille`, `img_5286` | Façade avec une grille projetée | 5 712 × 4 284 | — | ÷ 2 |
| `photos-redressees/x` | Photo brute redressée à la main dans Millumin | 5 512 × 5 512, TIFF | JPEG qualité 92 | ÷ 8 |
| `verres/verre-x` | Fichier envoyé à LightBurn | 15 433 × 15 433, PNG couleur | PNG en niveaux de gris | ÷ 8 |
| `facade/portail-deroule` | Le fond sur lequel la fresque est composée | 5 145 × 3 508 | copie exacte | ÷ 4 |
| `mire/` | La mire actuelle (page 2 du fichier Illustrator, rendue à 600 dpi) et sa source Illustrator | 6 615 × 6 615 | copie exacte | ÷ 4 |
| `reglages/` | Projets Millumin et LightBurn du chantier | — | Millumin : copie exacte ; LightBurn : sans ses images incorporées | — |
| `compositions/composition-a.ai` | Composition Illustrator de la diapositive A (154 Mo) | — | — | — |

Une référence a toujours la même taille en pixels que l'original. Les
références « allégées » suffisent pour comparer des géométries ; elles ne
sont pas faites pour comparer des valeurs de pixels au niveau près.

### Ce qu'il faut savoir avant de s'en servir

- **Les photos brutes sont pivotées.** Leur orientation EXIF vaut 8 : l'image
  enregistrée est couchée. `manifeste.json` donne `orientationExif` pour
  chaque photo. Les réduits gardent la disposition des pixels de l'original
  et n'ont plus d'EXIF.
- **Les photos brutes font 12 Mpx.** La mire y occupe 1 790 à 1 950 px de
  côté (mesuré par l'étude 2.1) : les photos redressées à 5 512 px sont donc
  un agrandissement d'un facteur 3 environ. La résolution utile d'une photo
  redressée est de l'ordre de 0,15 mm par pixel, pas 0,05 mm.
- **Les PNG du dossier « Mires » du Dropbox ne sont pas la mire** : ils
  montrent une frise de fleurs. La mire du jeu d'essai est fabriquée à partir
  de la page 2 de `mire-280.ai`, un carré de 280 mm.
- **La façade est un déroulé, pas une vue de face.** Le portail a été mis à
  plat à partir d'un relevé d'architecte, pour que toutes les surfaces soient
  visibles, voussures comprises. La correspondance entre ce déroulé et ce que
  voit un projecteur n'est donc pas un simple trapèze, même localement.
- **Les rôles `x-mur`, `b-grille`, `img_5286` et le projecteur des autres
  `img_…` sont déduits** de l'image et de l'heure de prise de vue. Ils sont
  marqués « à confirmer » dans `sources.json`.
- **Les photos redressées sont une référence humaine**, pas une vérité : les
  coins ont été posés à la main sur un papier qui gondole.
- **La composition Illustrator n'est pas dans le dépôt** (trop lourde). Elle
  sera nécessaire à la tâche 6.3 ; il faudra alors la récupérer depuis le
  Dropbox.

### Réglages de gravure relevés dans `reglages/epinal.lbrn2`

| Réglage | Valeur |
| --- | --- |
| Machine | « Laser Fibre Luminariste » |
| Mode | Image, seuil (`threshold`), négatif activé |
| Résolution | 1 000 dpi (intervalle 0,0254 mm) |
| Passes | 2 |
| Puissance, vitesse, fréquence | 90 %, 2 000 mm/s, 100 kHz |
| Taille et position de l'image | 280,0 mm, centrée en (150 ; 150) sur la plaque de 300 mm |
| Miroir | aucun |

## Trévarez, novembre 2026

Les écuries du domaine de Trévarez, pour Noël 2026 : la cour entière (quatre
façades et leurs avant-toits) mise à plat dans un déroulé vectoriel, douze
projecteurs A à L. Prises de vue du 11 novembre 2026 à l'iPhone 16 Pro, avec
l'ancienne mire. Benjamin y a monté un vrai projet InSitu avec la version
0.1.28 ; c'est le premier projet enregistré par l'app elle-même.

Les fichiers d'origine restent dans le Dropbox de Luminariste (« 06 -
Projets - Parcours Lumière / Diffusions / 2026-11 - Noel a Trevarez / 05 -
PROJET / 04 - Eccuries / Ecuries - Fresque »).

| Fichier | Contenu | Référence | Réduit |
| --- | --- | --- | --- |
| `facade/ecuries-planbase.pdf` | Le déroulé, PDF vectoriel d'Illustrator, page carrée de 500 mm | copie exacte | — |
| `projet/projet.json` | Le projet de Benjamin : 12 projecteurs, B à G redressés, B et C recalés, 95 dessins, 2 posés | copie exacte | — |
| `dessins/algue-h02.png`, `algues-horizontales.png` | Les deux dessins posés (sur B et sur C) | copie exacte | — |
| `photos-brutes/a-brut.jpg` | Photo de mire A : grille presque illisible | copie exacte | ÷ 4 |
| `photos-brutes/b-brut.jpg` | Photo de mire B : redressée sans aide | copie exacte | ÷ 4 |
| `verres/trevarez-c.png` | Diapositive C exportée par l'app à 1 400 dpi | — | ÷ 14 (1 102 px) |

Ce qu'il faut savoir :

- **Le projet est une copie à un instant donné** (12 novembre 2026, 22 h 37).
  La déformation du dessin posé sur B a été retouchée après l'export de
  `Trevarez-B.png` : seul l'export de C correspond encore au projet, et c'est
  lui que le test compare.
- **Les photos C à H sont en HEIC** (4 à 7 Mo chacune) et ne sont pas
  versées : aucun test n'en a besoin, `heic/a-brut.heic` couvre la lecture.
- **La composition Illustrator** (`ECURIES-COMPOSITION.ai`, 50 Mo, environ
  245 images liées) n'est pas dans le dépôt. Ses liens sont des chemins
  absolus pris sur le Mac de Marc ; 20 des 100 liens pointent vers un autre
  dossier du projet (« 17 - Projection gobo finale »).
- **Ce qui manque encore** : aucune photo de la nouvelle mire (2.4b), et le
  projecteur A, qui éclaire à la fois un plafond en bois et un mur de brique,
  n'est ni redressé ni recalé.

Tests : `crates/insitu-core/tests/trevarez.rs`.

## Photo HEIC (tâche 2.5)

`heic/a-brut.heic` est la photo brute A d'Épinal (`a-brut.jpg`) réencodée en
HEIC par libheif, avec un encodeur indépendant du moteur (`pillow-heif`),
pour tester la lecture du HEIC sur une vraie mire : 485 Ko, qualité 40,
mêmes pixels couchés que le JPEG, sans EXIF. Ce n'est pas un fichier
d'origine : les photos d'Épinal ont été livrées en JPEG. Pour la refaire :
`pip install pillow pillow-heif`, puis `python3 jeu-essai/heic/fabriquer.py`.
Une vraie photo HEIC d'iPhone de Luminariste reste à verser si un test en a
besoin.

## Ajouter ou modifier un fichier

Il faut avoir les fichiers d'origine sous la main, rangés par famille comme
dans `sources.json` (champ `fichier`).

1. L'ajouter à `<jeu>/sources.json` : rôle, nom d'origine, type de référence
   (`identique`, `jpeg-92`, `gris-8-bits`, `lightburn-allege` ou `null`) et
   diviseur du réduit.
2. Lancer `python3 jeu-essai/construire.py jeu-essai/<jeu> <dossier des originaux>`
   (il faut Pillow). Le script refait `references/`, `reduits/` et le manifeste.
3. Lancer `cargo test` et valider le tout ensemble.

Le dépôt grossit définitivement de tout ce qu'on y verse : ne garder en
référence que ce qu'un test utilise vraiment.

Rôles utilisés : `raw_photo`, `raw_photo_alternate`, `lit_photo`,
`wall_photo`, `grid_photo`, `rectified_photo`, `engraving_export`, `facade`,
`composition`, `target`, `settings`, `project` (un `projet.json`
enregistré par l'app), `drawing` (un dessin de la bibliothèque).

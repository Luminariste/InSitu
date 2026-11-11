# Jeu d'essai

Des fichiers réels de Luminariste, pour que le moteur soit testé sur ce qu'il
devra vraiment traiter. Un dossier par projet ; pour l'instant un seul.

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
| `mire/` | La mire actuelle (280 mm à 2 400 dpi) et sa source Illustrator | 26 458 × 26 458 | copie exacte | ÷ 16 |
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
- **Les photos brutes font 12 Mpx.** La mire y occupe environ 2 300 px de
  côté : les photos redressées à 5 512 px sont donc un agrandissement. La
  résolution utile d'une photo redressée est de l'ordre de 0,12 mm par pixel,
  pas 0,05 mm.
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
`composition`, `target`, `settings`.

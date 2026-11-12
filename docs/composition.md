# Poser des dessins sur la façade (tâches 4.2 à 4.4)

Dans l'étape **Façade** (⌘1), les dessins de la bibliothèque se posent
directement sur le plan, comme dans Illustrator. Il faut un plan **mis à
l'échelle** : les dessins sont posés en vraies dimensions (mm de façade).

## Poser un dessin

1. Importer des dessins dans l'étape Bibliothèque (⌘5) si ce n'est pas fait.
2. Dans l'étape Façade, la rubrique **Dessins** (panneau de gauche, sous les
   projecteurs) montre la bibliothèque ; un champ permet de chercher.
3. Glisser une vignette sur le plan. Le dessin arrive au point de chute, large
   du quart de ce qu'on voit, aux proportions du fichier. Un double-clic sur
   la vignette le pose au centre de la vue.

Tout s'enregistre dans le projet à la fin de chaque geste.

## Gestes à la souris

| Geste | Effet |
| --- | --- |
| Cliquer un dessin | Le choisir (avec tout son groupe) |
| ⇧-clic | Ajouter ou retirer un dessin de la sélection |
| Glisser dans le vide | Cadre de sélection (⇧ : ajoute) |
| Cliquer dans le vide | Plus rien de choisi |
| Glisser un dessin choisi | Le déplacer ; ⇧ : à l'horizontale, à la verticale ou à 45° |
| ⌥-glisser | Déplacer une copie |
| Poignée de coin (carré blanc) | Agrandir ou réduire, proportions gardées ; ⌥ : depuis le centre |
| Juste au-delà d'un coin | Tourner autour du centre ; ⇧ : par pas de 45° |
| Espace + glisser, ou défiler à deux doigts | Déplacer la vue |

## Raccourcis (ceux d'Illustrator)

| Raccourci | Effet |
| --- | --- |
| ⌘A / ⇧⌘A | Tout choisir / ne rien choisir ; Échap aussi |
| ⌫ | Retirer les dessins choisis de la façade (la bibliothèque n'est pas touchée) |
| ← → ↑ ↓ | Déplacer de 10 mm ; avec ⇧ : 100 mm |
| ⌘C, ⌘X, ⌘V | Copier, couper, coller au centre de la vue |
| ⌘F / ⌘B | Coller devant / derrière, à la même place |
| ⌘D | Répéter le dernier geste : après un ⌥-glisser, chaque ⌘D pose une copie de plus au même écart |
| ⌘G / ⇧⌘G | Grouper / dégrouper |
| ⌘] / ⌘[ | Avancer / reculer d'un cran ; avec ⇧ : premier plan / arrière-plan |

Sur un clavier français, les « crochets » sont les deux touches à droite du P
(^ et $).

## Inspecteur (à droite)

La rubrique **Dessins posés** apparaît quand un dessin est choisi :

- centre X et Y, largeur (la hauteur suit) : taper par exemple `2,5 m`,
  `250 cm` ou `2500`, puis ↩ ;
- rotation en degrés (un seul dessin) ; boutons ↺ 90° et ↻ 90° ;
- symétries ⇆ (gauche-droite) et ⇅ (haut-bas) ;
- avec plusieurs dessins : aligner à gauche, au centre, à droite, en haut, au
  milieu, en bas ; répartir l'espacement (trois dessins ou plus) ; un groupe
  compte pour un seul bloc ;
- grouper, dégrouper, premier plan, arrière-plan, retirer.

## Calques (tâche 4.3)

La rubrique « Calques », dans le panneau de gauche, au-dessus des dessins.
Le calque du haut de la liste est devant.

- **+** : nouveau calque, juste au-dessus du calque choisi.
- **Clic** sur un calque : les dessins qu'on pose ensuite vont dessus.
- **Œil** : masquer, réafficher. **Cadenas** : les dessins du calque ne se
  choisissent plus (ni au clic, ni au cadre, ni par ⌘A).
- **Double-clic** sur le nom : le renommer (↩ valide, Échap annule).
- Au survol : **flèches** pour monter ou descendre le calque, **corbeille**
  pour le supprimer avec ses dessins (⌘Z le rend).
- Dans l'inspecteur, **Calque** range les dessins choisis sur un autre calque.

Un projet sans calque en montre un seul, « Calque 1 ».

## Annuler (tâche 4.3)

⌘Z annule la dernière modification du projet, ⇧⌘Z la rétablit, sans limite
de nombre et quel que soit l'écran (aussi dans le menu Projet). L'historique
reste après la fermeture de l'app. Si le projet a été modifié ailleurs entre
temps (l'autre Mac, par Dropbox), l'app refuse d'annuler plutôt que
d'écraser ce changement.

## Déformer (tâche 4.4)

Dans l'inspecteur, rubrique « Déformation » :

- **Quatre coins** : quatre points bleus aux coins du dessin ; on les tire
  comme les coins d'un trapèze (les droites restent droites).
- **Grille…** : 2 × 2, 3 × 3, 4 × 4, 6 × 1, 6 × 2 ou 8 × 4 cases. Chaque
  point bleu se tire, le dessin suit en souplesse. Pour une voussure,
  « 6 × 1 » puis remonter les points du milieu, en haut et en bas.
- **Redresser** : le dessin redevient droit.

Plusieurs dessins choisis ensemble (ou un groupe) se déforment d'un bloc :
une seule grille sur l'ensemble. Déplacer, tourner, agrandir ou retourner
un dessin déformé garde sa forme. Une déformation qui replierait le dessin
sur lui-même est refusée (message en bas, l'état d'avant revient). ⌘Z
annule chaque geste. L'export suit exactement la forme vue à l'écran.

### Essai sur Mac

1. Poser un dessin, puis « Grille… » → « 6 × 1 ».
2. Tirer vers le haut les cinq points du milieu de la rangée du haut, plus
   au centre qu'aux bords ; pareil pour la rangée du bas. Le dessin doit
   s'arrondir en arc, sans saut quand on prend un point.
3. Déplacer et tourner le dessin : l'arc suit.
4. Exporter les diapositives où tombe le dessin : dans LightBurn, le dessin
   doit avoir la même forme qu'à l'écran (vue Diapositive comprise).
5. Choisir deux dessins, « Quatre coins », tirer un coin : les deux se
   déforment ensemble. ⌘Z revient en arrière.

## Ce qui n'est pas encore là

- **Export** : fait. Les dessins posés partent dans les diapositives et dans
  la vue Diapositive, sauf ceux des calques masqués (`docs/export.md`).
- Les dessins sont affichés par leur aperçu de 1 024 px : flous de très près.

## Sans écran

```
insitu place list <projet.insitu>
insitu place add <projet.insitu> <dessin> --at X,Y --width MM [--rotation DEG] [--mirror] [--group NOM]
insitu place remove <projet.insitu> <id>…
insitu place set <projet.insitu> <placements.json> [--history DIR]
insitu place deform <projet.insitu> <id>… (--corners | --grid CxR | --none)
insitu project history <projet.insitu> --history DIR
insitu project undo <projet.insitu> --history DIR
insitu project redo <projet.insitu> --history DIR
```

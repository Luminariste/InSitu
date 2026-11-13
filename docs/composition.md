# Poser des dessins sur la façade (tâches 4.2 à 4.5)

Dans l'étape **Façade** (⌘1), les dessins de la bibliothèque se posent
directement sur le plan, comme dans Illustrator. Il faut un plan **mis à
l'échelle** : les dessins sont posés en vraies dimensions (mm de façade).

## Poser un dessin

1. Importer des dessins dans l'étape Bibliothèque (⌘5) si ce n'est pas fait.
2. Dans l'étape Façade, l'onglet **Dessins** du panneau de droite montre la
   bibliothèque (rubrique **Dessins**) ; un champ permet de chercher.
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
| Poignée de coin (carré blanc) | Étirer en largeur et en hauteur à la fois ; ⇧ : proportions gardées ; ⌥ : depuis le centre |
| Poignée au milieu d'un côté | Étirer seulement en largeur (côtés gauche et droit) ou en hauteur (haut et bas) ; ⌥ : depuis le centre |
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

## Outils (barre du haut) et mesures (à droite)

Les outils sont des icônes dans la barre d'outils, en haut de l'écran
Façade ; ils agissent sur les dessins choisis (bulle d'aide au survol) :

- tourner d'un quart de tour à gauche ou à droite ; symétries gauche-droite
  et haut-bas ;
- déformer : quatre coins, grille (menu), redresser ;
- avec plusieurs dessins : aligner à gauche, au centre, à droite, en haut, au
  milieu, en bas ; répartir l'espacement (trois dessins ou plus) ; un groupe
  compte pour un seul bloc ;
- premier plan, avancer, reculer, arrière-plan ; grouper, dégrouper.

La rubrique **Dessins posés** (onglet **Dessins**, qui s'ouvre seul) apparaît
quand un dessin est choisi : centre X et Y, largeur (la hauteur suit ; taper
par exemple `2,5 m`, `250 cm` ou `2500`, puis ↩), rotation en degrés (un seul
dessin), calque, diapositive, retirer.

## Calques (tâche 4.3)

L'onglet « Calques », dans le panneau de droite.
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

Dans la barre d'outils, groupe des déformations :

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

## Diapositive de chaque dessin (tâche 4.5)

Chaque dessin posé part dans **une seule** diapositive, jamais coupé entre
deux projecteurs. Sur la façade, son contour a la couleur de son projecteur
(les mêmes couleurs que les emprises). Un contour **rouge** : le dessin est
à cheval, aucune diapositive ne le contient en entier, ou bien celle
choisie à la main le coupe. Un contour gris : il ne tombe dans aucune
emprise.

**Ce qui ne sera pas projeté est en rouge** (version 0.1.42) : la part
d'un dessin qui sort de l'emprise de sa diapositive est couverte d'un aplat
rouge sur la façade. Personne ne la projette, même si une diapositive
voisine couvre cet endroit (un dessin ne part que dans une diapositive).
Seuls les traits comptent (la silhouette du dessin, pas son rectangle) : un
dessin dont seule la marge vide dépasse n'a pas de rouge. Un dessin envoyé
à la main dans une diapositive qui ne l'atteint pas est rouge en entier.
Pour corriger : déplacer le dessin, le réduire, ou le rendre à
« Automatique ». La vue Diapositive le rappelle dans un bandeau orange
(« … dépasse de cette diapositive ; 0,32 m² ne seront pas projetés »).

La proposition est automatique : parmi les diapositives qui contiennent le
dessin en entier, celle où il est le plus loin du bord. Elle suit le dessin
quand on le déplace.

Dans l'inspecteur, « Diapositive » montre où part la sélection et permet de
la choisir à la main (les diapositives qui couperaient le dessin sont
signalées). Ce choix reste, même si l'on déplace le dessin, jusqu'à ce
qu'on revienne à « Automatique ». ⌘Z l'annule.

### Arborescence « Diapositives »

La rubrique **Diapositives** range les dessins posés par diapositive, comme
les calques d'Illustrator : un groupe par projecteur recalé (même couleur
que son emprise), et « Hors de toute diapositive » pour les dessins qui ne
tombent dans aucune emprise. Dans un groupe, le dessin du haut est devant.

- **Clic** sur une ligne : choisit ce dessin (et son groupe) sur la façade.
  **⌘-clic** l'ajoute à la sélection ou l'en retire ; **⇧-clic** choisit
  toutes les lignes entre la dernière cliquée et celle-ci.
- **Œil** (au survol de la ligne) : masque le dessin. Masqué, il ne
  s'affiche plus et ne part plus à l'export. **Cadenas** : le verrouille ; il
  reste visible et exporté, mais ne se choisit plus sur la façade. Un œil ou
  un cadenas actif reste affiché.
- **Clic droit** : Grouper (⌘G), Dégrouper, Masquer, Verrouiller, ordre
  (premier plan, en avant, en arrière, arrière-plan), envoyer vers une
  diapositive ou « Automatique », Supprimer. Le menu porte sur la sélection
  si la ligne en fait partie, sinon sur la ligne seule.
- **Glisser une ligne sur le titre d'une autre diapositive** : le dessin y
  part, choisi à la main (comme dans l'inspecteur).
- **Glisser une ligne au-dessus d'un autre dessin** : il passe juste devant
  lui (et sur son calque), et dans sa diapositive si elle est différente.
- « choisie à la main » et « à cheval » (en rouge) s'affichent à droite.
- La flèche d'un titre replie ou déplie le groupe. ⌘Z annule.

### Essai sur Mac

1. Ouvrir un projet dont plusieurs projecteurs sont recalés, avec des
   dessins posés.
2. Chaque dessin doit être entouré de la couleur d'un projecteur. Poser un
   dessin dans une zone de recouvrement : une seule couleur.
3. Exporter les diapositives : le dessin n'apparaît que dans la diapositive
   de sa couleur, en entier.
4. Choisir ce dessin, puis dans « Diapositive » l'autre projecteur :
   la couleur change ; le déplacer un peu : le choix reste. Réexporter : il
   a changé de diapositive.
5. Agrandir un dessin jusqu'à dépasser de toutes les emprises : contour
   rouge.
6. Rubrique Diapositives : glisser un dessin d'un groupe à l'autre, puis
   au-dessus d'un autre dessin. Sa couleur suit, et il passe devant.
7. Rubrique Diapositives : ⌘-clic sur deux dessins, puis clic droit,
   Grouper ; cliquer l'un d'eux sur la façade : les deux sont choisis.
8. Masquer un dessin avec l'œil : il disparaît de la façade ; exporter : il
   n'est pas dans la diapositive. Le verrouiller : on ne peut plus le
   choisir sur la façade.

## Importer une composition Illustrator (tâche 6.3)

Pour reprendre une fresque déjà composée dans Illustrator, dessin par
dessin :

1. Dans Illustrator, la composition doit être sur un plan de travail **de la
   taille du plan de façade** (comme pour la fresque), et les dessins
   **liés** (pas incorporés). Enregistrer le `.ai` avec « Créer un fichier
   compatible PDF » (réglage par défaut).
2. Garder les dossiers des dessins **à côté du `.ai`**, comme Illustrator
   les attend (Algues/, Poissons/…). Le `.ai` peut venir d'un autre Mac :
   les liens sont retrouvés par leur dossier relatif, puis par leur nom.
3. Écran Façade (plan à l'échelle), onglet **Calques**, rubrique
   **Composition Illustrator** : **Importer un fichier .ai…**.
4. Chaque fichier lié entre dans la bibliothèque (dossier du nom du `.ai`),
   chaque image placée devient un dessin posé, avec sa position, sa taille,
   sa rotation et son retournement, dans un calque du nom de celui
   d'Illustrator (masqué ou verrouillé comme dans Illustrator).
5. Le compte rendu dit combien de dessins sont posés, et nomme les liens
   introuvables, les images incorporées (à lier dans Illustrator pour les
   retrouver) et les images déformées en biais (posées sans le biais).

⌘Z annule l'import en deux fois (dessins posés, puis bibliothèque). Les masques d'écrêtage, l'opacité et les tracés
dessinés dans Illustrator ne sont pas repris : seules les images liées.

À essayer sur le Mac : importer ECURIES-COMPOSITION.ai depuis son dossier
« Ecuries - Fresque » ; comparer à l'écran avec Illustrator (positions,
rotations, dessins retournés, calques). Les 20 liens vers « 17 - Projection
gobo finale » doivent être retrouvés s'ils sont sur le Mac.

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
insitu place slides <projet.insitu>
insitu place assign <projet.insitu> <id>… (--to PROJECTEUR | --auto)
insitu place adjust <projet.insitu> <id> (--corners | --grid CxR | --none)
insitu illustrator inspect <fichier.ai>
insitu illustrator import <projet.insitu> <fichier.ai> [--frame X Y L H] [--cache DIR]
insitu project history <projet.insitu> --history DIR
insitu project undo <projet.insitu> --history DIR
insitu project redo <projet.insitu> --history DIR
```

# Bibliothèque de dessins (tâche 4.1)

La bibliothèque rassemble les dessins du projet : des traits blancs sur fond
noir, en PNG, JPEG, TIFF, BMP, PSD (image aplatie) ou SVG. Ils seront posés
sur la façade à l'étape suivante (placement, tâche 4.2).

## Dans l'app

1. Ouvrir ou créer le projet (menu **Fichier** du Mac), puis
   aller dans l'étape **Bibliothèque** (⌘5).
2. **Importer un dossier…** : choisir un dossier de dessins. Tout son contenu
   est copié dans le projet, sous-dossiers compris, dans
   `bibliotheque/<nom du dossier>/`. Une barre montre l'avancement ; à la
   fin, une phrase dit combien de dessins sont arrivés, combien étaient déjà
   là, et quels fichiers sont illisibles.
   **Importer des dessins…** : choisir un ou plusieurs fichiers ; ils vont
   dans `bibliotheque/`. On peut aussi **glisser depuis le Finder** des
   fichiers ou des dossiers sur la fenêtre : un voile bleu dit « Déposer pour
   importer » ; un dossier déposé garde son nom (D-4.1-f). Dans l'étape
   Façade, les dessins lâchés sur le plan sont aussi posés à l'endroit du
   lâcher ; lâchés ailleurs, ils entrent seulement dans la bibliothèque et
   l'onglet Dessins s'ouvre.
3. À gauche, les dossiers (avec le nombre de dessins) ; au milieu, les
   vignettes ; à droite, l'aperçu du dessin choisi et sa fiche (dossier,
   taille, fichier).
4. **Chercher** : chaque mot doit se trouver dans le nom du dessin ou dans
   son dossier, sans tenir compte des accents ni des majuscules (« oiseau
   mer » trouve `Oiseaux/Mer/goéland`).
5. **Fond** : le noir des dessins est rendu transparent. Damier, noir ou
   pierre montrent ce qu'il y a derrière ; sur le noir, le dessin est tel que
   dans son fichier. Le même choix se retrouve dans la barre de la gomme et
   du crayon, de la retouche de silhouette et de la comparaison des
   vecteurs ; il est retenu d'un lancement à l'autre.
6. Les flèches du clavier passent d'un dessin à l'autre (cliquer d'abord
   dans les vignettes).
7. **Retirer de la bibliothèque…** (en bas de l'inspecteur, ou ⌫) : après
   confirmation, le dessin sort de la bibliothèque, ainsi que ses placements
   sur la façade (la fenêtre dit combien). Le fichier reste dans le dossier
   du projet ; réimporter le même dossier le remet dans la bibliothèque.

## Ranger la bibliothèque

- **Renommer un dessin** : double-clic sur sa vignette, ou le crayon à côté
  de son nom dans le panneau de droite ; ↩ valide, Échap annule. Le fichier
  garde son nom.
- **Ranger des dessins** : les glisser sur un dossier de la colonne de
  gauche. ⌘-clic ajoute un dessin au choix, ⇧-clic choisit tous ceux entre
  deux ; on glisse alors tout le choix. Lâchés sur « Tous les dessins », ils
  sortent de tout dossier.
- **Dossiers** : « + » en haut de la colonne crée un dossier (dans le dossier
  choisi) ; double-clic pour le renommer ; clic droit pour renommer, créer
  un sous-dossier ou retirer un dossier vide.
- ⌘Z annule chaque rangement. Rien ne bouge sur le disque : dans le Finder,
  `bibliotheque/` garde le rangement de l'import (D-4.1-g).

### Essai sur Mac (version 0.1.72)

1. Ouvrir un projet avec une bibliothèque (Trévarez, par exemple), étape
   Bibliothèque.
2. Cliquer « + » en haut de « Dossiers » : « Nouveau dossier » apparaît, prêt
   à être renommé. Taper « Essai », ↩.
3. Choisir trois dessins (clic, puis ⌘-clic sur deux autres), les glisser
   sur « Essai » : le dossier compte 3, un message le confirme.
4. Double-clic sur une vignette, taper un autre nom, ↩ : le nom change dans
   la grille, dans l'onglet Dessins de la Façade et dans l'arborescence.
5. Double-clic sur « Essai », le renommer : les dessins suivent.
6. ⌘Z plusieurs fois : tout revient comme avant.
7. Fermer et rouvrir le projet : le rangement est gardé.

## Bibliothèque commune

Des dessins partagés par tous les projets, rangés dans Dropbox (D-4.1-i).

- **La choisir** : menu InSitu → **Réglages…** (⌘,), « Choisir un
  dossier… ». Un dossier ordinaire reçoit « Bibliothèque commune.insitu » ;
  un dossier `.insitu` existant sert tel quel. Chaque Mac fait ce choix une
  fois (le dossier Dropbox peut avoir un chemin différent sur l'autre Mac).
- **La remplir** : dans la Bibliothèque, vue **Commune** (en haut à droite),
  « Importer un dossier… » comme pour un projet ; ou, dans la vue **Ce
  projet**, choisir des dessins puis « Envoyer dans la bibliothèque
  commune ».
- **S'en servir** : vue Commune, choisir des dessins, « Copier dans le
  projet ». Ils arrivent dans le projet avec leurs retouches et leur
  silhouette, étiquetés « Commune ».
- **Quand un dessin commun change** (redessiné, retouché), sa copie dans un
  projet est étiquetée « Nouvelle version » ; rien ne change tant qu'on ne
  clique pas « Reprendre la nouvelle version ». Renommer ou ranger un
  dessin commun ne compte pas comme un changement.

### Essai sur Mac (version 0.1.74)

1. ⌘, : la fenêtre Réglages s'ouvre. « Choisir un dossier… », prendre un
   dossier de Dropbox. Le chemin affiché finit par « Bibliothèque
   commune.insitu ».
2. Bibliothèque, vue « Commune » : elle est vide. Importer un petit dossier
   de dessins.
3. Ouvrir un projet (Trévarez), vue « Commune », choisir deux dessins,
   « Copier dans le projet ». Vue « Ce projet » : ils y sont, étiquetés
   « Commune », dans le même dossier.
4. Vue « Commune », retoucher un de ces dessins à la gomme. Vue « Ce
   projet » : sa copie dit « Nouvelle version ». « Reprendre la nouvelle
   version » : la retouche apparaît, le dessin reste posé sur la façade.
5. Vue « Ce projet », choisir un dessin propre au projet, « Envoyer dans la
   bibliothèque commune » : il apparaît dans la vue Commune, dans un
   dossier au nom du projet.

## Vecteurs (tâche 5.2)

Chaque dessin est vectorisé juste après l'import (une ligne montre
l'avancement ; on peut déjà parcourir la bibliothèque) : son trait devient
des courbes, qui serviront à l'export SVG (tâche 5.3). Sous l'aperçu, la rubrique
**Vecteurs** donne le nombre de contours, de morceaux (droites et courbes)
et l'**écart** maximal entre les courbes et l'image, en pixels du dessin.
Sous 0,5 px, le vecteur suit l'image.

**Comparer image et vecteurs…** ouvre la comparaison en plein écran :

- **Image**, **Vecteurs** (courbes remplies de blanc), **Contour sur
  l'image** (courbes en rouge par-dessus les pixels) et **Rideau** (image à
  gauche, vecteurs à droite ; glisser la poignée jaune) ;
- molette, pincement ou boutons **+** / **−** pour zoomer, glisser pour se
  déplacer, **Ajuster** pour revoir tout le dessin, **100 %** pour un pixel
  du dessin par pixel d'écran. À partir de 200 %, on voit les pixels du
  fichier ; le contour rouge doit longer le bord du trait ;
- à droite, les réglages **du dessin** : seuil (plus bas, les gris pâles
  entrent dans le trait), **lissage** (adoucit le bord du trait avant le
  tracé : efface le tremblement de la main et les petites bosses ; 0 = le
  trait tel quel), **réduction des points** (l'ancienne « tolérance » : plus
  grande, moins de points et tracé plus simple, mais moins fidèle), angle
  des coins, aire minimale (retire les poussières). Seuil, lissage et
  réduction des points ont un curseur ; la ligne sous l'aperçu donne le
  nombre de morceaux, pour voir l'effet.
  **Appliquer** refait les vecteurs et enregistre le réglage dans le projet ;
  **Réglages par défaut** revient au réglage commun ;
- **Fermer** ou la touche Échap pour revenir à la bibliothèque.

Les vecteurs sont rangés dans le cache de l'app, pas dans le projet : sur
l'autre Mac, ils sont refaits la première fois qu'on choisit le dessin
(quelques dixièmes de seconde à quelques secondes). Seuls les réglages
propres à un dessin voyagent avec le projet.

Importer deux fois le même dossier n'ajoute rien. Un fichier modifié depuis
le premier import arrive à côté de l'ancien, avec un suffixe (`merle-2.png`).
Le fichier copié dans le projet n'est jamais modifié.

## Silhouettes (tâche 4.7)

Chaque dessin a une **silhouette** : son contour extérieur, rempli de noir.
Elle est posée sous le dessin et cache ce qui est derrière lui : la fresque
et les dessins placés plus bas dans l'empilement. On le voit sur la façade,
dans la vue Diapositive et dans les PNG exportés.

L'app la trouve seule : le trait est épaissi d'environ 1 % du dessin pour
boucher les petites ouvertures, le fond est rempli depuis les bords, et tout
ce que le fond n'atteint pas est dans la silhouette, qui déborde d'un rien
autour du trait extérieur (D-4.7-a). Deux parties d'un dessin très proches
l'une de l'autre sont réunies.

Dans l'inspecteur, la section **Silhouette** dit si elle est détectée ou
retouchée. **Retoucher la silhouette…** ouvre le dessin en grand avec son
contour en rouge :

- tirer un point pour le déplacer ;
- double-clic sur un bord : un point de plus ; ailleurs : un nouveau
  contour carré, à façonner ;
- ⌥-clic sur un point : le retirer (un contour de moins de trois points
  disparaît) ;
- **Résultat** montre la silhouette en noir sous le dessin, sur le fond
  choisi (damier ou pierre ; sur le noir, un fond rayé) : ce qui reste
  visible du fond n'est pas caché ;
- **Enregistrer** écrit la silhouette dans le projet ; **Détecter à
  nouveau** oublie les retouches ; **Aucune silhouette** (puis Enregistrer)
  : le dessin ne cache plus rien.

## Gomme et crayon (tâche 4.8)

Pour enlever d'un dessin des traits inutiles venus de l'original, ou en
ajouter : dans l'inspecteur, section **Gomme et crayon**, **Retoucher le
dessin…** ouvre le dessin en grand.

- **Gomme** (touche E) efface, **Crayon** (touche N) ajoute du blanc.
- **Trait** (T) peint le long du geste, à l'épaisseur choisie à droite
  (touches [ et ] pour l'affiner) ; le cercle autour du pointeur montre
  l'épaisseur. **Zone** (Z) : entourer une partie, elle est effacée ou
  remplie au lâcher. Pratique pour enlever d'un coup tout un morceau.
- Chaque retouche s'enregistre seule, peu après le geste. ⌘Z retire la
  dernière, ⇧⌘Z la remet ; **Revenir au dessin d'origine** les retire toutes
  (⌘Z les remet aussi).
- Espace + glisser déplace la vue ; molette ou pincement : zoom.

Le fichier importé n'est jamais modifié. L'app fabrique à côté le dessin
retouché (dossier `retouches/` du projet) et s'en sert partout : vignette,
façade, vecteurs, silhouette détectée, vue Diapositive, simulation, fiche et
export (D-4.8-a à d). Un dessin SVG retouché devient une image de 4 096 px
(D-4.8-c).

## En ligne de commande

```
insitu library import Épinal.insitu "Dessins de Marc" --cache /tmp/vignettes
insitu library list Épinal.insitu --cache /tmp/vignettes
insitu library remove Épinal.insitu merle
insitu library thumbnail dessin.psd vignette.png --size 1024
insitu library vectors Épinal.insitu --cache /tmp/vignettes
insitu library vectors Épinal.insitu merle --cache /tmp/vignettes --threshold 90
insitu library vectors Épinal.insitu merle --cache /tmp/vignettes --default
insitu library silhouette Épinal.insitu merle --cache /tmp/vignettes --mask merle-silhouette.png
insitu library silhouette Épinal.insitu merle --cache /tmp/vignettes --from contours.json
insitu library silhouette Épinal.insitu merle --cache /tmp/vignettes --none
insitu library silhouette Épinal.insitu merle --cache /tmp/vignettes --default
insitu library retouch Épinal.insitu merle
insitu library retouch Épinal.insitu merle --from retouches.json --out merle-retouche.png
insitu library retouch Épinal.insitu merle --clear
```

`retouch` affiche les retouches à la gomme et au crayon et le fichier lu à
la place de l'original ; `--from` les remplace (liste JSON, voir
`docs/format-projet.md`), `--clear` revient au dessin d'origine, `--out`
écrit le dessin retouché.

`silhouette` affiche la silhouette en JSON ; `--mask` l'écrit aussi en image
(blanc dedans), pour la comparer au dessin.

Sans nom de dessin, `vectors` donne un tableau de toute la bibliothèque
(contours, morceaux, écart, durée) : pratique pour repérer les dessins à
regarder de près.

## Essai sur Mac (version 0.1.6)

1. Ouvrir un projet, étape **Bibliothèque**.
2. Importer un vrai dossier de dessins de Marc, de préférence avec des
   sous-dossiers et au moins un PSD et un BMP. Noter combien de temps dure
   l'import (la barre avance) et vérifier qu'aucun fichier n'est signalé
   illisible à tort.
3. Faire défiler toutes les vignettes : elles doivent être déjà là, sans
   attente ni trou.
4. Chercher un nom, puis un nom de dossier ; cliquer un dossier à gauche.
5. Cliquer un dessin : l'aperçu apparaît à droite. Changer le fond (damier,
   noir, pierre) : seul le trait doit se voir, sans halo noir autour.
6. Importer une seconde fois le même dossier : « Aucun nouveau dessin,
   N déjà présents ».
7. Fermer l'app, la rouvrir : la bibliothèque est toujours là.

## Essai sur Mac des vecteurs (tâche 5.2)

Critère du plan : dix dessins de styles différents validés à l'œil. Idéalement
avec les dessins de Marc : crayon scanné, Procreate, traits fins et épais,
avec et sans gris, un PSD, un SVG.

1. Importer le dossier des dix dessins (ou le réimporter : les dessins déjà
   présents sont vectorisés à leur premier choix).
2. Pour chaque dessin : le choisir, lire la ligne **Vecteurs** (l'écart
   doit être sous 0,5 px), puis **Comparer image et vecteurs…**.
3. En **Contour sur l'image**, zoomer à 400 % ou plus sur des endroits
   délicats : traits très fins, croisements, pointes, gris. Le rouge doit
   longer le bord du blanc partout. Passer en **Rideau** pour comparer
   l'allure générale.
4. Sur un dessin avec des gris pâles : baisser le seuil (par exemple 60),
   **Appliquer**, et vérifier que les gris apparaissent dans les vecteurs ;
   puis **Réglages par défaut**.
5. Fermer l'app, la rouvrir : un dessin réglé garde son réglage (« réglages
   propres » sous l'aperçu).
6. Noter les dessins dont le résultat ne convient pas, avec ce qui ne va pas.

## Essai sur Mac de la gomme et du crayon (tâche 4.8)

1. Étape **Bibliothèque**, choisir un dessin avec un trait en trop.
2. **Retoucher le dessin…** ; gomme, trait : effacer le trait en trop. Le
   cercle du pointeur doit avoir la taille du trait effacé ; le geste doit
   suivre la souris sans retard, même zoomé.
3. Passer en **Zone** et entourer un morceau : il disparaît au lâcher.
4. **Crayon** : ajouter un trait, puis une zone remplie.
5. ⌘Z deux fois, puis ⇧⌘Z : les retouches partent et reviennent.
6. **Fermer** : la vignette et l'aperçu montrent le dessin retouché ; la
   ligne « Gomme et crayon » compte les retouches.
7. Étape **Façade** : le dessin posé est retouché ; l'export des
   diapositives (et le SVG) aussi.
8. Fermer l'app, la rouvrir : les retouches sont toujours là.
9. Dire si les épaisseurs proposées conviennent et si la gomme doit aussi
   pouvoir effacer « en dur » (sans bord adouci).

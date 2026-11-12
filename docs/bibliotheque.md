# Bibliothèque de dessins (tâche 4.1)

La bibliothèque rassemble les dessins du projet : des traits blancs sur fond
noir, en PNG, JPEG, TIFF, BMP, PSD (image aplatie) ou SVG. Ils seront posés
sur la façade à l'étape suivante (placement, tâche 4.2).

## Dans l'app

1. Ouvrir ou créer le projet (menu **Projet**, en haut à gauche), puis
   aller dans l'étape **Bibliothèque** (⌘5).
2. **Importer un dossier…** : choisir un dossier de dessins. Tout son contenu
   est copié dans le projet, sous-dossiers compris, dans
   `bibliotheque/<nom du dossier>/`. Une barre montre l'avancement ; à la
   fin, une phrase dit combien de dessins sont arrivés, combien étaient déjà
   là, et quels fichiers sont illisibles.
3. À gauche, les dossiers (avec le nombre de dessins) ; au milieu, les
   vignettes ; à droite, l'aperçu du dessin choisi et sa fiche (dossier,
   taille, fichier).
4. **Chercher** : chaque mot doit se trouver dans le nom du dessin ou dans
   son dossier, sans tenir compte des accents ni des majuscules (« oiseau
   mer » trouve `Oiseaux/Mer/goéland`).
5. **Fond** : le noir des dessins est rendu transparent. Damier, noir ou
   pierre montrent ce qu'il y a derrière ; sur le noir, le dessin est tel que
   dans son fichier.
6. Les flèches du clavier passent d'un dessin à l'autre (cliquer d'abord
   dans les vignettes).

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
  entrent dans le trait), tolérance (plus grande, tracé plus lisse et plus
  léger), angle des coins, aire minimale (retire les poussières).
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

## En ligne de commande

```
insitu library import Épinal.insitu "Dessins de Marc" --cache /tmp/vignettes
insitu library list Épinal.insitu --cache /tmp/vignettes
insitu library thumbnail dessin.psd vignette.png --size 1024
insitu library vectors Épinal.insitu --cache /tmp/vignettes
insitu library vectors Épinal.insitu merle --cache /tmp/vignettes --threshold 90
insitu library vectors Épinal.insitu merle --cache /tmp/vignettes --default
```

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

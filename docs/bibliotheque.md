# Bibliothèque de dessins (tâche 4.1)

La bibliothèque rassemble les dessins du projet : des traits blancs sur fond
noir, en PNG, JPEG, TIFF, BMP, PSD (image aplatie) ou SVG. Ils seront posés
sur la façade à l'étape suivante (placement, tâche 4.2).

## Dans l'app

1. Ouvrir ou créer le projet dans l'onglet **Façade**, puis aller dans
   l'onglet **Bibliothèque**.
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

Importer deux fois le même dossier n'ajoute rien. Un fichier modifié depuis
le premier import arrive à côté de l'ancien, avec un suffixe (`merle-2.png`).
Le fichier copié dans le projet n'est jamais modifié.

## En ligne de commande

```
insitu library import Épinal.insitu "Dessins de Marc" --cache /tmp/vignettes
insitu library list Épinal.insitu --cache /tmp/vignettes
insitu library thumbnail dessin.psd vignette.png --size 1024
```

## Essai sur Mac (version 0.1.6)

1. Ouvrir un projet, onglet **Bibliothèque**.
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

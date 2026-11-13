# Vue façade (tâche 3.3)

La vue façade montre ensemble, sur le plan, tous les projecteurs recalés :
leur **emprise** (le contour de la diapositive sur la façade), leur **photo
redressée** posée en transparence, et les **zones de recouvrement** entre
deux projecteurs, avec leur surface.

## Avant de commencer

- Le plan de la façade est importé et mis à l'échelle (étape **Façade**).
- Chaque projecteur à montrer est redressé (étape **Redressement**) puis
  recalé avec au moins 4 paires de points (étape **Recalage**).

## Dans l'app

1. Étape **Façade**. Le panneau **Projecteurs**, à gauche, se remplit à
   l'ouverture du projet. Après un recalage fait dans une autre étape,
   cliquer la flèche circulaire de son en-tête (ou ⌘R).
2. Chaque projecteur a sa couleur, celle de son emprise. À côté : le nombre
   de points du recalage et la surface couverte en m². Un projecteur pas
   encore recalé est grisé, avec ce qui lui manque.
3. La case devant un projecteur le masque ou le remontre (photo, emprise et
   recouvrements).
4. Les photos, les emprises, les recouvrements et l'**opacité des photos**
   (100, 80, 50 ou 25 %) se règlent dans le menu **Affichage** (point 6),
   plus dans le panneau de gauche (version 0.1.66).
5. En bas du panneau, la liste des **recouvrements** : quels projecteurs, et
   quelle surface. Sur le plan, ils sont en rouge.
6. Menu **Affichage**, en haut à droite (à côté de « Tout voir ») : montrer
   ou cacher **le plan**, **la fresque**, **les dessins posés**, les photos,
   les emprises et les recouvrements ; **Plan en négatif** (le plan
   d'architecte en traits blancs sur fond noir, comme les dessins) ; **Tout
   en négatif** (fond blanc, traits noirs, comme une planche Illustrator ;
   les couleurs des repères gardent leur teinte). Quand des dessins sont
   cachés, on ne peut ni les choisir ni en poser : le menu le rappelle par
   son nombre, « Affichage (1) ». **Tout montrer, sans négatif** remet tout.
   Ces choix ne changent pas le projet ni l'export ; ils sont retenus d'un
   lancement à l'autre.

## Sans écran

```
insitu overview Mon-projet.insitu
```

écrit, pour chaque projecteur, ses points, sa surface et son problème
éventuel, puis les recouvrements et leur surface.

## Dans un simple navigateur

`npm run dev` montre dix projecteurs inventés (et un onzième non recalé)
pour juger de l'écran. Leurs recouvrements y sont approchés par des
rectangles ; dans l'app, c'est le moteur qui les calcule exactement.

## Limites connues

- Les photos sont des aperçus de 1 024 px (voir D-3.3-b) : pour juger d'un
  détail fin, utiliser l'étape Recalage.
- La surface d'un recouvrement est mesurée sur une grille fine (D-3.3-d),
  à environ 1/400 près.

# Vue façade (tâche 3.3)

La vue façade montre ensemble, sur le plan, tous les projecteurs recalés :
leur **emprise** (le contour de la diapositive sur la façade), leur **photo
redressée** posée en transparence, et les **zones de recouvrement** entre
deux projecteurs, avec leur surface.

## Avant de commencer

- Le plan de la façade est importé et mis à l'échelle (onglet **Façade**).
- Chaque projecteur à montrer est redressé (onglet **Redressement**) puis
  recalé avec au moins 4 paires de points (onglet **Recalage**).

## Dans l'app

1. Onglet **Façade**. Le panneau **Projecteurs**, à droite, se remplit à
   l'ouverture du projet. Après un recalage fait dans l'autre onglet,
   cliquer **Relire**.
2. Chaque projecteur a sa couleur, celle de son emprise. À côté : le nombre
   de points du recalage et la surface couverte en m². Un projecteur pas
   encore recalé est grisé, avec ce qui lui manque.
3. La case devant un projecteur le masque ou le remontre (photo, emprise et
   recouvrements).
4. **Photos**, **Emprises**, **Recouvrements** : montrer ou masquer chaque
   famille. **Opacité des photos** : de transparent à plein.
5. En bas du panneau, la liste des **recouvrements** : quels projecteurs, et
   quelle surface. Sur le plan, ils sont en rouge.

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
  détail fin, utiliser l'onglet Recalage.
- La surface d'un recouvrement est mesurée sur une grille fine (D-3.3-d),
  à environ 1/400 près.

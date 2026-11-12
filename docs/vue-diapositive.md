# Vue diapositive (tâche 3.6)

La vue diapositive montre, pour un projecteur, la **diapositive calculée**
(ce qui sera gravé) posée sur sa **photo redressée**. Si le recalage est
bon, les dessins tombent sur l'architecture visible dans la photo : arcs,
colonnes, fenêtres.

## Avant de commencer

- Le projecteur est redressé (onglet Redressement) et recalé (onglet
  Recalage, 4 paires de points au moins).
- La fresque est importée (onglet Façade, section Fresque).

## Dans l'app

1. Onglet **Diapositive**. L'écran reprend le projet ouvert ; **Relire**
   recalcule après un changement fait ailleurs.
2. En haut, un bouton par projecteur ; grisé s'il manque quelque chose (le
   message dit quoi). Les **flèches ← →** du clavier passent au projecteur
   précédent ou suivant.
3. La photo redressée remplit le carré de 280 mm ; les dessins sont
   par-dessus, en **cyan** par défaut. La liste propose d'autres couleurs ;
   le curseur règle leur opacité ; **Dessins** les masque.
4. Zoomer pour juger d'un détail. L'aperçu fait 2 048 px (0,14 mm par
   pixel), comme le détail utile d'une photo redressée.

Si un dessin tombe à côté de l'élément visé, corriger le recalage (onglet
Recalage) : la vue se met à jour avec **Relire**.

## Sans écran

```
insitu slides Mon-projet.insitu --cache /tmp/apercus
```

écrit l'aperçu de chaque diapositive dans le dossier donné et décrit le
résultat en JSON.

## Ce qui est vérifié, et ce qui ne l'est pas

- Vérifié par les tests : l'aperçu est exactement l'export à basse
  résolution ; il n'est refait que si la fresque ou le recalage change.
- Vérifié dans un navigateur : la bascule entre projecteurs, la couleur et
  l'opacité, sur des images inventées.
- **Pas vérifié** : le critère du plan, « les dessins tombent sur
  l'architecture visible dans la photo », demande un vrai projet (photos
  redressées, recalage, fresque) : essai de Benjamin sur son Mac.

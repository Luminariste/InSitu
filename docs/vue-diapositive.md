# Vue diapositive (tâche 3.6)

La vue diapositive montre, pour un projecteur, la **diapositive calculée**
(ce qui sera gravé) posée sur sa **photo redressée**. Si le recalage est
bon, les dessins tombent sur l'architecture visible dans la photo : arcs,
colonnes, fenêtres.

## Avant de commencer

- Le projecteur est redressé (étape Redressement) et recalé (étape
  Recalage, 4 paires de points au moins).
- La fresque est importée (étape Façade, inspecteur à droite, rubrique Fresque).

## Dans l'app

1. Étape **Diapositive** (⌘4). L'écran reprend le projet ouvert ; menu
   **Projet** → **Relire depuis le disque** (⌘R) recalcule après un
   changement fait ailleurs.
2. À gauche, la liste des projecteurs ; une pastille orange s'il manque
   quelque chose (la ligne et le bandeau disent quoi). Les **flèches ← →**
   (ou ↑ ↓ dans la liste) passent au projecteur précédent ou suivant.
3. La photo redressée remplit le carré de 280 mm ; les dessins sont
   par-dessus, en **cyan** par défaut. Le groupe **Dessins** de la barre
   d'outils propose d'autres couleurs, règle leur opacité, et la case
   **Montrer** les masque.
4. Zoomer pour juger d'un détail. L'aperçu fait 2 048 px (0,14 mm par
   pixel), comme le détail utile d'une photo redressée.

Si un dessin tombe à côté de l'élément visé, corriger le recalage (étape
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

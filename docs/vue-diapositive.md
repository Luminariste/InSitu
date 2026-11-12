# Vue diapositive (tâches 3.6 et 4.6)

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

## Retoucher un dessin dans une diapositive (tâche 4.6)

Quand le recalage ne suffit pas (façade très en relief), on retouche un
dessin posé **dans cette diapositive seulement**, sur la photo : la façade et
les autres diapositives ne changent pas.

1. Cliquer un dessin : son contour apparaît en bleu, et le groupe
   **Retouche** dans la barre d'outils.
2. Le glisser le décale dans la diapositive. **Quatre coins** ou **Grille**
   donne des points bleus à tirer pour l'épouser sur la photo.
3. À chaque geste lâché, la retouche est enregistrée et l'aperçu recalculé ;
   l'export la reprend.
4. **Réinitialiser** retire la retouche : le dessin revient où la façade le
   place. ⌘Z annule chaque geste.

### Essai sur Mac

1. Ouvrir un projet recalé avec des dessins posés, étape Diapositive.
2. Cliquer un dessin, le glisser de quelques mm : après le geste, l'aperçu
   montre le dessin décalé ; l'étape Façade, elle, n'a pas bougé.
3. « Quatre coins », tirer un coin ; exporter cette diapositive : la retouche
   est dans le PNG (LightBurn). Les autres diapositives sont identiques.
4. « Réinitialiser » : l'aperçu et l'export reviennent comme avant.

## Sans écran

```
insitu slides Mon-projet.insitu --cache /tmp/apercus
```

écrit l'aperçu de chaque diapositive dans le dossier donné et décrit le
résultat en JSON.

## Ce qui est vérifié, et ce qui ne l'est pas

- Vérifié par les tests : l'aperçu est exactement l'export à basse
  résolution, dessins posés compris ; il n'est refait que si la fresque, le
  recalage ou un dessin posé change.
- Vérifié dans un navigateur : la bascule entre projecteurs, la couleur et
  l'opacité, sur des images inventées.
- **Pas vérifié** : le critère du plan, « les dessins tombent sur
  l'architecture visible dans la photo », demande un vrai projet (photos
  redressées, recalage, fresque) : essai de Benjamin sur son Mac.

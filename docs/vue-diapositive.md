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
   **Fichier** → **Relire depuis le disque** (⌘R) recalcule après un
   changement fait ailleurs.
2. À gauche, la liste des projecteurs ; une pastille orange s'il manque
   quelque chose (la ligne et le bandeau disent quoi). Les **flèches ← →**
   (ou ↑ ↓ dans la liste) passent au projecteur précédent ou suivant.
3. La photo redressée remplit le carré de 280 mm ; les dessins sont
   par-dessus, en **cyan** par défaut.
4. Menu **Affichage** (en haut à droite) : montrer ou cacher la photo
   redressée, la fresque, le plan de la Façade vu par ce projecteur (en
   gris bleuté, pour se repérer) et les dessins ; le fond sans la photo
   (**Noir**, **Blanc** : les dessins blancs passent alors en noir,
   **Pierre**) ; la couleur et l'opacité des dessins. La fresque et le plan
   sont les mêmes cases que le menu Affichage de la Façade. Le bouton
   **Comme dans LightBurn** montre la diapositive telle qu'elle part dans
   LightBurn : traits blancs pleins sur fond noir, sans photo ; un second
   clic revient à l'affichage d'avant. Ces réglages sont retenus d'un
   lancement à l'autre.
5. À droite, trois onglets, comme dans la Façade, limités à cette
   diapositive : **Dessins** (la bibliothèque, à glisser sur la
   diapositive, voir plus bas), **Calques** (les dessins de cette
   diapositive, rangés comme dans Illustrator, et les calques), **Export**.
6. Onglet **Export** : les réglages de l'onglet Export de la Façade
   (résolution, fresque gravée ou dessins seuls, plaque, miroir, négatif,
   1 bit, SVG), puis **Exporter cette diapositive…** (⌘E) écrit le PNG de
   la diapositive affichée seulement. **Plan de contrôle…** : le plan vu
   par ce projecteur, à graver pour vérifier le recalage (voir
   `docs/export.md`). L'export ne grave jamais le plan.
7. Zoomer pour juger d'un détail. L'aperçu fait 2 048 px (0,14 mm par
   pixel), comme le détail utile d'une photo redressée.

Si un dessin tombe à côté de l'élément visé, corriger le recalage (étape
Recalage) : la vue se met à jour avec **Relire**.

## Outils des dessins dans une diapositive (tâche 4.6, D-3.6-f)

La barre du haut a **les mêmes outils que la Façade**. Ceux qui déplacent ou
déforment agissent **dans cette diapositive seulement**, sur la photo : la
façade et les autres diapositives ne changent pas (c'est la « retouche » du
dessin dans la diapositive, tâche 4.6). Ceux qui changent le dessin lui-même
agissent partout, comme dans la Façade ; leur bulle le dit.

- Dans cette diapositive seulement : glisser le dessin, le cadre et ses
  poignées (coin : agrandir, ⇧ garde les proportions ; côté : étirer ; ⌥ :
  depuis le centre ; rond au-dessus du cadre : tourner, ⇧ par pas de 5°),
  quart de tour, **mode 4 coins** (Échap pour revenir au cadre), **grille**,
  aligner, répartir, flèches du clavier. **Redresser** (l'icône, bulle
  « Comme sur la façade ») retire la retouche : le dessin revient tel qu'il
  est calé sur la façade.
- Partout (façade et toutes les diapositives) : miroir, ordre (⌘[ ⌘]),
  grouper (⌘G), dégrouper.
- Comme sur la façade, un clic ne prend que le dessin lui-même (sa
  silhouette), et **⌘-clic** répété descend d'un dessin à chaque clic
  (D-4.2-t).
- ⇧-clic ajoute ou retire un dessin de la sélection ; cliquer dans le vide
  désélectionne (les flèches ← → changent alors de projecteur).
- **⌘D** copie les dessins choisis, un peu décalés ; **⌥ + glisser** les
  copie là où on les lâche. La copie n'apparaît que dans cette
  diapositive : sur la façade, elle est sous l'original, affectée à la main
  à ce projecteur.

Pendant le geste, le dessin est montré en bleu ; une fois lâché, la retouche
est enregistrée, « Recalcul de la diapositive… » s'affiche en bas, puis
l'aperçu à jour remplace le bleu. L'export la reprend. ⌘Z annule chaque
geste.

## Poser un dessin dans une diapositive (D-3.6-f)

Dans un projecteur **recalé**, onglet **Dessins** à droite : glisser un dessin
sur la diapositive (ou double-cliquer : il est posé au centre de la vue). Il
est posé sur la façade, là où ce projecteur l'éclaire, droit, sur le calque
actif, affecté à cette diapositive ; on le retrouve dans l'étape Façade.

Dans un projecteur **pas encore recalé** (D-3.6-g), on peut aussi poser des
dessins : ils sont posés **dans cette diapositive seulement**, sur la photo
redressée, sans passer par la Façade (pratique avec un seul projecteur).
L'onglet Dessins le rappelle. Tous les outils de la barre marchent sur eux,
et l'onglet Export grave la diapositive avec ces seuls dessins (ni fresque,
ni plan de contrôle sans recalage). Ils ne se voient pas dans la Façade ; si
le projecteur est recalé plus tard, ils restent où ils sont dans la
diapositive, et les dessins de la façade s'y ajoutent. Retirer le
projecteur retire aussi ses dessins.

### Essai sur Mac

1. Ouvrir un projet recalé avec des dessins posés, étape Diapositive.
2. Cliquer un dessin, le glisser de quelques mm : après « Recalcul de la
   diapositive… », l'aperçu montre le dessin décalé ; l'étape Façade, elle,
   n'a pas bougé. Tirer un coin du cadre, tourner par le rond : même chose.
3. ⇧-clic sur un deuxième dessin, **Aligner en haut** : les deux s'alignent
   dans la diapositive seulement.
4. **Mode 4 coins**, tirer un coin ; exporter cette diapositive (onglet
   Export) : la retouche est dans le PNG (LightBurn). Les autres diapositives
   sont identiques.
5. **Redresser** (« Comme sur la façade ») : l'aperçu revient comme avant.
6. ⌘D, puis ⌥ + glisser un dessin : une copie apparaît à chaque fois, dans
   cette diapositive seulement.
7. Onglet **Dessins** : glisser un dessin sur la photo. Il apparaît à
   l'endroit lâché ; dans l'étape Façade, il est posé au bon endroit.
8. Onglet **Calques** : seuls les dessins de cette diapositive sont listés ;
   cliquer une ligne choisit le dessin sur la diapositive.
9. Un projecteur **non recalé** (avec sa photo redressée) : glisser un
   dessin sur la photo, le tourner, l'agrandir ; onglet Export, **Exporter
   cette diapositive…** : le PNG contient le dessin, à la place voulue.
   L'étape Façade ne le montre pas.

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
- Vérifié par les tests : les outils de la diapositive (déplacer, étirer,
  tourner, aligner, répartir, grille, copie) et la pose d'un dessin lâché
  dans une diapositive (il tombe sur la façade au point voulu).
- Vérifié dans un navigateur : la bascule entre projecteurs, le cadre, les
  poignées, le quart de tour et le mode 4 coins, sur des images inventées.
- **Pas vérifié** : le critère du plan, « les dessins tombent sur
  l'architecture visible dans la photo », demande un vrai projet (photos
  redressées, recalage, fresque) : essai de Benjamin sur son Mac.

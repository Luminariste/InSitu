# Fiche de calepinage (tâche 6.2)

Un PDF A3 couché à imprimer et à emporter sur le site : où va chaque
projecteur, quelle diapositive il porte, avec quels réglages.

## Dans l'app

Étape **Façade**, inspecteur à droite, rubrique **Export des diapositives** :
**Fiche de calepinage…**, puis choisir où l'enregistrer (nom proposé :
`<projet> - calepinage.pdf`). Les réglages d'export inscrits sur la fiche
sont ceux de la rubrique à ce moment-là.

## Ce qu'elle contient

- **Page 1, le plan** : le plan de la façade éclairci ; l'emprise de chaque
  projecteur dans sa couleur, avec son nom ; les recouvrements en tirets
  rouges ; le contour de chaque dessin posé dans la couleur de sa
  diapositive (rouge s'il est coupé, gris en tirets s'il est hors emprise).
  À droite : la liste des diapositives (nombre de dessins, nom du fichier
  d'export), les réglages d'export, les alertes de la simulation
  (`docs/simulation.md`) et la légende.
- **Pages suivantes, les diapositives**, six par page : la vignette de la
  diapositive calculée (traits noirs sur blanc pour l'impression), son nom,
  son fichier d'export, le nombre de points de recalage, la photo redressée
  et la liste de ses dessins. Un projecteur pas encore recalé a une case
  grise qui dit ce qui manque.

Imprimer en A3, « taille réelle » ou « ajuster à la page » : la fiche n'a
pas de cote à mesurer, seulement des repères.

## Sans écran

```
insitu sheet Mon-projet.insitu calepinage.pdf --cache /tmp/insitu-cache
```

## Ce qui est vérifié, et ce qui ne l'est pas

- Vérifié par les tests : le PDF fait 420 × 297 mm et se relit avec le
  moteur de rendu des plans (le même genre de lecteur qu'Aperçu) ; le plan
  et les emprises sont sur la page 1 ; sept projecteurs donnent trois pages
  (six puis un) ; la vignette d'une diapositive montre son dessin ; aucun
  texte n'est sous 3 mm de corps ; les lettres accentuées passent.
- Vu par Claude sur une fiche d'essai rendue en image (fichiers du projet,
  `captures/6.2/`).
- **Pas vérifié** : l'impression réelle en A3 et la lecture sur site ; le
  rendu dans Aperçu sur Mac ; un projet réel avec beaucoup de dessins (les
  noms trop nombreux sont coupés par « … »).

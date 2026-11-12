# Export des diapositives (tâche 3.5)

L'export calcule, pour chaque projecteur recalé, l'image de sa diapositive :
la partie de la fresque qu'il doit projeter, déformée pour tomber juste sur
la façade. Un PNG par projecteur, prêt pour LightBurn.

## Avant de commencer

- Le plan de la façade est importé et **mis à l'échelle** (étape Façade).
- Chaque projecteur à exporter est **recalé** (au moins 4 paires de points,
  étape Recalage).
- La fresque est composée dans Illustrator **sur la façade entière**, sur
  un plan de travail de la taille de la façade, et exportée en une image
  (PNG ou TIFF, traits blancs sur fond noir).

## Dans l'app

1. Étape **Façade**, inspecteur à droite, rubrique **Fresque** :
   **Importer la fresque…**. L'image est copiée dans le projet et posée sur
   toute la largeur de la façade, à partir du coin haut-gauche. Elle
   s'affiche sur le plan ; la case **Montrer** la masque.
2. Vérifier qu'elle tombe bien : les dessins doivent être à leur place sur
   le plan et dans les emprises des projecteurs.
3. Rubrique **Export des diapositives** : régler
   - **Résolution** (1 400 dpi par défaut ; Épinal a été gravé à 1 000) ;
   - **Plaque de 300 mm** : décochée par défaut, le PNG fait 280 mm,
     l'image seule, comme les verres d'Épinal. Cochée : il couvre toute la
     plaque, l'image de 280 mm au centre, avec le nom et les repères gravés
     dans la marge de 10 mm ;
   - **Miroir**, **Négatif** ;
   - **Noir et blanc pur (1 bit)** et son **seuil** ; sinon niveaux de gris.
   La taille du PNG s'affiche dessous.
4. **Exporter les diapositives…** (⌘E), choisir un dossier. Un fichier par
   projecteur : `<projet>-<diapositive>.png`. Un projecteur pas encore
   recalé est signalé, pas exporté. Les réglages sont gardés dans le projet.

Compter une quinzaine de secondes par diapositive à 1 400 dpi.

## Dans LightBurn

Importer le PNG : il doit mesurer **280 mm** (ou **300 mm** avec la plaque)
sans rien régler, car la résolution est inscrite dans le fichier.

## Sans écran

```
insitu fresco Mon-projet.insitu fresque.png
insitu export Mon-projet.insitu Gravure/ --dpi 1000 --no-margin
```

`insitu export --help` donne toutes les options.

## Ce qui est vérifié, et ce qui ne l'est pas

- Vérifié par les tests : le vrai Verre-A, posé sur une façade puis
  réexporté à 1 400 dpi, ressort à la même place à 2 µm près, avec tous ses
  traits ; le PNG annonce 280 mm (300 mm avec la marge) ; repères et nom
  restent dans la marge.
- **Pas vérifié** : l'ouverture dans LightBurn elle-même, et une gravure
  réelle. Pas encore essayé sur une vraie fresque composée sur la façade.

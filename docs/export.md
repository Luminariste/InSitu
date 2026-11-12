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

1. Étape **Façade**, panneau de droite, onglet **Plan**, rubrique **Fresque** :
   **Importer la fresque…**. L'image est copiée dans le projet et posée sur
   toute la largeur de la façade, à partir du coin haut-gauche. Elle
   s'affiche sur le plan ; la case **Montrer** la masque.
2. Vérifier qu'elle tombe bien : les dessins doivent être à leur place sur
   le plan et dans les emprises des projecteurs.
3. Onglet **Export**, rubrique **Export des diapositives** : régler
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

Compter une quinzaine de secondes par diapositive à 1 400 dpi, un peu plus
avec beaucoup de dessins posés.

## Les dessins posés dans l'app

Les dessins posés sur la façade (étape Façade, `docs/composition.md`)
partent aussi dans les diapositives, chacun depuis son fichier d'origine en
pleine résolution, avec sa taille, sa rotation et sa symétrie. Les dessins
d'un calque masqué ne partent pas. Un projet peut n'avoir que des dessins
posés, sans fresque. Depuis la tâche 4.7, chaque dessin cache, sous sa
silhouette (`docs/bibliotheque.md`), la fresque et les dessins placés
derrière lui ; ses traits s'ajoutent ensuite. Un dessin sans silhouette ne
cache rien : là où il croise la fresque ou un autre dessin, le plus clair
l'emporte. La vue Diapositive (⌘4) les montre de la même façon.

Depuis la tâche 4.5, chaque dessin posé ne part que dans **une** diapositive,
celle que montre sa couleur sur la façade : il n'est ni coupé entre deux
projecteurs, ni projeté deux fois là où deux emprises se recouvrent
(`docs/composition.md`, « Diapositive de chaque dessin »). La fresque
importée en une image, elle, reste découpée entre les diapositives.

## SVG en millimètres (tâche 5.3)

Cocher **SVG en plus (courbes des dessins posés)** : à côté de chaque PNG,
un `<projet>-<diapositive>.svg` de 280 mm (300 mm avec la plaque). Il
contient les **courbes des dessins posés**, celles qu'on voit dans
« Comparer image et vecteurs… » de la Bibliothèque, transportées vers la
diapositive sans repasser par des pixels : même place, même taille, même
déformation, même retouche de diapositive, même miroir que dans le PNG.

- Les traits sont des surfaces **remplies de noir**, sans fond ; un dessin
  par chemin, nommé d'après son placement.
- Un dessin placé derrière un autre est **découpé par la silhouette** de
  celui de devant, comme dans le PNG.
- La **fresque importée en une image** n'est pas dans le SVG (elle n'a pas
  de courbes), ni le nom et les repères de la marge.
- Un **cadre rouge** de 280 mm (trait de 0,1 mm, sans remplissage) marque
  le bord de la diapositive. Dans LightBurn il arrive sur son propre calque
  (rouge) : le laisser en « Sortie » désactivée pour ne pas le graver.
- Le **négatif** et le **noir et blanc pur** ne s'appliquent qu'au PNG.
- Compter quelques secondes de plus par diapositive : chaque dessin est
  vectorisé avec ses réglages de la Bibliothèque.

## Dans LightBurn

Importer le PNG : il doit mesurer **280 mm** (ou **300 mm** avec la plaque)
sans rien régler, car la résolution est inscrite dans le fichier.
LightBurn peut afficher **279,999 mm** : c'est normal (D-5.3-g), 280 mm
tombant entre deux pixels ; l'écart est d'un millième de millimètre. Le SVG
aussi s'ouvre à sa taille (ses dimensions sont écrites en mm). Les réglages
conseillés pour graver le SVG viendront avec l'essai de la tâche 5.4.

## Sans écran

```
insitu fresco Mon-projet.insitu fresque.png
insitu export Mon-projet.insitu Gravure/ --dpi 1000 --no-margin
insitu export Mon-projet.insitu Gravure/ --svg
```

`insitu export --help` donne toutes les options.

## Ce qui est vérifié, et ce qui ne l'est pas

- Vérifié par les tests : le vrai Verre-A, posé sur une façade puis
  réexporté à 1 400 dpi, ressort à la même place à 2 µm près, avec tous ses
  traits ; le PNG annonce 280 mm (300 mm avec la marge) ; repères et nom
  restent dans la marge. Un dessin posé sort à l'identique de la même image
  importée en fresque sur le même rectangle (à 2 niveaux de gris près),
  miroir compris ; un dessin d'un calque masqué ne sort pas ; un dessin hors
  de la diapositive n'est pas lu.
- Vérifié par les tests (SVG, tâche 5.3) : à 1 400 dpi, les courbes du SVG
  posées sur le PNG de la même diapositive s'en écartent de moins de
  0,02 mm (dessin tourné et déformé, avec et sans miroir) ; les courbes
  transportées suivent la chaîne exacte à 0,004 mm près ; un dessin derrière
  un autre est découpé par sa silhouette.
- **Pas vérifié** : l'ouverture dans LightBurn elle-même (PNG et SVG), et
  une gravure réelle. Pas encore essayé sur une vraie fresque composée sur la façade.

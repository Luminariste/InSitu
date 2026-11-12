# InSitu — plan d'implantation

Ce fichier est le plan de référence pour les agents. Il est mis à jour dans la
même demande de fusion que le changement qui le motive. Le document Claude
« Atelier de calepinage — plan d'implantation » est la version d'origine, du
11 novembre 2026 ; il n'est plus tenu à jour.

## État d'avancement

Au 12 novembre 2026. Version publiée : **0.1.1** (visionneuse et images
d'Épinal embarquées). Chaque demande de fusion qui termine une tâche met ce
tableau à jour.

| Tâche | État |
| --- | --- |
| 1.1 Squelette du dépôt | Faite, validée par l'essai de Benjamin sur Mac |
| 1.2 Fabrication automatique | Faite, validée : les versions 0.1.0 et 0.1.1 s'installent et s'ouvrent |
| 1.3 Jeu d'essai | Faite |
| 1.4 Modèle de projet | Faite ; le verrou n'a pas encore été essayé entre deux Mac par Dropbox |
| 1.5 Visionneuse haute résolution | Fusionnée, **non validée** : voir ci-dessous |
| 2.1 Étude de détection | Faite ; ses propositions sont intégrées à ce plan |
| 2.2 Nouvelle mire | Fusionnée ; PDF dans `docs/mire/`. **Non validée** : il reste à l'imprimer, la mesurer à 280 mm et relire les 16 repères sur une photo |
| 2.3 Géométrie | Faite ; seuils réglés sur simulation, à ajuster sur données réelles en 2.4a et 3.2 |
| 5.1 Moteur de vectorisation | Faite : écart de 0,33 px au plus sur les quatre verres d'Épinal (critère : 0,5 px) ; reste à l'essayer sur des dessins de Marc (5.2) |
| 2.5 Écran de redressement | Fusionnée (version 0.1.4), **non validée** : essai sur Mac à faire par Benjamin (`docs/redressement.md`) ; un HEIC d'iPhone récent reste à essayer |
| 3.2 Recalage par points | Fusionnée (version 0.1.5 ; points déplaçables au glisser depuis la 0.1.7, demande de Benjamin), **non validée** : essai sur Mac à faire par Benjamin (`docs/recalage.md`) ; seuil des points aberrants réglé sur simulation, à reprendre sur ses premiers points cliqués ; photo « lum » pas encore prise en charge |
| 4.1 Bibliothèque | Fusionnée (version 0.1.6), **non validée** : essai sur Mac à faire par Benjamin (`docs/bibliotheque.md`), si possible sur un vrai dossier de dessins de Marc |
| 3.3 Vue façade | Fusionnée (version 0.1.8), **non validée** : essai sur Mac à faire par Benjamin (`docs/vue-facade.md`) ; le critère « dix projecteurs sans ralentissement » ne se vérifie que sur son Mac |
| 3.5 Export des diapositives | Fusionnée (version 0.1.9), **non validée** : ouverture dans LightBurn à faire par Benjamin (`docs/export.md`) ; export à 280 mm par défaut, choix de Benjamin (D-3.5-a) |
| 3.6 Vue diapositive | Fusionnée (version 0.1.10), **non validée** : essai sur Mac à faire par Benjamin (`docs/vue-diapositive.md`) sur un vrai projet recalé avec une fresque |
| Toutes les autres | À faire |

**Ce qui bloque la validation de la tâche 1.5.** Dans la version 0.1.1,
l'app installée affiche bien les images sur le Mac de Benjamin. Sa mesure
« Dernier geste » : 6 images, 39,0 ms en moyenne, 66,0 ms au pire, 6 en
retard, calcul 1,5 ms. Cette mesure est trompeuse : la scène ne se redessine
qu'à la demande, donc l'écart entre deux images suit le rythme des gestes
autant que la fluidité. Une correction existe sur la branche
`tache-1.5-mesure` (redessin à chaque rafraîchissement d'écran pendant un
geste). Elle n'a été essayée ni dans un navigateur ni sur Mac, et n'a ni
demande de fusion ni version. Suite : la vérifier, la fusionner, publier une
0.1.2, et redemander à Benjamin son impression (fluide ou saccadé) avec le
nouveau chiffre. Tant que ce n'est pas fait, le risque « fluidité de
l'affichage » reste ouvert.

**Tâches prêtes à lancer :** 2.4a, 3.4, 5.1, et la correction de 1.5.

## Résumé

L'app remplace Millumin et le travail de déformation manuel sous Illustrator :
on compose la fresque une seule fois sur la façade, et chaque diapositive de
280 mm est calculée automatiquement, déformations en trapèze comprises.

Choix structurants :

- **App de bureau Mac en technologies web (Tauri)** : interface en
  React/TypeScript, moteur de calcul en Rust. Une app Swift native est exclue,
  car une session cloud tourne sous Linux et ne peut pas la compiler.
- **Moteur de calcul séparé de l'interface**, pilotable en ligne de commande.
  Les agents peuvent ainsi le tester dans le cloud sur les vrais fichiers.
- **Fabrication de l'app Mac par GitHub** : une version se publie toute seule
  dès que le numéro de version change sur `main` (`docs/publier.md`).
- **Dépôt privé `Luminariste/InSitu`**, sans aucun lien avec PRISMA.
- **Six jalons**, dans l'ordre des priorités : recalage des photos,
  déformation automatique, composition, puis vectorisation, contrôle et
  finitions. Chaque jalon est utilisable sur un vrai projet.

## Le métier, en bref

Luminariste projette des fresques sur des façades avec des rétroprojecteurs.
Chaque rétroprojecteur reçoit une diapositive carrée : une plaque de verre de
300 mm dont l'image utile fait 280 mm, gravée au laser fibre via LightBurn. Les
dessins sont des traits blancs sur fond noir ; seul le dessin est gravé, pour
laisser passer la lumière. Un projet compte souvent 5 projecteurs, parfois 15,
découpés en zones de 10 au plus, avec recouvrement entre projecteurs voisins.

Sur site, pour chaque projecteur, on pose une mire papier de 280 mm sur la
vitre, on couvre l'ensemble d'un tissu (chambre noire) et l'image de la façade
se forme sur le papier. On photographie ce papier à l'iPhone. La photo,
redressée sur la mire, montre la façade telle que le projecteur la voit, en
coordonnées de diapositive. Les projecteurs sont en biais : la déformation est
au minimum un trapèze, et la façade a du relief.

## Principe de fonctionnement

Tout repose sur une correspondance calculée, pour chaque rétroprojecteur,
entre un point de la façade et un point de la diapositive. Elle se construit
en deux recalages.

1. **Photo vers diapositive.** L'app détecte la mire dans la photo et redresse
   l'image. On obtient la façade vue par le projecteur, en millimètres de
   diapositive.
2. **Diapositive vers façade.** L'utilisateur clique des points communs entre
   cette image redressée et le plan. Quatre points donnent le trapèze ; les
   points suivants corrigent le relief.

Ensuite, tout dessin posé sur la façade est transporté vers la bonne
diapositive, sans intervention.

### Précision

- **La détection s'appuie sur toute la grille de la mire**, pas sur quatre
  coins : cela corrige le papier qui gondole et la déformation de l'objectif
  de l'iPhone. Mesuré sur les 8 photos d'Épinal par l'étude 2.1 : le
  redressement à la main laisse les croisements à 0,6 à 0,9 mm de leur place
  en moyenne ; la grille entière donne 0,08 à 0,15 mm.
- **La mire actuelle se détecte déjà** (8 photos sur 8, amorce par le cercle
  rouge et la croix). Elle reste utilisable.
- **Une nouvelle mire ajoute 16 repères codés** (AprilTag 36h11 de 16 mm, en
  4 × 4) à la grille de 10 mm, à la croix et au cercle rouge, avec des traits
  épaissis à 0,35 mm. Les repères servent à trouver la mire, son sens et son
  identité quand la photo est mauvaise ; la précision vient toujours de la
  grille. Détail et chiffres : `docs/etudes/2.1-detection-mire.md`.
- **L'écart résiduel est affiché** en millimètres sur la diapositive et en
  centimètres sur la façade, pour chaque projecteur.
- **La photo fait foi.** Dans la vue d'une diapositive, chaque dessin reste
  ajustable à la main par-dessus la photo redressée (déplacement, quatre
  coins, grille). Cet ajustement local ne modifie pas la fresque d'ensemble.

L'écart constaté aujourd'hui est de 5 à 10 cm sur la façade. La nouvelle mire
traite le papier et l'objectif ; la position de la plaque dans le projecteur
et la mise au point restent mécaniques. L'objectif chiffré sera fixé après le
premier essai réel.

### Préconisations de prise de vue

- iPhone en photo, pas en vidéo : objectif principal 1x, résolution maximale,
  format HEIF ou ProRAW. À Épinal, les photos de chambre noire font 12 Mpx et
  la mire n'y occupe que 1 790 à 1 950 px : viser 24 ou 48 Mpx si l'iPhone
  l'accepte dans le noir.
- Appareil le plus perpendiculaire possible à la mire, exposition verrouillée,
  mire entièrement visible avec ses quatre repères.
- Deux ou trois photos par projecteur ; l'app garde la meilleure détection.

## Architecture technique

| Couche | Technologie | Rôle |
| --- | --- | --- |
| Moteur | Rust (bibliothèque + outil en ligne de commande) | Géométrie, détection de mire, déformation, vectorisation, exports pleine résolution |
| Coque | Tauri 2 | Fenêtre Mac, menus, accès aux fichiers, pont entre interface et moteur |
| Interface | React, TypeScript, affichage WebGL | Vue façade, vues diapositive, bibliothèque, outils de transformation |

### Haute résolution

- Une diapositive de 15 433 px en niveaux de gris pèse environ 240 Mo en
  mémoire. Dix diapositives en parallèle restent confortables sur un M1 Max.
- À l'écran, l'app travaille sur des pyramides de tuiles : fluide à tout
  niveau de zoom, quelle que soit la taille des images.
- La pleine résolution n'est calculée qu'à l'export, par le moteur, sur tous
  les cœurs. Chaque dessin est rééchantillonné une seule fois depuis son
  fichier source.
- La résolution d'export est réglable (1 400 dpi par défaut, soit 15 433 px).

### Fichier projet

- Un dossier `.insitu` contenant `projet.json`, lisible, et les images, avec
  des chemins relatifs. Il se partage par Dropbox. Format décrit dans
  `docs/format-projet.md`.
- Pas d'édition simultanée : un verrou avertit si l'autre personne a le projet
  ouvert. Il avertit, il ne garantit rien (Dropbox le transmet avec retard).
- Écriture atomique ; un projet plus ancien est mis à niveau à l'ouverture, un
  projet plus récent que l'app est refusé.
- Annulation illimitée et enregistrement automatique : à faire avec la
  composition (tâche 4.3).

### Formats

| Usage | Formats |
| --- | --- |
| Plan ou photo de façade | PDF, PNG, JPEG, TIFF, HEIC, SVG, DXF. Le DWG est à convertir en PDF ou DXF avant import |
| Dessins | PNG, BMP, TIFF, PSD aplati, SVG |
| Composition existante | Fichier Illustrator `.ai` : positions des images et liens récupérés (jalon 6) |
| Export gravure | PNG 1 bit ou niveaux de gris, SVG en millimètres, options miroir et négatif |
| Documents | Mire imprimable en PDF, fiche de calepinage en PDF |

### Vectorisation et silhouettes

Chaque dessin est vectorisé une fois à l'entrée en bibliothèque, à pleine
résolution, avec un seuil réglable et un aperçu. Les courbes sont ensuite
transportées mathématiquement vers chaque diapositive, sans repasser par des
pixels. Les niveaux de gris sont seuillés.

Chaque dessin reçoit aussi une silhouette : son contour extérieur, détecté
automatiquement et rempli de noir, pour qu'un dessin placé devant en masque un
autre (tâche 4.7).

## Jalons

Chaque jalon se termine par une app téléchargeable, essayée sur un vrai projet
avant de lancer le suivant.

1. **Fondations.** Dépôt, fabrication automatique de l'app Mac, squelette qui
   s'ouvre, jeu d'essai.
2. **Mire et redressement.** Détection automatique de la mire actuelle puis
   de la nouvelle mire à repères codés, correction manuelle, export de la
   photo redressée. *Millumin n'est plus nécessaire.*
3. **Façade et déformation automatique.** Import du plan, recalage de chaque
   projecteur par points, emprises sur la façade, import de la fresque en une
   image, export des diapositives PNG déformées. *La composition reste dans
   Illustrator, mais sur la façade entière, sans déformation manuelle.*
4. **Composition.** Bibliothèque, placement et transformations, déformation
   quatre coins et grille, calques, affectation automatique, ajustement par
   diapositive, silhouettes. *Illustrator devient facultatif.*
5. **Vectorisation et SVG.** Vectorisation en bibliothèque, export SVG par
   diapositive, essai de gravure vectorielle dans LightBurn.
6. **Contrôle et finitions.** Simulation de tous les projecteurs, alertes,
   fiche PDF, import de fichiers Illustrator, assemblage de photos de façade.

## Tâches pour agents

Une tâche correspond à une session, une branche `tache-N.N` et une demande de
fusion. Les tâches d'un même jalon sans dépendance entre elles peuvent être
lancées en parallèle. Les tâches « moteur » sont vérifiées automatiquement ;
les tâches « interface » demandent un essai sur Mac.

### Jalon 1 — Fondations

| N° | Tâche | Livrable | Critère d'acceptation | Dépend de |
| --- | --- | --- | --- | --- |
| 1.1 | Squelette du dépôt | Projet Tauri + React + moteur Rust vide, `CLAUDE.md` décrivant règles et vocabulaire | L'app se lance en développement ; tests et vérifications passent | — |
| 1.2 | Fabrication automatique | Workflow GitHub qui fabrique et publie l'app Mac (Apple Silicon) quand le numéro de version change sur `main` | Un fichier `.dmg` téléchargeable s'ouvre sur le M1 Max | 1.1 |
| 1.3 | Jeu d'essai | Projet d'Épinal (4 projecteurs) : photos brutes, photos redressées, verres, façade, mire ; références pleine résolution allégées et versions réduites | Les tests du moteur lisent ces fichiers ; les fichiers d'origine lourds restent hors du dépôt (voir décision D7) | 1.1 |
| 1.4 | Modèle de projet | Format du dossier `.insitu`, lecture, écriture, versions, verrou | Un projet enregistré puis rouvert est identique ; test automatique | 1.1 |
| 1.5 | Visionneuse haute résolution | Composant d'affichage à tuiles (PixiJS) : zoom, déplacement, superposition de calques avec opacité, repère en millimètres | Une image de 15 433 px se manipule sans saccade sur le M1 Max, d'après l'impression de Benjamin et une mesure au rythme de l'écran | 1.1 |

### Jalon 2 — Mire et redressement

| N° | Tâche | Livrable | Critère d'acceptation | Dépend de |
| --- | --- | --- | --- | --- |
| 2.1 | Étude de détection | Comparaison de deux ou trois types de repères sur des photos simulées (flou, biais, faible lumière, papier gondolé) ; choix argumenté | Rapport court avec taux de détection et précision mesurée | 1.3 |
| 2.2 | Nouvelle mire | Générateur de mire PDF 280 mm : grille, croix, cercle rouge, 16 repères codés AprilTag 36h11, identifiant (étude 2.1, candidat B) | PDF imprimé mesuré à 280 mm ; les 16 repères sont relus sur une photo de la mire imprimée ; l'ancienne mire reste utilisable | 2.1 |
| 2.3 | Géométrie | Moteur : transformation en trapèze (4 points), déformation par grille de points, calcul d'écart résiduel | Tests numériques ; aller-retour exact à moins de 0,01 mm | 1.1 |
| 2.4a | Détection et redressement (grille) | Moteur : amorce par le cercle rouge et la croix (ancienne mire), mesure des croisements de la grille, trapèze et correction locale, image redressée 280 mm, carte de confiance | Sur les 8 photos d'Épinal, la détection réussit sans aide et les croisements de l'image redressée tombent à moins de 0,3 mm de leur place (95e centile) | 2.1, 2.3 |
| 2.4b | Détection par repères codés | Moteur : amorce par les 16 repères de la nouvelle mire, puis même mesure de la grille que 2.4a | Sur des photos de la mire imprimée, mêmes critères que 2.4a | 2.2, 2.4a |
| 2.5 | Écran de redressement | Import de photos (HEIC compris), détection, poignées de correction (quatre coins posés à la main servent d'amorce, la grille affine ensuite), loupe, export TIFF ou PNG | Une photo réelle est redressée en moins d'une minute | 1.5, 2.4a |

### Jalon 3 — Façade et déformation automatique

| N° | Tâche | Livrable | Critère d'acceptation | Dépend de |
| --- | --- | --- | --- | --- |
| 3.1 | Import du plan | Import PDF, image, SVG, DXF ; mise à l'échelle par deux points et une cote | Un plan PDF d'architecte s'affiche net à tout zoom, à l'échelle | 1.4, 1.5 |
| 3.2 | Recalage par points | Vue double (photo redressée / façade), paires de points, écart affiché par point ; la photo « lum » peut servir de repère large | 4 points donnent le trapèze ; les suivants réduisent l'écart ; un point aberrant est signalé ; seuil d'aberrance réglé sur des points réels | 2.3, 3.1 |
| 3.3 | Vue façade | Emprise de chaque projecteur sur le plan, photos recalées en transparence, zones de recouvrement | Dix projecteurs affichés ensemble sans ralentissement | 3.2 |
| 3.4 | Moteur de déformation | Déformation pleine résolution par tuiles, multi-cœur, rééchantillonnage de qualité pour traits fins | Une diapositive de 15 433 px en moins de 30 s ; traits blancs sans crénelage ni épaississement | 2.3 |
| 3.5 | Export des diapositives | Import de la fresque en une image, export PNG par diapositive : résolution, miroir, négatif ; nom et repères dans la marge de 10 mm de la plaque de 300 mm | Le PNG s'ouvre dans LightBurn à 280 mm ; comparaison avec Verre-A | 3.3, 3.4 |
| 3.6 | Vue diapositive | Diapositive calculée superposée à la photo redressée, bascule rapide entre projecteurs | Les dessins tombent sur l'architecture visible dans la photo | 3.5 |

### Jalon 4 — Composition

| N° | Tâche | Livrable | Critère d'acceptation | Dépend de |
| --- | --- | --- | --- | --- |
| 4.1 | Bibliothèque | Import de dossiers de dessins, vignettes, recherche, fond noir rendu transparent, liens relatifs | 200 dessins de 4 096 px importés et parcourus sans attente | 1.4 |
| 4.2 | Placement | Glisser-déposer sur la façade, déplacer, tourner, redimensionner, dupliquer, symétrie, alignement, groupes | Les gestes courants d'Illustrator fonctionnent avec les mêmes raccourcis | 3.3, 4.1 |
| 4.3 | Calques et historique | Calques, verrouillage, masquage, ordre, annulation illimitée | Toute action s'annule ; l'état survit à la fermeture | 4.2 |
| 4.4 | Déformation libre | Quatre coins et grille de déformation sur un dessin ou un groupe | Un dessin épouse une voussure ; export identique à l'aperçu | 3.4, 4.2 |
| 4.5 | Affectation | Proposition automatique dessin vers diapositive (aucun dessin coupé), correction manuelle, couleur par diapositive | Sur la composition d'essai, aucun dessin n'est à cheval ; un changement manuel est conservé | 4.2 |
| 4.6 | Ajustement par diapositive | Dans la vue diapositive, retouche locale d'un dessin sur la photo, sans toucher à la fresque | L'ajustement apparaît à l'export et se réinitialise d'un clic | 3.6, 4.4 |
| 4.7 | Silhouette et superposition | Détection du contour extérieur de chaque dessin, fond noir à l'intérieur, retouche manuelle du contour ; un dessin au-dessus masque ceux du dessous | Deux dessins superposés : celui de devant cache l'autre à l'écran, dans le PNG et dans le SVG | 4.1, 4.3 |

### Jalon 5 — Vectorisation et SVG

| N° | Tâche | Livrable | Critère d'acceptation | Dépend de |
| --- | --- | --- | --- | --- |
| 5.1 | Moteur de vectorisation | Tracé des contours à pleine résolution, seuil réglable, lissage maîtrisé | Écart maximal entre vecteur et image source inférieur à un demi-pixel | 1.3 |
| 5.2 | Vecteurs en bibliothèque | Vectorisation à l'import, aperçu comparatif image / vecteur, réglage par dessin | Dix dessins de styles différents validés visuellement | 4.1, 5.1 |
| 5.3 | Export SVG | Transport des courbes vers la diapositive, SVG en millimètres, miroir | Le SVG superposé au PNG de la même diapositive coïncide à moins de 0,02 mm | 3.5, 5.2 |
| 5.4 | Essai LightBurn | Procédure d'import et réglages conseillés, allègement du nombre de points si nécessaire | Une plaque gravée en vectoriel, comparée à une plaque gravée en image | 5.3 |

### Jalon 6 — Contrôle et finitions

| N° | Tâche | Livrable | Critère d'acceptation | Dépend de |
| --- | --- | --- | --- | --- |
| 6.1 | Simulation | Façade avec tous les projecteurs superposés ; alertes : dessin coupé, hors emprise, doublon en recouvrement | Chaque alerte mène au dessin concerné | 4.5 |
| 6.2 | Fiche de calepinage | PDF : plan avec emprises, liste des diapositives, vignettes, réglages d'export | Imprimable en A3, lisible sur site | 6.1 |
| 6.3 | Import Illustrator | Lecture d'un `.ai` : images placées, positions, liens | Composition A recréée à l'identique dans l'app | 4.2 |
| 6.4 | Assemblage de photos | Recalage de plusieurs photos de façade entre elles par points | Trois photos assemblées servent de fond de plan | 3.2 |
| 6.5 | Finitions | Préférences, raccourcis, messages d'erreur clairs, aide intégrée, icône | Marc installe et utilise l'app sans explication | tout |

## Méthode de travail

- **Dépôt séparé.** `Luminariste/InSitu` ne partage rien avec PRISMA : ni
  code, ni session, ni réglages.
- **Une tâche, une session ou un agent.** Chacun lit `CLAUDE.md`, ce plan et
  `docs/decisions.md`, travaille sur sa branche `tache-N.N` et ouvre une
  demande de fusion. Plusieurs agents peuvent travailler en parallèle sur des
  tâches sans dépendance entre elles ; un orchestrateur relit et fusionne.
- **Fusion déléguée.** Benjamin a délégué les fusions : une demande est
  fusionnée dès que les vérifications GitHub sont au vert. Ce qui demande son
  essai sur Mac lui est signalé à part.
- **Vérification avant fusion.** Les tests tournent sur GitHub. Pour le
  moteur, ils comparent les résultats aux fichiers de référence. Une seconde
  session relit les tâches sensibles (géométrie, export).
- **Essai sur Mac.** Benjamin essaie l'app installée, téléchargée depuis
  l'onglet « Releases », pas depuis le code : tout écran à essayer doit donc
  fonctionner dans une version publiée. Sans
  compte développeur Apple, macOS bloque le premier lancement ; la marche à
  suivre est dans `docs/installer.md`.
- **Journal de décisions.** Chaque choix technique non prévu ici est noté dans
  `docs/decisions.md`.

Limite : une session cloud ne voit pas l'app tourner sur un Mac. Les défauts
d'affichage ou de fluidité ne se découvrent qu'à l'essai.

## Risques

| Risque | Conséquence | Parade |
| --- | --- | --- |
| Détection de mire peu fiable en conditions réelles (nuit, encre, plis, reflets) | Retour au placement manuel | L'étude 2.1 réussit sur les 8 photos d'Épinal, toutes de jour ; restent les 9 essais sur site listés dans son rapport ; correction manuelle toujours disponible |
| Façade très en relief | Le recalage par points ne suffit pas partout | Ajustement par diapositive sur la photo (4.6), qui fait foi |
| SVG trop lourd pour LightBurn (millions de points) | Import lent ou gravure plus longue qu'en image | Essai 5.4 ; l'export PNG reste la voie sûre |
| Fluidité de l'affichage dans la fenêtre web | Composition moins agréable qu'Illustrator | **Ouvert** : l'app affiche bien les images sur Mac, mais la fluidité n'est pas encore jugée (voir « État d'avancement ») ; repli possible : interface Swift sur le même moteur Rust |
| Lecture des fichiers Illustrator | Import partiel | Placé en dernier ; l'import d'une image aplatie fonctionne dès le jalon 3 |
| App non signée | Avertissement macOS au premier lancement | Accepté |
| Poids du dépôt | Clonage plus lent pour chaque session | Pas de Git LFS possible ; ne verser au jeu d'essai que ce qu'un test utilise |
| Lecteur de repères codés jeune et lourd (`kornia-apriltag`) | Fabrication de l'app compliquée | Vérifier sa compilation sur Mac au début de 2.4b ; repli : lecteur limité à nos 16 repères |

## Décisions du 12 novembre 2026

- **Mire et détection** (étude 2.1) : nouvelle mire à 16 repères codés
  (2.2) ; la détection commence par l'ancienne mire (2.4a), les repères
  codés suivent (2.4b). Le critère de 2.4 ne compare plus à A.tiff, qui
  s'écarte lui-même de la grille de 0,6 mm en moyenne.
- **Pas de Git LFS** : références allégées dans le dépôt, originaux dans le
  Dropbox (décisions D7 et D8).
- **Publication** : par changement du numéro de version (décision D10).
- **Images de démonstration embarquées dans l'app** : provisoire, à retirer
  avec l'écran de démonstration quand l'app saura ouvrir un projet
  (décision D13).
- **Développement poursuivi dans le fil « IN SITU »** du projet Claude.

## Décisions du 11 novembre 2026

- **Résolution** : l'export reste à 1 400 dpi (15 433 px).
- **Niveaux de gris** : seuil simple ; pas de trame dans la première version.
- **Repères gravés** : nom et repères de calage dans la marge de 10 mm de la
  plaque de 300 mm, hors de la zone projetée.
- **Pilotage de LightBurn** : piste pour plus tard, à étudier après 5.4.
- **Silhouette des dessins** : tâche 4.7.
- **Nom** : l'app et le dossier projet s'appellent InSitu, comme le dépôt.

## À faire par Benjamin

- Essayer la version 0.1.2 quand elle sera publiée et dire si la visionneuse
  est fluide.
- Imprimer la nouvelle mire (`docs/mire/`), vérifier 280 mm à la règle, et
  faire les essais listés à la fin de `docs/etudes/2.1-detection-mire.md`
  (de jour et de nuit).
- Confirmer le rôle des photos `x-mur`, `b-grille`, `img_5286`, et le
  projecteur des photos `img_5293`, `5295`, `5298`, `5302`
  (`jeu-essai/LISEZMOI.md`).
- Demander à Marc une vingtaine de dessins représentatifs (crayon scanné,
  Procreate, avec et sans gris) pour la vectorisation du jalon 5.

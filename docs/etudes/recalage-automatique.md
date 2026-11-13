# Ajustement automatique de la photo redressée sur le plan (étude, 13 novembre 2026)

Demande de Benjamin (13 novembre 2026) : dans l'onglet Recalage, un
algorithme qui replace tout seul la photo sur le plan. La façade n'est pas
toujours plate : plusieurs propositions, et la retouche à la main reste.

Deux formes ont été proposées à Benjamin : chercher la photo sur tout le
plan et proposer plusieurs places, ou seulement **ajuster** une pose faite
à la main. Il a choisi l'ajustement seul (13 novembre 2026). Décision :
D-3.2-i. Code : `crates/insitu-core/src/registration/auto.rs`, commande
`insitu registration adjust`, bouton « Ajuster sur le plan ».
Scripts de l'étude (Python, hors moteur) : non versionnés, la méthode
retenue est entièrement dans le module Rust.

## Ce qu'il faut comparer

D'un côté la photo redressée : la façade éclairée par le projecteur, vue à
travers la mire (carré de 280 mm), avec la grille de la mire par-dessus.
De l'autre le plan d'architecte : un dessin au trait, schématique (à
Trévarez, le plan PDF ne montre ni pierres ni ombres). Les deux n'ont en
commun que la **forme** : bords des fenêtres, des arcs, des toits. Les
couleurs, les contrastes et même le sens des contrastes (trait noir sur
blanc contre fenêtre sombre sur pierre claire) diffèrent.

Ce qui a été essayé :

| Méthode | Résultat |
| --- | --- |
| Points d'intérêt (SIFT) entre photo redressée et plan | Aucun appariement juste : le plan n'a pas de texture |
| SIFT entre photo redressée et photos d'assemblage (2024) | Aucun appariement juste : éclairage, saison et angle trop différents |
| **Orientation des contours** (retenue) | Cale une pose approchée à quelques dizaines de centimètres ; sur tout le plan, la bonne place n'est pas toujours la première |

## Méthode retenue (ajustement)

1. **Grille de la mire effacée** : fermeture (maximum puis minimum sur 3 px)
   de la photo réduite à 560 px ; les traits fins de la grille
   disparaissent, les fenêtres restent.
2. **Champ d'orientation** : en chaque pixel, la direction du contour
   (doublée, pour ignorer le sens du contraste), pondérée par la racine de
   sa force. Photo et plan deviennent comparables.
3. **Zone regardée** : le cadre de la pose à la main, élargi d'une
   demi-emprise de chaque côté, rendu à 1 024 px.
4. **Recherche locale** : la photo (96 px) est corrélée avec cette zone
   (transformée de Fourier, `rustfft`) à 5 tailles (0,8 à 1,25 fois la
   pose) et 5 angles (−6° à +6°). Les 3 meilleures places dont le centre est
   à moins d'un quart d'emprise de la pose servent de points de départ, en
   plus de la pose elle-même.
5. **Ajustement fin** : depuis chaque départ, chaque coin du trapèze est
   poussé tant que les contours s'accordent mieux (recherche par motifs ;
   trapèze convexe, surface entre 0,4 et 2,5 fois celle du départ, aucun
   coin à plus de 30 % du côté). Le meilleur l'emporte, seulement s'il fait
   mieux que la pose à la main et reste à moins d'un quart d'emprise
   d'elle. Sinon la photo ne bouge pas.

Le résultat : quatre paires aux coins de la diapositive, le même format que
« Poser la photo sur la façade ». Les points épinglés en plus sont
remplacés (ils corrigeaient l'ancienne pose).

## Chiffres sur Trévarez

Photos A et B (les deux qu'on sait redresser), plan PDF des écuries. Place
juste = vérifiée à l'œil (photo posée en transparence sur le plan) ; les
poses à quatre coins du projet de Benjamin sont faites à l'œil et ne servent
pas de vérité.

**Ajustement** (test `l_ajustement_cale_a_et_b_depuis_une_pose_approchee`) :
chaque photo est posée en carré à 5 endroits faux (décalée jusqu'à 80 cm,
côté de 9 à 10,5 m, sans rotation), puis ajustée.

| Pose de départ (décalage, côté) | A : écart du centre après | B : écart du centre après |
| --- | --- | --- |
| juste, 9,5 m | 8 cm | 17 cm |
| 80 cm à droite, 9 m | **1,21 m** | **1,03 m** |
| 80 cm à gauche et 30 cm bas, 10 m | 34 cm | 60 cm |
| 60 cm haut, 9,5 m | 12 cm | 31 cm |
| 50 cm à droite et en bas, 10,5 m | 6 cm | 23 cm |

Quatre poses sur cinq finissent à moins d'un mètre de la bonne place
(l'emprise fait 9 à 10 m de côté). Le cas raté s'arrête sur une place dont
l'accord des contours vaut celui de la bonne (0,359 contre 0,355 pour A) :
le plan ne permet pas de les départager. Appuyer une seconde fois sur
« Ajuster » ne l'améliore pas. Durée : environ 1,5 s par ajustement dans la
session cloud (non mesuré sur le Mac).

**Recherche sur tout le plan** (forme non retenue, pour mémoire) : la
bonne place sortait 1ʳᵉ à 3ᵉ parmi 5 quand la vue était cadrée, et
parfois pas du tout pour B, selon la taille supposée. Sur une façade aux
travées répétées, plusieurs places se valent.

Lecture :

- Partir d'une pose à la main lève le principal défaut de la recherche
  globale (les travées voisines identiques) : l'ajustement reste près de la
  pose.
- La précision finale est de quelques dizaines de centimètres sur une
  emprise de 9 à 10 m : un bon départ pour épingler, pas un recalage fini.
- La place est un **trapèze** : elle suppose la façade plate. Le relief
  (pli de Trévarez, avant-toit) se corrige ensuite à la main en épinglant
  des détails (D-3.2-h), comme avant.

## Limites, pistes

- Pas de vérité terrain précise : les places « justes » sont jugées à
  l'œil, à quelques dizaines de centimètres près.
- Seulement deux photos réelles essayées. À refaire sur Épinal et sur les
  photos C à G de Trévarez quand elles seront redressées.
- Piste : la photo « lum » (projecteur allumé avec la mire, vue de loin)
  donnerait la place directement ; elle n'est pas encore prise en charge.
- Piste : une photo de la façade (assemblage, tâche 6.4) comme fond au lieu
  du plan ressemblerait davantage à la photo redressée.

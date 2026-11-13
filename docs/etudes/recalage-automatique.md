# Placement automatique de la photo redressée sur le plan (étude, 13 novembre 2026)

Demande de Benjamin (13 novembre 2026) : dans l'onglet Recalage, un
algorithme qui replace tout seul la photo sur le plan. La façade n'est pas
toujours plate : plusieurs propositions, et la retouche à la main reste.

Décision : D-3.2-i. Code : `crates/insitu-core/src/registration/auto.rs`,
commande `insitu registration propose`, bouton « Placer automatiquement ».
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
| **Orientation des contours** (retenue) | Bonne place trouvée parmi les premières propositions, pas toujours en première |

## Méthode retenue

1. **Grille de la mire effacée** : fermeture (maximum puis minimum sur 3 px)
   de la photo réduite à 560 px ; les traits fins de la grille
   disparaissent, les fenêtres restent.
2. **Champ d'orientation** : en chaque pixel, la direction du contour
   (doublée, pour ignorer le sens du contraste), pondérée par la racine de
   sa force. Photo et plan deviennent comparables.
3. **Recherche globale** : la photo (96 px) est glissée sur toute la zone du
   plan (448 px) par corrélation (transformée de Fourier, `rustfft`), à 14
   tailles et 5 angles (−6° à +6°). On garde les 8 meilleurs pics de chaque
   essai.
4. **Ajustement fin** : les 10 meilleures places distinctes sont affinées en
   déplaçant chaque coin du trapèze (recherche par motifs), contraintes :
   trapèze convexe, surface entre 0,4 et 2,5 fois celle du départ, aucun
   coin à plus de 30 % du côté de son point de départ.
5. **Propositions** : jusqu'à 5 places, la plus ressemblante d'abord ; deux
   places dont les centres sont à moins de 30 % du côté l'une de l'autre
   n'en font qu'une. Chaque place = quatre paires aux coins de la
   diapositive, le même format que « Poser la photo sur la façade ».

La taille de l'emprise est inconnue pour le premier projecteur (on cherche
de 8 % à 60 % du grand côté de la zone) ; dès qu'un autre projecteur est
recalé, on cherche entre la moitié et le double de sa taille moyenne.

La zone de recherche est la partie de la façade **visible à l'écran** :
cadrer la zone du projecteur aide beaucoup (voir les chiffres).

## Chiffres sur Trévarez

Photos A et B (les deux qu'on sait redresser), plan PDF des écuries. Place
juste = vérifiée à l'œil (photo posée en transparence sur le plan) ; les
poses à quatre coins du projet de Benjamin sont faites à l'œil et ne servent
pas de vérité (B y semble mal placé).

| Cas | A | B |
| --- | --- | --- |
| Tout le plan, taille tirée des autres projecteurs (projet du 13 nov.) | 3ᵉ | 3ᵉ |
| Façade du haut cadrée, même taille (projet du 13 nov.) | **1ʳᵉ** | 2ᵉ |
| Façade du haut cadrée, taille inconnue (test automatique) | 2ᵉ | 3ᵉ |
| Façade du haut cadrée, taille 9 m (test automatique) | **1ʳᵉ** | absente |
| Façade du haut cadrée, taille tirée du projet du 12 nov. (test) | non mesuré | absente |

Durée : 4,5 à 8 s par recherche sur les 4 cœurs de la session cloud (le Mac
M1 Max devrait être plus rapide ; non mesuré).

Lecture :

- La méthode trouve des places « du bon genre » (une fenêtre sur une
  fenêtre, un arc sur un arc). Sur une façade **répétitive** (travées
  identiques), plusieurs places se ressemblent presque autant : la bonne
  n'est pas toujours la première. D'où les propositions à parcourir.
- B (l'arc central) est fragile : selon la taille supposée, la bonne place
  sort 2ᵉ, 3ᵉ ou pas du tout. Une fausse indication de taille fait plus de
  mal qu'aucune.
- La place proposée est un **trapèze** : elle suppose la façade plate. Le
  relief (pli de Trévarez, avant-toit) se corrige ensuite à la main en
  épinglant des détails (D-3.2-h), comme avant.

## Limites, pistes

- Pas de vérité terrain précise : les places « justes » sont jugées à
  l'œil, à ± 1 m environ sur une emprise de 9 à 10 m.
- Seulement deux photos réelles essayées. À refaire sur Épinal et sur les
  photos C à G de Trévarez quand elles seront redressées.
- Piste : la photo « lum » (projecteur allumé avec la mire, vue de loin)
  donnerait la place directement ; elle n'est pas encore prise en charge.
- Piste : une photo de la façade (assemblage, tâche 6.4) comme fond au lieu
  du plan ressemblerait davantage à la photo redressée.

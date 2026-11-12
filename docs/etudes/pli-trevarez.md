# Étude — Un projecteur à cheval sur un pli (Trévarez)

12 novembre 2026, à la demande de Benjamin, après lecture du projet des
écuries de Trévarez. Script : `etudes/pli/simuler.py` (numpy seulement).

## En bref

- **À Trévarez, un même projecteur éclaire deux surfaces à angle.** Sur la
  photo « A LUM », le projecteur A couvre à la fois le dessous de
  l'avant-toit en bois et le mur de brique. Le déroulé met ces deux surfaces
  à plat, l'une à côté de l'autre, le long de leur arête.
- **Le recalage actuel ne suit pas ce pli.** Il ajuste un seul trapèze, puis
  une correction lisse. Or, de part et d'autre de l'arête, la vraie
  correspondance est faite de deux trapèzes différents, avec une cassure à
  l'arête. En simulation, il reste 40 cm d'écart (95e centile) avec
  20 points cliqués, et encore 20 cm avec 40 points. Pour comparer, l'écart
  constaté à Épinal était de 5 à 10 cm.
- **Un recalage par surface règle le problème.** On trace l'arête sur la
  photo redressée et on clique 4 points au moins de chaque côté ; chaque
  côté reçoit son propre trapèze. En simulation, l'écart tombe à 10 cm avec
  20 points (bruit de clic compris), et à zéro sans bruit de clic.
- **Ce n'est pas encore vérifié sur une vraie photo** : le projecteur A
  n'est ni redressé ni recalé dans le projet de Benjamin.

## 1. Pourquoi un pli casse le modèle

La diapositive est projetée par un objectif, comme une photo à l'envers.
Sur une surface plane, la correspondance entre la diapositive et cette
surface à plat est exactement un trapèze (une homographie). Quand l'image
tombe sur deux plans, il y a deux trapèzes. Ils se recollent le long de
l'arête, qui est une droite dans la diapositive, mais ils n'ont pas la même
pente de part et d'autre.

Le moteur (`geometry/warp.rs`) ajuste **un** trapèze sur tous les points,
puis une spline en plaque mince qui passe par les points. Cette spline est
lisse par construction : elle arrondit la cassure, et l'erreur s'étale sur
toute la diapositive. Ajouter des points près de l'arête n'aide pas. Au
contraire, deux points proches mais de part et d'autre de la cassure forcent
la spline à tourner brusquement, et elle oscille ailleurs.

## 2. Simulation

Scène inspirée de la photo « A LUM » : un mur de 4 m de haut, un avant-toit
horizontal qui avance de 3 m, un projecteur posé à 1,2 m du sol et 5,5 m du
mur, qui vise l'arête. La diapositive couvre 5,8 m × 5,3 m du déroulé, dont
47 % sur l'avant-toit et 53 % sur le mur. Les points « cliqués » sont tirés
au hasard sur la diapositive, avec un bruit de clic de 0,3 mm sur la photo
redressée et de 2 cm sur le plan. On fait 30 tirages par ligne, et l'écart
est mesuré sur toute la diapositive (4 900 points).

| Points cliqués | Moteur actuel : p95 | max | Par surface : p95 | max |
| --- | --- | --- | --- | --- |
| 4 | 229 cm | 541 cm | impossible (il faut 4 points de chaque côté) | |
| 8 | 84 cm | 154 cm | 97 cm | 435 cm |
| 12 | 54 cm | 95 cm | 20 cm | 36 cm |
| 12, dont 4 près de l'arête | 100 cm | 178 cm | 19 cm | 34 cm |
| 20 | 39 cm | 78 cm | 10 cm | 19 cm |
| 20, dont 6 près de l'arête | 60 cm | 108 cm | 9 cm | 16 cm |
| 40, dont 10 près de l'arête | 20 cm | 43 cm | 6 cm | 11 cm |

Ce sont des médianes sur les 30 tirages. Avec 8 points, le recalage par
surface n'a souvent que 4 points sur un côté, placés au hasard : il
extrapole mal. Il faut 5 à 6 points bien répartis par surface.

Sans bruit de clic (`--sans-bruit`), le moteur actuel garde le même écart
(39 cm avec 20 points) : c'est l'erreur du modèle, pas des clics. Le
recalage par surface tombe alors à zéro.

## 3. Ce que cela demanderait

Proposition, à faire seulement avec l'accord de Benjamin :

1. Dans l'onglet Recalage, un outil « Arête » : on trace sur la photo
   redressée une ligne (ou une ligne brisée) qui sépare deux surfaces.
2. Les points cliqués sont rangés d'un côté ou de l'autre. Chaque côté a
   son propre recalage : un trapèze, plus la correction actuelle s'il a plus
   de 4 points.
3. L'export et la vue diapositive utilisent, pour chaque point de la
   diapositive, le recalage de son côté. L'emprise sur la façade devient
   deux morceaux.
4. Le format du projet garde les points actuels ; l'arête et le côté de
   chaque point sont ajoutés, et un projet sans arête reste lu comme avant.

Critère d'acceptation possible : sur le projecteur A de Trévarez, une fois
redressé, les dessins tombent sur l'avant-toit et sur le mur à moins de
10 cm, vérifié sur la photo « A LUM ».

En attendant, l'ajustement par diapositive (tâche 4.6) permet de retoucher
à la main un dessin à cheval sur l'arête. Il vaut mieux éviter de poser un
dessin à cheval sur un pli quand c'est possible.

## 4. Limites

- La scène est simulée : la hauteur du mur, l'avancée de l'avant-toit et la
  place du projecteur sont estimées d'après la photo « A LUM », pas
  mesurées.
- Le déroulé est supposé exact : chaque surface est mise à plat à la bonne
  échelle, et l'arête du déroulé est bien l'arête réelle.
- La photo brute de A est presque illisible (grille floue). Il faut la
  redresser par les quatre coins (version 0.1.29), puis recaler A avant de
  vérifier sur le vrai projet.

# Journal de décisions

Chaque choix technique qui n'est pas écrit dans `docs/plan.md` est noté ici,
du plus récent au plus ancien : la décision, la raison, et ce qu'elle interdit.
Une session ne revient pas sur une décision sans l'accord de Benjamin.

## 2026-11-12 — Tâche 2.5

Écran de redressement : `src/rectification/`, module
`crates/insitu-core/src/rectification/projector.rs`, lecture des photos
`crates/insitu-core/src/photo.rs`, commande `insitu projector`, mode
d'emploi `docs/redressement.md`.

**D-2.5-a. HEIC lu par `zune-heic`, en Rust pur ; rotation du conteneur
appliquée.**
`zune-heic` (0.5, MIT, Apache-2.0 ou Zlib) contient son propre décodeur HEVC :
aucune bibliothèque système, compilation identique sous Linux et sur Mac,
tests en session cloud. Écartés : `libheif` (bibliothèque C à embarquer),
la conversion par `sips` de macOS (impossible à tester hors Mac), `heic`
(licence AGPL). Le décodeur matériel d'Apple est désactivé : il convertit
les couleurs autrement, le Mac et les tests ne verraient pas les mêmes
pixels. Vérifié sur une vraie photo d'iPhone (grille de 49 tuiles, 4 284 ×
5 712, échantillon public du projet zune, non versé) : pixels identiques à
libheif à un niveau près ; et sur la photo A d'Épinal réencodée en HEIC par
libheif (`jeu-essai/heic/`, 485 Ko) : coins de la mire à moins de 0,07 mm
de ceux du JPEG. La rotation et le miroir inscrits dans le conteneur HEIF
(`irot`, `imir`) et son recadrage (`clap`) sont appliqués, comme le veut la
norme : un HEIC apparaît droit, alors qu'un JPEG garde ses pixels couchés
(D9). Les points du redressement sont comptés dans l'image ainsi décodée.
Limites : HEIC 8 bits seulement (un HEIC 10 bits est refusé avec un message),
cartes de profondeur et de gain ignorées. Le HEIC n'est pas encore accepté
pour la façade (D-3.1-f) : la lecture existe, l'ouverture est à faire.

**D-2.5-b. Un projecteur, une photo brute ; la détection part dès l'import.**
Importer une photo remplace la précédente et oublie son redressement. Le
plan évoque deux ou trois photos par projecteur dont l'app garderait la
meilleure : pas fait ici, faute de place dans le modèle (`rawPhoto` est un
seul chemin). Si la recherche automatique échoue, l'écran passe directement
à la pose des quatre coins au clic.

**D-2.5-c. Photo redressée du projet : PNG à 500 dpi, nommée d'après le
projecteur.**
`photos-redressees/<libellé>.png` (« A » donne `a.png` ; suffixe si un autre
projecteur a déjà ce nom), réécrite à chaque détection. 500 dpi comme les
photos redressées d'Épinal (5 512 px) ; compression PNG rapide (le PNG
habituel prenait 5 s de plus). Une photo redressée qu'aucun projecteur ne
cite plus est remplacée sans question.

**D-2.5-d. L'export recalcule l'image depuis la photo brute.**
Les correspondances enregistrées dans le projet suffisent à refaire le même
modèle (trapèze + plaque mince, D-2.4-b) : `from_rectification`. L'export à
1 400 dpi n'est donc pas un agrandissement de la photo à 500 dpi. TIFF
compressé sans perte (LZW et prédiction horizontale) ou PNG ; résolution
inscrite dans le fichier (balises TIFF, bloc pHYs du PNG). Les crates `png`
et `tiff` deviennent des dépendances directes du moteur pour cela, aux
versions qu'utilisait déjà `image`. Résolution acceptée : 50 à 1 400 dpi.

**D-2.5-e. Photo brute affichée en pixels.**
Dans la vue « Photo brute », une unité de la visionneuse vaut un pixel de la
photo, et la barre d'état l'écrit « px » (nouvelle option `cursorUnit` de la
visionneuse). Les coins se numérotent dans le repère de la diapositive, pas
de la photo.

**D-2.5-f. La loupe est faite d'images, pas d'une seconde visionneuse.**
Elle affiche les tuiles pleine résolution (niveau 0) sous forme d'images,
pixels non lissés, avec une croix de visée en SVG. Essai écarté : une
seconde scène PixiJS (deux contextes WebGL) faisait apparaître un carré noir
dans la vue principale, dans le navigateur de la session ; en plus d'être
plus lourde.

**D-2.5-g. Couleurs des croisements et jugement d'ensemble.**
Un croisement est vert sous 0,15 mm d'écart de validation croisée (précision
attendue d'un croisement, étude 2.1), orange sous 0,3 mm, rouge au-delà. Le
résumé est jugé comme le critère de la tâche 2.4a : bon si 95 % des
croisements sont à moins de 0,3 mm, mauvais si la mire est couverte à moins
de 80 % ou si ce 95e centile dépasse 0,6 mm.

**D-2.5-h. Les écrans Façade et Redressement partagent le dernier projet.**
Le chemin du dernier projet ouvert est gardé au même endroit pour les deux
écrans (`src/project/current.ts`) : passer d'un onglet à l'autre garde le
projet. L'écran Redressement ne crée pas de projet : l'écran Façade le fait.

## 2026-11-12 — Tâche 2.4a

Détection de l'ancienne mire et redressement : module
`crates/insitu-core/src/rectification/`, commande `insitu rectify`, tests
`crates/insitu-core/tests/redressement.rs`.

**D-2.4-a. Portage fidèle du prototype de l'étude 2.1.**
L'amorce (cercle rouge, croix, six petits cercles) et la mesure des
croisements reprennent `etudes/2.1-detection/detecteurs.py` avec les mêmes
réglages. Vérifié sur les 8 photos d'Épinal : même sens de la mire, mêmes
croisements à 0,003 px près (médiane). Écarts assumés : les filtres
morphologiques utilisent un carré au lieu d'un disque (plus rapide, sans
effet mesurable), l'ellipse est ajustée par une conique aux moindres carrés
(`A + C = 1`) au lieu de la fonction d'OpenCV, les tirages au sort sont
reproductibles.

**D-2.4-b. Le modèle final est celui de la tâche 2.3, sans lissage.**
Trapèze + plaque mince (`fit_with_report`), sur tous les croisements gardés ;
les points que le diagnostic déclare aberrants sont retirés une fois, puis le
modèle est réajusté (D-2.3-f laissait ce choix à la détection). Seuil
plancher des points aberrants : 0,15 mm. Le lissage de la plaque mince vaut 0 :
en mettant un croisement sur quatre de côté, l'erreur de prédiction est de
0,04 à 0,08 mm en moyenne et 0,11 à 0,21 mm au 95e centile, la meilleure de
toutes les valeurs essayées (0 à 0,1), et meilleure que la régression locale
du prototype (0,08 à 0,15 / 0,20 à 0,33 mm dans l'étude). L'image est ensuite
calculée par `GridWarp` (pas de 2,5 mm, D-2.3-b), interpolation bicubique
dans la photo, sur tous les cœurs.

**D-2.4-c. Pixels de la photo : le centre du premier pixel est en (0,5 ; 0,5).**
C'est la convention de la visionneuse (un calque posé par son coin). Les
points écrits dans le fichier projet (`rectification.points[].photo`) la
suivent. Le prototype Python utilisait celle d'OpenCV (centre en 0) : il faut
ajouter 0,5 pour comparer.

**D-2.4-d. Carte de confiance : erreur estimée par cases de 2 mm.**
Une case est « couverte » si un croisement mesuré est à moins de 8 mm de son
centre (définition de l'étude). Son erreur estimée est la moyenne
quadratique, pondérée par une gaussienne de 10 mm, des écarts de validation
croisée (chaque croisement prédit sans lui) des croisements voisins : c'est
un majorant. Image : blanc = sûr, noir = 1 mm et plus ou non couvert.
Conséquence visible : le disque blanc central et une bande de 4 mm le long
de la croix épaisse (où il n'y a pas de croisement mesurable) sont marqués
« non couverts », alors que le redressement y est interpolé entre des
croisements à 10 mm.

**D-2.4-e. Quatre coins approchés peuvent remplacer l'amorce automatique.**
`DetectOptions::corners` (et `insitu rectify --corners`) : un trapèze tiré de
quatre coins à 1 ou 2 mm près suffit à lancer la mesure de la grille, qui
affine ensuite (tâche 2.5). Les coins fixent le sens de la mire. Le
redressement obtenu est alors noté « manuel » dans le fichier projet.

**D-2.4-f. A.tiff n'est qu'un témoin.**
Critère retenu par Benjamin le 12 novembre 2026 : celui de l'étude
(croisements de l'image redressée à moins de 0,3 mm au 95e centile, 8 photos
détectées sans aide). Le test compare aussi notre image de la photo A à
A.tiff : même sens (ressemblance 0,98, contre moins de 0,6 tournée ou
retournée) et écart de 0,61 mm en moyenne, 1,04 mm au 95e centile, 1,29 mm au
plus, soit l'écart propre à A.tiff ; il exige seulement moins de 2 mm.

**D-2.4-g. Le moteur est optimisé même en développement et en test.**
`Cargo.toml` : `opt-level = 3` pour `insitu-core` et toutes les
dépendances dans le profil `dev` (donc aussi `test`). Sans cela, détecter la
mire sur une photo de 12 Mpx prend plusieurs minutes. Contrepartie : la
première compilation est plus longue, et le pas-à-pas au débogueur moins
lisible dans le moteur.

**D-2.4-h. Limites connues de cette amorce.**
Les réglages (tailles de filtres, seuils en pixels) sont ceux des photos
d'Épinal (12 Mpx, 6,7 px par mm). Une photo de 48 Mpx n'a pas été essayée.
Le HEIC n'est pas lu (tâche 2.5). La nouvelle mire (tâche 2.2) n'a plus les
six petits cercles : cette amorce devrait y trouver la position puis refuser
faute de sens (non essayé) ; c'est la tâche 2.4b qui la lira par ses repères.

## 2026-11-12 — Économie de minutes GitHub

**D14. Les vérifications ne tournent que sur ce qui a changé.**
Le quota gratuit de GitHub Actions a été épuisé le 12 novembre ; une minute
de machine Mac y compte pour dix. Désormais, dans `ci.yml`, une première
étape Linux regarde les fichiers modifiés : le moteur n'est testé que si le
Rust ou le jeu d'essai change, l'interface que si `src/` ou ses réglages
changent, la coque Mac que si la coque, le moteur qu'elle embarque, les
images embarquées ou les réglages Cargo changent. Une partie sautée compte
comme réussie. Les caches Rust ne sont réécrits que sur `main`. Chaque
tâche a une durée maximale, pour qu'un blocage ne consomme pas d'heures.
Précise D10 : la fabrication d'essai du `.dmg` sur une demande de fusion
n'a plus lieu que si l'emballage change (`fabrication.yml`,
`tauri.conf.json`, icônes) ; le code de la coque est vérifié par la
compilation de `ci.yml`. Conséquence : une app qui compile mais plante au
démarrage n'est attrapée qu'à la fabrication de la version sur `main`, qui
alors ne publie rien.

## 2026-11-12 — Tâche 3.1

Import du plan de façade. Les décisions de cette tâche portent un numéro
local (D-3.1-a…), comme celles des tâches menées en parallèle.

**D-3.1-a. Plans lus en Rust pur : `hayro` (PDF), `resvg` (SVG), `dxf`.**
Aucune bibliothèque système ni binaire à embarquer : tout se compile dans le
moteur, sous Linux comme sur Mac, et se teste en session cloud. `hayro` (0.9,
Apache-2.0 ou MIT) est le moteur de rendu PDF le plus complet en Rust pur ;
il ne lit pas les PDF protégés par mot de passe, ni quelques effets rares
(mélanges, groupes « knockout »). `resvg` (0.50, mêmes licences) change les
textes en tracés avec les polices du Mac. `dxf` (0.6, MIT) lit les DXF texte
et binaires. Écartés : PDFium (binaire C++ à embarquer et signer), MuPDF
(licence AGPL), et l'affichage du PDF par pdf.js dans l'interface (le calcul
sortirait du moteur, contre la règle d'architecture).

**D-3.1-b. Un plan vectoriel est rendu depuis ses vecteurs à chaque niveau
de zoom, les niveaux fins à la demande.**
Le plan reçoit une pyramide au format habituel (D-1.5-d) dont le niveau 0
mesure 65 536 px sur son plus grand côté (`VECTOR_LEVEL0_SIDE`) : sur un A0,
environ 1 400 dpi du papier. Chaque niveau est rendu directement depuis les
vecteurs, sans réduire d'image : un trait fin reste fin et net partout. Les
niveaux de moins de 4 096 × 4 096 px (environ 85 tuiles) sont rendus à
l'ouverture ; les plus fins, tuile par tuile, quand la visionneuse les
demande, puis gardés dans le cache. Le schéma `tuiles://` devient donc
asynchrone : le rendu se fait hors du fil de la fenêtre, au plus un rendu par
cœur à la fois. Raison : pré-rendre 16 000 tuiles prendrait plusieurs
minutes ; on ne rend que ce qu'on regarde. Une image (PNG, JPEG, TIFF) garde
sa pyramide ordinaire : elle n'est pas plus nette que ses pixels.

**D-3.1-c. Tuiles de plan en PNG, en gris si le plan est en noir et blanc.**
Le plus petit niveau est rendu d'abord : s'il ne contient aucune couleur, les
tuiles sont en niveaux de gris. Limite : un détail en couleur invisible à
cette taille sera rendu en gris.

**D-3.1-d. Repère d'un DXF : coin haut-gauche du dessin, y vers le bas ;
coin gardé dans `facade.origin`.**
Un DXF a son origine où l'architecte l'a mise et son axe y vers le haut. Le
moteur prend le rectangle qui englobe tout ce qu'il sait tracer et en fait le
repère du fichier (comme une image). Ce coin est enregistré pour rester
traçable. Limite : un objet égaré loin du dessin agrandit ce rectangle ;
l'échelle n'en est pas faussée, mais le plan paraît petit dans la vue.
Ce qui est tracé : lignes, polylignes et leurs arrondis, cercles, arcs,
ellipses, splines, solides (contour), blocs (imbriqués, en réseau) et blocs
de cotes, en noir d'un pixel d'écran à tout zoom. Pas encore : textes,
hachures, images, objets 3D. `$INSUNITS` est lu et affiché par
`insitu facade inspect`, sans remplacer la mise à l'échelle.

**D-3.1-e. Mise à l'échelle : deux points et une cote, rien d'autre.**
Comme le prévoit le plan. Les deux points doivent être distants d'au moins un
millième du plan, la cote positive. Importer un autre plan efface l'échelle
précédente (elle valait pour l'ancien fichier). Les points de recalage déjà
posés, en mm, ne sont pas transformés : changer de plan après le recalage
demandera de le refaire (à traiter avec la tâche 3.2 si le cas se présente).

**D-3.1-f. Formats refusés avec un conseil : DWG, HEIC, Illustrator.**
Le DWG est à exporter en PDF ou DXF (plan). Le HEIC n'est pas encore lu pour
la façade : le lecteur HEIC viendra avec l'écran de redressement (tâche 2.5),
d'ici là l'exporter en JPEG. Un `.ai` est à enregistrer en PDF.

**D-3.1-g. L'app a des onglets : « Façade » et « Démonstration ».**
L'écran Façade s'ouvre au lancement et rouvre le dernier projet (mémorisé
dans le navigateur de l'app, simple confort). La démonstration de la
visionneuse reste accessible tant que D13 n'est pas levée. Les fenêtres de
choix de fichiers passent par le module officiel `tauri-plugin-dialog`
(2.8). Un nouveau projet se crée par une fenêtre « Enregistrer » ; `.insitu`
est ajouté au nom s'il manque.

## 2026-11-12 — Tâche 3.4

Moteur de déformation : `crates/insitu-core/src/resample/`, commande
`insitu warp`.

**D-3.4-a. Une déformation d'image part d'une correspondance quelconque
« pixel de sortie → pixel source », évaluée sur un treillis.**
`warp_raster` ne connaît ni la façade ni les millimètres : l'appelant lui
donne une fonction (par exemple pixels de diapositive → mm de diapositive →
mm de façade → pixels de la fresque, pour la tâche 3.5). Elle n'est évaluée
exactement qu'un pixel sur 8 dans chaque sens ; position et dérivée sont
interpolées entre ces nœuds. Écart mesuré sur une diapositive de 15 433 px
(trapèze marqué et ondulation de 2 mm) : 0,014 px au plus, soit 0,25 µm. Les
nœuds ne sortent jamais de l'image de sortie : au-delà, une grille de
déformation change d'allure et fausserait les derniers pixels.

**D-3.4-b. Filtre : cubique de Catmull-Rom, étiré là où l'image est réduite.**
Sans réduction, c'est une interpolation bicubique : une transformation
identité ou une translation entière recopie la source au niveau près. Là où
la transformation réduit l'image, le noyau est étiré selon la direction et
la force de la réduction (calculées à partir de la dérivée locale), pour
couvrir tout ce que le pixel voit ; au-delà d'une réduction de 2, on lit une
version de la source réduite par moitiés (calculée seulement si besoin).
L'étirement est plafonné à 4 : sous un trapèze extrême, la direction la
moins réduite est un peu plus floue que nécessaire. Mesures des tests : le
centre d'un trait tourné de 3° s'écarte d'une droite de 0,03 px (0,28 px
sans filtre) ; des hachures d'un pixel réduites de 3,3 donnent un gris
uniforme (écart type 0 au lieu de 127). Limite connue : agrandi 3,8 fois, un
trait d'un pixel reçoit 8 % de lumière en plus au pied de ses flancs, et son
épaisseur au seuil de 50 % dépasse l'idéal d'un quart de pixel de sortie
(5 µm). Les niveaux de la pyramide ne sont pas mélangés : une couture très
légère est possible là où la réduction franchit 2, 4, 8.

**D-3.4-c. Le calcul porte sur les valeurs enregistrées, pas sur la
lumière.**
Contrairement aux vues d'ensemble de la visionneuse (D-1.5-e), la
déformation moyenne les valeurs telles quelles. Raison : la gravure
applique un seuil (LightBurn en mode seuil) ; moyenner en lumière
rendrait le bord d'un trait blanc plus clair qu'il n'est, donc le trait plus
épais une fois seuillé. Conséquence à surveiller pour l'export (3.5) : un
trait plus fin qu'un pixel de sortie (18 µm à 1 400 dpi) devient gris et
peut disparaître au seuil.

**D-3.4-d. Calcul en parallèle par bandes de 256 lignes, sans dépendance.**
Les fils de la bibliothèque standard se partagent les bandes ; pas de
`rayon`. Le temps ne dépend pas du contenu (un fond noir coûte autant qu'un
dessin). Hors de la source, la sortie vaut 0 (noir : rien n'est gravé).

**D-3.4-e. Le critère des 30 s est vérifié par un test en compilation
optimisée.**
`cargo test --release --test deformation` déforme le vrai Verre A
(15 433 px) ; la tâche « Moteur » des vérifications GitHub le lance. En
compilation de mise au point (`cargo test`), ce test est ignoré, car il y
serait dix fois plus lent sans rien dire de l'app. Mesuré en session cloud :
9,3 s sur 4 cœurs, lecture du fichier non comprise.

**D-3.4-f. `insitu warp` : paires diapositive (mm) → image (pixels).**
Même format de fichier que `insitu geometry fit`. La déformation est
échantillonnée sur une grille de 2,5 mm (D-2.3-b). Le PNG est écrit sans
réglage (ni marge, ni miroir, ni négatif, ni résolution inscrite) : l'export
des diapositives est l'objet de la tâche 3.5.
## 2026-11-12 — Mesure de fluidité de la visionneuse

**D-1.5-k. Pendant un geste, la scène redessine à chaque rafraîchissement de
l'écran ; la mesure « Dernier geste » porte sur ce rythme.**
Précise D-1.5-a (« la scène ne redessine que lorsque quelque chose change ») :
c'est toujours vrai hors geste, mais un mouvement (glisser, pincer, molette)
ouvre une fenêtre de 180 ms pendant laquelle la scène redessine à chaque
image de l'écran, fenêtre repoussée à chaque nouveau mouvement. Raison :
l'ancienne mesure comptait l'écart entre deux images dessinées, qui suivait
le rythme des mouvements de la main (39 ms en moyenne mesurées par Benjamin
sur la 0.1.1), pas la fluidité. Un geste n'est rapporté qu'à partir de
5 écarts mesurés ; une image arrivée plus de 25 ms après la précédente compte
« en retard ». Le calcul vit dans `src/viewer/frameMeter.ts`, testé sans
écran. Le chiffre « calcul » ne compte que le travail de l'interface, pas
celui de la carte graphique.

## 2026-11-12 — Tâche 2.2

Nouvelle mire : module `crates/insitu-core/src/target/`, commande
`insitu target`, PDF prêt à imprimer dans `docs/mire/`.

**D-2.2-a. Le PDF est écrit par le moteur, à la main, sans dépendance.**
Une page, uniquement des traits, des cercles (huit arcs de Bézier, écart au
vrai cercle inférieur à 0,001 mm) et des rectangles pleins ; texte en
Helvetica, police que tout lecteur PDF possède. Les coordonnées sont en mm
avec 4 décimales, l'échelle mm → point avec 9 décimales (4 laisseraient
5 µm d'erreur sur 280 mm). Le fichier ne contient ni date ni numéro de
version de l'app : un même dessin donne le même fichier, ce qui permet de
verser `docs/mire/mire-insitu-v2.pdf` et de tester qu'il est à jour. Une
bibliothèque PDF aurait apporté des dépendances pour une seule page fixe.

**D-2.2-b. Page A3 par défaut, mire au centre, pointillés de coupe à 297 mm.**
L'étude 2.1 a mesuré sur les photos d'Épinal une feuille d'environ 297 mm,
un A3 recoupé (marge blanche de 8,5 mm). Le PDF reprend ce format : la
consigne d'impression et deux pointillés gris sont dans les bandes que l'on
coupe ; l'identifiant reste dans la marge de 8,5 mm, sous la grille.
`--page carre` donne une page de 297 × 297 mm sans consigne.

**D-2.2-c. Les six petits cercles et le texte « Mire 280 x 280 mm » sont
retirés.**
Ils ne servaient qu'à trouver le sens de la mire ; n'importe quel repère lu
le donne désormais (D-2.1-a). Le premier cercle tomberait en outre sur le
repère 12. Le texte est remplacé par l'identifiant : « InSitu - mire 280 mm -
version 2 - repères AprilTag 36h11 n° 0 à 15 ». La mire d'Épinal est la
version 1. Conséquence pour la tâche 2.4 : sur la nouvelle mire, sans aucun
repère lu, le cercle rouge et la croix donnent la position mais pas le sens.

**D-2.2-d. Les codes des 16 repères sont recopiés d'OpenCV, dans l'ordre de
l'image.**
`apriltag.rs` les écrit ligne par ligne, 1 = module blanc ; l'ordre des bits
diffère de celui de la bibliothèque AprilTag d'origine, le dessin est le
même. Vérifié par un test (au moins 11 bits d'écart entre deux repères, dans
les quatre sens) et, une fois, par deux lecteurs indépendants du moteur
(OpenCV et `kornia-apriltag`, script `etudes/2.2-mire/verifier_reperes.py`) :
16 repères relus sur 16, sans bit corrigé, sur le PDF rendu et sur une photo
simulée en biais et floue.

**D-2.2-e. Repères à 2 mm du pourtour de leur bloc ; rouge 230-30-40.**
Comme dans l'étude : repère de 16 mm centré dans un bloc de 20 mm dont le
pourtour reste tracé et les traits intérieurs sont coupés. La zone blanche
autour du repère fait 1,8 mm (un module de 2 mm moins le demi-trait du
pourtour) ; les deux lecteurs s'en accommodent. La couleur exacte du cercle
de l'ancienne mire n'est pas connue (la référence du jeu d'essai est en
gris) : on garde l'encre rouge de l'étude.

**D-2.2-f. Ce que le moteur expose pour la détection.**
`target::TargetLayout::coded()` donne, en mm dans le repère de la mire, les
16 repères (numéro, quatre coins dans l'ordre haut-gauche, haut-droit,
bas-droit, bas-gauche) et `grid_nodes()`, les 660 croisements mesurables :
ni à moins de 34 mm du centre (disque et son bord), ni sur un bloc de repère,
pourtour compris, comme dans l'étude. `insitu target info` les écrit en JSON.

**D-2.2-g. Le module s'appelle `target`, malgré `.gitignore`.**
Le vocabulaire impose `target` pour la mire, mais `.gitignore` excluait tout
dossier `target/` (dossier de compilation de Cargo). Une exception y est
ajoutée pour `crates/insitu-core/src/target/`.

## 2026-11-12 — Essai de la visionneuse

**D13. L'app installée embarque les images de démonstration d'Épinal.**
Les quatre Verres et les quatre photos redressées (environ 55 Mo) sont
ajoutés aux ressources de l'app, pour que Benjamin essaie la visionneuse
depuis la version téléchargée, sans Terminal. Provisoire : à retirer dès que
l'app sait ouvrir un projet (jalon 2), l'écran de démonstration avec.

## 2026-11-11 — Tâche 2.1

Étude de détection de la mire : `docs/etudes/2.1-detection-mire.md`. Les
points D-2.1-a à D-2.1-d sont des **propositions** : ils ne deviennent des
décisions qu'une fois validés par Benjamin, après l'essai d'une mire imprimée.

**D-2.1-a (proposition, approuvée sur plan par Benjamin le 12 novembre 2026 ;
à confirmer après impression). Nouvelle mire : la grille actuelle, plus 16
repères codés AprilTag.**
On garde la grille de 10 mm, la croix et le cercle rouge. On épaissit les
traits fins (0,35 mm) et on ajoute 16 repères codés de 16 mm (famille
AprilTag 36h11, numéros 0 à 15), répartis sur toute la feuille, chacun dans
un bloc de 2 × 2 cases. Raison : les repères disent où est la mire, dans quel
sens, et laquelle, même si une partie de la feuille est sombre ou cachée ; la
grille donne la précision. Une couronne de damier (ChArUco) tient moins bien
au flou et cache le bord de l'image ; des repères aux seuls quatre coins ne
corrigent pas le papier qui gondole.

**D-2.1-b (proposition). La précision vient des croisements de la grille, pas
des repères.**
Le moteur mesure les croisements de proche en proche à partir des repères,
puis ajuste un trapèze et une correction locale lissée. Les coins des repères
ne servent qu'à amorcer : un coin de repère est mesuré à 0,15 à 0,5 mm près,
un croisement à moins de 0,1 mm. Interdit : redresser une photo à partir des seuls repères.

**D-2.1-c (proposition). L'ancienne mire reste exploitable automatiquement.**
Sans repère codé, le moteur cherche le cercle rouge et la croix, puis mesure
la grille de la même façon ; l'orientation vient des six petits cercles. En
dernier recours, quatre coins cliqués à la main servent d'amorce. Conséquence
pour la tâche 2.2 : la nouvelle mire garde le cercle rouge et la croix.

**D-2.1-d (proposition). Repères codés en Rust : crate `kornia-apriltag`,
sous réserve.**
Elle est en Rust pur, sous licence Apache-2.0, et a relu les 16 repères sur
les essais de l'étude. Réserves : crate jeune (0.2.0), environ 180
dépendances, non vérifiée sur Mac. Si elle pose problème à la compilation de
l'app, le repli est d'écrire un lecteur limité à nos 16 repères. Aucune
dépendance à OpenCV.

**D-2.1-e. Les scripts d'étude vivent dans `etudes/`, hors du moteur.**
Python et OpenCV y sont admis ; rien dans `crates/` ni dans `src/` n'en
dépend, et aucun test ne les lance. `etudes/2.1-detection/essai-rust` est un
paquet Cargo isolé (il porte sa propre rubrique `[workspace]`).

**D-2.1-f. Constat sur le jeu d'essai : `mire/mire-280.png` n'est pas la
mire.**
Ce fichier (et ses deux réduits) montre une frise de seize fleurs, sans
grille. La vraie mire est la page 2 de `mire/mire-280.ai`. L'étude s'appuie
sur cette page. Le jeu d'essai est à corriger dans une tâche à part.
Corrigé le 12 novembre 2026 : la référence `mire/mire-280-page2.png` est
fabriquée à partir de cette page ; les deux PNG fautifs sont retirés.

## 2026-11-11 — Tâche 1.5

**D-1.5-a. Affichage par PixiJS 8, en WebGL.**
La visionneuse (`src/viewer/`) est une scène PixiJS (version 8.23), forcée en
WebGL : c'est ce que la fenêtre Tauri (WebKit) sait faire sur toutes les
versions de macOS visées. Raison du choix : la vue façade puis la composition
auront besoin de dessins tournés et déformés par maillage, que PixiJS sait
déjà afficher (`Mesh`) ; une simple visionneuse d'images aurait été à refaire.
La scène ne redessine que lorsque quelque chose change.

**D-1.5-b. Le repère de la scène est en millimètres.**
Le conteneur `world` est gradué en mm (origine en haut à gauche, y vers le
bas, comme D4). Chaque calque y est posé par un coin et la taille d'un de ses
pixels (`Placement`). Un Verre de 15 433 px et une photo redressée de
5 512 px, tous deux étalés sur 280 mm, se superposent donc sans calcul.

**D-1.5-c. Tout nouvel objet affichable passe par `SceneNode`.**
`src/viewer/nodes.ts` définit le contrat (`container`, `bounds`, `update`,
`destroy`) et l'union `LayerSpec`. Un nouveau type (image non tuilée, dessin
déformé, tracé, emprise) s'ajoute par une classe, une variante de `LayerSpec`
et une ligne dans `createNode`. Seules les tuiles sont codées pour l'instant.

**D-1.5-d. Format de pyramide : niveaux ÷ 2, tuiles de 512 px sans
recouvrement, `pyramid.json`.**
Niveau 0 = pleine résolution ; chaque niveau est la moitié du précédent,
arrondie au pixel supérieur, jusqu'à tenir dans une tuile. Fichiers
`niveau/colonne_ligne.png|jpg`. Les tuiles de bord sont plus petites, pas
complétées. Pas de recouvrement entre tuiles : avec un calque semi-transparent
il se verrait. Un changement de ce format augmente `PYRAMID_VERSION`.

**D-1.5-e. Réduction par moyenne de la lumière, pas des valeurs.**
Chaque niveau moyenne des carrés de 2 × 2 pixels en lumière linéaire (courbe
sRGB défaite puis refaite). Un trait blanc d'un pixel réduit 16 fois reste à
28 % de gris au lieu de 6 % : les traits fins restent lisibles en vue
d'ensemble sans être épaissis (un filtre « maximum » les aurait empâtés et
aurait bouché les hachures). Le même filtre sert aux photos.

**D-1.5-f. Tuiles PNG pour le trait, JPEG pour les photos, choisi d'après
l'image.**
Une image dont au moins 92 % des pixels sont presque noirs ou presque blancs,
ou qui a de la transparence, est tuilée en PNG (sans perte : le JPEG ferait
baver les traits). Le reste part en JPEG qualité 88. `insitu tiles --format`
permet d'imposer le format. Les tuiles gardent le nombre de canaux de l'image
(un Verre reste en gris).

**D-1.5-g. Dans l'app, les tuiles vivent dans le cache et sont servies par
le schéma `tuiles://`.**
`open_tiled_image` appelle `tiles::ensure_pyramid`, qui range la pyramide dans
`~/Library/Caches/com.luminariste.insitu/tuiles/<clé>` ; la clé dépend du
chemin, de la taille et de la date du fichier : une image inchangée n'est
jamais retuilée, une image modifiée l'est. La coque sert ce dossier par
`register_uri_scheme_protocol` (adresse `tuiles://localhost/<clé>/…`, forme
propre à macOS). Le contrôle des chemins demandés est fait par le moteur. Le
cache n'est pas encore nettoyé : à prévoir quand les projets existeront.

**D-1.5-h. À 100 %, l'image est calée sur les pixels de l'écran.**
« 100 % » veut dire un pixel d'image par pixel physique (sur Retina, un
demi-point). La position de la scène est arrondie au pixel physique : à
100 %, l'affichage est identique au fichier, sans flou d'interpolation.

**D-1.5-i. Molette, trackpad : règle de tri.**
Pincement, Cmd + défilement et molette à crans zooment sur le curseur ; le
défilement à deux doigts déplace. Molette et défilement arrivent par le même
événement : on reconnaît la molette à l'absence de composante horizontale et
à un pas en multiples de 120 (`classifyWheel`). Règle à confirmer sur Mac
avec une vraie souris ; Cmd + défilement zoome dans tous les cas.

**D-1.5-j. Tests de l'interface avec vitest ; dossier `tuiles-demo/`.**
`npm test` lance les tests de la logique pure de la visionneuse ; le workflow
« Vérifications » l'exécute. En développement, Vite sert sous `/tuiles/` le
dossier `tuiles-demo/` (ignoré par Git) rempli par `insitu tiles` : il sert à
contrôler la visionneuse dans un navigateur, et de secours à l'écran de
démonstration si le moteur ne répond pas.

## 2026-11-11 — Tâche 2.3

Les décisions de cette tâche portent un numéro local (D-2.3-a…), parce que
plusieurs tâches écrivaient dans ce journal en même temps.

**D-2.3-a. Une déformation = un trapèze + une correction lisse.**
Le sens direct vaut `trapèze(source) + correction(source)`. Le trapèze
(`Homography`) est exact avec 4 paires de points, ajusté aux moindres carrés
au-delà (coordonnées normalisées, solution linéaire puis affinage sur l'écart
mesuré côté cible). La correction (`PointWarp`) est une spline en plaque
mince ajustée sur ce que le trapèze laisse comme écart. Raison : la plaque
mince accepte des points placés n'importe où (clics comme grille de mire),
passe exactement par eux, n'a aucun réglage obligatoire, et se réduit à zéro
quand le trapèze suffit. Un réglage facultatif, le lissage (0 par défaut,
sans unité), la laisse s'écarter de points bruités.

**D-2.3-b. Pour les images, la correction est échantillonnée sur une grille.**
Évaluer la plaque mince coûte un temps proportionnel au nombre de points de
contrôle : acceptable pour des clics, pas pour 238 millions de pixels avec
600 points de mire. `GridWarp` garde le trapèze exact et stocke la correction
aux nœuds d'une grille régulière, interpolée par cubiques de Catmull-Rom :
16 nœuds lus par point, quel que soit le nombre de points d'origine. Écart
mesuré entre la grille et le modèle d'origine (225 points, papier gondolé de
quelques mm) : 0,005 mm avec un pas de 2,5 mm, 0,02 mm avec un pas de 5 mm.
La même structure, construite directement à partir des déplacements de ses
nœuds, pourra porter la grille de déformation libre de la tâche 4.4.

**D-2.3-c. Le sens inverse est calculé, pas ajusté une seconde fois.**
Il n'existe pas de formule pour l'inverse. Plutôt que d'ajuster un second
modèle dans l'autre sens (les deux ne seraient réciproques qu'à peu près), le
sens inverse résout le sens direct par itérations de Newton, en partant du
trapèze inverse. L'aller-retour est donc exact aux arrondis près : 3 × 10⁻¹⁰ mm
mesuré sur tout le carré de 280 mm, pour un critère de 0,01 mm. Limite : si
la déformation se replie (deux points source envoyés au même endroit),
l'inverse n'est plus défini ; la fonction renvoie alors « pas de résultat » ou
l'un des antécédents, jamais un point faux, et `has_fold` permet de le
détecter avant. Une transformation renvoie aussi « pas de résultat » pour un
point envoyé à l'infini par le trapèze.

**D-2.3-d. Un point se juge par validation croisée, pas par son écart direct.**
Une déformation qui passe par ses points laisse un écart nul en chacun : cet
écart ne dit rien. Le compte rendu (`fit_with_report`) donne donc, par point,
l'écart entre sa cible et ce que prédisent *les autres points sans lui*, en
plus de l'écart laissé par le trapèze seul. Chaque écart existe côté cible et
côté source (par exemple cm sur la façade et mm sur la diapositive). Le
calcul utilise une formule exacte (Rippa) qui évite de refaire N ajustements :
0,3 s pour 625 points.

**D-2.3-e. Règle de signalement d'un point aberrant.**
L'écart de validation croisée est d'abord divisé par ce que la position du
point amplifie naturellement (un point au bord est prédit par extrapolation) :
c'est son score. Parmi les 4 plus grands scores, on retire le point dont le
retrait assainit le mieux les autres ; il est déclaré aberrant si son score
dépasse 6 fois le score médian des points restants, recalculé sans lui. On
recommence tant qu'on en trouve, sans jamais retirer plus d'un tiers des
points. Le facteur 6 a été choisi par essai : sur 300 tirages de 12 bons
points (± 0,3 mm) plus un point déplacé de 5 mm, les facteurs 5 à 8 désignent
le bon point, et lui seul, 300 fois sur 300, sans fausse alerte sur les
tirages sans point faux ; 4 donne des fausses alertes, 10 des oublis. Il faut
au moins 6 points pour un diagnostic. Limites connues : un point faux de
moins de quelques fois la précision des autres passe inaperçu ; au bord d'une
mire, un bon point peut être signalé à tort (2 sur 222 dans le test), ce
qu'on évite en donnant la précision attendue (`outlier_floor`).

**D-2.3-f. La déformation rendue garde tous les points, aberrants compris.**
Le moteur signale, il ne retire rien : c'est à l'utilisateur de corriger ou
de supprimer un point signalé (tâche 3.2), ou à la détection de mire de
décider (tâche 2.4).

**D-2.3-g. Pas de bibliothèque d'algèbre linéaire.**
Le module n'a besoin que de résoudre des systèmes denses et de trouver les
vecteurs propres d'une matrice 9 × 9 : environ 150 lignes, testées, dans
`geometry/linalg.rs`. `nalgebra` aurait ajouté une dépendance lourde à
compiler pour le même résultat. À revoir si un besoin plus large apparaît.

**D-2.3-h. Format d'entrée de `insitu geometry fit`.**
Un fichier JSON `{ "pairs": [{ "source": [x, y], "target": [x, y] }, …] }`
avec trois réglages facultatifs (`smoothing`, `outlierFactor`,
`outlierFloor`). Un point s'écrit `[x, y]` en JSON partout dans le moteur.
Les noms de champs sont en anglais, comme ceux de `insitu info`.

## 2026-11-11 — Tâche 1.4

Le format lui-même est décrit dans `docs/format-projet.md`.

**D-1.4-a. Noms de fichiers en français, noms de champs en anglais.**
Dans le dossier `.insitu`, ce qu'on voit dans le Finder est en français :
`projet.json`, `verrou.json`, `facade/`, `photos-brutes/`,
`photos-redressees/`, `fresque/`. Les champs du JSON suivent le code
(`rawPhoto`, `registration`…), en « camelCase » comme `insitu info`.

**D-1.4-b. Les fichiers importés sont copiés dans le dossier projet.**
Le modèle n'accepte que des chemins relatifs au dossier, écrits avec des `/`
(type `RelativePath`). Un chemin absolu, un `..` ou un `\` fait refuser le
projet à la lecture. Raison : le dossier doit s'ouvrir tel quel sur l'autre
Mac. Interdit : référencer un fichier resté ailleurs dans le Dropbox.

**D-1.4-c. Champs inconnus : conservés, jamais perdus en silence.**
Chaque objet du modèle garde les champs qu'il ne connaît pas et les réécrit
tels quels à l'enregistrement ; `insitu project validate` les signale par un
avertissement. Exception : dans un point ou un rectangle, un champ inconnu
fait refuser le fichier. Conséquence : ajouter un champ facultatif ne demande
pas de changer `formatVersion` (c'est ainsi que le jalon 4 ajoutera
bibliothèque, placements et calques). Limite : une app ancienne conserve un
champ inconnu mais ne le tient pas à jour.

**D-1.4-d. `formatVersion` ne monte que pour un changement incompatible.**
Champ renommé, supprimé, changé de sens ou d'unité, nouvelle valeur d'un
champ à choix. Un projet plus ancien est mis à niveau en mémoire à
l'ouverture, par une suite de fonctions qui transforment le JSON brut
(`migrate.rs`) ; le fichier n'est réécrit qu'au prochain enregistrement, et
l'ancien est gardé sous le nom `projet.v<N>.json`. Un projet plus récent que
l'app est refusé sans être modifié. Le format n'a aujourd'hui qu'une version
(1) : le mécanisme est testé avec des étapes fictives.

**D-1.4-e. Verrou : `verrou.json`, périmé après 15 minutes sans signe de vie.**
L'app qui tient le verrou le rafraîchit toutes les 2 minutes. Un verrou du
même Mac et du même utilisateur est repris sans question (plantage). Un
verrou de l'autre personne est repris s'il n'a pas été rafraîchi depuis plus
de 15 minutes ; sinon l'ouverture est refusée, sauf ouverture forcée après
avertissement. Celui à qui on a pris le verrou l'apprend à son prochain
rafraîchissement. Le verrou passe par Dropbox : il avertit, il ne garantit
rien, et la règle des 15 minutes suppose deux Mac à l'heure. Le moteur ne
lance aucune minuterie : la coque devra appeler `refresh` (à faire avec
l'enregistrement automatique). L'enregistrement n'exige pas le verrou.

**D-1.4-f. Écriture atomique, fichiers temporaires en `.~`.**
`projet.json` et `verrou.json` sont écrits en entier dans un fichier voisin
nommé `.~<nom>.<aléa>`, vidé sur le disque, puis renommé à la place de
l'ancien. Le préfixe `.~` est de ceux que Dropbox ne synchronise pas, d'après
sa documentation ; ce n'est pas vérifié sur les Mac de Luminariste.

**D-1.4-g. Nombres relus à l'identique : option `float_roundtrip`.**
`serde_json` est compilé avec cette option, pour tout l'espace de travail.
Sans elle, un nombre décimal peut être relu avec un dernier chiffre
différent, et un projet rouvert ne serait pas strictement identique. Un
nombre infini ou indéfini fait refuser l'enregistrement, car JSON ne sait pas
l'écrire.

**D-1.4-h. Aucune dépendance nouvelle.**
Dates (temps universel, à la seconde, `2026-11-11T14:03:22Z`), identifiants
(16 caractères hexadécimaux tirés au hasard) et nom de la machine (commande
`hostname`) sont faits avec la bibliothèque standard. `serde_json` devient
une dépendance du moteur (elle n'était utilisée que par ses tests).

**D-1.4-i. Contenu du modèle au format 1.**
- Façade : fichier, dimensions du fichier dans son unité, et mise à l'échelle
  gardée sous sa forme d'origine (deux points du fichier et une cote en mm) ;
  le facteur mm par unité s'en déduit. Le repère de la façade est celui du
  fichier multiplié par ce facteur.
- Redressement : une liste de correspondances photo brute (pixels) ↔
  diapositive (mm), quatre coins en manuel, davantage en automatique, plus la
  méthode et l'écart résiduel. Les pixels sont ceux du fichier tel
  qu'enregistré, sans rotation EXIF (cohérent avec D9). À confirmer par les
  tâches 2.3 et 2.4, qui produiront ce résultat.
- Photo redressée : un chemin seulement ; elle couvre par définition le carré
  de 280 mm, sa résolution se lit dans le fichier.
- Fresque en une image : fichier et rectangle occupé sur la façade, en mm.
- Export : `dpi`, `mirror`, `negative`. Les deux derniers valent « non » par
  défaut (constantes ajoutées à `units.rs`) : à Épinal il n'y avait pas de
  miroir et c'est LightBurn qui appliquait le négatif.

## 2026-11-11 — Tâche 1.2

**D12. Signature « ad hoc », sans compte développeur Apple.**
`bundle.macOS.signingIdentity` vaut `-`. Sans signature du tout, une app pour
puce Apple ne se lance pas. Avec cette signature locale, elle se lance mais
macOS la bloque au premier lancement après téléchargement ; la marche à
suivre est dans `docs/installer.md`. Pas de notarisation, pas de mise à jour
automatique.

**D11. Le numéro de version n'est écrit qu'à un endroit.**
`Cargo.toml` à la racine, rubrique `[workspace.package]`. `tauri.conf.json`
n'a plus de champ `version` : Tauri reprend celui de Cargo. Numérotation
`0.<jalon>.<ordre>`. 

**D10. Une version se publie en changeant le numéro de version sur `main`.**
`.github/workflows/fabrication.yml` tourne à chaque fusion : si la version
écrite dans `Cargo.toml` n'a pas encore de publication, il fabrique
`InSitu-X.Y.Z.dmg` (puce Apple uniquement) et crée la version, étiquette
comprise, dans l'onglet « Releases ». Raison : une session cloud n'a pas le
droit d'envoyer une étiquette ni de déclencher un workflow à la main. La même
fabrication tourne, sans publier, sur les demandes de fusion qui touchent à
la coque ou au workflow. Procédure : `docs/publier.md`.

## 2026-11-11 — Tâche 1.3

**D9. Le manifeste est produit par un script Python, pas par le moteur.**
`jeu-essai/construire.py` (Pillow) fabrique les références, les réduits et
`manifeste.json` à partir de `sources.json` et d'un dossier d'originaux.
Raison : c'est un outil d'atelier, lancé rarement ; il n'a pas à alourdir le
moteur. Les images produites gardent la disposition des pixels de l'original
(aucune rotation EXIF) et n'ont plus d'EXIF.

**D8. Références allégées plutôt qu'originaux.**
Photos brutes de chambre noire, façade, mire : copies exactes. Photos
redressées : JPEG qualité 92 à la taille d'origine (au lieu de TIFF de
116 Mo). Verres : PNG en niveaux de gris à la taille d'origine. Les photos
« lum », « mur » et « grille » n'existent qu'en réduit (÷ 2). La composition
Illustrator (154 Mo) n'est pas versée. Total : environ 100 Mo. Conséquence :
ces références servent à comparer des géométries, pas des valeurs de pixels.

**D7. Pas de Git LFS, contrairement au plan initial.**
Le plan prévoyait de stocker les originaux hors historique par Git LFS.
L'essai a échoué : le réseau des sessions cloud refuse l'accès au serveur de
stockage LFS de GitHub (`github-cloud.s3.amazonaws.com`, refus de politique).
Des originaux en LFS seraient donc invisibles pour les agents, ce qui
annulerait l'intérêt du jeu d'essai. Les fichiers d'origine restent dans le
Dropbox de Luminariste ; le manifeste garde leur empreinte.

## 2026-11-11 — Tâche 1.1

**D6. Une vérification GitHub existe dès la tâche 1.1.**
`.github/workflows/ci.yml` teste le moteur et l'interface sous Linux, et
compile la coque Tauri sous macOS (seulement si elle peut être touchée, D14). Raison : la coque ne peut pas être compilée
dans une session cloud (voir D2), GitHub est donc le seul endroit où elle est
vérifiée. La fabrication du `.dmg` reste l'objet de la tâche 1.2.

**D5. Identifiants en anglais, tout le reste en français.**
Noms de fonctions, types, fichiers de code : anglais (`mm_to_px`,
`EngineInfo`). Commentaires, documentation, messages de commit, textes de
l'interface : français. Le vocabulaire métier garde une traduction fixe,
donnée dans `CLAUDE.md`.

**D4. Les longueurs du modèle sont en millimètres.**
Type `f64`, origine en haut à gauche, y vers le bas. Les pixels n'existent
qu'à l'entrée (images sources) et à la sortie (exports). Raison : la
résolution d'export est un réglage, pas une donnée du projet.

**D3. Trois crates dans un seul espace de travail Cargo.**
`crates/insitu-core` (moteur), `crates/insitu-cli` (binaire `insitu`),
`src-tauri` (coque `insitu-app`). Le moteur ne dépend jamais de Tauri. Toute
capacité du moteur doit être appelable depuis `insitu-cli`.

**D2. `cargo test` à la racine ne compile que le moteur.**
`default-members` exclut `src-tauri`, qui exige des bibliothèques système
(WebKit) absentes des sessions cloud Linux et impossibles à y installer. Pour
compiler la coque : `cargo check -p insitu-app`, sur Mac ou sur GitHub.

**D1. Versions de départ.**
Tauri 2 (2.12), React 19, Vite 8, TypeScript 7, Rust stable (1.97 au moment
de la tâche). Identifiant de l'app : `com.luminariste.insitu`. Cible :
macOS sur Apple Silicon uniquement.

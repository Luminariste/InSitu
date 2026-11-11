# Journal de décisions

Chaque choix technique qui n'est pas écrit dans `docs/plan.md` est noté ici,
du plus récent au plus ancien : la décision, la raison, et ce qu'elle interdit.
Une session ne revient pas sur une décision sans l'accord de Benjamin.

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
compile la coque Tauri sous macOS. Raison : la coque ne peut pas être compilée
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

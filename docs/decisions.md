# Journal de décisions

Chaque choix technique qui n'est pas écrit dans `docs/plan.md` est noté ici,
du plus récent au plus ancien : la décision, la raison, et ce qu'elle interdit.
Une session ne revient pas sur une décision sans l'accord de Benjamin.

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
`0.<jalon>.<ordre>`. La fabrication refuse une étiquette qui ne correspond pas.

**D10. Une version se publie en posant une étiquette `vX.Y.Z` sur `main`.**
`.github/workflows/fabrication.yml` fabrique alors `InSitu-X.Y.Z.dmg` (puce
Apple uniquement) et crée la version dans l'onglet « Releases ». La même
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

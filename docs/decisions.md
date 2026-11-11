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

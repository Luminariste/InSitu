# Journal de décisions

Chaque choix technique qui n'est pas écrit dans `docs/plan.md` est noté ici,
du plus récent au plus ancien : la décision, la raison, et ce qu'elle interdit.
Une session ne revient pas sur une décision sans l'accord de Benjamin.

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

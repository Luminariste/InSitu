# Journal de décisions

Chaque choix technique qui n'est pas écrit dans `docs/plan.md` est noté ici,
du plus récent au plus ancien : la décision, la raison, et ce qu'elle interdit.
Une session ne revient pas sur une décision sans l'accord de Benjamin.

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

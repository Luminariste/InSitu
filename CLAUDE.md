# InSitu

Application Mac de Luminariste pour préparer des fresques projetées par
rétroprojecteurs : redressement des photos de mire, recalage sur la façade,
composition, déformation automatique vers chaque diapositive, export pour la
gravure laser. Usage interne (Benjamin et Marc), pas un produit public.

Avant toute tâche, lire `docs/plan.md` (le quoi et l'ordre) et
`docs/decisions.md` (les choix déjà tranchés). Avant de toucher à
l'interface (`src/`), lire aussi `docs/design.md` (le système de design).

## Règles de travail

- **Une tâche du plan par session.** Ne faire que la tâche demandée. Une idée
  hors périmètre se note dans la demande de fusion, elle ne se code pas.
- **Branche `tache-N.N`**, puis demande de fusion vers `main`. Jamais de
  poussée directe sur `main`.
- **La session fusionne elle-même** sa demande une fois toutes les
  vérifications GitHub au vert (fusion « squash »). Benjamin a délégué les
  fusions le 11 novembre 2026. Ce qui demande son essai sur Mac lui est
  signalé à part : la fusion ne l'attend pas, mais la tâche n'est pas dite
  validée avant son retour.
- **La demande de fusion est écrite en français simple** pour quelqu'un qui
  n'a jamais développé d'app : ce qui a été fait, comment le vérifier, ce qui
  reste incertain. Dire explicitement ce qui n'a pas pu être vérifié.
- **Le critère d'acceptation du plan est prouvé par un test** chaque fois que
  c'est possible. Si le critère demande un essai sur Mac, le dire et décrire
  l'essai pas à pas.
- **Tout choix technique non prévu se note dans `docs/decisions.md`.** Ne pas
  revenir sur une décision existante sans l'accord de Benjamin.
- **Si le plan est ambigu ou semble faux, s'arrêter et poser la question.**
- **Ce dépôt n'a aucun lien avec PRISMA** ni avec les autres dépôts de
  l'organisation : ne rien y lire, ne rien y écrire, ne rien en reprendre.
- **Pas de stockage Git LFS** : les sessions cloud ne peuvent ni l'envoyer ni
  le télécharger. Les fichiers d'origine lourds (TIFF, `.ai`, exports) restent
  hors du dépôt ; seules des références allégées et des réduits y entrent, par
  `jeu-essai/construire.py`. Le dépôt grossit définitivement de tout ce qu'on
  y verse : n'ajouter que ce qu'un test utilise.

## Vocabulaire

Le métier se dit en français ; le code utilise toujours la même traduction.

| Français | Dans le code | Sens |
| --- | --- | --- |
| façade | `facade` | Le mur à éclairer, et le repère dans lequel on compose la fresque (plan d'architecte ou photo d'ensemble) |
| fresque | `fresco` | L'ensemble des dessins posés sur la façade |
| dessin | `drawing` | Une image au trait blanc sur fond noir (PNG, BMP…), élément de la bibliothèque |
| rétroprojecteur, projecteur | `projector` | Un appareil sur site ; il projette une diapositive |
| diapositive | `slide` | L'image carrée de 280 mm calculée pour un projecteur |
| plaque | `plate` | Le verre de 300 mm gravé ; marge de 10 mm autour de l'image |
| mire | `target` | La feuille de 280 mm posée sur le projecteur pour la prise de vue |
| photo brute | `raw_photo` | La photo iPhone de la mire en chambre noire |
| photo « lum » | `lit_photo` | Photo de la façade, projecteur allumé avec la mire : repères larges pour situer la diapositive |
| déroulé | `unrolled facade` | Façade mise à plat à partir d'un relevé, pour montrer toutes les surfaces ; ce n'est pas une vue de face |
| photo redressée | `rectified_photo` | La photo brute ramenée au carré de la mire, en mm de diapositive |
| redressement | `rectification` | Passage de la photo brute à la photo redressée |
| recalage | `registration` | Mise en correspondance diapositive ↔ façade par points communs |
| emprise | `footprint` | La zone de façade couverte par un projecteur |
| calepinage | `layout` / `assignment` | Répartition des dessins entre les diapositives, sans couper un dessin |
| silhouette | `silhouette` | Contour extérieur d'un dessin, rempli de noir, qui masque ce qui est derrière |
| déformation | `warp` | Transformation géométrique d'une image (trapèze, grille) |
| gravure | `engraving` | Passage au laser fibre via LightBurn |

Chiffres de référence : diapositive 280 mm, plaque 300 mm, export 1 400 dpi
soit 15 433 px, photo redressée historique 5 512 px (500 dpi). Ils sont
définis une seule fois, dans `crates/insitu-core/src/units.rs`.

## Organisation du dépôt

```
crates/insitu-core/   Moteur de calcul (Rust). Aucun lien avec l'interface.
crates/insitu-cli/    Binaire `insitu` : le moteur en ligne de commande.
src-tauri/            Coque de bureau (Tauri 2). Appelle le moteur, ne calcule rien.
src/                  Interface (React + TypeScript).
src/engine.ts         Seul fichier de l'interface qui appelle le moteur.
src/ui/               Système de design : jetons, composants, icônes, raccourcis (docs/design.md).
src/viewer/           Visionneuse haute résolution (PixiJS), base des vues façade et diapositive.
src/facade/           Écran Façade : projet, import du plan, mise à l'échelle.
src/compose/          Dessins posés sur la façade : gestes, sélection, inspecteur (tâche 4.2).
docs/                 Plan, décisions, notice d'installation, publication d'une version.
jeu-essai/            Fichiers réels pour les tests (voir jeu-essai/LISEZMOI.md).
etudes/               Scripts des études (hors moteur) ; rapports dans docs/etudes/.
```

Le jeu d'essai contient des références pleine résolution et des versions
réduites de fichiers réels ; les fichiers d'origine restent dans le Dropbox de
Luminariste. Un test rapide s'appuie sur les réduits.

Règles d'architecture :

- Le moteur ne dépend jamais de Tauri ni de l'interface.
- Tout calcul vit dans le moteur. Une commande Tauri fait une ligne : elle
  appelle le moteur.
- Toute capacité du moteur est appelable depuis `insitu-cli`, pour être
  vérifiable sans écran.
- Les longueurs sont en millimètres (`f64`), origine en haut à gauche, y vers
  le bas. Les pixels n'existent qu'aux entrées et aux sorties.
- Identifiants en anglais ; commentaires, commits et textes d'interface en
  français.

## Commandes

```
cargo test                    # tests du moteur (fonctionne en session cloud)
cargo run -p insitu-cli -- info
npm ci                        # dépendances de l'interface
npm run check                 # vérification des types
npm test                      # tests de l'interface (logique pure, sans écran)
npm run build                 # construction de l'interface
npm run dev                   # interface seule dans un navigateur (sans moteur)
npm run tauri dev             # l'app complète — sur Mac uniquement
cargo check -p insitu-app     # compile la coque — sur Mac ou sur GitHub
```

## Ce qu'une session cloud ne peut pas faire

La coque Tauri ne se compile pas sous Linux en session cloud (bibliothèques
WebKit absentes et non installables), et l'app ne peut pas y être lancée.

- La compilation de la coque est vérifiée par GitHub (`.github/workflows/ci.yml`,
  tâche macOS). Après une poussée, vérifier que cette tâche passe ; si son
  résultat n'est pas consultable depuis la session, le dire.
- L'affichage, la fluidité et l'ergonomie ne sont vérifiés que par l'essai de
  Benjamin sur son Mac (M1 Max). Ne jamais écrire qu'un écran « fonctionne »
  sur la seule foi d'une compilation.
- L'interface seule peut être contrôlée dans le navigateur de la session
  (`npm run dev` ou `npx vite preview`, puis capture d'écran).

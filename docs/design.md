# Système de design d'InSitu

Règles d'interface communes à tous les écrans. Toute session qui touche
`src/` les suit ; un écart se note dans `docs/decisions.md` (décision D15).
Modèle : les logiciels pro du Mac (Final Cut, Logic, Pixelmator Pro,
Affinity) : une fenêtre calme, des images au centre, des panneaux qui disent
ce qu'il y a à gauche et ce qu'on règle à droite.

Le code est dans `src/ui/` :

| Fichier | Contenu |
| --- | --- |
| `tokens.css` | Les valeurs : polices, tailles, espacements, rayons, couleurs des deux thèmes |
| `ui.css` | Le dessin des composants |
| `components.tsx` | Les composants React (boutons, barres, panneaux, listes, menus, dialogues…) |
| `icons.tsx` | Les icônes, au trait, sur 16 px |
| `shortcuts.ts` | Écriture et affichage des raccourcis, liste des raccourcis de l'app |
| `appearance.ts` | Thème automatique, sombre ou clair |

On importe tout depuis `src/ui` (`import { Button, Panel } from "../ui"`).

## 1. Principes

1. **L'image d'abord.** La zone de travail (toile) est la plus grande et
   reste sombre dans les deux thèmes : les dessins sont des traits blancs
   sur fond noir. Rien ne flotte sur l'image sans raison (loupe, message
   d'attente).
2. **Toujours au même endroit.** Même gabarit sur chaque écran : on
   retrouve le projet en haut à gauche, les objets à gauche, les réglages à
   droite, l'état en bas.
3. **Dire, pas deviner.** Chaque bouton dit ce qu'il fait par un verbe
   (« Importer un plan… », pas « Plan »). Les points de suspension
   signalent qu'une fenêtre va s'ouvrir avant l'action.
4. **Le calme par défaut.** Les actions d'une ligne n'apparaissent qu'au
   survol ou sur la ligne choisie ; la couleur est réservée à l'état
   (sélection, réussite, alerte, erreur).
5. **Le clavier partout.** Toute action fréquente a un raccourci, montré
   dans les menus et les bulles d'aide (§ 7).

## 2. Gabarit d'un écran

```
┌──────────────────────────────────────────────────────────────────────┐
│ InSitu   [1 Façade][2 Redressement][3 Recalage][4 Diapositive][5 Bibliothèque]  Présentation ▾ │  barre de l'app
├──────────────────────────────────────────────────────────────────────┤
│ [▣ Projet ▾] │ actions de l'étape …                 │ affichage    │  barre d'outils
├──────────────────────────────────────────────────────────────────────┤
│ bannière (message, erreur, progression) — seulement si besoin       │
├───────────┬──────────────────────────────────────┬───────────────────┤
│ Panneau   │                                      │ Inspecteur        │
│ des objets│           Zone de travail            │ (réglages de ce   │
│ (liste)   │           (toile, images)            │  qui est choisi)  │
│ 240 px    │                                      │ 300 px            │
├───────────┴──────────────────────────────────────┴───────────────────┤
│ barre d'état : où on en est, légende des couleurs                    │
└──────────────────────────────────────────────────────────────────────┘
```

Composant `Screen` (`toolbar`, `banner`, `sidebar`, `inspector`, `status`).
Les deux panneaux sont facultatifs : un écran sans objets à lister n'a pas
de panneau de gauche.

- **Barre de l'app** : les étapes du travail, numérotées dans l'ordre
  (⌘1 à ⌘5), puis le menu Présentation (apparence, raccourcis,
  démonstration de la visionneuse). Un nouvel écran du plan y ajoute son
  étape, à sa place dans l'ordre du travail.
- **Barre d'outils** (`Toolbar`), de gauche à droite : le menu Projet
  (`ProjectMenu`, le même partout), un séparateur, les actions de l'étape
  (importer, détecter…), un `ToolSpacer`, puis l'affichage (choix de vue,
  « Tout voir ») et l'export tout à droite. Un groupe peut porter un petit
  libellé (`ToolGroup label="Mire"`) quand les boutons seuls ne disent pas de
  quoi ils parlent.
- **Panneau de gauche** (`Panel side="left"`) : *ce qu'il y a* — les
  projecteurs, les dossiers de dessins, plus tard les calques. Le bouton
  « + » de son en-tête ajoute un objet.
- **Inspecteur** (`Panel side="right"`) : *ce qu'on règle* sur l'objet
  choisi ou sur la vue — échelle du plan, points communs, détail d'un dessin,
  export. Découpé en rubriques (`Section`) au titre en petites capitales.
  Quand un écran a plus de trois sujets à régler, l'inspecteur devient un
  **panneau de propriétés en onglets** (`TabbedPanel`, D15-i) : une rangée
  d'onglets (icône et libellé court, bulle d'aide) à la place du titre.
  L'onglet suit le contexte (choisir un dessin ouvre « Dessins », poser A et
  B ouvre « Plan ») et reste celui qu'on a choisi tant que rien de neuf ne
  se passe. Une pastille dit qu'un onglet attend quelque chose : un nombre
  (dessins choisis) ou un point orange (plan pas à l'échelle). Façade :
  Plan (plan, mise en plan, fresque) · Dessins · Calques · Export.
  L'inspecteur porte les *réglages et mesures* ; les *outils* (actions sur
  la sélection) sont des icônes dans la barre d'outils (D15-j).
- **Barre d'état** (`StatusBar`) : une phrase d'état (avec sa couleur), la
  consigne du geste en cours, et à droite la légende des pastilles.

## 3. Ajouter, modifier, supprimer

Le même geste pour tout objet d'une liste (projecteur, point, dessin,
calque…) :

| Geste | Où | Comment |
| --- | --- | --- |
| **Ajouter** | Bouton « + » (`IconButton icon="plus"`) dans l'en-tête du panneau qui liste l'objet ; dans l'état vide, le même ajout en bouton principal | L'objet ajouté est aussitôt choisi |
| **Choisir** | Clic sur la ligne (`ListRow`) ; ↑ ↓ quand la liste a le focus | La ligne choisie est bleue ; l'inspecteur montre l'objet |
| **Modifier** | Dans l'inspecteur, ou directement sur l'image (glisser un point, une poignée) | Le changement s'enregistre tout seul, pas de bouton « Enregistrer » |
| **Renommer** | Double-clic sur le nom, ou icône crayon de la ligne | Champ en place ; ↩ valide, Échap annule |
| **Supprimer** | Icône corbeille de la ligne (au survol) ou ⌫ sur l'objet choisi | Voir ci-dessous |

Suppression : l'annulation (⌘Z, tâche 4.3) rattrape toute modification du
projet. Un nouvel objet se supprime donc sans confirmation (les calques et
les dessins posés, par exemple). Les confirmations déjà en place
(`ConfirmDialog` : projecteur, plan, dessin de la bibliothèque) restent
jusqu'à ce qu'une session les retire (voir D-4.3-d) ; une confirmation reste
justifiée seulement si l'action efface un fichier, ce que ⌘Z ne rend pas.

Règle : **pas d'objet sans ses trois gestes.** Une session qui ajoute un
nouveau type d'objet lui donne ajout, modification et suppression, ou note
dans la demande de fusion ce qui manque. Exemple complet : les projecteurs
de l'étape Redressement (« + », double-clic ou crayon pour renommer,
corbeille ou ⌫ avec confirmation). Le dessin de la bibliothèque (« Retirer
de la bibliothèque… » dans l'inspecteur, ou ⌫) et le plan de la façade
(« Retirer le plan… » dans l'inspecteur Plan) ont aussi les trois gestes
(D15-h). Retirer n'efface jamais un fichier du disque.

## 4. États vides, attente, messages, erreurs

| Situation | Composant | Règle |
| --- | --- | --- |
| Rien à montrer (pas de projet, pas de photo) | `EmptyState` au centre de la toile | Un titre qui dit ce qui manque, une phrase qui explique, **le bouton qui règle le problème** |
| Travail en cours | `Busy` (pastille en haut de la toile) | Verbe en « …ant » ou nom d'action suivi de « … » : « Détection de la mire… » ; ne bloque pas l'écran |
| Long travail mesurable | `Progress` dans la bannière | « 120 dessins sur 200 » |
| Résultat d'une action | `Banner tone="success"` | Ce qui a été fait, avec les chiffres utiles ; se ferme |
| Avertissement | `Banner tone="warning"` ou texte orange de la barre d'état | Ce qui risque de mal se passer |
| Erreur | `Banner tone="danger"` | Ce qui s'est passé **et quoi faire** : « Cote illisible : écrire par exemple 12,5 m » ; jamais un message technique seul |
| Consigne du geste en cours | Barre d'état, en bleu | « Cliquer le point B sur le plan (Échap : annuler) » |
| Choix à faire avant de continuer | `Dialog` | Titre en question, bouton principal à droite |
| Aperçu avant un import | `Dialog size="large"` | Image du résultat à gauche, liste à cocher à droite, phrase de ce qui sera fait à côté du bouton |

## 5. Typographie

Police du système (SF Pro), comme toute app Mac.

| Jeton | Taille | Usage |
| --- | --- | --- |
| `--ui-text-xs` | 11 px | Titres de rubrique en petites capitales, raccourcis |
| `--ui-text-sm` | 12 px | Barre d'état, détails d'une ligne, aides |
| `--ui-text-md` | 13 px | Texte courant, boutons, champs (taille macOS) |
| `--ui-text-lg` | 15 px | Nom de l'objet choisi dans l'inspecteur, titre de dialogue |
| `--ui-text-xl` | 20 px | Titre d'un état vide |

Graisses : 400 (texte), 500 (boutons, titres de ligne), 600 (titres). Les
nombres sont en chiffres de même largeur dans les listes et la barre d'état.
Nombres et unités en français : virgule décimale, espace des milliers
(`toLocaleString("fr-FR")`), unité après une espace (« 12,5 m »).

## 6. Couleurs, espacements, formes

Deux thèmes, **sombre par défaut** (on travaille sur des images au trait
blanc), **clair** au choix, et **automatique** qui suit macOS : menu
Présentation → Apparence. La toile reste sombre dans les trois cas.

| Rôle | Jeton | Sombre | Clair |
| --- | --- | --- | --- |
| Fond de fenêtre | `--ui-bg` | #1e1e20 | #e8e8ea |
| Barres et panneaux | `--ui-surface` | #262628 | #f5f5f6 |
| Menus, dialogues | `--ui-surface-raised` | #2f2f32 | #ffffff |
| Toile des images | `--ui-canvas` | #161618 | #161618 |
| Texte | `--ui-text` | #f2f2f4 | #1d1d1f |
| Texte secondaire | `--ui-text-secondary` | #a9a9b0 | #5c5c63 |
| Sélection, action principale | `--ui-accent` | #0a84ff | #007aff |
| Réussite | `--ui-success` | #32d74b | #1f8a3b |
| Attention | `--ui-warning` | #ff9f0a | #b35900 |
| Erreur, suppression | `--ui-danger` | #ff453a | #d70015 |
| Information, consigne | `--ui-info` | #64d2ff | #0070a8 |
| Repère sur l'image | `--ui-marker` | #ed1c24 | #ed1c24 |

- **Une seule couleur d'accent, le bleu du Mac**, pour la sélection et
  l'action principale. Le rouge de la mire (`--ui-marker`) est réservé aux
  repères posés sur les images.
- **Pastilles d'état** (`Dot`) : vert = bon, orange = à surveiller,
  rouge = douteux ou en erreur, bleu = pas encore mesuré, gris = rien.
  Toujours accompagnées d'une légende dans la barre d'état.
- Un écran **n'écrit jamais une couleur en dur** : il emploie un jeton. Seuls
  les dessins posés sur les images (dans la visionneuse) ont leurs couleurs
  propres, choisies pour se voir sur une photo.
- Espacements sur une grille de 4 px (`--ui-space-1` à `--ui-space-6` :
  4, 8, 12, 16, 24, 32). Rayons : 4 (pastilles), 6 (boutons, lignes), 10
  (menus, cartes). Hauteur d'un contrôle : 26 px ; d'une ligne : 28 px.

## 7. Menus et raccourcis

Les menus suivent l'ordre des menus du Mac et montrent leurs raccourcis à
droite (`MenuButton`). Dans la fenêtre : le menu Projet et le menu
Présentation. En haut de l'écran, la **barre de menus du Mac**
(`src-tauri/src/menu.rs`) : InSitu (À propos, Masquer, Quitter), Fichier
(Nouveau, Ouvrir, Relire, Importer, Exporter, Fermer), Édition (Annuler et
Rétablir, propres à InSitu depuis la tâche 4.3 : le projet hors d'un champ,
la frappe dans un champ ; Couper, Copier, Coller, Tout sélectionner, pour
les champs de texte),
Présentation (les étapes, Tout voir, Plein écran), Fenêtre, Aide
(Raccourcis clavier). Une entrée propre à InSitu porte le raccourci de
l'interface comme identifiant (`raccourci:mod+n`) : la coque l'envoie à
l'interface, qui déclenche la même action que la touche (`runShortcut`).
Un raccourci ajouté à la barre de menus doit donc exister dans un
`useShortcuts` ; sinon l'entrée ne fait rien.

| Raccourci | Action |
| --- | --- |
| ⌘N / ⌘O / ⌘R | Nouveau projet / Ouvrir / Relire depuis le disque |
| ⌘Z / ⇧⌘Z | Annuler / rétablir la dernière modification du projet, quel que soit l'écran (tâche 4.3) ; dans un champ de texte, la frappe |
| ⌘I | Importer : le plan (Façade), la photo (Redressement), un dossier (Bibliothèque) |
| ⌘E | Exporter : la photo redressée (Redressement), les diapositives (Façade, tâche 3.5) |
| ⌘1 à ⌘5 | Étapes Façade, Redressement, Recalage, Diapositive, Bibliothèque |
| ⌘0 | Tout voir |
| ⌘/ | Liste des raccourcis |
| ↑ ↓ | Objet précédent, suivant de la liste qui a le focus |
| ⌫ | Supprimer l'objet choisi |
| Échap | Annuler le geste en cours, fermer un menu ou un dialogue |
| ↩ | Valider un dialogue |
| ⌘A / ⇧⌘A | Façade : choisir tous les dessins posés / aucun |
| ⌘C ⌘X ⌘V | Façade : copier, couper, coller au centre de la vue |
| ⌘F / ⌘B | Façade : coller devant / derrière, à la même place |
| ⌘D | Façade : répéter le dernier geste (dupliquer, avec ⌥-glisser) |
| ⌘G / ⇧⌘G | Façade : grouper / dégrouper |
| ⌘] ⌘[ (⇧ : tout au bout) | Façade : avancer, reculer, premier plan, arrière-plan |
| ← → ↑ ↓ (⇧ : × 10) | Façade : déplacer les dessins choisis de 10 mm |
| Espace + glisser | Façade : déplacer la vue (quand des dessins sont posés) ; Gomme et crayon : déplacer la vue |
| E / N, T / Z, [ ] | Gomme et crayon (Bibliothèque, tâche 4.8) : gomme / crayon, trait / zone, épaisseur ; ⌘Z y annule la dernière retouche |

Réservés pour la suite (ne pas les prendre pour autre chose) : ⌘+ / ⌘− zoomer, ⌘, préférences (6.5).

Un raccourci se branche par `useShortcuts([{ keys: "mod+i", run, enabled }])`
dans l'écran qui l'offre, s'affiche par la propriété `shortcut` du bouton,
et s'ajoute à `APP_SHORTCUTS` (`src/ui/shortcuts.ts`) et au tableau
ci-dessus. Sans ⌘, un raccourci ne se déclenche pas pendant la saisie dans
un champ.

## 8. Rédaction

- Français simple, phrases complètes dans les messages ; boutons à
  l'infinitif (« Importer la photo… », « Valider l'échelle »).
- Le vocabulaire métier de `CLAUDE.md` (façade, diapositive, mire,
  recalage…), jamais de terme du code.
- Majuscule seulement au premier mot (« Points communs », pas « Points
  Communs »).
- Une bulle d'aide (`title`) sur tout bouton dont le libellé ne suffit pas,
  et toujours sur un bouton à icône seule.

## 9. Ajouter un écran : liste de contrôle

1. Une étape dans `STEPS` (`src/App.tsx`), à sa place dans l'ordre du travail.
2. `Screen` avec `Toolbar` qui commence par `ProjectMenu`.
3. Objets à gauche (`Panel side="left"`, `List`, `ListRow`), réglages à
   droite (`Panel side="right"`, `Section`, `Property`).
4. Un `EmptyState` pour chaque cas où la toile est vide, avec son bouton.
5. Erreurs en `Banner tone="danger"`, attente en `Busy`, état en `StatusBar`.
6. Raccourcis branchés, affichés, ajoutés à la liste.
7. La feuille de style de l'écran ne contient que ce qui lui est propre,
   avec les jetons ; pas de bouton, champ ou panneau redessiné.
8. Capture de l'écran dans les deux thèmes dans la demande de fusion.

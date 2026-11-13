# Journal de décisions

Chaque choix technique qui n'est pas écrit dans `docs/plan.md` est noté ici,
du plus récent au plus ancien : la décision, la raison, et ce qu'elle interdit.
Une session ne revient pas sur une décision sans l'accord de Benjamin.

## 2026-11-13 — Réglages d'export depuis la Diapositive (demande de Benjamin)

Demande de Benjamin : « dans diapositive > export : il y a la composition
qui s'affiche (il faudrait des filtres, comme dans façade export) ».

**D-3.5-n. La fresque importée peut ne pas être gravée, et la Diapositive
règle son export.** Nouveau réglage `export.fresco` (oui par défaut, comme
avant ; absent du fichier quand il vaut oui) : décoché, l'export grave les
dessins posés seuls, sans la fresque importée en une image (ou PDF, SVG).
Case « Graver la fresque importée » dans l'onglet Export de la Façade
(montrée seulement si le projet a une fresque), option
`insitu export --sans-fresque`. Sans fresque gravée ni dessin posé, l'export
refuse et le dit. À la demande de Benjamin, cette décision revient sur la
partie de D-3.5-l « on ne règle pas l'export depuis la Diapositive » :
**Exporter cette diapositive…** (⌘E) ouvre une fenêtre avec les mêmes
réglages que l'onglet Export (même composant), partant de ceux du projet,
la case de la fresque cochée comme la case **Fresque** de l'aperçu (ce qui
se voit est ce qui part). Les réglages choisis deviennent ceux du projet,
comme depuis la Façade. Les dessins des calques masqués restent écartés
(inchangé). Le plan de contrôle garde les réglages du projet.

## 2026-11-13 — Façade : tout l'affichage dans le menu Affichage (demande de Benjamin)

Demande de Benjamin : « dans façade, il y a dans le menu de gauche
afficher, tout devrait être dans le menu affichage ».

**D15-q. Le panneau Projecteurs ne règle plus l'affichage.** La rubrique
« Afficher » du panneau de gauche (photos, emprises, recouvrements,
opacité des photos) disparaît : ces réglages sont dans le menu
**Affichage** de la barre du haut, où photos, emprises et recouvrements
étaient déjà. L'opacité devient un choix parmi 100, 80, 50 et 25 % (un
menu n'a pas de curseur). La pastille du menu compte aussi ces réglages,
et « Tout montrer » les remet à l'ouverture. Seules les cases devant
chaque projecteur restent à gauche : elles disent quel projecteur on
regarde, dans la liste même.

## 2026-11-13 — Points réglés aux flèches (demande de Benjamin)

Demande de Benjamin : « décalage, 4 coins etc : quand on clique sur un
point on peut déplacer avec les flèches. avec shift le pas est plus grand ».

**D15-p. Un point cliqué se déplace aux flèches, d'un point d'écran (dix
avec ⇧).** Le pas est compté en points d'écran et non en millimètres : les
points vivent dans des unités différentes selon l'écran (mm de façade, mm de
diapositive, pixels de photo, fractions de dessin), et un pas d'écran suit
le zoom comme l'œil (zoomer donne un réglage plus fin). Les endroits :
- **Façade, mode 4 coins** : le point cliqué est dessiné plus gros, cœur
  blanc ; les flèches le déplacent. Sans point cliqué, les flèches gardent
  leur sens (la sélection de 10 mm, 100 avec ⇧). Échap oublie d'abord le
  point, puis quitte le mode 4 coins.
- **Diapositive, retouche** : le point cliqué de la grille bouge ; sans
  point cliqué, tout le dessin choisi se décale dans la diapositive. Tant
  qu'un dessin est choisi, ← et → ne changent plus de projecteur (cliquer
  dans le vide pour retrouver ce raccourci).
- **Recalage** et **assemblage de photos** : le point choisi bouge du côté
  cliqué en dernier (façade ou photo ; photo à placer ou photo d'appui) ;
  la consigne le dit.
- **Redressement, coins seuls** : la dernière poignée saisie.
- **Silhouette** (Bibliothèque) : le point choisi.
Une flèche tenue enfoncée répète le pas ; en Diapositive, Recalage et
Assemblage, l'enregistrement (et le calcul du moteur) attend 0,4 s après le
dernier appui (`NudgeSettle`, `src/viewer/nudge.ts`). En Façade, chaque
appui est un pas d'annulation, comme le déplacement de la sélection.

## 2026-11-13 — Plan de contrôle à graver (demande de Benjamin)

Demande de Benjamin : « pouvoir graver le plan vu d'une diapositive, pour
vérifier que le recalage est bon : on grave le plan, via la déformation ;
si c'est juste, tous les dessins seront nickel ».

**D-3.5-m. Le plan de contrôle est un export à part.** `export_control_plans`
(`insitu export --plan-de-controle`, boutons **Plans de contrôle…** dans
l'onglet Export et **Plan de contrôle…** dans la Diapositive) écrit pour
chaque projecteur choisi `<projet>-<diapositive> - PLAN DE CONTROLE.png` :
le plan de la façade sous l'emprise (D-3.6-e), en traits blancs sur noir,
passé par le recalage et par la même chaîne que les diapositives (marge,
miroir, négatif, noir et blanc pur, résolution), « PLAN DE CONTROLE »
ajouté au texte de la marge. Le plan n'est pas mêlé aux dessins : un
fichier distinct évite de le graver par erreur sur la plaque définitive.
Les réglages d'export du projet ne sont pas réenregistrés. Le plan est
rendu en 8 192 px au plus grand côté de l'emprise (mémoire raisonnable) :
pour une emprise de 4 m, un pixel du plan fait 0,5 mm sur la façade, assez
pour juger un recalage. Il faut un plan mis à l'échelle ; un projecteur
dont l'emprise tombe hors du plan est signalé, pas exporté.

**D-3.5-n. La fresque se retire.** Demande de Benjamin (13 novembre 2026) :
une fresque importée ne pouvait qu'être remplacée. Bouton **Retirer** à côté
de **Remplacer…** (onglet Plan, rubrique Fresque), avec confirmation ;
`remove_fresco`, `insitu fresco <projet> --remove`. Comme pour le plan
(D15-h), ses fichiers restent dans `fresque/` : rien n'est effacé du disque,
on peut la réimporter. Les dessins posés et leurs calques ne changent pas.

## 2026-11-13 — Fresque et plan dans la Diapositive (demande de Benjamin)

Demande de Benjamin : « fresque : si elle est masquée, elle reste affichée
dans diapositive. Idem, dans diapositive, il faut pouvoir masquer les
plans. » Benjamin a précisé (carte de choix) qu'il s'agit des plans de la
Façade, à montrer dans la Diapositive avec une case pour les cacher.

**D-3.6-d. La fresque cachée l'est aussi dans la Diapositive.** Les cases
**Fresque** et **Plan** de la Diapositive et celles du menu Affichage de la
Façade sont les mêmes réglages (gardés sur ce Mac, pas dans le projet).
Fresque cachée, l'aperçu est recalculé sans elle (`slide_views_with`,
`insitu slides --sans-fresque`) : les dessins posés gardent leur silhouette.
L'export, lui, grave toujours la fresque : cacher est une façon de
regarder, pas un réglage d'export (comme D15-l).

**D-3.6-e. Le plan de la Façade vu par chaque projecteur.** Le moteur rend la
partie du plan sous l'emprise (3 072 px au plus grand côté), en négatif,
puis la ramène dans le carré de 280 mm par le recalage, exactement comme la
fresque, en un PNG de 2 048 px rangé dans le cache
(`diapositives/plan-<clé>.png` ; la clé tient compte du fichier du plan, de
son échelle, de sa rotation, de sa page et du recalage). Il n'est calculé
que si la case **Plan** est cochée (`insitu slides --plan`). L'interface le
montre en gris bleuté sous les dessins, plus foncé sur fond blanc. Il n'est
jamais exporté ; « Comme dans LightBurn » le cache. Seul le plan de
référence de la Façade est montré : les plans supplémentaires (fil
« plusieurs plans ») devront s'y ajouter.

## 2026-11-13 — Exporter une diapositive ou toutes (demande de Benjamin)

Demande de Benjamin : « export : propose l'export d'une diapo ou de toutes
les diapos », et « dans le menu diapositive, prévoir un export (pour
exporter qu'une diapo) ».

**D-3.5-l. On peut exporter une seule diapositive.** Le moteur savait déjà
n'exporter que certains projecteurs (`only`, ligne de commande
`insitu export --only`) ; la commande de la coque le reçoit maintenant.
Onglet Export de la Façade : choix « Toutes » ou « A seulement » (seuls
les projecteurs recalés sont proposés ; le bouton dit ce qu'il exporte).
Étape Diapositive : bouton « Exporter cette diapositive… » (⌘E), avec les
réglages d'export enregistrés dans le projet (revu par D-3.5-n : une
fenêtre les propose maintenant), relus au moment d'exporter
(on ne règle pas l'export depuis la Diapositive, pour n'avoir qu'un seul
endroit où le faire). Le choix « une ou toutes » n'est pas gardé dans le
projet : il revient à « Toutes » d'un lancement à l'autre.

## 2026-11-13 — Plusieurs plans dans un projet (demande de Benjamin)

Demande de Benjamin (13 novembre) : « pouvoir importer plusieurs plans, les
déplacer et mettre à l'échelle […] un espace de travail qu'on compose selon
les projets » ; « un groupe de plans (plan archi détaillé, plan simplifié,
plan avec des guides, tous issus du même fichier) […] afficher ou non chaque
plan, en fonction des moments du travail ».

**D-PL-a. Le premier plan reste la référence** (choix de Benjamin, carte du
13 novembre, « valable pour un groupe de plans »). `project.facade` ne change
pas : il donne l'échelle et le repère en mm ; dessins posés, recalages,
fresque y restent attachés, et il ne se déplace pas. Les autres plans sont
dans `project.planGroups` (absent tant qu'il n'y en a pas : un projet comme
Trévarez s'écrit exactement comme avant). Tourner le plan de référence fait
tourner les groupes avec lui (comme le reste, D15-k). Retirer ou remplacer
le plan de référence garde les groupes, en mm.

**D-PL-b. Un groupe = un fichier, posé à une place et une échelle ; ses vues
bougent ensemble.** `PlanGroup` : `source` (copiée dans `facade/plans/`),
`origin` (coin haut-gauche, mm), `mmPerUnit`, `rotation` (quarts de tour),
`hidden`, `plans` (les vues : `page`, `layers`, dimensions, `hidden`). Tirées
du même fichier, les vues restent calées entre elles : c'est la règle « le
premier sert de référence » dans le groupe. La taille du groupe est celle de
sa première vue (pages de tailles différentes : alignées sur leur coin
haut-gauche).

**D-PL-c. Les calques d'un PDF ou d'un `.ai` deviennent des vues.** Un `.ai`
enregistré « compatible PDF » (réglage par défaut d'Illustrator) garde ses
calques en « contenus facultatifs » (OCG) : `ECURIES-COMPOSITION.ai` en a 14,
dont « Calque 1 » (le plan) et « Découpes ». `hayro` sait masquer un calque
mais ne lit l'état des calques que dans le fichier ; pour choisir, le moteur
ajoute au fichier, en mémoire seulement, une mise à jour incrémentale
(nouveau catalogue, réglage des calques, table de renvois), comme Acrobat
(`facade/pdf_layers.rs`). Le fichier du projet n'est jamais modifié ; ce qui
n'est sur aucun calque reste montré ; un PDF chiffré refuse le choix des
calques. La clé du cache de tuiles contient les calques choisis (sans choix,
elle ne change pas). Par voie de conséquence, un `.ai` est accepté partout
où un plan PDF l'est (il était refusé avec le conseil de l'enregistrer en
PDF, D-3.1-f) ; un `.ai` sans sa partie PDF reçoit ce conseil.

**D-PL-d. Place de départ.** Même fichier que le plan de référence (contenu
identique) : même place, même échelle, même sens, les calques tombent pile.
Autre fichier : en haut à gauche, à la largeur du plan de référence. Il faut
un plan de référence à l'échelle pour ajouter un plan.

**D-PL-e. Déplacer, mettre à l'échelle : champs, souris, deux points ; pas
de rotation libre.** Position et largeur en mètres dans l'inspecteur ;
« Déplacer à la souris » (glisser sur la toile ; la vue cesse de se
recadrer toute seule pendant le geste) ; « Caler par deux points… » : deux
points du plan et leur place sur la façade donnent agrandissement et
déplacement (`facade::alignment`) ; l'écart d'angle qui reste est annoncé
(un plan ne tourne que par quarts de tour, comme le plan de référence,
D15-k).

**D-PL-f. Les plans se superposent comme du papier calque.** Chaque vue est
affichée en « produit » (blend `multiply`, nouveau dans la visionneuse) sur
une feuille blanche à sa taille, sous tout : le blanc laisse voir dessous,
les traits s'ajoutent. En négatif (menu Affichage, « Plan en négatif »),
« superposition claire » sans feuille. Ordre : feuilles, plan de référence,
groupes du fond vers l'avant, puis fresque et projecteurs.

**D-PL-g. Montrer ou cacher est enregistré dans le projet**, contrairement
au menu Affichage (D15-l) : c'est l'état d'un moment du travail, partagé
entre Benjamin et Marc par Dropbox, et annulable par ⌘Z. L'œil du plan de
référence reste celui du menu Affichage. Pas encore : les plans ajoutés ne
sont ni dans la fiche de calepinage, ni sous la Diapositive (le fil
« options d'affichage » y rend le plan de référence seul), ni dans l'export ;
pas de verrou par plan.

## 2026-11-13 — Diapositive comme dans LightBurn, menu Projet retiré (demande de Benjamin)

Demandes de Benjamin (13 novembre) : « menu diapositive : ajouter option
pour voir ou non la photo. fond noir / blanc / pierre (= pouvoir voir la
diapo telle qu'elle va être envoyée à LightBurn) » ; « menu horizontal : pas
besoin du sélecteur de projet puisque présent dans le menu Mac ».

**D-3.6-a. L'écran Diapositive montre la diapositive avec ou sans photo, sur
un fond choisi.** Groupe « Photo » (montrer ou non la photo redressée) et
groupe « Fond » (noir, blanc, pierre) : le fond est un carré de 280 mm sous
la photo, visible quand elle est cachée. Sur fond blanc, des dessins blancs
passent en noir (et inversement en revenant au noir ou à la pierre) pour
ne jamais disparaître ; une couleur choisie (cyan…) est gardée. Bouton
« Comme dans LightBurn » : sans photo, fond noir, traits blancs à pleine
opacité, c'est-à-dire l'aperçu calculé par le moteur avec le même calcul
que l'export (D-3.6) ; un second clic revient au réglage d'avant. Réglages
retenus d'un lancement à l'autre (`localStorage`). L'aperçu fait 2 048 px :
c'est le dessin de l'export, pas sa finesse (15 433 px).

**D15-o. Le menu Projet ne s'affiche plus dans l'app.** Nouveau, Ouvrir,
Relire, Annuler, Rétablir sont dans la barre de menus du Mac (Fichier,
Édition, D15-g) : `ProjectMenu` ne dessine plus rien quand le moteur est là,
mais garde ses raccourcis (c'est lui qui les exécute quand le menu du Mac
les envoie). Le nom du projet ouvert passe dans la barre de l'app, à côté
d'« InSitu » (dossier en bulle d'aide). Dans le navigateur seul, sans barre
de menus, le menu reste en tête de la barre d'outils. Un séparateur en tête
de barre ne s'affiche pas.

## 2026-11-13 — Options d'affichage et rangement des menus (demande de Benjamin)

Demande de Benjamin (13 novembre) : « retouche dessin et silhouette : garde
option noir / damier pierre » ; « ajouter montrer/cacher le plan, la
fresque, inverser les couleurs etc. » ; « barre menu : remplacer plan et
assembler photo dans onglet plan ».

**D15-l. Un menu Affichage dans la barre d'outils de la Façade.** Il range,
à droite avec « Tout voir », ce qui change la façon de regarder sans changer
le projet : montrer le plan, la fresque, les dessins posés, les photos, les
emprises, les recouvrements (ces trois derniers sont les mêmes cases que le
panneau Projecteurs) ; plan en négatif ; tout en négatif. Choix :
- *« Inverser les couleurs » est lu de deux façons*, les deux sont offertes :
  le plan seul en négatif (filtre de couleur PixiJS sur son calque, à la
  finesse de l'écran : traits blancs sur noir, comme les dessins), et toute
  la vue en négatif (filtre CSS sur la toile, avec rotation de teinte pour
  garder la teinte des repères : fond blanc, traits noirs, comme dans
  Illustrator). À confirmer par Benjamin.
- *Des dessins cachés ne se touchent pas* : ni sélection, ni outils, ni
  pose (message dans l'onglet Dessins), pour ne jamais modifier ce qu'on ne
  voit pas. Leurs contours et le cadre de sélection sont cachés aussi.
- *Rien n'est écrit dans le projet ni dans l'export* : les choix sont retenus
  dans l'app (`localStorage`, comme l'onglet choisi, D15-i). La case
  « Montrer sur la façade » de la fresque (onglet Plan) est le même réglage.
- Pas de raccourci clavier pour l'instant : à ajouter si Benjamin s'en sert
  souvent.

**D15-m. Le plan et l'assemblage de photos s'ouvrent depuis l'onglet Plan.**
« Importer un plan… » (ou « Remplacer le plan… », ⌘I inchangé) et
« Assembler des photos… » quittent la barre d'outils pour le haut de
l'onglet Plan du panneau de droite : la barre ne garde que les outils des
dessins (D15-j) et l'affichage.

**D15-n. Un seul fond pour la bibliothèque et ses éditeurs.** Le choix
noir, damier ou pierre est dans la barre de la bibliothèque et dans celle
de chaque éditeur ouvert par-dessus (gomme et crayon, silhouette,
comparaison des vecteurs), qui la cachent ; c'est le même réglage, retenu
d'un lancement à l'autre (`src/library/backdrop.ts`). Le « Résultat » de la
silhouette suit le fond choisi, sauf le noir, où la silhouette noire ne se
verrait pas : fond rayé.

## 2026-11-12 — Essai du 13 novembre : sens de la nouvelle mire

**D-essai-a. Le sens de la mire version 2 se lit sur ses repères codés.**
La mire version 2 n'a plus les six petits cercles (D-2.2-c). Conséquence
constatée avant l'essai grandeur nature : la détection seule échouait
toujours (« sens de la mire incertain »), et après la pose des coins la
diapositive pouvait sortir tournée d'un quart de tour sans aucun
avertissement, donc une plaque gravée de travers. Correction : quand les
petits cercles ne tranchent pas, on lit chacun des 16 repères AprilTag à sa
place connue (modules de 2 mm, seuil propre à chaque repère entre son
pourtour noir et son module le plus clair), pour chacun des quatre sens ; le
bon sens donne presque tous les modules justes, les autres environ la
moitié. On garde la moyenne de la meilleure moitié des repères (un repère
dans l'ombre ou un reflet ne compte pas) ; il faut 0,15 d'écart entre le
meilleur sens et le suivant. Ce n'est pas la tâche 2.4b : l'amorce reste le
cercle rouge et la croix (ou les coins), les repères ne servent qu'au sens.
Raison : une lecture à position connue est simple, sans dépendance, et
suffit pour l'essai ; le lecteur de repères complet (2.4b) reste à faire.
Vérifié par `crates/insitu-core/tests/mire_v2_sens.rs` sur le PDF à
imprimer rendu par le moteur, dans les quatre sens, détection seule et
coins donnés dans le désordre ; pas encore sur une vraie photo de la mire
imprimée.

## 2026-11-13 — Import Illustrator : images incorporées, un seul bouton d'import (demandes de Benjamin)

**D-6.3-i. Les images incorporées sont extraites en PNG.** Une image copiée
dans le `.ai` (bloc `%AI5_BeginRaster` au chemin vide, ou au chemin d'un
fichier temporaire d'Illustrator `/var/folders/…`) est relue dans les
données privées, pixels bruts après `%%BeginData` : masque d'un bit (peint
là où le bit vaut 0, comme `imagemask`), gris, RVB ou CMJN sur 8 bits.
Elle devient un PNG en niveaux de gris, trait blanc sur noir, rangé dans
`bibliotheque/<nom du .ai>/Incorporées/Incorporée NN - <calque>.png`, puis
un dessin posé comme une image liée (même matrice, même calque, à sa place
dans l'empilement). Deux images incorporées identiques donnent un seul
fichier. Une image incorporée hors de tout calque (réserve de symboles ou
de motifs d'Illustrator, posée loin du plan de travail) est laissée de côté.
Un second import reprend le fichier déjà extrait. Sur ECURIES-COMPOSITION.ai :
6 images relues (5 masques d'un bit jusqu'à 11 598 × 6 521 px, une image
CMJN du calque « Intentions »), 5 fichiers, orientation et place vérifiées
contre le rendu PDF du même fichier. Limite : le masque d'écrêtage qui
recadre parfois ces images dans Illustrator n'est pas repris (D-6.3-b).

**D-6.3-j. Un seul bouton : « Importer la fresque… ».** Accord de Benjamin
le 13 novembre : un `.ai` passe par la fenêtre d'aperçu et ses images
deviennent des dessins dans les calques ; une image, un PDF ou un SVG
restent un fond de référence (D-3.5-h à k). La fenêtre garde « Poser en fond
seulement » pour un `.ai` fait de tracés, sans images. La rubrique
« Composition Illustrator » de l'onglet Calques est retirée.

## 2026-11-13 — Import Illustrator : aperçu, choix, lien avec la bibliothèque (demande de Benjamin)

Benjamin veut que l'import d'un `.ai` crée un dessin par image, fasse le lien
avec les dessins déjà dans la bibliothèque, garde les calques d'Illustrator,
et passe par une fenêtre qui montre le résultat et laisse choisir quoi
importer. Module `crates/insitu-core/src/illustrator/plan.rs`, commande
`insitu illustrator preview` et options `--images`, `--layer` de `import`,
fenêtre `src/compose/IllustratorPreviewDialog.tsx`.

**D-6.3-f. Un lien reprend le dessin déjà dans la bibliothèque.** Avant
toute copie, chaque fichier lié est comparé à la bibliothèque du projet :
même contenu, octet pour octet (taille d'abord, quel que soit le dossier) →
ce dessin est repris, avec ses retouches, sa silhouette et ses réglages ; si
le fichier lié n'est pas sur ce Mac, un dessin du même nom de fichier (sans
accents ni casse, de préférence dans un dossier du même nom) est repris.
Sinon le fichier entre dans `bibliotheque/<nom du .ai>/` comme avant
(D-6.3-e). Raison : Benjamin importe souvent les dossiers de dessins avant
la composition ; les copier une seconde fois doublait la bibliothèque et
perdait les retouches. Limite : un dessin du même nom mais d'un autre
contenu, quand le fichier lié est présent, n'est pas repris (le fichier du
`.ai` fait foi).

**D-6.3-g. L'import passe par un aperçu, sans rien écrire.** La lecture du
`.ai` rend calques (nom, masqué, verrouillé, couleur d'Illustrator, images
liées et incorporées), fichiers liés (bibliothèque, bibliothèque par le nom,
nouveau, introuvable) et chaque image à sa place sur la façade. La fenêtre
dessine les images en rectangles de la couleur de leur calque, liste les
calques (l'avant en haut, comme dans Illustrator) et leurs fichiers, avec
des cases à cocher. Tout ce qui peut être importé est coché au départ ; un
lien introuvable ne se coche pas. Le moteur reçoit le rang des images
choisies. Les calques sans image liée (tracés seulement) sont nommés, pas
repris. Une image incorporée hors de tout calque (réserve de symboles) n'est
comptée que dans le total.

**D-6.3-h. Calques : ceux de premier niveau, toujours.** Le fichier de
Trévarez n'a pas de sous-calques ; l'app non plus (D-4.3). Les images d'un
sous-calque restent dans le calque de premier niveau, comme en D-6.3-b. Si
un fichier réel en a besoin, ce sera une demande à part.

## 2026-11-12 — Tâche 6.3 : import Illustrator

Module `crates/insitu-core/src/illustrator/`, commande `insitu illustrator
inspect|import`, rubrique « Composition Illustrator » de l'onglet Calques de
l'écran Façade (`src/compose/IllustratorImport.tsx`), mode d'emploi
`docs/composition.md`, « Importer une composition Illustrator ».

**D-6.3-a. La composition est lue dans les données privées d'Illustrator.**
La partie PDF d'un `.ai` ne garde que des images recopiées, sans leurs liens.
La composition telle qu'Illustrator la relit est rangée à part
(`/PieceInfo /Illustrator /Private /AIPrivateData1…n`), en texte PostScript
compressé : Zstandard depuis Illustrator 2020 (`%AI24_ZStandard_Data`), zlib
avant (`%AI12_CompressedData`). Deux dépendances en Rust pur : `ruzstd` et
`miniz_oxide` (déjà tirée par `png`). Chaque image liée est un bloc
`%AI5_BeginPlace` : matrice, rectangle de l'image, chemin absolu, et depuis
Illustrator 2024 son dossier relatif au `.ai`. Les anciens blocs
`%AI5_BeginRaster` avec un chemin sont lus aussi. Format non documenté par
Adobe : déduit des fichiers d'Épinal et de Trévarez, et vérifié en
superposant le résultat au rendu PDF du même `.ai` (Trévarez : 90 % des
pixels posés tombent sur un trait du PDF à 1 mm près ; le reste vient du PDF,
dont les images sont réduites à 150 ppi).

**D-6.3-b. Ce qui est repris, ce qui ne l'est pas.** Repris : position,
taille, rotation, retournement, calque de premier niveau (nom, masqué,
verrouillé), ordre (le premier lu est au fond). Une image masquée dans un
sous-calque d'un calque visible est posée masquée, seule (`placements[].hidden`,
D-4.5-g), dans son calque.
Non repris, et dit dans le compte rendu : les images incorporées (sans lien),
le biais (posé sans), et ce que l'app ne sait pas faire : masques
d'écrêtage, opacité, modes de fusion, tracés vectoriels d'Illustrator.

**D-6.3-c. Les liens sont retrouvés à côté du `.ai`.** Le chemin absolu est
celui du Mac qui a enregistré le fichier (`/Users/marcsicard/…`). Dans
l'ordre : le dossier relatif noté par Illustrator, compté depuis le `.ai`
lui-même (`../Algues/` désigne `Ecuries - Fresque/Algues`) ; le chemin
absolu ; un fichier du même nom sous le dossier du `.ai` (5 niveaux), de
préférence dans un dossier du même nom que celui du lien. Les noms sont
comparés sans accents ni casse (le `.ai` écrit « Mure\u0300ne », décomposé).
Un lien introuvable est signalé, pas bloquant. Les fichiers temporaires
d'Illustrator (`/var/folders/…`) sont ignorés.

**D-6.3-d. Le plan de travail couvre la façade, comme la fresque.** Comme
pour la fresque (D-3.5-b), le plan de travail occupe toute la largeur de la
façade à partir du coin haut-gauche : la composition se fait sur un plan de
travail de la taille du plan de façade. La ligne de commande accepte un
autre rectangle (`--frame`).

**D-6.3-e. L'import s'ajoute.** Les fichiers liés vont dans la bibliothèque
sous `bibliotheque/<nom du .ai>/`, en gardant leurs sous-dossiers ; un
fichier déjà importé à l'identique est repris, pas copié deux fois. Les
dessins posés s'ajoutent à ceux de la façade, dans des calques repris par
leur nom. ⌘Z annule l'import en deux pas : les dessins posés, puis
l'ajout à la bibliothèque (deux enregistrements du projet).

## 2026-11-12 — Fresque en .ai, PDF ou SVG (demande de Benjamin)

Benjamin veut importer la fresque depuis un fichier Illustrator `.ai`, un PDF
ou une image. Module `crates/insitu-core/src/export/fresco.rs`, champ
`fresco.vector` du projet, option `insitu fresco --page`, bouton de l'écran
Façade inchangé (il accepte les nouveaux fichiers). Ce n'est pas la tâche
6.3 : celle-ci relira les images placées d'un `.ai` pour en faire des
dessins posés un par un (voir plus haut).

**D-3.5-h. Un `.ai` est lu comme un PDF.** Illustrator enregistre par défaut
(« Créer un fichier compatible PDF ») une copie PDF de la composition dans
le `.ai`. On la lit avec `hayro`, déjà utilisé pour les plans (D-3.1-a) :
pas de nouvelle dépendance. Le format est reconnu au contenu (`%PDF-`), pas
à l'extension. Un `.ai` sans cette copie (ancien format PostScript) est
refusé avec un message qui dit quoi cocher. Le SVG est lu par `resvg`,
comme pour les plans. Essayé sur deux vrais fichiers : « Epinal Coupe.ai »
(Dropbox d'Épinal) et la composition PDF des Écuries de Trévarez.

**D-3.5-i. Le fichier d'origine est gardé et relu pour chaque diapositive.**
Une façade de 27 m rendue en une seule image à la finesse d'une diapositive
(≈ 0,25 mm par pixel) ferait plus de 100 000 px de côté. On garde donc le
PDF (ou `.ai`, ou SVG) dans `fresque/` et, à l'export, on ne rend que la
partie de façade couverte par la diapositive, à un pixel de fresque par
pixel de diapositive là où elle est la plus fine (24 000 px de côté au plus,
576 Mo). Sur Trévarez, une diapositive de 15 433 px se calcule en 13 s en
session cloud (4 cœurs). À côté, un aperçu en niveaux de gris de 8 192 px au
plus grand côté (`fresque/<nom>-apercu.png`) sert à l'affichage, aux aperçus
de diapositives et à la simulation : `fresco.source` garde donc son sens
d'image, et rien ne change pour les écrans existants. Pas de changement de
`formatVersion` : le champ est facultatif.

**D-3.5-j. Ce qui n'est couvert par rien est noir.** La page est rendue sur
du noir, comme une transparence d'image (D-3.5-c) : pas de lumière là où il
n'y a pas de dessin. En contrepartie, tout ce qui est visible dans le
fichier est gravé, plan de fond et textes compris : il faut masquer le
calque du plan avant d'enregistrer (`docs/export.md`).

**D-3.5-k. Première page par défaut.** Dans l'app, seule la première page
d'un PDF (le premier plan de travail d'un `.ai`) est lue ; la ligne de
commande accepte `--page N`, gardé dans `fresco.vector.page`. Un choix de
page dans l'app viendra si Benjamin en a besoin.

## 2026-11-12 — Mise à jour automatique (hors plan, demande de Benjamin)

**D16. L'app se met à jour toute seule, avec l'accord de l'utilisateur.**
Demande de Benjamin : ne plus réinstaller le `.dmg` à chaque version. Revient
sur la fin de D12 (« pas de mise à jour automatique ») avec son accord. Au
lancement, l'app cherche une version plus récente ; si elle en trouve une, un
dialogue propose « Installer et relancer » ou « Plus tard » (la question
revient au lancement suivant). Une recherche ratée (pas de réseau) ne dit
rien. Code : `src-tauri/src/updates.rs`, `src/update/`.

**D16-a. Module de mise à jour de Tauri (`tauri-plugin-updater`), appelé
depuis la coque.** Deux commandes (`update_check`, `update_install`) passent
par `src/engine.ts` comme le reste ; l'interface n'utilise pas le module
JavaScript du greffon. Après installation, la coque relance l'app.

**D16-b. Les mises à jour sont signées par une clé à nous.** Indépendante de
la signature Apple (toujours « ad hoc », D12). Clé publique dans
`tauri.conf.json`, clé privée dans le secret GitHub
`TAURI_SIGNING_PRIVATE_KEY` (sans mot de passe), nulle part ailleurs. L'app
refuse une archive qui n'est pas signée par cette clé. La fabrication
contrôle, avant de publier, que la signature vient bien de la clé attendue
(`.github/scripts/mise_a_jour.py`). Si la clé privée était perdue : en créer
une autre, et réinstaller une fois l'app à la main.

**D16-c. Le dépôt devient public.** Choix de Benjamin (12 novembre 2026),
plutôt qu'une clé GitHub en lecture seule gravée dans l'app : les Releases
d'un dépôt privé ne se téléchargent pas sans identification. Le code et le
jeu d'essai (images d'Épinal comprises) deviennent visibles de tous ; en
échange, plus de jeton à renouveler, et les minutes GitHub Actions d'un dépôt
public sont gratuites. Tant que le dépôt reste privé, la recherche de mise à
jour échoue sans bruit.

**D16-d. Le manifeste est joint à chaque version.** La fabrication écrit
`latest.json` (version, signature, adresse de l'archive) et le joint à la
version avec l'archive `InSitu-X.Y.Z-mac.app.tar.gz`. L'app le lit à
l'adresse fixe `…/releases/latest/download/latest.json`, qui suit toujours la
dernière version publiée. Sans le secret de signature, la fabrication publie
comme avant, sans archive ni manifeste.

**D16-e. La première version qui sait se mettre à jour s'installe à la
main.** Les versions déjà installées n'ont pas ce module.

**D16-f. « Rechercher les mises à jour… » dans le menu InSitu** (demande de
Benjamin, 13 novembre). Même recherche qu'au lancement, mais dite : à jour
(avec le numéro), nouvelle version (même dialogue qu'au lancement), ou
pourquoi on ne sait pas (pas de réseau…). L'entrée n'a pas de raccourci ;
elle passe par l'évènement `menu` avec l'identifiant `mises-a-jour`, que
l'interface reconnaît avant les raccourcis.

## 2026-11-12 — Jeu d'essai Trévarez (hors plan, demande de Benjamin)

Deuxième projet réel, `jeu-essai/trevarez`, tests dans
`crates/insitu-core/tests/trevarez.rs`.

**D-JE-a. Un vrai `projet.json` enregistré par l'app entre au jeu
d'essai.** Copie exacte du projet de Benjamin (0.1.28). Les tests vérifient
qu'il se relit tel quel et que la diapositive C, refaite depuis ce projet,
retombe sur celle qu'il a exportée (à 0,5 mm près, à 100 dpi). Un
changement du format qui casserait la relecture d'un projet réel se voit
donc tout de suite.

**D-JE-b. Seule la diapositive C sert de référence.** Le dessin posé sur B
a été déformé autrement après l'export : comparer B testerait l'historique
du projet, pas le moteur.

**D-JE-c. Rien n'est versé sans test.** Les photos HEIC C à H, les photos
« lum », les 93 autres dessins et la composition Illustrator restent dans
le Dropbox (environ 5,9 Mo versés au total).

## 2026-11-12 — Étude du pli (Trévarez, hors plan)

**D-PLI-a. Constat, pas encore de changement du moteur.** Un projecteur qui
éclaire deux surfaces à angle (mur et avant-toit à Trévarez) n'est pas bien
recalé par le modèle actuel (un trapèze et une correction lisse) : 40 cm
d'écart en simulation avec 20 points. Un recalage par surface, séparé par
une arête tracée à la main, ramène l'écart à 10 cm. Détail :
`docs/etudes/pli-trevarez.md`. Le coder demande l'accord de Benjamin.

## 2026-11-12 — Tâche 4.8 : gomme et crayon

Demande de Benjamin (12 novembre) : une gomme pour enlever des traits
inutiles venus du dessin d'origine, un crayon pour ajouter des zones. Module
`crates/insitu-core/src/library/retouch.rs` ; écran
`src/library/RetouchEditor.tsx`, ouvert depuis l'inspecteur de la
Bibliothèque ; `insitu library retouch`.

**D-4.8-a. Retouches écrites dans le projet, fichier importé intact.** Le
champ `library[].retouch` garde chaque retouche dans l'ordre : l'outil
(gomme = noir, crayon = blanc) et ses points en fractions du dessin, comme
la silhouette (D-4.7-b). Soit un trait rond d'une épaisseur en fraction du
plus grand côté, soit une zone fermée remplie (pair-impair). Bord adouci
sur un pixel. La règle de la 4.1 (« le fichier copié n'est jamais
modifié ») tient : on revient toujours au dessin d'origine.

**D-4.8-b. Un dessin retouché fabriqué une fois, lu partout.** Le moteur
peint les retouches à pleine résolution et écrit un PNG dans le dossier
`retouches/` du projet, sous un nom qui dépend des retouches. Vignettes,
aperçus, vecteurs, silhouette détectée, export (PNG et SVG), vue
Diapositive, simulation et fiche lisent ce fichier à la place de
l'original (`retouch::retouched_file`) : leurs caches, rangés par fichier,
se refont d'eux-mêmes. Il est dans le projet (et pas dans le cache de
l'app) pour que l'export, qui n'a pas de cache, le trouve, et pour que
l'autre Mac le reçoive par Dropbox ; s'il manque, il est refait à
l'identique. Les anciens fichiers d'un dessin sont effacés à chaque
enregistrement.

**D-4.8-c. Un SVG retouché devient une image de 4 096 px**, la taille où la
vectorisation le rend déjà (D-5.2-d). Un SVG non retouché garde son rendu à
la taille du placement à l'export.

**D-4.8-d. Enregistrement à chaque geste, annulation dans l'éditeur.** Pas
de bouton « Enregistrer » (docs/design.md, § 3) : chaque retouche part au
moteur au lâcher ; un seul enregistrement à la fois, le dernier état
gagne. Dans l'éditeur, ⌘Z retire la dernière retouche : les raccourcis du
menu Projet (dont l'annulation de l'historique du projet, 4.3) y sont
suspendus, comme sous les autres éditeurs plein écran de la Bibliothèque
(où ⌫ ouvrait sinon « Retirer le dessin »). Hors de l'éditeur, ⌘Z de
l'historique du projet annule aussi les retouches, enregistrement par
enregistrement. L'éditeur peint sur l'aperçu d'origine (1 024 px) avec le
canevas du navigateur ; le rendu final est celui du moteur, aux mêmes
fractions.

## 2026-11-12 — Tâche 6.2 : fiche de calepinage

Module `crates/insitu-core/src/sheet/` ; bouton « Fiche de calepinage… » sous
l'export des diapositives (étape Façade) ; `insitu sheet`.

**D-6.2-a. Le PDF est écrit avec `pdf-writer`.** Déjà utilisé par les tests
(plans d'essai), en Rust pur, il passe en dépendance du moteur pour écrire
des images (plan, vignettes, en JPEG) en plus des tracés. La mire (2.2)
reste écrite à la main.

**D-6.2-b. Mise en page fixe, A3 couché.** Page 1 : le plan éclairci (gris
à 45 %) avec emprises, recouvrements en tirets rouges, contours des dessins
posés dans la couleur de leur diapositive (rouge : coupé, gris en tirets :
hors emprise) et noms des projecteurs ; à droite la liste des diapositives
(16 au plus, les autres sur les pages suivantes), les réglages d'export
(ceux de l'écran au moment de la fiche, sinon ceux du projet), les alertes
de la simulation (6.1) et une légende. Pages suivantes : six diapositives
par page. Couleurs des projecteurs : les teintes de l'écran, plus foncées
pour le papier blanc.

**D-6.2-c. Vignettes en traits noirs sur blanc** (remplacé par D-6.2-e). La diapositive gravée est
blanche sur noir ; à l'impression, des carrés noirs useraient l'encre et se
liraient mal. La vignette est donc inversée.

**D-6.2-e. Vignettes sur fond noir (remplace D-6.2-c).** Demande de
Benjamin après l'essai de la 0.1.28 (12 novembre 2026) : les diapositives
de la fiche sont montrées comme la plaque gravée, traits blancs sur noir,
pour que les tracés se voient. L'encre noire des six carrés par page est
acceptée.

**D-6.2-d. « Lisible sur site » = corps de 3 mm au moins.** Aucun texte de
la fiche sous 3 mm (environ 8,5 points), vérifié par test ; les noms des
projecteurs font 7 mm. Les lignes trop longues sont coupées d'après une
largeur approchée de l'Helvetica, sans mesure exacte des lettres.

## 2026-11-12 — Tâche 6.1 : simulation et alertes

Module `crates/insitu-core/src/simulation/` ; écran : rubrique « Simulation »
du panneau de gauche de l'étape Façade.

**D-6.1-a. La simulation pose les diapositives calculées sur la façade.**
Chaque aperçu de diapositive (le même que la vue Diapositive, calculé comme
l'export, `export::preview`) est tendu sur le maillage du projecteur de la
vue façade, teint de la couleur du projecteur et éclaircissant (« screen ») :
deux diapositives qui allument le même point mêlent leurs couleurs. Pas de
nouvelle image de façade calculée par le moteur. La simulation se lance par
un bouton (« Simuler ») et ne se recalcule pas toute seule après un
changement : « Simuler à nouveau ».

**D-6.1-b. « Dessin coupé ».** Un dessin posé visible dont la diapositive
(proposée ou choisie à la main, D-4.5-b) ne le contient pas en entier : soit
aucune ne le contient (« à cheval », D-4.5-c), soit le choix à la main en
désigne une dont il dépasse alors qu'une autre le contient (le message la
nomme). **« Hors emprise »** : un dessin qui ne tombe dans aucune emprise.
Les dessins d'un calque masqué ne partent pas à l'export : pas d'alerte.

**D-6.1-c. « Doublon en recouvrement » = lumière projetée deux fois.** Depuis
D-4.5-a, un dessin posé n'est jamais envoyé dans deux diapositives ; ce qui
peut encore être projeté deux fois, c'est la fresque importée en une image
(découpée entre les diapositives) ou deux dessins voisins rangés dans deux
diapositives qui se touchent dans un recouvrement. Le constat se fait sur
les aperçus (2 048 px) : chaque pixel allumé (≥ 128) de la première
diapositive est porté sur la façade puis dans la seconde ; s'il y est allumé
aussi (à un pixel près), le point reçoit deux fois la lumière. Les points
sont groupés par blocs de 48 pixels d'aperçu : une alerte par groupe (moins
de 4 points : ignoré). Si un dessin posé de l'une des deux diapositives
touche la zone, l'alerte mène à lui ; sinon elle nomme la fresque.

**D-6.1-d. Une alerte mène au dessin.** Chaque alerte donne le placement
concerné (s'il y en a un) et son rectangle sur la façade. Un clic dans la
liste choisit le dessin et cadre la vue sur ce rectangle, élargi de moitié
et d'au moins 200 mm (un trait fin reste visible).

## 2026-11-12 — Tâche 5.3 : export SVG

Module `crates/insitu-core/src/export/svg.rs`.

**D-5.3-a. Le SVG est écrit à côté du PNG, sur option.** Réglage d'export
`svg` (case « SVG en plus », `insitu export --svg`), désactivé par défaut :
le PNG reste la voie sûre (risque « SVG trop lourd » du plan). Même nom que
le PNG, extension `.svg`.

**D-5.3-b. Transport des courbes par morceaux, à 0,002 mm près.** La chaîne
est celle du PNG prise à l'envers : position dans le dessin → façade
(placement et déformation) → diapositive (inverse du recalage) → retouche de
diapositive → miroir. Chaque droite est transportée par ses extrémités ;
chaque cubique aussi, ses points de contrôle suivant la dérivée de la chaîne
(différences finies). Le morceau est comparé à trois points du morceau
d'origine transportés un à un ; au-delà de 0,002 mm, il est coupé en deux.
Les courbes restent des courbes (moins de quatre morceaux par morceau
d'origine dans le test). Les dessins sont vectorisés au moment de l'export,
avec leurs réglages de la Bibliothèque, depuis le même fichier qu'en
bibliothèque (un SVG rendu à 4 096 px, D-5.2-d).

**D-5.3-c. Silhouettes : différence de polygones (`i_overlay`).** Un dessin
dont une silhouette de devant touche le rectangle est aplati (0,002 mm) et
découpé par la réunion de ces silhouettes ; ses contours deviennent des
segments, remplis « pair-impair ». Les autres gardent leurs courbes,
remplies « non nul ». `i_overlay` (licence MIT, Rust pur, quatre petites
dépendances) est le moteur des opérations booléennes de `geo`.

**D-5.3-d. Contenu : courbes remplies de noir, sans fond, en mm.**
`width`/`height` en mm et `viewBox` de 280 (ou 300 avec la plaque, image
décalée de 10 mm). Un chemin par dessin posé. Pas de fresque (elle n'est
qu'une image), pas de nom ni de repères (ils ne sont que dans le PNG). Le
négatif et le noir et blanc pur ne concernent que le PNG. Couleur et réglages
de gravure à confirmer par l'essai LightBurn (5.4).

**D-5.3-e. Mesure du critère.** Le test pose un dessin tourné et déformé à
1 400 dpi, calcule la partie utile du PNG, ramène les courbes du SVG en
pixels de diapositive et applique la mesure indépendante de la tâche 5.1
(écart entre courbes et bord du trait seuillé). L'écart doit rester sous
0,02 mm, avec et sans miroir.

**D-5.3-f. Cadre de la diapositive dans le SVG.** Demande de Benjamin
après son essai LightBurn (12 novembre 2026). Chaque SVG porte, en plus des
dessins, un carré de 280 mm (`id="cadre-diapositive"`), au bord de la
diapositive (décalé de 10 mm avec la plaque) : trait rouge de 0,1 mm, sans
remplissage. Il donne le contour de la diapositive, et sa couleur le met
sur un calque à part dans LightBurn, à graver ou non. Il complète D-5.3-d,
qui ne mettait que les dessins.

**D-5.3-g. 279,999 mm dans LightBurn : la limite du pixel, pas une erreur.**
À 1 400 dpi, 280 mm font 15 433,07 pixels ; l'image en a 15 433, soit
279,9987 mm, et le PNG note sa résolution en pixels par mètre entiers
(55 118), d'où 279,999 mm. L'écart (1 µm, moins d'un vingtième de pixel)
est sous la finesse du laser. Il n'existe pas de nombre entier de pixels
qui fasse exactement 280 mm à 1 400 dpi ; changer le chiffre de référence
(15 433 px) demanderait l'accord de Benjamin, et n'est pas proposé.

## 2026-11-12 — Tâche 4.7 : silhouettes et superposition

Module `crates/insitu-core/src/library/silhouette.rs` ; écran
`src/library/SilhouetteEditor.tsx`.

**D-4.7-a. Détection par fermeture sur une grille réduite.** Le dessin est
ramené à environ 1 024 cases (une case est encrée si un de ses pixels
dépasse 48 sur 255 : un trait fin n'est jamais perdu). L'encre est épaissie
de 12 cases (environ 1,2 % du dessin), le fond est rempli depuis les bords,
puis la silhouette est amincie de 10 cases : elle déborde de 2 cases autour
du trait extérieur. Ses contours sont suivis comme pour la vectorisation,
puis simplifiés (0,75 case). Raison : les traits d'un dessin à la main ne
sont pas toujours fermés ; une ouverture de moins de 24 cases est bouchée.
Contrepartie : deux parties à moins de 24 cases l'une de l'autre sont
réunies, et une ouverture plus large laisse le fond entrer. La retouche à la
main règle ces cas.

**D-4.7-b. Polygones en fractions du dessin.** La silhouette suit le dessin
dans toutes ses transformations, déformations et retouches, comme ses
pixels. Retouchée, elle est écrite dans `library[].silhouette` ; détectée,
elle est rangée dans le cache de l'app (`silhouettes/`) et refaite si le
fichier change. Une liste vide veut dire « pas de silhouette ».

**D-4.7-c. Composition dans l'export.** Les dessins sont posés du fond vers
l'avant (ordre des calques, puis de la liste). Pour chacun : sous sa
silhouette, ce qui est déjà dans la diapositive est assombri en proportion
du masque (bord adouci par le rééchantillonnage), puis ses traits sont
ajoutés en gardant le plus clair. Le masque est la silhouette rendue à
2 048 px au plus et passe par la même chaîne que le dessin. Il n'est
calculé que s'il y a quelque chose dessous. Dans l'export, une silhouette
non retouchée est détectée sur l'image lue en pleine résolution (même
calcul que dans l'app).

**D-4.7-d. Sur la façade, un polygone noir opaque** est posé juste sous
chaque dessin. La vue Diapositive montre l'aperçu calculé par le moteur,
donc le même masquage que l'export.

**D-4.7-e. Le SVG de la diapositive (critère « … et dans le SVG ») viendra
avec l'export SVG (tâche 5.3)**, qui n'existe pas encore : la silhouette y
sera appliquée en découpant les courbes des dessins de derrière.

## 2026-11-12 — Tâche 4.6 : ajustement d'un dessin par diapositive

Module `crates/insitu-core/src/placement/adjust.rs` ; écran
`src/slide/useSlideAdjust.ts`.

**D-4.6-a. Une retouche est une grille de déformation en mm de
diapositive.** Même grille que la déformation libre (D-4.4-a, b), mais son
cadre est le rectangle droit qui contient le dessin tel que le projecteur
le montre. Elle est rangée dans le dessin posé (`placements[].adjustment`)
avec le projecteur concerné, et ne compte que pour lui. Raison : le plan
demande une retouche « sans toucher à la fresque » ; un dessin ne part que
dans une diapositive (D-4.5-a), une seule retouche par dessin suffit.

**D-4.6-b. Chaîne d'export.** Pixel de la diapositive → mm de diapositive →
retouche défaite (inverse de la grille, par Newton) → façade (recalage) →
dessin. L'aperçu de la vue Diapositive suit la même chaîne (sa clé de cache
comprend la retouche).

**D-4.6-c. Ce que fait l'écran.** Un clic choisit un dessin dans la vue
Diapositive. Le glisser le décale dans cette diapositive : tous les nœuds
d'une grille de quatre coins se déplacent ensemble, car un cadre déplacé
seul ne décale rien. « Quatre coins » ou « Grille » (3 × 3) donne des
points bleus à tirer, et « Réinitialiser » retire la retouche. Pendant un
geste, le dessin retouché s'affiche en bleu par-dessus l'aperçu, qui montre
encore l'état d'avant. À la fin du geste, la retouche est enregistrée et
l'aperçu est recalculé.

**D-4.6-d. La retouche reste quand on bouge le dessin sur la façade.** Elle
est liée à une place dans la diapositive : après un grand déplacement sur la
façade, mieux vaut la refaire (« Réinitialiser »). Une copie du dessin part
sans retouche. Si le dessin change de diapositive, la retouche de l'ancienne
est ignorée.

**D-4.6-e. Le bleu ne reste que le temps du recalcul.** Retour d'essai de
Benjamin (13 novembre 2026) : après une retouche, il devait désélectionner
le dessin pour voir le résultat. Le dessin retouché, montré en bleu par-dessus
l'aperçu tant qu'il était choisi, cachait l'aperçu recalculé. Désormais le
bleu s'affiche pendant le geste et jusqu'à l'arrivée de l'aperçu recalculé
(« Recalcul de la diapositive… » dans la barre d'état) ; ensuite seuls le
contour et les points restent. Sans moteur (navigateur seul), l'aperçu ne se
recalcule pas : le bleu reste.

**D-4.6-f. Les outils du dessin dans la barre, comme sur la façade.**
Demande de Benjamin (13 novembre 2026) : déplacer un dessin dans la
diapositive sans toucher à la façade lui convient ; il veut la même barre
d'outils que sur la façade et un retour au dessin calé sur la façade. La
barre montre, quand un dessin est choisi : tourner d'un quart de tour (à
gauche, à droite), quatre coins, grille (mêmes tailles que sur la façade),
et « Comme sur la façade », qui retire la retouche (ancien
« Réinitialiser »). Tourner fait tourner les **nœuds** de la retouche autour
du centre du dessin retouché (le cadre seul ne changerait rien) : c'est une
rotation exacte, en quatre coins comme en grille, prouvée par test ; sans
retouche, on part de quatre coins au repos. Pas de miroir ni d'alignement,
d'ordre ou de groupe : un miroir dans la retouche replierait la grille (le
moteur la refuse, D-4.4), et les autres changent la façade, donc toutes les
diapositives.

## 2026-11-12 — Tâche 4.5 : affectation des dessins aux diapositives

Module `crates/insitu-core/src/placement/assign.rs`.

**D-4.5-a. Un dessin posé ne part que dans une diapositive.** L'export et la
vue Diapositive ne prennent plus tous les dessins qui touchent la
diapositive, mais seulement ceux qui lui sont affectés. Raison : c'est le
calepinage (aucun dessin coupé) ; un dessin dans une zone de recouvrement
n'est plus projeté deux fois. La fresque importée en une image n'est pas
concernée.

**D-4.5-b. La proposition est recalculée, le choix à la main est
enregistré.** Le projet ne garde que le choix fait à la main
(`placements[].projector`) ; la proposition du moteur est recalculée à chaque
fois, d'après la position des dessins et le recalage. Déplacer un dessin
fait donc suivre sa proposition ; un choix à la main reste, même si le
dessin bouge, jusqu'à « Automatique ». Un choix qui désigne un projecteur
supprimé ou pas encore recalé est ignoré (proposition à la place). Une copie
d'un dessin repart en proposition.

**D-4.5-c. Règle de proposition.** Le contour du dessin (96 points,
déformation comprise) est ramené dans chaque diapositive par l'inverse du
recalage. Parmi les diapositives qui le contiennent en entier : celle où il
est le plus loin du bord (en mm de diapositive). Si aucune : celle qui en
contient la plus grande part, et le dessin est signalé « à cheval » (contour
rouge). À égalité, le premier projecteur du projet. Pas d'équilibrage du
nombre de dessins entre diapositives.

**D-4.5-d. La composition d'essai du critère est construite par le test.**
Le plan ne la définit pas : dix projecteurs en trapèze, deux rangées de
cinq, recouvrements de 0,8 m, 80 dessins tirés au hasard (tirage fixe ;
tournés, retournés, déformés) et un dessin au milieu de chaque zone de
recouvrement. Les dessins plus grands que toute diapositive en sont retirés
(ils ne peuvent pas être posés sans coupe) ; un autre test vérifie qu'ils
sont signalés.

**D-4.5-e. Couleur par diapositive : le contour du dessin.** Sur la façade,
chaque dessin visible est entouré de la couleur de son projecteur (mêmes
couleurs que les emprises) ; en rouge s'il est à cheval, en gris s'il ne
tombe dans aucune emprise. Le choix à la main se fait dans l'inspecteur
(« Diapositive »). Sans moteur (navigateur seul), rien de tout cela ne
s'affiche.

**D-4.5-f. Arborescence des dessins par diapositive.** Demande de Benjamin
(12 novembre 2026), « comme les calques d'Illustrator ». Rubrique
« Diapositives » (`src/compose/SlidesSection.tsx`, logique dans
`slideTree.ts`) : un groupe par projecteur recalé, dans l'ordre du projet,
plus « Hors de toute diapositive » ; dans un groupe, le dessin de devant en
haut (ordre des calques puis de la liste, D-4.3-e). Glisser une ligne sur
un titre de groupe fait un choix à la main (`placements[].projector`,
D-4.5-b) sans changer l'ordre ; la glisser sur un dessin la place juste
devant lui dans `placements[]` et sur son calque, pour que l'ordre affiché
soit celui de la liste. On ne lâche rien sur « Hors de toute diapositive »
(ce n'est pas un choix possible). La rubrique est en tête de l'onglet
« Calques » du panneau de droite (D15-i).

**D-4.5-g. L'arborescence suit le panneau Calques d'Illustrator.** Demande
de Benjamin (12 novembre 2026) : « afficher masqué un dessin, clic sur
plusieurs dessins, clic droit grouper etc. ». Chaque dessin posé peut être
**masqué** (`placements[].hidden`) ou **verrouillé** (`placements[].locked`),
un à un, en plus de son calque ; ces champs ne s'écrivent que vrais, un
projet ancien se relit tel quel. Masqué, un dessin ne s'affiche pas et ne
part ni dans l'export ni dans la fiche ni dans la simulation (même règle
que les calques masqués, `Project::visible_placements`). Verrouillé, il
s'affiche et s'exporte mais ne se choisit plus sur la façade. Dans
l'arborescence : clic = ce dessin seul, ⌘-clic = l'ajouter ou le retirer,
⇧-clic = toutes les lignes jusqu'à la dernière cliquée, dans l'ordre affiché
(`treeClick`) ; un groupe se choisit en entier, comme sur la façade ; un
dessin masqué ou verrouillé n'entre pas dans la sélection. Le clic droit
ouvre un menu (`ContextMenu`, ajouté au système de design) : Grouper,
Dégrouper, Masquer/Afficher, Verrouiller, ordre (premier plan… arrière-plan),
envoi vers une diapositive ou « Automatique », Supprimer. Il porte sur la
sélection si la ligne en fait partie, sinon sur la ligne seule. Écart
assumé avec Illustrator : on ne peut pas choisir un dessin masqué depuis
l'arborescence (seuls l'œil et le cadenas s'y appliquent).

**D-4.5-h. Ce qui ne sera pas projeté est montré en rouge.** Essai de
Benjamin sur la 0.1.39 (13 novembre 2026) : « dans diapositive, je ne
retrouve pas le travail fait dans l'onglet façade ». Sur son projet
Trévarez (relu dans le Dropbox), le calage était juste : chaque trait
projeté tombait à sa place. Ce qui manquait venait de D-4.5-a : un dessin
envoyé à la main dans B mais posé dans A n'était projeté par personne, et
le bout d'un dessin qui sortait de A n'était pas repris par B, qui couvre
pourtant cet endroit. Deux voies lui ont été proposées : faire compléter
par la diapositive voisine, ou garder la règle et montrer ce qui se perd.
**Benjamin a choisi de garder la règle et de montrer.** Le moteur calcule,
pour chaque dessin posé visible qui n'est pas projeté en entier, la part
perdue (`placement::lost`, `insitu place lost`, commande
`placements_lost`) : sa silhouette (tâche 4.7 ; sans silhouette, son
rectangle) posée sur la façade, moins l'emprise de sa diapositive
(différence de polygones, `i_overlay`, remplissage pair-impair) ; hors de
toute emprise, tout est perdu. Les morceaux de moins de 25 mm² (bruit le
long du bord) sont oubliés. La façade les couvre d'un aplat rouge (45 %)
cerné de rouge, par-dessus les dessins ; la vue Diapositive affiche un
bandeau qui nomme les dessins de cette diapositive qui en dépassent et la
surface perdue (`SlideView.cut`). La silhouette sert parce que le rectangle
d'un fichier a souvent de grandes marges vides : sur Trévarez, deux dessins
« à cheval » au sens de D-4.5-c ne perdent en fait aucun trait. Si une
silhouette ne peut être lue (fichier absent), le dessin compte pour son
rectangle, sans bloquer l'écran.

**D-4.5-i. Le chevauchement se vérifie à la demande.** Retour de Benjamin
(13 novembre 2026, sur la 0.1.45) : à l'import, tout était rouge, les
diapositives n'existant pas encore ; il veut une vue épurée pour composer
la fresque, puis une vérification avant l'export. Le menu Affichage de la
façade gagne une rubrique « Vérifier » avec **Chevauchement**, décoché par
défaut et retenu d'un lancement à l'autre comme les autres options
(`ViewOptions.overlap`). Décoché, la façade ne montre ni le contour de
chaque dessin dans la couleur de sa diapositive (D-4.5-e), ni le rouge de
ce qui ne sera pas projeté (D-4.5-h), et le moteur ne calcule pas les parts
perdues. Coché, les deux reviennent. Ne changent pas : la simulation (6.1),
l'arborescence par diapositive, et le bandeau de la vue Diapositive, qui
sont déjà des écrans de vérification.

## 2026-11-12 — Tâche 4.4 : déformation libre

Modules `crates/insitu-core/src/placement/mesh.rs` (moteur) et
`src/compose/deformation.ts` (aperçu).

**D-4.4-a. Une déformation est une grille tendue sur un cadre.** Le cadre
est un rectangle tourné de la façade ; la grille a de 1 à 16 cases par côté,
et chaque nœud est noté en fractions du cadre (`placements[].deformation`).
Le dessin est d'abord posé droit (centre, taille, rotation, miroir), puis
chaque point de la façade passe par la grille. Raison : déplacer, tourner,
agrandir ou retourner un dessin déformé ne touche que le cadre, la forme
reste la même.

**D-4.4-b. Une seule case : quatre coins en perspective ; plus : surface
lisse.** Quatre coins donnent une homographie (les droites restent droites,
comme un trapèze de projecteur). Une grille donne une surface de
Catmull-Rom qui passe par chaque nœud ; au-delà du bord, les nœuds
manquants sont prolongés en parabole, pour qu'une rangée de six cases
épouse un arc de cercle à moins de 1 % près (test « voussure »). Le chemin
inverse (pour l'export) se fait par la méthode de Newton.

**D-4.4-c. Un groupe se déforme d'un bloc.** « Quatre coins » ou « Grille »
sur plusieurs dessins tend une seule grille sur le rectangle droit qui les
contient, copiée à l'identique dans chacun ; tirer un nœud le déplace pour
tous les dessins qui partagent cette grille. Une nouvelle grille remplace
la précédente.

**D-4.4-d. Le moteur refuse une grille qui replie le dessin.** Même
contrôle que la fresque (tâche 3.4) : grille retournée ou nœud tiré au-delà
de son voisin. L'enregistrement est refusé, le message s'affiche, l'état
d'avant revient.

**D-4.4-e. « Export identique à l'aperçu » est prouvé par des cas calculés
par le moteur.** `src/compose/deformation-cases.json` contient des dessins
déformés (quatre coins, grilles, tournés, retournés) et, pour chacun, la
position sur la façade de points du dessin calculée par le moteur ; le test
de l'interface vérifie que l'aperçu tombe au même endroit (écart inférieur
à un milliardième de mm). Le test du moteur refait ces cas et échoue s'ils
ont changé (`INSITU_REFAIRE_CAS=1` pour les réécrire). L'export lui-même
passe par la même fonction que ces cas (test d'export d'un dessin déformé).

**D-4.4-f. À l'écran, un dessin déformé est affiché par une maille de
16 × 16 cases**, chaque sommet calculé exactement ; entre deux sommets,
l'image est tirée en ligne droite (écart invisible à l'écran, aucun à
l'export). Les nœuds sont des points bleus ronds ; ils passent avant les
poignées carrées du cadre quand les deux se touchent.

## 2026-11-12 — Export des dessins posés (après la tâche 4.3)

Demande de Benjamin (réponse « oui » du 12 novembre) : lever la limite de la
tâche 4.2, les dessins posés dans l'app partent dans les diapositives.
Module `crates/insitu-core/src/export/drawings.rs`.

**D-4.2-g. Chaque dessin est calculé depuis son fichier d'origine, par la
chaîne de la fresque.** Pixel de la diapositive → mm de diapositive → mm de
façade (recalage) → position dans le dessin (inverse du placement) → pixel
du fichier. Mêmes filtres que la fresque (tâche 3.4) : un dessin posé sort
comme la même image importée en fresque sur le même rectangle (test). Seul
le rectangle de diapositive que le dessin peut toucher est calculé (son
contour ramené dans la diapositive par l'inverse du recalage, 3 px de
marge) ; un dessin qui ne tombe pas dans la diapositive n'est pas lu. Un
SVG est rendu à la taille de son plus grand placement à la résolution
d'export (entre 512 et 16 384 px).

**D-4.2-h. Fresque et dessins se superposent en gardant le plus clair.**
Traits blancs sur fond noir : un trait n'en efface jamais un autre. Le
masquage d'un dessin par celui de devant est la tâche 4.7 ; l'ordre des
calques est déjà suivi, pour elle.

**D-4.2-i. Les calques masqués ne sont pas exportés, les verrouillés oui.**
Masquer sert à ne pas graver ; verrouiller sert seulement à ne pas déplacer
par erreur. Un projet sans fresque mais avec des dessins posés s'exporte.

**D-4.2-j. La vue Diapositive suit la même règle.** Sa clé de cache tient
compte des placements visibles et de la taille et de la date des fichiers
des dessins : bouger un dessin refait l'aperçu.

**D-4.2-k. Glisser un dessin sur la façade : la fenêtre ne capte plus les
glisser.** Essai de la 0.1.28 par Benjamin : le dessin glissé ne se posait
pas. Tauri capte par défaut tout glisser au-dessus de la fenêtre (pour
recevoir des fichiers du Finder), ce qui empêche la page de recevoir le
lâcher ; l'app n'utilise pas ce glisser de fichiers, il est coupé
(`dragDropEnabled: false` dans `tauri.conf.json`). Par prudence, le dessin
glissé est aussi gardé en mémoire au départ du geste et porté en texte
simple, pour le cas où la vue web du Mac ne rendrait pas le type maison.
Le double-clic sur une vignette pose toujours le dessin au centre de la
vue. Non vérifiable en session cloud : seul l'essai sur Mac le prouve.

**D-4.2-l. Poignées du cadre comme Illustrator : étirer en x et en y.**
Demande de Benjamin (12 novembre 2026). Le cadre de sélection a huit
poignées : un coin étire en largeur et en hauteur à la fois (⇧ garde les
proportions, ce qui était le comportement par défaut depuis D-4.2-c), le
milieu d'un côté étire seulement en largeur ou en hauteur ; ⌥ part du
centre ; ⌘D répète l'étirement. Les axes sont ceux de la façade, comme le
cadre. Un dessin droit ou tourné d'un quart change de largeur et de
hauteur ; un dessin tourné d'un autre angle, ou déjà déformé, ne peut pas
rester un rectangle tourné (il serait cisaillé) : il reçoit une déformation
(D-4.4) qui porte l'étirement exactement, sans approximation, puisque
l'interpolation de la grille suit une transformation affine. Prouvé par
test sur un dessin tourné de 30°.

**D-4.2-m. Tourner à la souris par une poignée ronde, curseurs visibles,
miroir dans l'inspecteur.** Retours d'essai de Benjamin sur la 0.1.39
(13 novembre 2026) : « il faut un outil pour faire la rotation de manière
graphique », « la souris doit changer de curseur au niveau des poignées »,
« ajouter miroir sur un dessin ». La rotation juste au-delà d'un coin
(tâche 4.2, comme Illustrator) et les curseurs existaient, mais le curseur ne
changeait jamais : la zone de la visionneuse imposait la main (`cursor: grab`
dans `viewer.css`), qui l'emportait sur le curseur posé par l'écran. Sans
curseur, la zone de rotation était introuvable. Choix :
- le curseur passe par la variable CSS `--viewer-cursor` que la zone de la
  visionneuse lit (`setViewerCursor`, `src/viewer/cursor.ts`) ; la main
  reste le curseur par défaut ; l'écran Diapositive en profite aussi ;
- une poignée ronde, au-dessus du milieu du côté haut du cadre (28 px
  d'écran, reliée par un trait), tourne la sélection autour du centre du
  cadre, ⇧ par pas de 45° ; la zone au-delà des coins est gardée pour les
  habitués d'Illustrator ;
- curseur de rotation dessiné en SVG (flèche courbe à deux pointes), tourné
  vers la poignée saisie et qui suit l'angle pendant le geste ; le système
  n'en a pas. Pendant le geste, l'angle s'affiche près du pointeur
  (l'angle du dessin s'il est seul, l'angle tourné sinon) ;
- le miroir existait en icônes dans la barre du haut (« Symétrie ») ; il est
  ajouté en boutons avec texte dans l'onglet Dessins (« Gauche ↔ droite »,
  « Haut ↕ bas »), et les bulles d'aide disent « Miroir ». Même calcul
  qu'avant (`flip`), donc même export. Pas de raccourci clavier :
  Illustrator n'en a pas par défaut.

**D-4.2-n. Dessin déformé choisi : ses points seuls, sans le cadre.**
Demande de Benjamin (13 novembre 2026) : « quand la fonction 4 coins est
activée, ne pas afficher le rectangle de sélection, mais juste les 4
coins ». Quand la sélection partage une déformation (quatre coins ou
grille), le cadre, ses huit poignées et la poignée ronde ne s'affichent plus
et ne se saisissent plus : on ne voit que les points bleus et le contour
déformé. Le choix vaut aussi pour les grilles, par cohérence et parce que
les poignées d'angle du cadre tombaient sur les points des coins. Glisser le
dessin le déplace toujours ; pour tourner ou agrandir à la souris, il faut
d'abord « Redresser » (barre du haut), ou passer par les quarts de tour, la
case Rotation et la case Largeur.

**D-4.2-o. Onglet Dessins : la bibliothèque classée par dossier.** Demande
de Benjamin (13 novembre 2026) : « mets l'arborescence de la bibliothèque
pour voir tous les dessins classés ». Les vignettes à poser sont rangées
sous les mêmes dossiers que dans l'étape Bibliothèque (les sous-dossiers du
dossier importé), avec le nombre de dessins de chaque dossier, sous-dossiers
compris. Tout est déplié à l'ouverture ; un clic sur un dossier le replie.
Tri naturel (« Raie 2 » avant « Raie 10 »). La recherche porte sur le nom et
le dossier, déplie tout et ne garde que les dossiers qui ont encore un
dessin. Les dessins à la racine viennent en premier, sans titre. Les
dossiers repliés ne sont pas retenus d'une session à l'autre. Calcul pur
dans `src/compose/drawingTree.ts`, testé.

**D-4.2-p. Réglages du dessin posé dans une barre en haut ; l'onglet
Dessins ne garde que la bibliothèque.** Demande de Benjamin (13 novembre
2026) : « dessin posé : mets tout dans le menu en haut, et ne laisse que les
dessins dans le menu dessins ». Une seconde barre, sous la barre d'outils de
l'écran Façade, comme le panneau Contrôle d'Illustrator : nom (et
« retourné »), calque, diapositive (l'explication passe en bulle d'aide,
« à cheval » en orange), X, Y, L, H, Angle, état de la déformation,
Retirer. Elle est là dès que l'on peut composer (plan à l'échelle, dessins
montrés), et invite à cliquer un dessin quand rien n'est choisi : la zone
de travail ne saute pas à chaque clic. Le miroir reste en icônes dans la
barre d'outils : les boutons texte de D-4.2-m quittent l'onglet Dessins.
Choisir un dessin n'ouvre plus l'onglet Dessins (règle de D15-i retirée),
puisque ses réglages n'y sont plus. `PlacementInspector` devient
`PlacementBar`.

**D-4.2-q. Un dessin étiré garde son cadre ; ⇧ en tournant : angle rond à
5°.** Retours de Benjamin (13 novembre 2026). (1) « Quand j'utilise les
poignées du centre, cela passe en 4 coins et je perds le rectangle de
sélection » : un dessin tourné, étiré par une poignée du milieu, reçoit une
déformation (D-4.2-l), que D-4.2-n traitait comme « quatre coins ».
Désormais, les points d'une déformation ne se montrent (et le cadre ne se
cache) que si l'on vient de cliquer « Quatre coins » ou une grille sur cette
sélection, ou si la déformation a déjà été tirée (elle n'est plus affine,
`isAffine`). Une déformation née d'un étirement reste affine : le dessin
garde son cadre, ses poignées et sa poignée ronde. (2) « Shift + rotation :
angle remarquable à 5° » : ⇧ pendant la rotation à la souris arrête un
dessin seul sur un multiple de 5° (son angle final, pas l'écart), et fait
tourner plusieurs dessins par pas de 5°. ⇧ en déplaçant garde les
directions à 45°.

**D-4.2-r. L'onglet Dessins descend jusqu'en bas de la fenêtre.** Retour de
Benjamin (13 novembre 2026) : « menu Dessins : il ne va pas jusqu'en bas de
la fenêtre ». La bibliothèque était limitée à 60 % de la hauteur de
l'écran, avec sa propre barre de défilement. Elle occupe désormais tout le
panneau, qui défile d'un bloc ; le champ de recherche reste collé en haut.

**D-4.2-s. Deux modes pour un dessin choisi : « Normal » et « 4 coins ».**
Demande de Benjamin (13 novembre 2026) : « sur un dessin, je dois avoir un
mode normal et un mode 4 coins (qui active les 4 coins), mais quand je
reviens en mode normal, je retrouve le rectangle de sélection ». Remplace la
règle automatique de D-4.2-n et D-4.2-q (points montrés si la déformation
n'était plus affine), qui ne laissait retrouver le cadre qu'en
« Redressant ». Le mode se choisit dans la barre des réglages (« Normal » |
« 4 coins »), ou par l'icône « 4 coins » de la barre d'outils, qui
s'allume et rebascule. En mode Normal : cadre, poignées, poignée ronde ;
la déformation est gardée et suit d'un bloc les déplacements, rotations
et étirements. En mode 4 coins : seuls les points de la déformation, sans
cadre ; un dessin sans déformation reçoit ses quatre coins au repos, un
dessin déjà déformé garde la sienne. Choisir une grille passe en mode
4 coins ; « Redresser » et Échap reviennent en mode Normal (Échap vide la
sélection ensuite). Le mode vaut pour la sélection où on l'a choisi : une
autre sélection commence en mode Normal.

**D15-l. ⌘Z garde le zoom et l'endroit vus sur la façade.** Retour de
Benjamin (13 novembre 2026) : « quand je fais pomme Z, le zoom change ».
L'annulation relit le projet, et la relecture recadrait tout le plan. La
vue (centre et zoom) est gardée avant la relecture (⌘Z, ⇧⌘Z, ⌘R) et remise
ensuite, sauf si l'échelle du plan a changé (une mise à l'échelle annulée
change la taille du plan dans la visionneuse) : on recadre alors.

**Limite.** Les fichiers des dessins utilisés par une diapositive restent
en mémoire pendant l'export (en niveaux de gris, un dessin de 4 096 px pèse
16 Mo) ; à surveiller avec une très grande bibliothèque.

## 2026-11-12 — Tâche 4.3

Calques de la composition (champ `layers`, `placement.layer`) et annulation
illimitée (module `crates/insitu-core/src/project/history/`, commandes
`insitu project undo|redo|history`, ⌘Z et ⇧⌘Z dans le menu Projet).

**D-4.3-a. L'annulation garde le `projet.json` d'avant chaque enregistrement.**
Chaque fois que l'app enregistre le projet, quel que soit l'écran, le
`projet.json` d'avant est copié dans un dossier de l'app ; ⌘Z le remet, ⇧⌘Z
remet celui d'après. Aucune limite de nombre. Raison : toute action passe
déjà par un enregistrement complet du projet ; garder des copies entières est
simple, sûr, et couvre d'un coup tous les écrans présents et à venir (un
`projet.json` pèse quelques dizaines de Ko). Ce que cela interdit : écrire
`projet.json` sans passer par `ProjectDir::save`.

**D-4.3-b. L'historique est rangé hors du projet, dans le cache de l'app.**
`~/Library/Caches/com.luminariste.insitu/historique/<identifiant du projet>/`.
Raison : il ne doit pas passer par Dropbox (des centaines de copies, et
l'autre Mac n'a pas à annuler mes gestes). Il survit à la fermeture de l'app ;
macOS peut toutefois vider ce dossier s'il manque de place, l'historique est
alors simplement perdu, jamais le projet.

**D-4.3-c. Garde-fou : on n'annule jamais par-dessus un changement fait ailleurs.**
Chaque entrée retient l'empreinte du `projet.json` qu'elle s'attend à
trouver. Si le fichier a changé autrement (l'autre Mac, par Dropbox, ou une
modification à la main), ⌘Z refuse, le dit, et vide l'historique.

**D-4.3-d. L'annulation ne touche qu'au `projet.json`.**
Les fichiers importés (plan, photos, dessins) restent dans le dossier du
projet : annuler un import retire la référence, pas le fichier. Les
confirmations avant suppression déjà en place (projecteur, plan, dessin de
la bibliothèque) sont gardées pour l'instant : les retirer, comme le prévoit
`docs/design.md`, est un changement à part.

**D-4.3-f. Annuler et Rétablir du menu Édition passent par l'interface.**
Les entrées toutes faites du Mac (`undo`, `redo`) ne savent annuler que la
frappe. Elles sont remplacées par des entrées InSitu (`raccourci:mod+z`,
`raccourci:mod+shift+z`) : hors d'un champ de texte, elles annulent la
dernière modification du projet ; dans un champ, elles annulent la frappe
(`document.execCommand`).

**D-4.3-e. Calques : une liste ordonnée, le premier au fond.**
`layers[]` porte `id`, `name`, `visible` (absent si vrai) et `locked`
(absent si faux) ; chaque dessin posé cite son calque par `layer`. Un dessin
sans calque, ou sur un calque inconnu, appartient au premier. Un projet sans
calque en montre un seul, « Calque 1 », écrit dans le projet à la première
action sur les calques. L'ordre d'affichage est celui des calques, puis
celui des dessins dans chaque calque (`Project::stacked_placements` dans le
moteur, `stack` dans l'interface). Un calque masqué ne s'affiche pas ; masqué
ou verrouillé, ses dessins ne se choisissent plus. Supprimer un calque
supprime ses dessins (⌘Z les rend). Les nouveaux dessins se posent sur le
calque choisi dans la liste ; s'il est masqué ou verrouillé, l'app refuse et
le dit.

**Limite.** « Avancer » et « reculer » (⌘] ⌘[) ne font pas passer un dessin
d'un calque à l'autre : on change son calque dans l'inspecteur.

## 2026-11-12 — Tâche 4.2

Placement des dessins sur la façade : champ `placements` du projet, module
`crates/insitu-core/src/placement/`, commande `insitu place`, calculs des
gestes dans `src/compose/` (testés sans écran), branchés sur l'écran Façade.
Mode d'emploi : `docs/composition.md`.

**D-4.2-a. Un placement = centre, largeur, hauteur, rotation, miroir.**
Le dessin est mis à la taille voulue (mm réels), retourné de gauche à droite
si `mirrored`, tourné (degrés, sens des aiguilles d'une montre puisque y
descend), puis posé par son centre. La symétrie de haut en bas s'écrit miroir
plus demi-tour : un seul booléen suffit. Pas de cisaillement ni de
déformation : ce sera la tâche 4.4 (quatre coins, grille). L'ordre de la
liste est l'ordre d'empilement (le dernier est devant).

**D-4.2-b. Groupes à un seul niveau.**
Un champ `group` facultatif : les dessins qui portent le même se
sélectionnent, s'alignent et se transforment ensemble. Grouper des groupes
les fond en un seul ; Illustrator permet des groupes de groupes, rarement
utiles ici. Ce que cela interdit : rien, un niveau de plus pourra s'ajouter
sans changer le format (champ facultatif).

**D-4.2-c. Agrandir garde toujours les proportions.**
Les poignées de coin agrandissent depuis le coin opposé (⌥ : depuis le
centre), sans déformer : un dessin de Marc ne s'étire pas. Dans Illustrator il
faut tenir ⇧ pour cela ; ici c'est le seul comportement, la déformation
viendra avec 4.4. Pas de poignées au milieu des côtés pour la même raison.

**D-4.2-d. Un dessin lâché sur la façade fait le quart de la largeur visible.**
Le fichier n'a pas de taille réelle (des pixels) ; on le pose donc à une
taille lisible à l'écran, aux proportions du fichier, puis on l'ajuste.
Flèches : 10 mm par appui, 100 mm avec ⇧ (un point d'Illustrator n'a pas de
sens sur une façade de 20 m). Copier-coller : ⌘V colle au centre de la vue,
⌘F devant à la même place, ⌘B derrière à la même place, comme Illustrator.

**D-4.2-e. Gestes à la souris comme Illustrator.**
Glisser dans le vide trace un cadre de sélection (et ne déplace plus la vue
dès qu'au moins un dessin est posé) ; pour déplacer la vue : défiler à deux
doigts, ou glisser avec la barre d'espace enfoncée (la main d'Illustrator).
Tourner : juste au-delà d'un coin du cadre ; ⇧ par pas de 45°. ⌥-glisser
copie ; ⌘D répète le dernier geste (avec sa copie s'il en faisait une), ce
qui donne les séries régulières d'Illustrator ; sans geste précédent, ⌘D
duplique avec un décalage de 10 cm. Crochets (⌘[ ⌘] et avec ⇧ : ordre
d'empilement) lus par la position de la touche, pas par le caractère : sur
un clavier français ce sont les deux touches à droite du P. À confirmer sur
le Mac de Benjamin.

**D-4.2-f. Affichage par l'aperçu de 1 024 px, enregistrement à la fin du geste.**
Chaque dessin posé est l'aperçu de la bibliothèque (fond transparent, D-4.1-d)
tendu sur ses quatre coins. De près, un dessin de 4 096 px est donc flou à
l'écran ; la pleine résolution reste pour l'export. Pendant un geste, rien ne
part au moteur ; à la fin, la liste entière lui est envoyée, il la contrôle
(dessin connu, taille positive, identifiants uniques) et l'enregistre, ou la
refuse sans rien écrire.

**Limite.** L'export des diapositives part toujours de la fresque en une image
(tâche 3.5) : les dessins posés dans l'app ne sont pas encore exportés. Le
plan ne le range dans aucune tâche du jalon 4 explicitement (4.4 parle
d'« export identique à l'aperçu ») : à décider avec Benjamin.
## 2026-11-12 — Tâche 6.6

Assemblage automatique, demandé par Benjamin après son essai de la 6.4
(« laborieux, je préférerais une analyse automatique ») :
`crates/insitu-core/src/assembly/features.rs` (détails et appariement),
`crates/insitu-core/src/assembly/auto.rs` (choix des appuis), commande
`insitu assembly auto`, bouton « Raccorder automatiquement » de l'écran
Assemblage, mode d'emploi `docs/assemblage.md`.

**D-6.6-a. Détection écrite dans le moteur, en Rust pur, sans OpenCV.**
Coins FAST classés par Harris sur une pyramide de 5 tailles, signatures de
type SIFT (histogrammes de directions, 128 octets), appariement réciproque
avec test du rapport (0,85), tri par RANSAC sur un trapèze. OpenCV aurait
demandé une bibliothèque système à embarquer dans l'app Mac et compliqué la
fabrication ; les algorithmes sont publics et courts. Le calcul est
déterministe (tirages pseudo-aléatoires à graine fixe) : mêmes photos, mêmes
points. Une première version avec des signatures binaires (ORB) trouvait
trop peu d'appariements entre deux vraies photos d'Épinal prises de deux
endroits (15 justes) ; les signatures SIFT en trouvent 51.

**D-6.6-b. Positions affinées au dixième de pixel.** Les coins sont trouvés
sur une image réduite à environ 2 000 px ; chaque appariement retenu est
ensuite recalé par corrélation locale (Lucas-Kanade, luminosité et
contraste compensés, fenêtre de 17 px transportée par le trapèze), puis le
trapèze est recalculé et les appariements à plus de 1 px écartés. Sans cet
affinage, l'erreur aux coins des photos atteignait 20 px sur l'essai à
trois photos ; avec, 0,12 px.

**D-6.6-c. Le résultat, ce sont des points communs ordinaires.** Pour
chaque photo, au plus 24 points bien répartis (le meilleur de chaque case
d'une grille 6 × 4 sur la zone commune, puis les meilleurs restants
jusqu'à 12) sont enregistrés dans `pairs`, comme s'ils avaient été
cliqués : même diagnostic, même correction à la main, même calcul de
l'image. Rien de nouveau dans le fichier projet. La première photo reste la
référence ; chaque autre s'appuie sur la photo déjà raccordée avec laquelle
elle partage le plus de détails (arbre de poids maximal, en partant de la
référence) : l'ordre d'ajout des photos n'a plus d'importance.

**D-6.6-d. Quand l'analyse se lance, et ce qu'elle remplace.** Elle se lance
seule après « Ajouter des photos… » si aucun point n'a encore été posé à la
main, et sur le bouton « Raccorder automatiquement » ; si des points
existent, le bouton demande confirmation, car il les remplace tous. Il faut
20 appariements justes au moins pour raccorder deux photos ; une photo sans
détail commun avec les autres garde ses points et son appui, et est nommée
dans le message. Quatre photos analysées à la fois au plus, pour borner la
mémoire (une photo de 48 Mpx occupe environ 150 Mo le temps d'être lue).

**D-6.6-e. Toujours un mur plan (D-6.4-a).** Sur un portail en relief, le
trapèze suit le plan qui porte le plus de détails communs : sur les photos
A et B d'Épinal, le mur de pierre au-dessus du porche, bien raccordé, alors
que le fond du porche apparaît dédoublé dans l'image assemblée. Pour un
assemblage net, les photos doivent être prises de face, du même recul, en
se décalant le long du mur.

## 2026-11-12 — Tâche 6.4

Assemblage de photos de façade : `crates/insitu-core/src/assembly/`,
commande `insitu assembly`, fenêtre « Assembler des photos… » de l'écran
Façade (`src/assembly/`), mode d'emploi `docs/assemblage.md`.

**D-6.4-a. Une homographie par photo : façade supposée plane.** Chaque
photo est raccordée à sa photo d'appui par une homographie (trapèze), et de
proche en proche à la référence. C'est exact pour des photos d'un même mur
plat ; un relief marqué (avancée, porche) laissera des décalages dans les
recouvrements. Pas de déformation par grille ici : un plan exact reste
préférable quand il existe. Interdit : présenter l'assemblage comme un relevé.

**D-6.4-b. Points douteux : celui dont le retrait améliore le plus les
autres.** Avec 6 points ou plus, le moteur cherche le point dont l'absence
réduit le plus l'écart des autres ; il est signalé si son écart dépasse
4 fois l'écart médian des autres (au moins 4 px). On recommence tant qu'il
reste plus de 5 points. Un simple « écart en laissant ce point de côté »
désignait parfois le mauvais point. Rien n'est retiré d'office : le point
douteux est montré en rouge et compté hors de l'écart moyen.

**D-6.4-c. Image assemblée : JPEG de 12 000 px au plus, fondu sur les
bords.** Au-delà, l'image est réduite (l'échelle est notée dans `output`).
Dans les recouvrements, chaque photo pèse moins près de ses bords (fondu sur
10 % de sa taille) : pas de couture nette. JPEG qualité 92 : une photo
d'ensemble n'a pas besoin du sans-perte, et un PNG de cette taille pèserait
des centaines de Mo dans le Dropbox.

**D-6.4-d. Recalculer garde l'échelle et ce qui est posé.** Si la façade
est déjà l'assemblage précédent et qu'elle est à l'échelle, l'échelle est
reprise et les points de façade des recalages et le cadre de la fresque
sont décalés de ce dont l'origine de l'image a bougé (ajout d'une photo à
gauche, par exemple), en tenant compte d'une réduction différente. Retirer
la photo de référence change le repère : l'échelle est alors à refaire.

**D-6.4-e. Changer la photo d'appui efface ses points.** Ils sont en
pixels de l'ancienne photo d'appui et n'ont pas de sens sur la nouvelle ;
l'écran demande confirmation.

**D-6.4-f. Rotation EXIF non appliquée.** Comme pour la façade (3.1) et
la photo brute, les pixels sont ceux du fichier. Une photo prise iPhone
tourné s'affiche couchée ; il faut l'enregistrer droite avant de l'ajouter.
À reprendre en 6.5 (finitions) si c'est gênant.

**D-6.4-g. Écran plein par-dessus l'écran Façade, pas une sixième étape.**
L'assemblage remplace l'import d'un plan : il s'ouvre depuis la barre de
l'écran Façade et y revient avec la nouvelle façade. Il reprend les deux
vues côte à côte et les couleurs des points de l'écran Recalage.

## 2026-11-12 — Tâche 5.2

Vecteurs en bibliothèque : `crates/insitu-core/src/library/vectors.rs`,
commande `insitu library vectors`, fenêtre « Comparer image et vecteurs »
de l'écran Bibliothèque (`src/library/VectorCompare.tsx`,
`src/library/vectorLogic.ts`), mode d'emploi `docs/bibliotheque.md`.

**D-5.2-a. Les vecteurs vont dans le cache de l'app, seuls les réglages
vont dans le projet.** Comme les vignettes (D-4.1-d) : la vectorisation se
refait à l'identique depuis le fichier et ses réglages, elle n'a pas à
voyager par Dropbox. Clé du cache : chemin, taille et date du fichier, et
les quatre réglages. Un dessin aux réglages par défaut n'a rien d'écrit
dans `projet.json` (champ `vector` absent) ; il suivra le défaut s'il change
un jour. Champ facultatif : `formatVersion` ne change pas.

**D-5.2-b. Vectorisé juste après l'import, pas pendant.** Mesuré en
session cloud (4 cœurs) sur les 200 dessins de 4 096 px du test de 4.1 :
vectoriser pendant l'import le faisait passer de 4 s à 26 s, ce qui ruinait
le critère « importés sans attente ». L'import reste donc tel quel ; dès
qu'il est fini, l'app vectorise les nouveaux dessins (8 à la fois au plus,
un fil chacun) pendant qu'on parcourt déjà la bibliothèque, avec une ligne
d'avancement (26 s pour les 200 dessins de 4 096 px sur 4 cœurs, mesure
comprise ; sans doute moins de la moitié sur le M1 Max, non mesuré). Sur
l'autre Mac, ou après un changement de réglage, les
vecteurs d'un dessin sont faits quand on le choisit. La liste de la
bibliothèque ne vectorise jamais : elle doit rester instantanée.

**D-5.2-c. L'écart est mesuré à chaque vectorisation.** Le compte rendu
garde la mesure indépendante de 5.1 (`compare`) : écart maximal en pixels
et pixels classés autrement. Il est affiché en vert sous un demi-pixel, en
rouge au-delà. Cela double à peu près le temps de calcul ; c'est le chiffre
qui permet de valider un dessin sans tout regarder à la loupe.

**D-5.2-d. Un SVG de la bibliothèque est vectorisé depuis son rendu à
4 096 px**, comme un dessin dessiné. Raison : un seul chemin pour le
transport des courbes (5.3), quelle que soit l'origine du dessin. Reprendre
directement les courbes du SVG serait plus fidèle mais demande de convertir
les traits (épaisseur, extrémités) en contours : à voir si un SVG de Marc le
justifie.

**D-5.2-e. Comparaison dans une fenêtre à part, en HTML, pas dans la
visionneuse.** L'image (4 096 px au plus, le noir rendu transparent) et deux
SVG (courbes remplies de blanc ; courbes au trait rouge d'un pixel d'écran)
sont superposés dans le même cadre. À partir de 200 %, l'image est agrandie
sans lissage : on voit les pixels du fichier et le contour qui les longe.
Quatre affichages : image, vecteurs, contour sur l'image, rideau. La
visionneuse PixiJS n'a pas de calque vectoriel ; un SVG de quelques
mégaoctets se dessine bien dans la page. Non vérifié : la fluidité du zoom
sur un dessin très chargé (des dizaines de milliers de contours) sur le Mac.

**D-5.2-f. Réglages par dessin : les quatre de 5.1, bornes du moteur.**
Seuil, tolérance, angle des coins, aire minimale. « Appliquer » enregistre
le projet et refait les vecteurs ; des réglages refusés ne touchent pas au
projet. Pas d'aperçu en direct pendant la frappe : un calcul coûte de
quelques dixièmes de seconde à quelques secondes.

## 2026-11-12 — Système de design

Demande de Benjamin : une interface bien lisible, où l'on peut ajouter,
modifier et supprimer, aux parties bien séparées et aux menus clairs, « digne
des meilleurs logiciels », fixée dès la V1. Hors plan (pas de numéro de
tâche) ; règles dans `docs/design.md`, code dans `src/ui/`.

**D15. Un seul système de design, sur le modèle des logiciels pro du Mac.**
Même gabarit pour chaque écran : barre des étapes (⌘1 à ⌘5), barre d'outils
qui commence par le menu Projet, objets à gauche, zone de travail au centre,
inspecteur à droite, barre d'état en bas. Composants communs (`src/ui/`) et
jetons de couleur, de taille et d'espacement (`src/ui/tokens.css`) ; un écran
ne redessine ni bouton, ni champ, ni panneau, et n'écrit aucune couleur en
dur. Raison : les cinq écrans avaient chacun leur barre, leurs boutons et
leurs messages, presque pareils mais pas tout à fait (le projet s'ouvrait
dans deux écrans sur quatre, les erreurs s'affichaient en bas, en haut ou
au milieu). Ce que cela interdit : une feuille de style d'écran qui refait
un composant commun ; un nouvel objet sans ses gestes ajouter, modifier,
supprimer (ou la mention de ce qui manque) ; un raccourci pris pour autre
chose que ce que `docs/design.md` lui réserve.

**D15-a. Thème sombre par défaut, clair au choix, automatique possible ;
toile toujours sombre.** Les dessins sont des traits blancs sur noir et les
photos de chambre noire sont sombres : une toile claire les écraserait. Le
choix est dans le menu Présentation, gardé dans le navigateur de l'app
(simple confort, comme le dernier projet ouvert).

**D15-b. Une seule couleur d'accent : le bleu du Mac.** Elle marque la
sélection et l'action principale. Le rouge de la mire, qui servait aussi aux
boutons enfoncés, est réservé aux repères posés sur les images. Vert, orange,
rouge et bleu ne disent que l'état (bon, à surveiller, douteux, pas encore
mesuré), avec une légende.

**D15-c. Menus dans l'interface d'abord, barre de menus native ensuite**
(faite le même jour, D15-g).
Le menu Projet (Nouveau ⌘N, Ouvrir ⌘O, Relire ⌘R) est le même dans chaque
écran ; le menu Présentation regroupe l'apparence, la liste des raccourcis
(⌘/) et l'écran de démonstration de la visionneuse (provisoire, D13), qui
quitte la rangée des étapes. La barre de menus native du Mac demande une
modification de la coque Tauri, donc une compilation Mac sur GitHub (dix
fois plus chère, D14) et un essai sur Mac : elle viendra dans une demande à
part, avec les mêmes entrées et les mêmes raccourcis.

**D15-d. Suppression : confirmation tant qu'il n'y a pas d'annulation.**
Un objet qui a demandé du travail se supprime après une confirmation dont le
bouton dit ce qu'il fait ; un point de recalage se supprime sans (⌫ ou
corbeille). Quand l'annulation existera (4.3), plus de confirmation.

**D15-f. Retirer un projecteur n'efface pas ses photos.** Le projecteur,
son redressement et son recalage sortent de `projet.json` ; ses photos
brute et redressée restent dans le dossier du projet (on peut les
réimporter, et rien ne se perd par erreur dans un dossier partagé par
Dropbox). Renommer garde l'identifiant, donc tout le travail. Le nom est
refusé s'il est vide ou déjà pris (sans tenir compte des majuscules), comme
à l'ajout. Commandes : `insitu projector rename|remove`.

**D15-g. La barre de menus du Mac envoie des raccourcis, pas des actions.**
Chaque entrée propre à InSitu porte le raccourci de l'interface ; la coque
le transmet (évènement `menu`) et l'interface déclenche l'action branchée
sur ce raccourci dans l'écran affiché. Raison : la coque ne calcule rien
et ne connaît pas l'état des écrans ; une seule liste d'actions, celle des
raccourcis. Limite : une entrée ne se grise pas quand son action est
indisponible ; la choisir ne fait alors rien.

**D15-i. L'inspecteur de la Façade est un panneau en onglets.** Demande de
Benjamin (12 novembre au soir) : un panneau de propriétés contextuel, par
onglets. Après son premier retour, quatre onglets : Plan (fichier, mise en
plan c'est-à-dire l'échelle, fresque, retrait), Dessins (dessins à poser,
mesures des dessins choisis), Calques (rangement des dessins posés ;
l'arborescence par diapositive y vient), Export (simulation, export des
diapositives, fiche de calepinage). Le panneau de gauche ne garde que les
projecteurs et l'affichage. Tous les onglets restent montés, cachés : ⌘E
exporte quel que soit l'onglet affiché. L'onglet change seul sur un
évènement (dessins choisis : Dessins ; pose de A et B : Plan ; plan
disparu : Plan), jamais en continu ; le dernier choisi est retenu
(`localStorage`, une commodité, pas une donnée du projet).

**D15-j. Les outils des dessins sont des icônes dans la barre d'outils.**
Demande de Benjamin : « les outils (déformation, etc.) sur la barre
horizontale en haut, sous forme d'icônes ». `CompositionToolbar` : tourner,
symétries, déformer (quatre coins, grille, redresser), aligner, répartir,
ordre, groupe ; chaque icône a sa bulle d'aide (avec son raccourci) et se
grise quand elle ne s'applique pas à la sélection. L'inspecteur garde les
mesures (centre, largeur, rotation, calque, diapositive). Pour la place,
« Assembler des photos… » et « Tout voir » passent aussi en icônes.

**D15-k. Le plan de façade tourne par quarts de tour, et ce qui est posé
tourne avec lui.** Demande de Benjamin : « import plan : ajouter fonction de
rotation ». `facade.rotation` (0 à 3 quarts de tour, sens des aiguilles
d'une montre) dit comment afficher le fichier ; le repère de la façade (et
donc les mm) est celui du plan tourné, si bien que tout le reste du moteur
(export, simulation, recalage) n'a rien à savoir. Choix :
- *Quarts de tour seulement.* Ils sont exacts : une image est tournée pixel
  pour pixel, un plan vectoriel est rendu tel quel puis ses tuiles tournées,
  sans perte de netteté. Un angle libre demanderait de rééchantillonner une
  photo (perte de netteté), d'agrandir le cadre (coins vides) et de tourner
  chaque moteur de rendu vectoriel : écarté tant qu'aucun cas réel ne le
  demande. Pour une photo d'ensemble penchée, la piste serait plutôt
  « redresser par deux points de l'horizontale ».
- *Ce qui est posé suit le plan.* La mise à l'échelle est gardée (ses deux
  points tournent) ; dessins posés (centre, rotation + 90°), cadres de
  déformation et points de recalage côté façade tournent autour du coin
  haut-gauche du plan, pour rester au même endroit du dessin
  d'architecte. Les retouches par diapositive (en mm de diapositive) ne
  bougent pas. Sans mise à l'échelle, rien n'est en mm et seul le plan
  tourne.
- *Une fresque importée bloque la rotation* (message clair) : c'est une
  image dans un cadre droit, qui ne sait pas tourner.
- Pyramides : une autre clé de cache par orientation (les plans déjà
  affichés gardent la leur). Un assemblage de photos recalculé sur un plan
  tourné repart sans échelle (l'image assemblée n'est pas tournée).
- Commande : `insitu facade rotate <projet> <quarts>`.

**D15-h. Retirer un dessin ou le plan n'efface aucun fichier.** Comme
pour un projecteur (D15-f), l'élément sort de `projet.json` et son fichier
reste dans `bibliotheque/` ou `facade/` ; un nouvel import du même fichier
le reprend sans copie en double. Un dessin retiré emporte ses placements
sur la façade (un placement cite toujours un dessin de la bibliothèque, la
confirmation dit combien). Le plan retiré emporte sa mise à l'échelle ; ce
qui est en mm de façade (dessins posés, points de recalage, fresque) est
gardé et retrouve sa place sur le plan suivant une fois mis à l'échelle.
Commandes : `insitu library remove`, `insitu facade remove`.

**D15-e. L'écran de démonstration garde son ancien style.** Il est
provisoire (D13) ; il profite des nouvelles couleurs par les anciens noms de
variables, gardés comme alias dans `tokens.css`.

## 2026-11-12 — Tâche 3.6

Vue diapositive : `crates/insitu-core/src/export/preview.rs`, commande
`insitu slides`, onglet « Diapositive » (`src/slide/`), mode d'emploi
`docs/vue-diapositive.md`.

**D-3.6-a. La diapositive montrée est calculée comme l'export, en aperçu de
2 048 px.** Même chaîne et mêmes filtres que l'export (3.5), sans marge,
miroir ni négatif, à 186 dpi environ (0,14 mm par pixel, la résolution utile
d'une photo redressée) : ce qu'on voit est ce qui sera gravé. Le noir
devient transparent (blanc à opacité variable) et l'interface colore les
traits (cyan par défaut) pour qu'ils se voient sur une photo claire.

**D-3.6-b. Aperçus dans le cache de l'app, refaits seulement si quelque
chose change.** Clé : fichier de la fresque (chemin, taille, date), son
rectangle, les paires du recalage, la taille. La fresque n'est lue que si un
aperçu manque ; la bascule entre projecteurs ne recalcule rien.

**D-3.6-c. Un onglet à part, après Recalage.** La vue diapositive sert à
vérifier projecteur par projecteur ; elle accueillera l'ajustement par
diapositive (4.6). Flèches ← → pour passer d'un projecteur à l'autre ; les
tuiles des photos déjà vues sont gardées.

## 2026-11-12 — Tâche 3.5

Export des diapositives : module `crates/insitu-core/src/export/`, commandes
`insitu fresco` et `insitu export`, section « Fresque » du panneau de
l'écran Façade (`src/facade/ExportPanel.tsx`), mode d'emploi
`docs/export.md`, critère prouvé par `crates/insitu-core/tests/export.rs`.

**D-3.5-a. Le PNG fait 280 mm par défaut ; une case donne la plaque de
300 mm avec nom et repères.** Le plan demandait à la fois des repères gravés
« dans la marge de 10 mm de la plaque de 300 mm » (décision du 11 novembre)
et un PNG qui « s'ouvre dans LightBurn à 280 mm ». Les deux ne tiennent pas
dans un même fichier. Benjamin a tranché le 12 novembre : **export à
280 mm** (15 433 px à 1 400 dpi), comme Verre-A. La plaque de 300 mm
(16 535 px), avec nom et repères dans la marge, reste disponible en option.

**D-3.5-b. La fresque est posée sur toute la largeur de la façade.**
À l'import, la fresque couvre la façade de son bord gauche à son bord
droit, à partir du coin haut-gauche, en gardant ses proportions : c'est le
cas d'une fresque exportée d'Illustrator sur le même plan de travail que la
façade. La façade doit donc être à l'échelle. La ligne de commande accepte
un autre rectangle (`--frame`). Recaler la fresque sur la façade par points
n'est pas prévu par le plan.

**D-3.5-c. La fresque est lue en niveaux de gris, rééchantillonnée une
seule fois.** Une fresque en couleur devient sa luminance (Rec. 601) ; une
transparence est posée sur du noir. Chaque pixel de la diapositive est
envoyé sur la façade par le recalage (3.2), puis lu dans la fresque par le
moteur de déformation (3.4), sans image intermédiaire. Le miroir est
appliqué à l'image de 280 mm seulement : le nom gravé dans la marge reste
lisible dans le fichier.

**D-3.5-d. Nom et repères : une police de points intégrée, en capitales
sans accents.** Pour ne dépendre d'aucune police du Mac, le texte est
dessiné avec une police de 5 × 7 points (3,5 mm de haut). Repères : une
équerre à chaque coin, qui prolonge les bords de l'image, et un trait au
milieu de chaque côté, à 1 mm de l'image, en traits de 0,25 mm. Ils sont
dessinés en blanc (gravés), avant le négatif éventuel. Légende : projet,
diapositive, résolution, miroir et négatif s'ils sont demandés.

**D-3.5-e. Noir et blanc pur : seuil simple, au choix.** Conformément à la
décision du 11 novembre (pas de trame), le PNG 1 bit applique un seuil (128
par défaut) ; le PNG en niveaux de gris reste le défaut, comme Verre-A.
La résolution est inscrite dans le PNG (pHYs), au point par mètre près.

**D-3.5-f. Les réglages utilisés deviennent ceux du projet.** Le format du
projet gagne trois champs d'export (`margin`, `oneBit`, `threshold`), avec
une valeur par défaut : pas de changement de `formatVersion`.

**D-3.5-g. Le critère est vérifié en pleine résolution, en compilation
optimisée.** Le vrai Verre-A (15 433 px) est posé sur une façade en trapèze
avec du relief (fresque de 18 400 × 17 200 px, 0,25 mm par pixel), puis
réexporté à 1 400 dpi sans marge. Mesuré en session cloud (4 cœurs) :
calcul 12,6 s, PNG 0,3 s ; 100 % des traits exportés tombent sur un trait de
Verre-A et 99,92 % des traits de Verre-A sont retrouvés, à un pixel près ;
décalage local au pire 0,10 px (2 µm) ; le PNG annonce 280,0 mm. La
vérification GitHub le relance à chaque fois.

## 2026-11-12 — Tâche 3.3

Vue façade : module `crates/insitu-core/src/overview/`, commande
`insitu overview`, panneau « Projecteurs » de l'écran Façade
(`src/facade/OverviewPanel.tsx`, `src/facade/overview.ts`), deux nouveaux
calques de la visionneuse (`src/viewer/overlays.ts`), mode d'emploi
`docs/vue-facade.md`.

**D-3.3-a. La vue façade est intégrée à l'écran Façade, pas un onglet de
plus.** Le plan, les emprises et les photos partagent la même visionneuse ;
un panneau à droite liste les projecteurs. Raison : c'est la vue sur
laquelle on composera la fresque (4.2) ; un onglet séparé dédoublerait le
plan. Ce que cela interdit : rien ; l'écran de composition pourra reprendre
le panneau tel quel.

**D-3.3-b. Les photos redressées sont montrées par un aperçu de 1 024 px
tendu sur un maillage de 16 × 16 cases.** Le moteur envoie chaque nœud d'une
grille régulière de la diapositive sur la façade (la même transformation que
l'emprise, relief compris) ; l'interface y tend l'aperçu (JPEG, qualité 85).
Raison : dix photos pleine résolution (5 512 px) ne tiennent pas en mémoire
graphique pour une vue d'ensemble, et 1 024 px suffisent à juger d'un
recalage à l'échelle de la façade. Une maille de 16 cases (17,5 mm de
diapositive) suit les déformations du recalage sans écart visible ; la
pleine résolution viendra, si besoin, par les tuiles de la visionneuse.

**D-3.3-c. Les aperçus vont dans le cache de l'app (`apercus/<clé>.jpg`),
pas dans le projet.** La clé tient compte du chemin, de la taille et de la
date du fichier : un nouveau redressement refait l'aperçu. Raison : comme
les tuiles (D-1.5), ils se refont à la demande et n'ont pas à voyager par
Dropbox.

**D-3.3-d. Les recouvrements sont calculés sur une grille de 400 cases,
puis détourés.** Chaque emprise est remplie ligne par ligne sur une grille
dont le grand côté (boîte commune aux deux emprises) a 400 cases ; les cases
communes sont détourées par le traceur de contours de la vectorisation
(5.1). Raison : une emprise n'est pas forcément convexe (relief), et un
calcul exact d'intersection de polygones quelconques est beaucoup plus
fragile. Précision : 1/400 du recouvrement, soit environ 1 cm sur 4 m, assez
pour juger d'un chevauchement. La surface affichée est celle de la grille.

**D-3.3-e. Photos en mode « écran », emprises et recouvrements à trait
constant.** Les photos s'éclaircissent l'une l'autre au lieu de se cacher
(leur fond noir devient transparent, comme D-4.1-b) ; une photo masquée
reste chargée pour revenir sans délai. Le trait des emprises garde la même
épaisseur à l'écran à tous les zooms.

## 2026-11-12 — Tâche 4.1

Bibliothèque de dessins : module `crates/insitu-core/src/library/`, commande
`insitu library`, écran `src/library/`, mode d'emploi `docs/bibliotheque.md`,
critère prouvé par `crates/insitu-core/tests/bibliotheque.rs`.

**D-4.1-a. « Liens relatifs » : les dessins sont copiés dans le projet.**
Le plan parle de liens relatifs ; D-1.4-b interdit de citer un fichier resté
hors du dossier projet. Les deux se concilient ainsi : un dossier importé est
copié sous `bibliotheque/<nom du dossier>/`, sous-dossiers compris, et chaque
dessin est inscrit dans `projet.json` (champ `library`) par son chemin
relatif. Le projet s'ouvre donc tel quel sur l'autre Mac. Un dessin déjà
importé à l'identique n'est pas recopié ; un fichier modifié depuis arrive à
côté, avec un suffixe (`merle-2.png`). Rien n'est encore retiré de la
bibliothèque (pas de bouton : à prévoir avec les placements, 4.2, pour ne pas
retirer un dessin posé sur la façade).

**D-4.1-b. Fond noir rendu transparent, à l'affichage seulement.**
Chaque pixel devient la couleur qui, posée sur du noir, redonne le pixel
d'origine : l'opacité est la plus forte des trois composantes, la couleur est
divisée par elle. Un trait blanc reste blanc et opaque, un gris devient un
blanc à demi transparent, le noir disparaît. Affiché sur du noir, le dessin
est donc identique au fichier ; sur la façade, seul le trait se voit. Un
fichier qui a déjà de la transparence est d'abord posé sur du noir. Le
fichier n'est jamais modifié. Limite : un dessin noir sur fond blanc
deviendrait un aplat blanc ; ce n'est pas l'usage de Luminariste.

**D-4.1-c. Formats : PNG, JPEG, TIFF, BMP, PSD aplati, SVG ; PSD lu par le
moteur lui-même.**
Le BMP est lu par la bibliothèque `image` (fonction ajoutée). Pour le PSD,
la bibliothèque Rust essayée (`zune-psd` 0.5) lisait mal les fichiers non
compressés et les 16 bits : le moteur lit lui-même l'image aplatie que
Photoshop enregistre toujours à la fin du fichier (`library/psd.rs`, une
centaine de lignes, testée en 8 et 16 bits, bitmap, compressée ou non). Les
calques ne sont pas lus. Refusés avec un message : CMJN, couleurs indexées,
32 bits, image aplatie compressée en ZIP (rare). Si Photoshop a enregistré
le fichier sans « compatibilité maximale », l'image aplatie peut manquer :
non vérifié sur un vrai fichier de Marc. Un SVG est rendu depuis ses
vecteurs à la taille de la vignette ; sa taille est en pixels CSS. Le JPEG,
hors liste du plan, est admis parce qu'il ne coûte rien.

**D-4.1-d. Vignettes (320 px) et aperçus (1 024 px) dans le cache de l'app.**
Une image est réduite de moitié en moitié, avec la moyenne en lumière des
pyramides (un trait fin reste visible), jusqu'à tenir dans le côté voulu :
256 px pour un dessin de 4 096 px. Elles sont rangées dans le cache de l'app
(`vignettes/`, servies par le schéma `tuiles://`), pas dans le projet : rien
de calculé ne passe par Dropbox. Clé : chemin, taille et date du fichier.
L'import les fabrique au passage ; à la première ouverture sur l'autre Mac,
la liste les refait avant de répondre (quelques secondes pour 200 dessins).
L'aperçu est fait à la première demande. La pleine résolution servira aux
placements (4.2).

**D-4.1-e. Import en parallèle, 8 dessins à la fois au plus.**
Un dessin est lu une seule fois : contrôle, dimensions et vignette. Un
fichier illisible est signalé et n'est pas copié ; les autres continuent.
Plafond de 8 fils parce qu'un dessin de 4 096 px en couleur occupe 64 Mo une
fois lu. Mesure (session cloud, 4 cœurs) : 200 dessins de 4 096 px importés
en 2,7 s, puis relus avec leurs vignettes en 1 ms. Les fichiers cachés
(`.DS_Store`) sont ignorés ; un « : » dans un nom devient « - ».

**D-4.1-f. Recherche et dossiers : dans l'interface.**
Filtrer une liste déjà chargée n'est pas un calcul du moteur : la recherche
(chaque mot dans le nom ou le dossier, sans tenir compte des accents ni des
majuscules) et l'arbre des dossiers sont dans `src/library/logic.ts`, testés.

## 2026-11-12 — Tâche 5.1

Moteur de vectorisation : `crates/insitu-core/src/vectorize/`, commande
`insitu vectorize`, tests `crates/insitu-core/tests/vectorisation.rs`.

**D-5.1-a. Contour brut : ligne de niveau du seuil, suivie case par case.**
Les nœuds sont les centres des pixels ; le contour passe entre un pixel
« dedans » (valeur ≥ seuil) et un pixel « dehors », là où l'interpolation
linéaire des deux valeurs vaut `seuil − 0,5`. Les gris du bord d'un trait
placent donc le contour au dixième de pixel près (mesuré sur un disque
anticrénelé : 0,09 px au plus), et un centre de pixel n'est jamais sur le
contour. Une bordure noire entoure l'image : tous les contours sont fermés.
Case en selle (deux coins opposés dedans) : la moyenne des quatre coins
tranche. Le trait est toujours à droite du sens de parcours (y vers le bas) :
bords extérieurs et trous tournent en sens opposés, le remplissage
« nonzero » et « evenodd » donnent le même dessin. Une image en couleur est
ramenée à sa luminance (Rec. 709), la transparence posée sur du noir.

**D-5.1-b. Lissage : droites et cubiques de Bézier (méthode de Schneider).**
Angles vifs cherchés sur le contour simplifié à 0,75 px (en dessous, ce sont
les marches d'une image sans anticrénelage) : virage de plus de 70° par
défaut. Entre deux angles, une droite si elle passe assez près de tous les
points, sinon une cubique aux moindres carrés, coupée en deux là où l'écart
dépasse la tolérance (0,3 px par défaut). L'écart est mesuré aux points bruts
et à trois endroits entre deux points (sinon une boucle entre deux points
passe inaperçue : vu sur le verre A) ; une poignée plus longue que le chemin
est refusée. Pas de lissage préalable de l'image : il déplacerait les bords.
« Assez près », pour une droite : toute la tolérance si elle mesure 16 px ou
plus, le quart sinon. Une droite courte posée sur un arc passe à l'intérieur
de l'arc et amincit le trait (à pleine tolérance, les traits d'un pixel des
réduits perdaient 10 % de surface ; 4 % avec cette règle, pour 2 % de
morceaux en plus) ; une droite longue ne remplace qu'un arc presque droit, et un
bord droit reste d'un seul tenant malgré ses coins arrondis.

**D-5.1-c. L'écart est mesuré par un calcul indépendant de l'ajustement.**
`compare` (et `insitu vectorize --check`) recalcule le contour brut, aplatit
les courbes à 0,005 px près, et mesure la distance de Hausdorff dans les deux
sens (courbes parcourues tous les 0,25 px ; contour brut à ses sommets et
aux milieux de ses côtés). Il remplit aussi les courbes au centre de chaque
pixel et relève la distance à la courbe de chaque pixel classé autrement que
par l'image seuillée. Le critère « moins d'un demi-pixel » porte sur ces deux
chiffres. Mesuré sur les quatre verres d'Épinal en pleine résolution : 0,325 px
au plus, pixels mal classés au plus à 0,30 px de la courbe, surface du trait
changée de moins de 1 % ; 1,6 à 3,8 s par verre sur 4 cœurs (contrôle non
compris, quelques secondes de plus).

**D-5.1-d. Coordonnées en pixels de l'image source.**
Centre du premier pixel en (0,5 ; 0,5), comme D-2.4-c. Le passage en
millimètres viendra avec le transport des courbes vers la diapositive
(tâche 5.3). Le SVG de `insitu vectorize --svg` est en pixels, blanc sur noir,
pour comparer à l'œil avec l'image ; ce n'est pas l'export de la tâche 5.3.

**D-5.1-e. Limites connues.**
Lecture des images par le module `image` déjà présent : PNG, JPEG, TIFF ;
le BMP et le PSD prévus pour la bibliothèque (4.1) n'y sont pas encore. Les
taches minuscules sont gardées par défaut (`--min-area` pour les retirer).
Les verres d'Épinal mêlent hachures fines et aplats gris : beaucoup de
petits contours (35 000 à 110 000 par verre, 0,5 à 1,6 million de morceaux).
Aucun dessin de Marc n'a encore été essayé : les dix dessins de styles
différents demandés pour 5.2 manquent.

## 2026-11-12 — Tâche 3.2

Recalage par points : module `crates/insitu-core/src/registration/`,
commande `insitu registration`, écran `src/registration/`, mode d'emploi
`docs/recalage.md`.

**D-3.2-a. Le diagnostic est calculé de la façade vers la diapositive.**
Les écarts et le seuil des points aberrants sont ainsi en mm de diapositive,
quelle que soit l'échelle de la façade ; l'écart sur la façade est déduit
(écart ramené côté source, D-2.3). La correspondance utilisée ensuite
(emprise, export) va de la diapositive vers la façade, par les mêmes points.

**D-3.2-b. Écart affiché selon le nombre de points.**
4 points : trapèze exact, pas d'écart. 5 points : écart de chaque point au
trapèze ajusté sur les cinq. 6 points et plus : écart de validation croisée
(le point prédit par les autres), et recherche des points aberrants (D-2.3).
Le résumé et la couleur d'un point (vert sous 1 mm) excluent les points
aberrants de la moyenne.

**D-3.2-c. Points aberrants : facteur 6 (inchangé), plancher 1 mm sur la
diapositive.** En dessous de 1 mm d'écart sur la diapositive (environ 7 px
de photo redressée à 500 dpi, et quelques centimètres sur la façade), un
point n'est jamais déclaré aberrant. Réglage vérifié par simulation (test
`a_misplaced_point_is_flagged_and_good_points_are_not`) : 16 points, erreur
de pointage de 0,3 mm, relief de 60 mm ; aucune fausse alerte sur 40 essais,
19 points faux sur 20 trouvés à 5 mm (le manqué est un point du bord), 20
sur 20 à 10 mm. **Non réglé sur des points réels** : aucun recalage cliqué
à la main n'existe encore. Un essai a été fait sur des correspondances
réelles d'Épinal tirées automatiquement des verres et du déroulé (étude hors
dépôt), mais ces verres ont été composés dessin par dessin dans Illustrator :
ils ne suivent pas une seule déformation, et ne disent rien de la précision
d'un clic. Le réglage sera repris sur les premiers points cliqués par
Benjamin (`insitu registration show` les relit).

**D-3.2-d. Un point de diapositive doit être dans le carré de 280 mm.**
Le moteur refuse les paires hors du carré, et des paires qui ne permettent
pas d'ajuster le trapèze (points confondus ou alignés) : le projet n'est
alors pas modifié et l'écran revient à l'état précédent. Les points de
façade ne sont pas bornés (le plan peut être plus petit que la façade).

**D-3.2-e. Pose en deux clics, sans mode.**
Un clic sur une vue pose le point de ce côté ; le clic sur l'autre vue
complète la paire, dans n'importe quel ordre. Un glisser déplace la vue sans
poser de point (même règle que la mise à l'échelle, `isClick`). Pour
déplacer un point existant, on le sélectionne dans la liste. Pas de glisser
des repères : à voir après l'essai de Benjamin.

**D-3.2-f. Non fait : la photo « lum ».** Le plan prévoit qu'elle puisse
servir de repère large ; le format de projet n'a pas encore de place pour
elle (pas de champ `litPhoto`). Noté pour une tâche suivante.

**D-3.2-g. La photo recalée posée sur la façade, dans l'écran Recalage.**
Demande de Benjamin (12 novembre 2026). Le calque est celui de l'écran
Façade (maillage et aperçu de `facade_overview`, tâche 3.3), relu après
chaque recalage enregistré, sans nouveau calcul dans le moteur. Fondu
normal à 60 % d'opacité, réglable, et non « écran » comme dans l'écran
Façade : sur un plan clair, le fondu « écran » rendrait la photo
invisible. La consigne des premiers points dit en clair qu'il faut des
détails d'architecture communs, pas les coins de la photo.

**D-3.2-h. Recalage direct sur la façade : poser, glisser les coins,
épingler.** Demande de Benjamin (12 novembre 2026, au soir, avant un essai
grandeur nature) : le recalage par deux clics est trop complexe. On garde le
même modèle (des paires diapositive ↔ façade, même calcul, même format de
projet), mais on le manipule directement sur la façade :

- **Poser la photo sur la façade** crée quatre paires aux coins de la
  diapositive, en carré au milieu de la vue (de la taille moyenne des
  emprises déjà recalées, sinon 40 % de la vue). La photo s'affiche aussitôt
  par-dessus le plan.
- On **glisse les coins** sur la façade jusqu'à ce que la photo tombe sur le
  plan ; la **poignée du centre** déplace toute la photo.
- Pour affiner, on **attrape un détail de la photo** sur la façade et on le
  lâche sur le même détail du plan : c'est une nouvelle paire (le point de
  diapositive est celui affiché sous le pointeur, retrouvé dans le maillage
  affiché, mêmes triangles). Un simple clic (sans glisser) garde la pose en
  deux clics de D-3.2-e.
- Pendant un glisser, la photo suit en direct : le moteur calcule l'emprise
  et le maillage des paires non enregistrées (`registration_preview`,
  `insitu registration preview`), une demande à la fois ; on enregistre au
  lâcher. Dans l'exemple du navigateur seulement (pas de moteur), un trapèze
  par les 4 premiers points est calculé dans l'interface.
- Barre d'espace + glisser déplace la vue, comme dans Illustrator (un
  glisser sur la photo l'épingle désormais). Le défilement à deux doigts
  déplace toujours la vue.
- La vue de la façade prend 3/5 de la largeur, la photo redressée 2/5.
- L'aperçu de la photo redressée est fabriqué dès qu'elle existe, recalée ou
  non (`facade_overview`).

Cela amende la consigne de D-3.2-g (« pas les coins de la photo ») : elle
valait pour des coins cliqués à l'aveugle sur le plan. Ici, les coins sont
calés à l'œil, photo posée sur le plan ; ce sont des paires comme les
autres, que les détails épinglés corrigent. Les coins gardent leur nom
dans la liste (« Coin haut-gauche »…). Rien n'est automatique : un
appariement automatique photo ↔ plan (analyse d'image, photo « lum ») reste
à étudier ; il serait trop fragile pour l'essai du 13 novembre.

**D-3.2-i. Ajustement automatique de la pose sur le plan.** Demande de
Benjamin (13 novembre 2026) : replacer tout seul la photo sur le plan, en
pensant aux façades qui ne sont pas plates. Deux formes lui ont été
proposées (plusieurs places cherchées sur tout le plan, ou ajustement d'une
pose à la main) ; il a choisi **l'ajustement seul**. Étude :
`docs/etudes/recalage-automatique.md`.

- On pose la photo à peu près à la main (D-3.2-h), puis « **Ajuster sur le
  plan** » la cale sur les lignes du plan autour de cette pose.
- Méthode : comparaison des **orientations de contours** de la photo
  redressée (grille de la mire effacée par une fermeture) et du plan, sur
  le cadre de la pose élargi d'une demi-emprise ; recherche locale par
  corrélation (transformée de Fourier, nouvelle dépendance `rustfft`, pur
  Rust), puis ajustement des quatre coins du trapèze. Le résultat ne
  s'éloigne pas de plus d'un quart d'emprise de la pose, et n'est gardé
  que s'il s'accorde mieux au plan qu'elle. Les points d'intérêt (SIFT)
  ont été essayés et ne marchent pas entre une photo et un plan au trait.
- Résultat : quatre paires aux coins (même format que « Poser la photo »),
  **enregistrées aussitôt** ; elles remplacent tous les points, y compris
  les détails épinglés (ils corrigeaient l'ancienne pose). « Annuler
  l'ajustement » remet les points d'avant, tant qu'on n'a rien retouché.
- La place est un trapèze (façade supposée plate) : le relief se corrige
  ensuite en épinglant des détails, comme avant.
- Commande : `insitu registration adjust <projet> <projecteur>
  [paires.json]`, Tauri `registration_adjust` (part des paires affichées).

**D-3.2-j. Points sûrs libres ; « Ajuster sur le plan » retiré de l'écran
pour l'instant.** Retours de Benjamin (13 novembre 2026) : l'ajustement
« tombe toujours à côté » avec le plan détaillé (`PlanBase-2`), et poser
quatre coins contredit l'usage, car ses points sûrs « ne sont pas forcément
dans un quadrilatère » (groupés, alignés…).

- **Les quatre coins** posés par « Poser la photo » (paires tenues par un
  coin de la diapositive) **ne sont qu'un guide**. Les autres paires sont
  les **points sûrs**, posés n'importe où sur la photo. Quand un point sûr
  est posé, glissé, poussé aux flèches ou supprimé, les coins guides sont
  recalculés pour s'accorder avec lui, selon ce que les points sûrs
  permettent de décider :
  - 1 point : la photo glisse ;
  - 2 points, ou plus mais alignés : elle glisse, tourne et change
    d'échelle, et garde la forme du guide ;
  - 3 points, ou plus mais peu étalés (écart type < 25 mm de diapositive
    sur l'axe le plus mince, ≥ 15 mm) : transformation affine ;
  - 4 points bien étalés (écart type ≥ 25 mm sur les deux axes) : la forme
    vient des seuls points sûrs (homographie aux moindres carrés), et les
    coins guides s'y posent.
  Glisser un coin guide le bouge seul, comme avant. Les coins restent des
  paires ordinaires dans `projet.json` (aucun nouveau champ), toujours
  d'accord avec les points sûrs : ils ne les tirent plus en arrière.
  Logique : `followPins` dans `src/registration/logic.ts`.
- **« Ajuster sur le plan » est caché** (`ADJUST_SHOWN`). Le moteur
  (`registration/auto.rs`) et `insitu registration adjust` restent. Trois
  autres méthodes ont été essayées sur les photos A, C, D de Trévarez avec
  le plan détaillé, sans succès (1 à 4 m d'écart, même en partant de la
  bonne place) : chamfer des lignes horizontales et verticales, lignes
  isolées (hachures retirées), contours d'une densité d'encre floutée. Les
  hachures du toit et les rangs de briques du plan, la photo sombre (grille
  de la mire, matériel devant le mur) brouillent la comparaison. Détails :
  `docs/etudes/recalage-automatique.md`. Choix de Benjamin (carte de décision,
  13 novembre 2026, parmi retirer, garder en petit coup de pouce ou
  continuer à chercher) : **le retirer pour l'instant**.
- Correctif 0.1.62 : avec des points sûrs (5 paires ou plus), une photo
  posée loin de l'origine de la façade (vers 35 m) était refusée, « la
  transformation écrase le plan sur une ligne ». Le contrôle d'inversibilité
  du trapèze comparait le déterminant au plus grand coefficient au cube,
  qui grandit avec le décalage d'origine ; il le compare désormais au
  produit des normes des colonnes (borne de Hadamard), indépendant de
  l'échelle et de l'origine.

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

**D-2.5-i. Coins posés à la main : glissés librement, dans n'importe quel ordre.**
Demande de Benjamin après son essai (12 novembre 2026) : plus de pose au clic
dans l'ordre 1-2-3-4. Les quatre poignées apparaissent tout de suite et se
glissent où l'on veut. Le moteur range les coins (sens des aiguilles d'une
montre à l'écran, en partant du plus proche du coin haut-gauche de la photo,
`order_corners`), puis lit le sens de la mire sur les six petits cercles,
comme la détection automatique. Si ce sens reste incertain, il garde celui de
la photo. Cela remplace la règle de D-2.4a selon laquelle l'ordre des coins
fixe le sens de la mire.

**D-2.5-j. Miroir de la diapositive, à l'écran de redressement.**
Demande de Benjamin (12 novembre 2026). Deux boutons à bascule, gauche-droite
et haut-bas, retournent la diapositive du projecteur. Le miroir est appliqué
aux coordonnées `slide` des points du redressement **et** des points du
recalage déjà posés : tout ce qui suit (photo redressée, export, recalage,
déformation) reste cohérent sans rien savoir du miroir. Le réglage est noté
dans `rectification.mirror` (champ facultatif, ancien fichier lisible tel
quel) pour être réappliqué quand la mire est détectée de nouveau. Ce n'est
pas le miroir de l'export pour la gravure (tâche 3.5), qui reste à part.

**D-2.5-k. HEIC : aussi pour la façade, et un secours par le Mac.**
Benjamin a vu un HEIC refusé (12 novembre 2026). Trois changements :
l'écran Façade accepte maintenant le HEIC (il le refusait exprès, avec un
message demandant un JPEG) ; un fichier est reconnu comme HEIF aussi par ses
marques compatibles, pas seulement par sa marque principale ; si notre
décodeur échoue (variante non prise en charge : c'était le cas de la photo
« C BRUT.heic » de Benjamin, en couleurs 4:4:4 ; ou 10 bits), le
moteur demande à `sips`, le convertisseur fourni avec macOS, d'en faire un
PNG provisoire. Ce secours dépend du système, mais seulement sur Mac et en
dernier recours ; les tests (Linux) ne le parcourent pas. Le décodeur à
nous reste le premier, pour que Mac et tests donnent les mêmes pixels
(D-2.5-a).

**D-2.5-l. Grille illisible : redressement par les quatre coins seuls.**
Benjamin a été bloqué (12 novembre 2026) : sur sa photo, même les coins
posés à la main étaient refusés (« seulement 7 croisements de la grille
mesurés, il en faut au moins 40 »). Quand les coins sont donnés et que la
grille ne se lit pas, le moteur redresse maintenant par le trapèze des
quatre coins, sans correction de l'objectif ni du papier. Rien n'est
caché : `grid_unread` (`gridUnread` à l'écran et en ligne de commande) dit
pourquoi, l'écran l'affiche en orange avec « précision non mesurée », et
aucun croisement n'est coloré. Le fichier projet ne change pas : c'est un
redressement « manuel » à quatre points, reconnu comme tel à la relecture.
Le sens de la mire est lu sur les petits cercles quand ils se voient ;
sinon le coin le plus en haut à gauche de la photo devient le haut-gauche,
et un **quart de tour** (`rotate_projector`, `insitu projector rotate`)
corrige le sens. Il tourne aussi les points du recalage, comme le miroir
(D-2.5-j), mais n'est pas gardé : une nouvelle détection retrouve le sens
d'elle-même. La détection automatique (sans coins) n'a pas de tel repli :
elle a besoin de la grille pour trouver la mire. Prouvé par
`an_unreadable_grid_is_rectified_by_the_corners_alone` (photo A réduite et
floutée).

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
automatique (mise à jour automatique ajoutée depuis, avec l'accord de
Benjamin : D16).

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

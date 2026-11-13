# Recalage d'un projecteur sur la façade (tâche 3.2)

Le recalage relie la diapositive d'un projecteur (sa photo redressée, en mm
de diapositive) à la façade (en mm réels), par des points communs : un coin
de fenêtre, le pied d'une colonne, la pointe d'un arc… visibles sur les deux.

## Avant de commencer

- Onglet **Façade** : le plan est importé **et mis à l'échelle** (les points
  de façade sont en mm réels).
- Onglet **Redressement** : la photo de mire du projecteur est redressée.

## Dans l'app : le plus simple

1. Étape **Recalage** (⌘3), choisir le projecteur dans le panneau de gauche.
2. **Poser la photo sur la façade** (bouton en haut du panneau de droite) :
   la photo redressée apparaît au milieu de la vue de la façade, en
   transparence, avec un rond à chaque coin et une poignée au centre.
   Astuce : cadrer d'abord la vue de la façade sur la zone du projecteur,
   la photo s'y pose.
3. **Glisser la poignée du centre** pour amener la photo à peu près à sa
   place, et au besoin **glisser les coins**. Les coins ne sont qu'un
   **guide** de départ.
4. **Caler par des points sûrs** : sur la vue de la façade, **attraper un
   détail sûr de la photo** (coin de fenêtre, pointe d'un arc…) et le
   **lâcher sur le même détail du plan**. Il devient un point (numéroté des
   deux côtés). Les points sûrs se posent **où l'on veut**, même groupés
   ou alignés : les coins guides les suivent.
   - 1 point : la photo glisse pour le suivre ;
   - 2 points (ou des points alignés, par exemple le long d'une corniche) :
     elle tourne et change aussi de taille, sans se déformer ;
   - 3 points : elle se penche aussi ;
   - 4 points bien répartis : sa forme ne dépend plus que d'eux.
   Une phrase sous les boutons dit où l'on en est. Au-delà, la photo
   reste un trapèze (une perspective, sans courbe), ajusté au mieux sur
   tous les points (D-3.2-l).
5. Pour déplacer la vue pendant ce travail : **barre d'espace + glisser**
   (comme dans Illustrator), ou défilement à deux doigts sur le pavé
   tactile. ⌘0 recadre les deux vues.

Tout est enregistré au lâcher, et chaque geste s'annule seul par ⌘Z. Un
point (coin compris) se corrige en le glissant à nouveau, se supprime par
la corbeille de sa ligne ou ⌫. ⌘Z pendant qu'on glisse un point remet ce
point où il était, sans rien annuler d'autre.

### Caler par lignes

Quand un bord se voit mieux qu'un point (bord de fenêtre, corniche, chaîne
d'angle), choisir **Lignes** en haut du panneau de droite. Le choix est
toujours là ; il faut seulement que la photo soit posée (par ses coins, ou
par 4 points). S'il manque des coins guides (points posés un à un, coin
déplacé sur la photo), ils sont remis à la pose actuelle, sans la changer.

1. Sur la **photo** (à gauche), appuyer sur un bout du bord et glisser
   jusqu'à l'autre : un trait jaune attend son double.
2. Sur le **plan** (à droite), tracer le même bord de la même façon. Les
   deux traits n'ont pas besoin d'avoir la même longueur. La ligne
   devient **L1** (rose, des deux côtés) et la photo suit.
3. Recommencer. Une horizontale : la photo glisse. Deux lignes : elle
   tourne et change de taille. **Deux horizontales et deux verticales :
   le trapèze est retrouvé.** Les points sûrs comptent aussi.

Un bout de ligne se corrige en le glissant, une ligne se supprime par la
corbeille de sa rangée dans la liste. Échap abandonne un trait en attente.
En mode Lignes, glisser trace un trait : pour déplacer la vue, barre
d'espace + glisser.

### Essai sur le Mac (à faire par Benjamin)

1. Projet Trévarez, étape Recalage, projecteur **D**. Supprimer ses points
   (ils la posent sur l'aile droite, alors qu'elle montre la façade du
   haut), puis **Poser la photo sur la façade** et la glisser vers les
   fenêtres de 29 à 33 m.
2. Attraper sur la photo la clé de la fenêtre du milieu et la lâcher sur
   celle du plan. Attendu : toute la photo glisse, sans se déformer.
3. Attraper un deuxième détail sur la même corniche (alignée avec le
   premier) et le lâcher sur le plan. Attendu : la photo tourne et change
   de taille, sans se pencher ni se tordre.
4. Poser deux autres points loin de la corniche (bas des fenêtres, coin du
   mur). Attendu : avec 4 points bien répartis, la photo prend la forme
   qu'ils donnent ; la phrase sous les boutons le dit.
5. Glisser un point puis, **pendant le glisser**, ⌘Z : le point revient,
   rien d'autre n'est annulé. Après le lâcher, ⌘Z annule ce seul geste.

Par lignes (D-3.2-k), sur le même projecteur :

6. Choisir **Lignes**. Tracer sur la photo le bas de la corniche, puis le
   même sur le plan. Attendu : la photo glisse pour que sa corniche tombe
   sur celle du plan.
7. Tracer ensuite une deuxième horizontale (appuis des fenêtres) et deux
   verticales (bords de deux fenêtres éloignées). Attendu : après la
   quatrième ligne, les fenêtres de la photo tombent sur celles du plan.
8. Glisser un bout d'une ligne sur le plan : la photo suit ; ⌘Z annule.

« Ajuster sur le plan » n'est plus montré pour l'instant (D-3.2-j) : sur
les photos de Trévarez et le plan détaillé, il tombait à côté.

## Dans l'app : point par point

La pose en deux clics reste possible, et se combine avec ce qui précède :

1. Étape **Recalage** (⌘3). L'écran reprend le projet ouvert dans les
   autres étapes (menu **Fichier** → **Relire depuis le disque**, ⌘R, le
   recharge). Le panneau **Projecteurs**, à gauche, ne montre que les
   projecteurs redressés ; le panneau **Points communs**, à droite, liste
   les points et leur écart.
2. Au centre, la photo redressée à gauche et la façade à droite. Chaque vue se
   déplace et se zoome comme d'habitude (un glisser hors de la photo ne pose
   pas de point).
3. **Quels points ?** Des **détails d'architecture visibles à la fois sur
   la photo et sur la façade** : coin de fenêtre, pied de colonne, pointe
   d'un arc, angle d'un bandeau.
   **Cliquer le détail sur la photo, puis le même sur la façade** (ou dans
   l'autre ordre). Le premier clic laisse une cible jaune en attente ; le
   second crée la paire, numérotée des deux côtés. Échap abandonne le point
   en attente.
4. Il faut **4 points** pour le trapèze, bien répartis (près des coins de la
   diapositive). L'emprise du projecteur se dessine alors en bleu sur la
   façade, **et la photo redressée s'y pose, par-dessus le plan**.
   En haut, **Photo recalée** la montre ou la cache, et le curseur à côté
   règle sa transparence.
5. Les points suivants affinent le trapèze, ajusté au mieux sur tous (la
   photo ne se courbe pas, D-3.2-l). À partir du 5e, chaque point a un
   **écart** au trapèze : en mm sur la diapositive, en cm sur la façade
   (colonne de droite). À partir du 6e, un point que les autres
   contredisent nettement est signalé.
   - vert : moins de 1 mm sur la diapositive ;
   - orange : plus de 1 mm, sans être jugé faux (un point cliqué un peu à
     côté, ou une façade qui n'est pas plane) ;
   - rouge : **point douteux**, que les autres contredisent nettement. Le
     vérifier, le déplacer ou le retirer.
6. **Corriger un point : le glisser.** Au-dessus d'un point, le pointeur
   devient une main : on le saisit et on le lâche à sa place, sur la photo
   ou sur la façade, dans n'importe quel ordre. Un simple clic sur un point
   (ou sur sa ligne dans la liste) le sélectionne : un clic ailleurs sur la
   photo ou sur la façade l'y amène. Cliquer à nouveau (ou Échap)
   désélectionne. La corbeille de sa ligne (visible au survol), ou ⌫ sur
   le point choisi, le supprime.

Chaque modification est enregistrée aussitôt dans le projet.

## En ligne de commande

```
insitu registration show Essai.insitu A
insitu registration set Essai.insitu A paires.json
insitu registration preview paires.json      # pose provisoire, rien n'est enregistré
insitu registration adjust Essai.insitu A    # cale la pose sur le plan (D-3.2-i), rien n'est enregistré
```

`paires.json` : `{ "pairs": [ { "slide": [21.4, 18.9], "facade": [612.0, 480.0] }, … ] }`.
Détail et réglages : `insitu registration --help`.

## Essayer l'écran dans un navigateur, sans l'app

Le moteur n'est pas joignable depuis `npm run dev` : l'écran montre un
exemple déjà tuilé, on peut y poser des points, mais aucun écart n'est
calculé et rien n'est enregistré (façade supposée à 4 mm par pixel). La
photo posée sur la façade n'y suit que les 4 premiers points (un simple
trapèze) ; elle demande un aperçu `apercu.jpg` à côté des tuiles.

```
I="cargo run --release -q -p insitu-cli --"
$I tiles jeu-essai/epinal/references/photos-redressees/a.jpg tuiles-demo/recalage-photo
$I tiles jeu-essai/epinal/references/facade/portail-deroule.png tuiles-demo/recalage-facade
python3 -c "from PIL import Image; i=Image.open('jeu-essai/epinal/references/photos-redressees/a.jpg'); i.thumbnail((1024,1024)); i.save('tuiles-demo/recalage-photo/apercu.jpg')"
npm run dev
```

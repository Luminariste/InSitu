# Recalage d'un projecteur sur la façade (tâche 3.2)

Le recalage relie la diapositive d'un projecteur (sa photo redressée, en mm
de diapositive) à la façade (en mm réels), par des points communs : un coin
de fenêtre, le pied d'une colonne, la pointe d'un arc… visibles sur les deux.

## Avant de commencer

- Onglet **Façade** : le plan est importé **et mis à l'échelle** (les points
  de façade sont en mm réels).
- Onglet **Redressement** : la photo de mire du projecteur est redressée.

## Dans l'app

1. Étape **Recalage** (⌘3). L'écran reprend le projet ouvert dans les
   autres étapes (menu **Projet** → **Relire depuis le disque**, ⌘R, le
   recharge). Le panneau **Projecteurs**, à gauche, ne montre que les
   projecteurs redressés ; le panneau **Points communs**, à droite, liste
   les points et leur écart.
2. Au centre, la photo redressée à gauche et la façade à droite. Chaque vue se
   déplace et se zoome comme d'habitude (un glisser ne pose pas de point).
3. **Quels points ?** Des **détails d'architecture visibles à la fois sur
   la photo et sur la façade** : coin de fenêtre, pied de colonne, pointe
   d'un arc, angle d'un bandeau. **Pas les coins de la photo** : ils ne
   tombent sur rien de reconnaissable sur le plan. Le panneau de droite le
   rappelle tant qu'il manque des points.
   **Cliquer le détail sur la photo, puis le même sur la façade** (ou dans
   l'autre ordre). Le premier clic laisse une cible jaune en attente ; le
   second crée la paire, numérotée des deux côtés. Échap abandonne le point
   en attente.
4. Il faut **4 points** pour le trapèze, bien répartis (près des coins de la
   diapositive). L'emprise du projecteur se dessine alors en bleu sur la
   façade, **et la photo redressée s'y pose, par-dessus le plan** : on voit
   d'un coup d'œil si les fenêtres de la photo tombent sur celles du plan.
   En haut, **Photo recalée** la montre ou la cache, et le curseur à côté
   règle sa transparence. Elle suit chaque point ajouté ou déplacé.
5. Les points suivants corrigent le relief. À partir du 5e, chaque point a
   un **écart** : en mm sur la diapositive, en cm sur la façade (colonne de
   droite). À partir du 6e, c'est l'écart du point **prédit par les
   autres** : un point mal cliqué s'y voit tout de suite.
   - vert : moins de 1 mm sur la diapositive ;
   - orange : plus de 1 mm, sans être jugé faux (souvent un vrai relief, ou
     un point au bord, prédit de loin) ;
   - rouge : **point douteux**, que les autres contredisent nettement. Le
     vérifier, le déplacer ou le retirer.
6. **Corriger un point : le glisser.** Au-dessus d'un point, le pointeur
   devient une main : on le saisit et on le lâche à sa place, sur la photo
   ou sur la façade, dans n'importe quel ordre. L'écart se recalcule au
   lâcher. Un simple clic sur un point (ou sur sa ligne dans la liste) le
   sélectionne : un clic ailleurs sur la photo ou sur la façade l'y amène.
   Cliquer à nouveau (ou Échap) désélectionne. La corbeille de sa ligne
   (visible au survol), ou ⌫ sur le point choisi, le supprime.

Chaque modification est enregistrée aussitôt dans le projet.

## En ligne de commande

```
insitu registration show Essai.insitu A
insitu registration set Essai.insitu A paires.json
```

`paires.json` : `{ "pairs": [ { "slide": [21.4, 18.9], "facade": [612.0, 480.0] }, … ] }`.
Détail et réglages : `insitu registration --help`.

## Essayer l'écran dans un navigateur, sans l'app

Le moteur n'est pas joignable depuis `npm run dev` : l'écran montre un
exemple déjà tuilé, on peut y poser des points, mais aucun écart n'est
calculé et rien n'est enregistré (façade supposée à 4 mm par pixel).

```
I="cargo run --release -q -p insitu-cli --"
$I tiles jeu-essai/epinal/references/photos-redressees/a.jpg tuiles-demo/recalage-photo
$I tiles jeu-essai/epinal/references/facade/portail-deroule.png tuiles-demo/recalage-facade
npm run dev
```

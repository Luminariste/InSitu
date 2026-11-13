# Plan de façade (tâche 3.1)

## Dans l'app

1. Étape **Façade** (⌘1). Le menu **Fichier** de la barre de menus du
   Mac : **Nouveau projet…** (⌘N) crée un projet (un dossier
   `Nom.insitu`, à mettre dans le Dropbox) ; **Ouvrir un projet…** (⌘O)
   rouvre un projet existant. Le nom du projet ouvert s'affiche en haut à
   gauche, à côté d'« InSitu ». L'app rouvre seule le dernier projet au
   lancement suivant.
2. **Importer un plan…** (⌘I, panneau de droite, onglet **Plan**, en haut) : PDF d'architecte, image (PNG, JPEG, TIFF, HEIC), SVG
   ou DXF. Pour un PDF de plusieurs pages, l'app demande laquelle est la
   façade. Le fichier est copié dans le dossier `facade/` du projet. Un DWG
   est à exporter d'abord en PDF ou en DXF.
3. Dans le panneau de droite, onglet **Plan**, rubrique **Mise en plan** : **Poser A
   et B**, puis cliquer deux points du plan dont on connaît la
   distance réelle (deux repères d'une ligne de cote, deux angles du
   bâtiment). Zoomer avant de cliquer donne un point plus juste : le plan
   reste net à tout zoom.
4. Écrire la **distance A–B** (`12,5 m`, `1250 cm` ou `12500`, en mm par
   défaut) et **Valider l'échelle**. La barre d'état affiche alors les dimensions de
   la façade en mètres ; les coordonnées de la barre d'état sont en mm réels.

5. **Orientation** (onglet **Plan**, sous les dimensions) : les deux flèches
   tournent le plan d'un quart de tour à gauche ou à droite, par exemple un
   PDF arrivé couché. On peut le faire avant ou après la mise à l'échelle :
   l'échelle est gardée, et ce qui est déjà posé (dessins, points de
   recalage) tourne avec le plan. Seule une fresque importée empêche de
   tourner : la retirer avant. Pas d'angle libre pour l'instant (D15-k).

6. **Retirer le plan…** (onglet **Plan**, à droite) : après confirmation, le plan et
   sa mise à l'échelle sortent du projet ; le fichier reste dans `facade/`.
   Dessins posés, points de recalage et fresque sont gardés en mm : ils
   retrouvent leur place sur le plan suivant une fois mis à l'échelle. Pour
   changer de plan sans le retirer : **Remplacer le plan…** (⌘I).

Avant la mise à l'échelle, la barre d'état compte dans l'unité du fichier
(points pour un PDF) : le bas de l'écran le rappelle en orange.

## En ligne de commande

```
insitu facade inspect plan.pdf                       # nature, dimensions, pages
insitu project new Essai.insitu
insitu facade import Essai.insitu plan.pdf --page 2  # copie dans le projet
insitu facade scale Essai.insitu 200 1484 1200 1484 35277.8
insitu facade rotate Essai.insitu 1                  # quart de tour à droite (-1 : à gauche)
insitu facade remove Essai.insitu                    # retire le plan du projet
insitu facade render plan.pdf apercu.png --width 4000
insitu facade tiles plan.pdf cache/                  # comme le fait l'app
```

Les points sont dans l'unité du fichier, origine en haut à gauche, y vers le
bas (voir `docs/format-projet.md`).

## Essayer l'écran dans un navigateur, sans l'app

Le moteur n'est pas joignable depuis `npm run dev` : l'écran montre alors un
plan déjà tuilé, et la mise à l'échelle n'est pas enregistrée. Pour le
préparer :

```
cargo run --release -p insitu-cli -- facade render plan.pdf /tmp/plan.png --width 8192
cargo run --release -p insitu-cli -- tiles /tmp/plan.png tuiles-demo/facade
npm run dev
```

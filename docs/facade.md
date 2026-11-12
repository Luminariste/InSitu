# Plan de façade (tâche 3.1)

## Dans l'app

1. Onglet **Façade**. **Nouveau…** crée un projet (un dossier `Nom.insitu`,
   à mettre dans le Dropbox) ; **Ouvrir…** rouvre un projet existant. L'app
   rouvre seule le dernier projet au lancement suivant.
2. **Importer un plan…** : PDF d'architecte, image (PNG, JPEG, TIFF), SVG ou
   DXF. Pour un PDF de plusieurs pages, l'app demande laquelle est la
   façade. Le fichier est copié dans le dossier `facade/` du projet. Un DWG
   est à exporter d'abord en PDF ou en DXF ; une photo HEIC, en JPEG.
3. **Poser A et B**, puis cliquer deux points du plan dont on connaît la
   distance réelle (deux repères d'une ligne de cote, deux angles du
   bâtiment). Zoomer avant de cliquer donne un point plus juste : le plan
   reste net à tout zoom.
4. Écrire la **cote A–B** (`12,5 m`, `1250 cm` ou `12500`, en mm par
   défaut) et **Valider**. Le bas de l'écran affiche alors les dimensions de
   la façade en mètres ; les coordonnées de la barre d'état sont en mm réels.

Avant la mise à l'échelle, la barre d'état compte dans l'unité du fichier
(points pour un PDF) : le bas de l'écran le rappelle en orange.

## En ligne de commande

```
insitu facade inspect plan.pdf                       # nature, dimensions, pages
insitu project new Essai.insitu
insitu facade import Essai.insitu plan.pdf --page 2  # copie dans le projet
insitu facade scale Essai.insitu 200 1484 1200 1484 35277.8
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

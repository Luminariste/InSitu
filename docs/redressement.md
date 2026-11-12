# Redressement des photos de mire (tâche 2.5)

## Dans l'app

1. Étape **Redressement** (⌘2). L'écran reprend le projet ouvert ailleurs
   (le menu **Projet**, en haut à gauche, en ouvre ou en crée un autre).
2. Le bouton **+** du panneau **Projecteurs**, à gauche, crée un
   projecteur, nommé par la première lettre libre (A, B, C…). La liste
   passe d'un projecteur à l'autre (clic, ou ↑ ↓) et dit où en est
   chacun : pastille verte = redressé, orange = photo à redresser.
3. **Importer la photo…** (⌘I) : la photo de la mire prise en chambre noire, en
   HEIC (format des iPhone), JPEG, PNG ou TIFF. Elle est copiée dans le
   dossier `photos-brutes/` du projet, et la mire y est cherchée aussitôt.
4. Si la mire est trouvée : les croisements retenus s'affichent en couleur
   sur la photo (vert : moins de 0,15 mm d'écart ; orange : moins de 0,3 mm ;
   rouge : au-delà), le contour bleu montre le carré de 280 mm, et le bas de
   l'écran résume le résultat. La photo redressée est enregistrée dans
   `photos-redressees/` ; **Photo redressée** l'affiche.
5. Si la mire n'est pas trouvée (ou si le résultat est faux) : **Poser les
   coins**. Quatre poignées apparaissent sur la photo : glisser chacune sur
   un coin de la mire, dans n'importe quel ordre. La loupe, en haut à
   droite, montre les pixels de la photo autour de la poignée. **Détecter
   depuis les coins** : l'app retrouve seule quel coin est lequel (par les
   petits cercles de la mire), et les coins n'ont pas besoin d'être exacts
   (1 ou 2 mm suffisent), la grille affine ensuite.
6. **Miroir** : **↔ Gauche-droite** et **↕ Haut-bas** retournent la photo
   redressée (et donc l'export et les points du recalage) ; un second clic
   annule. Le réglage est gardé si la mire est détectée de nouveau.
7. **Exporter…** : la photo redressée en TIFF ou en PNG, à 300, 500, 1 000
   ou 1 400 dpi. La résolution est inscrite dans le fichier : il s'ouvre à
   280 mm dans Illustrator, Photoshop ou LightBurn. L'image est recalculée
   depuis la photo brute à chaque export (ce n'est pas un agrandissement de
   la photo redressée du projet).

En vue « Photo brute », la barre d'état compte en pixels de la photo ; en
vue « Photo redressée », en mm de diapositive. Une photo d'iPhone en JPEG
peut apparaître couchée : l'app ne tourne pas les pixels (décision D9), la
détection n'en a pas besoin. Un HEIC apparaît droit (décision D-2.5-a).

## En ligne de commande

```
insitu project new Essai.insitu
insitu projector add Essai.insitu A
insitu projector photo Essai.insitu A IMG_1234.HEIC
insitu projector rectify Essai.insitu A                  # détection automatique
insitu projector rectify Essai.insitu A --corners 2500,420,2540,2600,750,2610,720,430   # coins dans n'importe quel ordre
insitu projector show Essai.insitu A                     # relit le résultat enregistré
insitu projector export Essai.insitu A A.tif --dpi 500
```

`insitu rectify <photo>` reste disponible pour une photo hors projet.

## Essayer l'écran dans un navigateur, sans l'app

Le moteur n'est pas joignable depuis `npm run dev` : l'écran montre alors un
exemple déjà préparé, et rien n'y est calculé (les poignées et la loupe
fonctionnent, pas la détection). Pour le préparer :

```
I="cargo run --release -q -p insitu-cli --"
$I project new /tmp/essai.insitu && $I projector add /tmp/essai.insitu A
$I projector photo /tmp/essai.insitu A jeu-essai/heic/a-brut.heic
$I projector rectify /tmp/essai.insitu A
$I tiles jeu-essai/heic/a-brut.heic tuiles-demo/redressement-brute
$I tiles /tmp/essai.insitu/photos-redressees/a.png tuiles-demo/redressement-redressee
$I projector show /tmp/essai.insitu A --nodes > tuiles-demo/redressement-vue.json
npm run dev
```

# Redressement des photos de mire (tâche 2.5)

## Dans l'app

1. Étape **Redressement** (⌘2). L'écran reprend le projet ouvert ailleurs
   (le menu **Fichier** du Mac en ouvre ou en crée un autre).
2. Le bouton **+** du panneau **Projecteurs**, à gauche, crée un
   projecteur, nommé par la première lettre libre (A, B, C…). La liste
   passe d'un projecteur à l'autre (clic, ou ↑ ↓) et dit où en est
   chacun : pastille verte = redressé, orange = photo à redresser.
   Au survol d'une ligne : le **crayon** renomme le projecteur (ou
   double-clic sur son nom ; ↩ valide, Échap annule), la **corbeille** le
   supprime après confirmation (ou ⌫). Le renommage garde tout le travail ;
   la suppression perd le redressement et le recalage, mais les photos
   restent dans le dossier du projet.
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
   petits cercles de l'ancienne mire, ou par les repères codés, les petits
   carrés noirs, de la nouvelle mire), et les coins n'ont pas besoin d'être exacts
   (1 ou 2 mm suffisent), la grille affine ensuite.
   **Si la grille ne se lit pas** (photo floue, sombre, mire mal éclairée),
   la photo est quand même redressée, par les quatre coins seuls : le bas
   de l'écran le dit en orange, avec la raison. C'est un simple trapèze,
   dont la précision n'est pas mesurée : poser les coins avec soin (loupe)
   et vérifier la photo redressée. Si ni les petits cercles ni les repères codés ne se voient
   non plus, le sens peut être faux : **↻ Quart de tour** (à côté du
   miroir) tourne la photo redressée, autant de fois qu'il faut.
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
insitu projector rotate Essai.insitu A    # quart de tour (redressement par les coins seuls)
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

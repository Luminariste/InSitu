# Format du dossier projet `.insitu`

Un projet InSitu est un **dossier** dont le nom se termine par `.insitu`, par
exemple `Épinal.insitu`. Il se partage tel quel par Dropbox. Tout ce qui le
concerne est dedans ; rien ne pointe vers l'extérieur.

Le code qui lit et écrit ce format est dans
`crates/insitu-core/src/project/`. Version du format décrite ici : **1**.

## Contenu du dossier

```
Épinal.insitu/
  projet.json          La description du projet (texte lisible)
  verrou.json          Présent tant que quelqu'un a le projet ouvert
  facade/              Le plan ou la photo de la façade
  photos-brutes/       Les photos de mire prises en chambre noire
  photos-redressees/   Les photos redressées
  fresque/             La fresque importée en une image
  projet.v<N>.json     Copie de l'ancien fichier après une mise à niveau
```

Quand on importe un fichier, l'app en met une **copie** dans le bon
sous-dossier. Si un fichier différent porte déjà ce nom, la copie reçoit un
suffixe (`a-brut-2.jpg`).

Les noms de dossiers et de fichiers sont en français, parce qu'on les voit
dans le Finder. Les noms de champs dans `projet.json` sont en anglais, comme
dans le code (voir le vocabulaire de `CLAUDE.md`).

## Règles générales

- **Chemins relatifs.** Un fichier est désigné par son chemin à l'intérieur du
  dossier, avec des `/` : `photos-brutes/a-brut.jpg`. Un chemin absolu
  (`/Users/…`), un `..` ou un `\` font refuser le projet à l'ouverture : il
  ne s'ouvrirait pas sur l'autre Mac.
- **Millimètres.** Toutes les longueurs sont en millimètres, origine en haut à
  gauche, y vers le bas. Sur une diapositive, de 0 à 280. Sur la façade, en
  millimètres réels du bâtiment.
- **Seule exception : les points repérés dans un fichier importé**
  (`facade.scale.pointA`, `rectification.points[].photo`). Ils sont dans
  l'unité du fichier : des pixels pour une image, comptés sur l'image telle
  qu'elle est enregistrée (sans la rotation EXIF).
- **Dates** en temps universel : `2026-11-11T14:03:22Z`.
- **Identifiants** (`id`) : 16 caractères tirés au hasard, attribués une fois
  pour toutes. Renommer un projecteur de « A » en « Gauche » ne change pas son
  identifiant ; c'est lui que les autres parties du projet citeront.

## Exemple

Projet d'Épinal réduit à deux projecteurs : A est complet, B vient d'être
ajouté. Les coordonnées sont inventées. Un test vérifie que cet exemple est
exactement ce que l'app écrit.

```json
{
  "formatVersion": 1,
  "id": "5f0c2a9e41b7d386",
  "name": "Épinal",
  "createdAt": "2026-11-11T14:03:22Z",
  "facade": {
    "source": "facade/portail-deroule.png",
    "sourceWidth": 5145.0,
    "sourceHeight": 3508.0,
    "scale": {
      "pointA": {
        "x": 1650.0,
        "y": 3205.0
      },
      "pointB": {
        "x": 3492.0,
        "y": 3198.0
      },
      "distanceMm": 4200.0
    }
  },
  "projectors": [
    {
      "id": "9b1e6d04c8a2f735",
      "label": "A",
      "rawPhoto": "photos-brutes/a-brut.jpg",
      "rectifiedPhoto": "photos-redressees/a.png",
      "rectification": {
        "method": "manual",
        "points": [
          {
            "photo": {
              "x": 857.2,
              "y": 354.8
            },
            "slide": {
              "x": 0.0,
              "y": 0.0
            }
          },
          {
            "photo": {
              "x": 3166.4,
              "y": 384.8
            },
            "slide": {
              "x": 280.0,
              "y": 0.0
            }
          },
          {
            "photo": {
              "x": 3135.6,
              "y": 2685.6
            },
            "slide": {
              "x": 280.0,
              "y": 280.0
            }
          },
          {
            "photo": {
              "x": 887.2,
              "y": 2656.4
            },
            "slide": {
              "x": 0.0,
              "y": 280.0
            }
          }
        ],
        "residualMm": 0.35
      },
      "registration": [
        {
          "slide": {
            "x": 21.4,
            "y": 18.9
          },
          "facade": {
            "x": 612.0,
            "y": 480.0
          }
        },
        {
          "slide": {
            "x": 262.7,
            "y": 24.3
          },
          "facade": {
            "x": 3980.5,
            "y": 240.0
          }
        },
        {
          "slide": {
            "x": 255.1,
            "y": 259.8
          },
          "facade": {
            "x": 3840.0,
            "y": 7600.0
          }
        },
        {
          "slide": {
            "x": 17.6,
            "y": 251.2
          },
          "facade": {
            "x": 760.25,
            "y": 7840.0
          }
        }
      ]
    },
    {
      "id": "27d4f8a1903e6cb5",
      "label": "B",
      "rawPhoto": null,
      "rectifiedPhoto": null,
      "rectification": null,
      "registration": []
    }
  ],
  "fresco": {
    "source": "fresque/fresque-epinal.png",
    "frame": {
      "x": 0.0,
      "y": 0.0,
      "width": 11730.0,
      "height": 7998.0
    }
  },
  "export": {
    "dpi": 1400.0,
    "mirror": false,
    "negative": false
  }
}
```

## Les champs

### Le projet

| Champ | Sens |
| --- | --- |
| `formatVersion` | Version du format (voir « Versions »). Obligatoire |
| `id` | Identifiant du projet, indépendant de son nom |
| `name` | Nom affiché |
| `createdAt` | Date de création |
| `facade` | La façade, ou `null` tant qu'aucun plan n'est importé |
| `projectors` | La liste des projecteurs, dans l'ordre d'affichage |
| `fresco` | La fresque importée en une image, ou `null` |
| `export` | Les réglages d'export |

### `facade` — la façade

| Champ | Sens |
| --- | --- |
| `source` | Le fichier importé (plan, déroulé ou photo d'ensemble) |
| `sourceWidth`, `sourceHeight` | Dimensions du fichier, dans son unité (voir ci-dessous) |
| `scale` | Mise à l'échelle « par deux points et une cote », ou `null` si elle n'est pas faite |
| `scale.pointA`, `scale.pointB` | Les deux points, repérés dans le fichier |
| `scale.distanceMm` | La distance réelle entre ces deux points, en mm |
| `page` | PDF seulement : la page retenue, à partir de 1. Absent pour les autres formats |
| `origin` | DXF seulement : le coin haut-gauche du repère du fichier, en coordonnées du DXF (y vers le haut). Absent sinon |

L'unité du fichier dépend de son format (tâche 3.1) :

| Format | Unité | Origine (x vers la droite, y vers le bas) |
| --- | --- | --- |
| Image (PNG, JPEG, TIFF) | pixel | coin haut-gauche de l'image |
| PDF | point (1/72 de pouce) | coin haut-gauche de la page telle qu'elle s'affiche |
| SVG | pixel CSS (1/96 de pouce) | coin haut-gauche du dessin |
| DXF | unité de dessin | coin haut-gauche du rectangle qui englobe tout le dessin (`origin`) |

Le repère de la façade se déduit de là : un point du fichier, multiplié par
l'échelle (`distanceMm` ÷ distance entre les deux points), donne sa position
en millimètres réels. Le coin haut-gauche du fichier est l'origine.

### `projectors[]` — un projecteur

| Champ | Sens |
| --- | --- |
| `id` | Identifiant stable |
| `label` | Libellé affiché et gravé : « A », « B »… |
| `rawPhoto` | La photo brute, ou `null` |
| `rectifiedPhoto` | La photo redressée, ou `null`. Elle couvre exactement le carré de 280 mm |
| `rectification` | Le résultat du redressement, ou `null` |
| `rectification.method` | `manual` (points posés à la main) ou `automatic` (mire détectée) |
| `rectification.points[]` | Des points de la mire : `photo` (pixels de la photo brute) et `slide` (mm sur la diapositive). Quatre coins en manuel ; davantage en automatique |
| `rectification.residualMm` | Écart résiduel en mm de diapositive, ou `null` s'il n'est pas connu |
| `registration[]` | Le recalage : des paires de points communs, `slide` (mm sur la diapositive) et `facade` (mm réels sur la façade) |

### `fresco` — la fresque en une image

| Champ | Sens |
| --- | --- |
| `source` | Le fichier image |
| `frame` | Le rectangle qu'elle occupe sur la façade, en mm réels : `x`, `y` (coin haut-gauche), `width`, `height` |

### `export` — les réglages d'export

| Champ | Sens | Valeur par défaut |
| --- | --- | --- |
| `dpi` | Résolution, en points par pouce | 1 400 |
| `mirror` | Image retournée gauche-droite | non |
| `negative` | Noir et blanc inversés | non |

Les valeurs par défaut sont définies dans `crates/insitu-core/src/units.rs`.

## Versions

`formatVersion` dit dans quelle version du format le fichier est écrit.

- **Même version que l'app** : ouverture normale.
- **Version plus ancienne** : le projet est mis à niveau à l'ouverture, en
  mémoire. Le fichier ne change qu'au prochain enregistrement ; l'ancien est
  alors gardé à côté sous le nom `projet.v<N>.json`.
- **Version plus récente** : l'app refuse d'ouvrir le projet et demande de la
  mettre à jour. Elle ne modifie rien.

### Champs inconnus

Un champ que l'app ne connaît pas n'est **jamais perdu en silence** :

- dans le projet, la façade, sa mise à l'échelle, un projecteur, un
  redressement, un point de redressement, une paire de recalage, la fresque
  ou les réglages d'export, il est **gardé** en mémoire et **réécrit tel
  quel** à l'enregistrement (après les champs connus). `insitu project
  validate` le signale par un avertissement ;
- dans un point (`x`, `y`) ou un rectangle, il fait **refuser** le fichier.

Conséquence pratique : une version future peut ajouter un champ facultatif
(la bibliothèque de dessins, les placements et les calques du jalon 4, par
exemple) sans changer `formatVersion`. Si une app plus ancienne ouvre et
enregistre ce projet, le champ ajouté est toujours là.

Limite à connaître : une app ancienne conserve un champ inconnu mais ne le
tient pas à jour. Si elle supprime un projecteur, un champ futur qui le
citerait par son identifiant resterait tel quel. C'est une raison de plus
d'avoir la même version de l'app sur les deux Mac.

### Quand changer de version

`formatVersion` ne monte que pour un changement qu'une app plus ancienne
lirait de travers : champ renommé, supprimé, changé de sens ou d'unité,
nouvelle valeur possible pour un champ à choix (`method`). La marche à suivre
est en tête de `crates/insitu-core/src/project/migrate.rs` : une fonction de
mise à niveau par version, un test, et une ligne dans le tableau ci-dessous.

| Version | Changement |
| --- | --- |
| 1 | Première version : façade, projecteurs, fresque en une image, réglages d'export |

## Enregistrement

Le fichier est écrit en entier sous un nom temporaire (`.~projet.json.…`),
puis il remplace l'ancien d'un seul coup. Une panne ou une coupure au mauvais
moment laisse donc soit l'ancien fichier, soit le nouveau, jamais un fichier
tronqué. Un reste de fichier temporaire est effacé à l'enregistrement
suivant.

## Verrou

Il n'y a pas d'édition simultanée. Quand l'app ouvre un projet, elle écrit
`verrou.json` dans le dossier :

```json
{
  "sessionId": "c41a7e09b35d2f68",
  "machine": "MacBook-Pro-de-Benjamin.local",
  "user": "benjamin",
  "openedAt": "2026-11-11T14:03:22Z",
  "refreshedAt": "2026-11-11T14:21:40Z",
  "appVersion": "0.1.0"
}
```

Tant que le projet est ouvert, l'app met `refreshedAt` à jour toutes les deux
minutes. À la fermeture, elle efface le fichier.

À l'ouverture :

| Situation | Ce qui se passe |
| --- | --- |
| Pas de `verrou.json` | Le projet s'ouvre |
| Verrou du même Mac et du même utilisateur | Le projet s'ouvre : l'app avait planté ou été fermée brutalement sur ce poste |
| Verrou de l'autre personne, sans signe de vie depuis plus de 15 minutes | Le verrou est **périmé** (app plantée, Mac éteint ou en veille) : le projet s'ouvre |
| Verrou de l'autre personne, récent | Refus, avec son nom, sa machine et l'heure. L'ouverture peut être forcée si l'on est sûr qu'elle a fermé le projet |

Si quelqu'un force l'ouverture, l'app de l'autre personne s'en aperçoit à son
prochain signe de vie (deux minutes au plus, une fois Dropbox passé) et doit
cesser d'enregistrer.

**Ce verrou avertit, il ne garantit rien.** Il voyage par Dropbox, avec
quelques secondes de retard, et bien plus si un Mac est hors ligne. Deux
personnes qui ouvrent le projet au même moment, ou sans réseau, peuvent
l'avoir toutes les deux. Dans ce cas Dropbox garde les deux versions de
`projet.json` (l'une sous le nom « copie en conflit de… ») : rien n'est
perdu, mais il faut choisir à la main laquelle garder. La règle des 15
minutes suppose aussi que les deux Mac sont à l'heure, ce qui est le cas avec
le réglage automatique de macOS.

## En ligne de commande

```
insitu project new Épinal.insitu [--name "Épinal"]   # crée un projet vide
insitu project info Épinal.insitu                    # résumé en JSON, verrou compris
insitu project validate Épinal.insitu                # contrôle le projet
```

`validate` vérifie que `projet.json` est lisible, que le modèle est cohérent
(identifiants uniques, échelle utilisable, résolution positive…) et que
chaque fichier cité existe. Il affiche une ligne par problème et se termine
en erreur s'il y a au moins une erreur ; les avertissements seuls ne sont pas
bloquants.

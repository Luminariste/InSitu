# Assemblage de photos de façade (tâches 6.4 et 6.6)

Quand il n'y a pas de plan d'architecte, la façade peut venir de plusieurs
photos prises de face, qui se recouvrent en partie. InSitu les recale entre
elles par des points communs et en fait une seule image : la façade du
projet, à mettre ensuite à l'échelle comme un plan.

Depuis la tâche 6.6, l'app **trouve elle-même les points communs** : il
suffit d'ajouter les photos. Les points posés à la main restent possibles,
pour corriger ou pour une photo que l'analyse n'a pas su raccorder.

## Prendre les photos

- De face, le plus perpendiculaire possible au mur, de gauche à droite.
- Chaque photo recouvre la précédente d'un tiers environ.
- Même focale, même exposition si possible ; pas de grand-angle déformant.
- L'iPhone tenu droit, ou les photos enregistrées droites avant l'import :
  la rotation EXIF n'est pas appliquée (D-6.4-f).

L'assemblage suppose un mur plat (D-6.4-a) : un porche ou une avancée
marquée laisse de petits décalages dans les recouvrements.

## Dans l'app

1. Écran **Façade**, panneau de droite, onglet **Plan** : bouton
   **« Assembler des photos… »**, à côté de « Importer un plan… ».
2. **« Ajouter des photos… »** : on peut en choisir plusieurs d'un coup.
   La première est la **référence** (l'image assemblée est dans son
   repère) ; l'ordre des autres n'a pas d'importance.
3. L'app cherche aussitôt les détails communs aux photos (quelques
   secondes par photo) : pour chacune, elle choisit la photo voisine qui
   en partage le plus (son **appui**) et pose jusqu'à 24 points communs
   bien répartis. Un message dit combien de photos sont raccordées, et
   nomme celles qui n'ont rien en commun avec les autres.
4. Vérifier en passant d'une photo à l'autre dans la liste de gauche :
   la photo à gauche, son appui à droite, les points dessus. Couleur des
   points : vert, écart sous 3 px ; orange, au-delà ; rouge, douteux (à
   déplacer en le glissant, ou à supprimer avec ⌫) ; bleu, pas encore
   mesuré (moins de 5 points).
5. Quand toutes les photos sont vertes dans la liste, **« Calculer
   l'assemblage »**. L'image (12 000 px au plus de côté) devient la façade du
   projet. Il reste à la **mettre à l'échelle** avec deux points et une cote.

**« Raccorder automatiquement »** relance l'analyse, par exemple après
avoir ajouté une photo à un assemblage où des points ont été posés à la
main. Elle remplace alors tous les points (l'app demande confirmation).

**À la main**, comme avant la 6.6 : choisir la photo, cliquer un détail
net (angle de fenêtre, joint, moulure) sur la photo de gauche, puis le même
sur celle de droite. Quatre points au moins, bien répartis dans la zone de
recouvrement ; six ou plus pour que l'app repère les points douteux.
L'appui se change dans « Appui », à droite (ses points sont alors effacés).

Recalculer plus tard (une photo ajoutée, un point corrigé) garde l'échelle,
les recalages des projecteurs et la fresque à leur place (D-6.4-d).

## Sans écran

```
insitu assembly add projet.insitu gauche.jpg
insitu assembly add projet.insitu milieu.jpg
insitu assembly auto projet.insitu
insitu assembly pairs projet.insitu milieu gauche paires.json
insitu assembly show projet.insitu
insitu assembly render projet.insitu
```

`auto` fait le raccord automatique (compte rendu en JSON : appui choisi,
détails communs trouvés, points enregistrés, photos non raccordées) ;
`pairs` pose des points à la main. `paires.json` : `{"pairs":[{"here":[x,y],"there":[x,y]}, …]}`, en pixels de
la photo (`here`) et de sa photo d'appui (`there`). Détail :
`insitu assembly --help`.

## Essai dans un navigateur, sans moteur

Pour voir l'écran avec `npm run dev`, tuiler deux photos qui se recouvrent :

```
cargo run -p insitu-cli -- tiles photo1.jpg tuiles-demo/assemblage-1
cargo run -p insitu-cli -- tiles photo2.jpg tuiles-demo/assemblage-2
```

On peut y poser des points ; rien n'est calculé ni enregistré.

## Essai sur Mac (critère du plan : « trois photos assemblées servent de fond de plan »)

1. Créer un projet neuf (menu Fichier, Nouveau projet…).
2. Écran Façade, « Assembler des photos… », ajouter **trois** photos d'une
   même façade prises de gauche à droite (par exemple celles d'Épinal).
3. Pour la 2e photo : poser 6 points communs avec la 1re. Pour la 3e : 6
   points avec la 2e. Vérifier que les points bien posés sont verts, et
   qu'un point volontairement mal posé devient rouge.
4. « Calculer l'assemblage ». Vérifier : l'image apparaît comme façade, sans
   couture visible ni dédoublement des détails dans les recouvrements.
5. Mettre la façade à l'échelle (deux points, une cote connue), puis dans
   l'écran Recalage, recaler un projecteur dessus : l'assemblage sert de fond
   de plan comme un plan d'architecte.
6. Revenir à l'assemblage, déplacer un point, recalculer : l'échelle et le
   recalage du projecteur doivent rester en place.

À noter : le temps du calcul, l'aspect des recouvrements, et toute photo qui
s'afficherait couchée.

## Essai sur Mac de l'assemblage automatique (tâche 6.6)

1. Créer un projet neuf (menu Fichier, Nouveau projet…).
2. Écran Façade, « Assembler des photos… », « Ajouter des photos… » et
   choisir **d'un coup** trois photos (ou plus) d'une même façade, prises
   de face en se décalant le long du mur, chacune recouvrant la
   précédente d'un tiers environ.
3. Sans rien cliquer d'autre : un message doit annoncer « N photos
   raccordées automatiquement », et toutes les photos de la liste doivent
   être vertes. Noter le temps affiché.
4. Passer d'une photo à l'autre : les points posés par l'app doivent
   tomber sur le même détail à gauche et à droite.
5. « Calculer l'assemblage » : l'image ne doit montrer ni couture ni
   détail dédoublé sur le mur (un relief marqué, comme un porche profond,
   peut rester dédoublé : D-6.6-e).
6. Essai de refus : ajouter une photo d'un autre bâtiment. Le message doit
   la nommer comme « sans détail commun », et les autres rester vertes.

À noter : le temps de l'analyse, toute photo non raccordée alors qu'elle
recouvre bien une autre, et tout point posé par l'app sur deux détails
différents.

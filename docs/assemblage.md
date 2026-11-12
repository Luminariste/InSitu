# Assemblage de photos de façade (tâche 6.4)

Quand il n'y a pas de plan d'architecte, la façade peut venir de plusieurs
photos prises de face, qui se recouvrent en partie. InSitu les recale entre
elles par des points communs et en fait une seule image : la façade du
projet, à mettre ensuite à l'échelle comme un plan.

## Prendre les photos

- De face, le plus perpendiculaire possible au mur, de gauche à droite.
- Chaque photo recouvre la précédente d'un tiers environ.
- Même focale, même exposition si possible ; pas de grand-angle déformant.
- L'iPhone tenu droit, ou les photos enregistrées droites avant l'import :
  la rotation EXIF n'est pas appliquée (D-6.4-f).

L'assemblage suppose un mur plat (D-6.4-a) : un porche ou une avancée
marquée laisse de petits décalages dans les recouvrements.

## Dans l'app

1. Écran **Façade**, bouton **« Assembler des photos… »** dans la barre.
2. **« Ajouter des photos… »** : on peut en choisir plusieurs d'un coup. La
   première est la **référence**. Chaque photo suivante s'appuie par défaut
   sur celle ajoutée juste avant (modifiable dans « Appui », à droite).
3. Choisir une photo dans la liste de gauche. Elle s'affiche à gauche, sa
   photo d'appui à droite. Cliquer un détail net (angle de fenêtre, joint,
   moulure) sur la photo de gauche, puis le même sur celle de droite : c'est
   un point commun. **Quatre points au moins**, bien répartis dans la zone
   de recouvrement ; six ou plus pour que l'app repère les points douteux.
4. Couleur des points : vert, écart sous 3 px ; orange, au-delà ; rouge,
   douteux (à déplacer en le glissant, ou à supprimer avec ⌫) ; bleu, pas
   encore mesuré (moins de 5 points).
5. Quand toutes les photos sont vertes dans la liste, **« Calculer
   l'assemblage »**. L'image (12 000 px au plus de côté) devient la façade du
   projet. Il reste à la **mettre à l'échelle** avec deux points et une cote.

Recalculer plus tard (une photo ajoutée, un point corrigé) garde l'échelle,
les recalages des projecteurs et la fresque à leur place (D-6.4-d).

## Sans écran

```
insitu assembly add projet.insitu gauche.jpg
insitu assembly add projet.insitu milieu.jpg
insitu assembly pairs projet.insitu milieu gauche paires.json
insitu assembly show projet.insitu
insitu assembly render projet.insitu
```

`paires.json` : `{"pairs":[{"here":[x,y],"there":[x,y]}, …]}`, en pixels de
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

1. Créer un projet neuf (menu Projet, Nouveau projet…).
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

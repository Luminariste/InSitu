# Installer InSitu sur un Mac

Pour Mac à puce Apple (M1 ou plus récent), macOS 12 ou plus récent.

1. Téléchargez le fichier `InSitu-….dmg` ci-dessous (rubrique « Assets »).
2. Ouvrez-le, puis glissez InSitu dans le dossier Applications.
3. Lancez InSitu depuis le dossier Applications.

## Au premier lancement, macOS refuse d'ouvrir l'app

C'est attendu : l'app n'est pas signée auprès d'Apple (il faudrait un compte
développeur payant). macOS affiche « InSitu n'a pas été ouvert » ou « est
endommagé ». Une seule fois par version installée :

1. Cliquez sur « Terminé » (surtout pas « Placer dans la corbeille »).
2. Ouvrez Réglages Système, rubrique « Confidentialité et sécurité ».
3. Descendez jusqu'à la ligne qui mentionne InSitu et cliquez sur
   « Ouvrir quand même », puis confirmez.

Si cette ligne n'apparaît pas, ouvrez le Terminal et collez :

```
xattr -cr /Applications/InSitu.app
```

puis relancez l'app.

## Mettre à jour

Au lancement, InSitu cherche toute seule une version plus récente. Si elle en
trouve une, elle affiche « Une nouvelle version est disponible » : cliquez
sur « Installer et relancer ». L'app télécharge la nouvelle version, vérifie
qu'elle vient bien de Luminariste, se remplace et se relance. « Plus tard » :
la question revient au lancement suivant.

Si cela ne marche pas (pas de réseau, ou version installée trop ancienne pour
se mettre à jour seule), téléchargez la nouvelle version ici et glissez-la
dans Applications en acceptant de remplacer l'ancienne.

Dans tous les cas, vos projets ne sont pas touchés : ils vivent dans leurs
propres dossiers, pas dans l'app.

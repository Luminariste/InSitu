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

Téléchargez la nouvelle version et glissez-la dans Applications en acceptant
de remplacer l'ancienne. Vos projets ne sont pas touchés : ils vivent dans
leurs propres dossiers, pas dans l'app.

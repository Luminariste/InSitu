# Publier une version

Une version est publiée à la fin de chaque jalon, ou dès qu'un essai sur Mac
est utile. Numérotation : `0.<jalon>.<numéro d'ordre>` — `0.1.0` est la
première version du jalon 1, `0.2.1` la deuxième du jalon 2.

1. Dans une demande de fusion, changer `version` dans `Cargo.toml` à la
   racine (rubrique `[workspace.package]`) et lancer `cargo check` pour mettre
   `Cargo.lock` à jour. C'est le seul endroit où le numéro est écrit : l'app
   le reprend automatiquement.
2. Une fois la demande fusionnée, poser l'étiquette sur `main` :

   ```
   git checkout main && git pull
   git tag v0.1.0
   git push origin v0.1.0
   ```

3. GitHub fabrique l'app (une dizaine de minutes) et la publie dans l'onglet
   « Releases » du dépôt, avec la notice d'installation `docs/installer.md`.

Si l'étiquette ne correspond pas à la version du code, la fabrication échoue
exprès : supprimer l'étiquette, corriger, recommencer.

La fabrication contrôle que l'app est bien pour puce Apple, qu'elle porte le
bon numéro, et qu'elle reste ouverte dix secondes après son lancement. Elle
ne contrôle pas ce qui s'affiche : cela reste l'essai de Benjamin.

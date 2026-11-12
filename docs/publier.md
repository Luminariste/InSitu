# Publier une version

Une version est publiée à la fin de chaque jalon, ou dès qu'un essai sur Mac
est utile. Numérotation : `0.<jalon>.<numéro d'ordre>` — `0.1.0` est la
première version du jalon 1, `0.2.1` la deuxième du jalon 2.

Pour publier, il suffit de changer le numéro de version :

1. Dans une demande de fusion, changer `version` dans `Cargo.toml` à la
   racine (rubrique `[workspace.package]`) et lancer `cargo check` pour mettre
   `Cargo.lock` à jour. C'est le seul endroit où le numéro est écrit : l'app
   le reprend automatiquement.
2. Fusionner. GitHub constate que cette version n'a pas encore été publiée,
   fabrique l'app (quelques minutes) et la publie dans l'onglet « Releases »
   du dépôt, avec la notice d'installation `docs/installer.md`.

Tant que le numéro ne change pas, les fusions suivantes ne publient rien.

Il n'y a pas d'étiquette à poser à la main : les sessions cloud n'ont pas le
droit d'en envoyer, c'est GitHub qui la crée en publiant.

La fabrication contrôle que l'app est bien pour puce Apple, qu'elle porte le
bon numéro, et qu'elle reste ouverte dix secondes après son lancement. Elle
ne contrôle pas ce qui s'affiche : cela reste l'essai de Benjamin.

## Mise à jour automatique (D16)

À chaque publication, la fabrication ajoute à la version une archive signée
de l'app (`InSitu-X.Y.Z-mac.app.tar.gz`) et remplace le manifeste
`latest.json` de la branche `mises-a-jour`. Les apps installées le lisent au
lancement et proposent la nouvelle version. Rien à faire de plus à chaque
version.

Il faut pour cela deux secrets dans le dépôt (Settings → Secrets and
variables → Actions), à poser une seule fois :

- `TAURI_SIGNING_PRIVATE_KEY` : la clé privée qui signe les mises à jour. Sa
  clé publique est dans `src-tauri/tauri.conf.json` ; les deux vont ensemble.
- `INSITU_UPDATE_TOKEN` : un jeton GitHub « fine-grained » en lecture seule
  (dépôt InSitu seulement, permission « Contents : Read-only »), que l'app
  utilise pour télécharger depuis ce dépôt privé. Quand il expire, en refaire
  un, remplacer le secret, publier une version et l'installer une fois à la
  main.

Sans eux, la fabrication publie comme avant et l'avertit dans son journal.
Si le secret de signature ne correspond pas à la clé publique, la
fabrication s'arrête avant de publier.

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

À chaque publication, la fabrication joint à la version une archive signée
de l'app (`InSitu-X.Y.Z-mac.app.tar.gz`) et le manifeste `latest.json`. Les
apps installées lisent le manifeste de la dernière version au lancement et
proposent la nouvelle version. Rien à faire de plus à chaque version.

Deux conditions, réglées une seule fois :

- Le dépôt est public (sinon l'app ne peut pas télécharger).
- Le secret `TAURI_SIGNING_PRIVATE_KEY` existe (Settings → Secrets and
  variables → Actions) : la clé privée qui signe les mises à jour. Sa clé
  publique est dans `src-tauri/tauri.conf.json` ; les deux vont ensemble.

Sans ce secret, la fabrication publie comme avant et l'avertit dans son
journal. S'il ne correspond pas à la clé publique, la fabrication s'arrête
avant de publier.

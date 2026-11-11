# InSitu

Atelier de calepinage pour fresques projetées par rétroprojecteurs : on
compose la fresque sur la façade, l'app calcule chaque diapositive de 280 mm,
déformations comprises, et l'exporte pour la gravure laser.

- Le plan et l'avancement : [`docs/plan.md`](docs/plan.md)
- Les choix techniques : [`docs/decisions.md`](docs/decisions.md)
- Les règles pour les sessions Claude Code : [`CLAUDE.md`](CLAUDE.md)

## Installer sur un Mac

Télécharger la dernière version dans l'onglet
[Releases](https://github.com/Luminariste/InSitu/releases) et suivre
[`docs/installer.md`](docs/installer.md).

## Lancer depuis le code

Pour qui veut essayer une modification avant qu'elle soit publiée. Il faut
[Node.js](https://nodejs.org) et [Rust](https://rustup.rs), puis :

```
npm ci
npm run tauri dev
```

La première compilation prend quelques minutes.

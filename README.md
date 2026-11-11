# InSitu

Atelier de calepinage pour fresques projetées par rétroprojecteurs : on
compose la fresque sur la façade, l'app calcule chaque diapositive de 280 mm,
déformations comprises, et l'exporte pour la gravure laser.

- Le plan et l'avancement : [`docs/plan.md`](docs/plan.md)
- Les choix techniques : [`docs/decisions.md`](docs/decisions.md)
- Les règles pour les sessions Claude Code : [`CLAUDE.md`](CLAUDE.md)

## Essayer sur un Mac

Il faut [Node.js](https://nodejs.org) et [Rust](https://rustup.rs), puis :

```
npm ci
npm run tauri dev
```

La première compilation prend quelques minutes. Une fenêtre « InSitu »
s'ouvre et affiche la version du moteur et la taille d'export (15 433 px).

À partir de la tâche 1.2, il ne sera plus nécessaire de compiler : GitHub
fournira l'app prête à installer.

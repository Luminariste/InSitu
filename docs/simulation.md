# Simulation et alertes (tâche 6.1)

La simulation pose toutes les diapositives calculées ensemble sur la façade,
telles qu'elles seront projetées, et signale ce qui ne va pas.

## Dans l'app

1. Étape **Façade**, panneau de gauche, rubrique **Simulation** :
   **Simuler**. L'app calcule la diapositive de chaque projecteur recalé (la
   même que dans la vue Diapositive) et la pose sur la façade, teinte de la
   couleur du projecteur. Là où deux diapositives allument le même point,
   leurs couleurs se mêlent.
2. La case **Montrer les diapositives calculées** les cache ou les remontre ;
   décocher un projecteur dans la liste le cache aussi.
3. Les **alertes** sont listées dessous, chacune encadrée de rouge sur la
   façade :
   - **Dessin coupé** : aucune diapositive ne contient le dessin en entier,
     ou la diapositive choisie à la main (inspecteur, « Diapositive ») le
     coupe alors qu'une autre le contient (le message dit laquelle) ;
   - **Hors emprise** : le dessin ne tombe dans aucune emprise, il ne sera
     pas projeté ;
   - **Projeté deux fois** : dans un recouvrement, un trait est allumé dans
     les deux diapositives. C'est le cas de la fresque importée en une
     image, découpée entre les diapositives ; ou de deux dessins voisins
     rangés dans deux diapositives différentes.
4. **Cliquer une alerte** : le dessin concerné est choisi et la vue se cadre
   sur lui (sur la zone, pour la fresque). On le corrige (le déplacer, le
   réduire, choisir une autre diapositive), puis **Simuler à nouveau** : la
   simulation ne se recalcule pas toute seule.

Les dessins d'un calque masqué ne partent pas à l'export : ils ne donnent pas
d'alerte.

## Sans écran

```
insitu simulate Mon-projet.insitu --cache /tmp/insitu-cache
insitu simulate Mon-projet.insitu --cache /tmp/insitu-cache --json
```

Une alerte par ligne, avec le rectangle de la façade (mm) et le placement
concerné ; `--json` donne tout (aperçus et alertes).

## Ce qui est vérifié, et ce qui ne l'est pas

- Vérifié par les tests : un dessin coupé, un dessin hors emprise et un
  choix à la main qui coupe sont signalés, chacun avec son placement et un
  rectangle qui contient le dessin ; un dessin bien rangé ou sur un calque
  masqué ne l'est pas. Un trait allumé dans les deux diapositives d'un
  recouvrement est trouvé à sa place (à 10 mm près), un trait d'une seule
  diapositive ne l'est pas ; s'il appartient à un dessin posé, l'alerte mène
  à ce dessin. De bout en bout : une fresque importée sur toute la façade
  est signalée projetée deux fois dans le recouvrement, et seulement là.
  Côté écran : une alerte choisit le bon dessin et cadre la vue dessus.
- Vu dans le navigateur de la session, avec des données inventées : la
  rubrique, la liste, le cadrage au clic (captures de la demande de fusion).
- **Pas vérifié** : l'écran dans l'app sur Mac avec un vrai projet (temps de
  calcul de la simulation, lisibilité des couleurs mêlées).

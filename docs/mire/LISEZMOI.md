# La mire à imprimer

`mire-insitu-v2.pdf` est la nouvelle mire (version 2), fabriquée par le
moteur : `cargo run -p insitu-cli -- target pdf docs/mire/mire-insitu-v2.pdf`.
Un test vérifie que ce fichier est à jour. Pour une page carrée de 297 mm sans
consigne : `--page carre`.

Ce qu'elle contient : la grille de 10 mm, la croix, le cercle rouge et le
disque central de l'ancienne mire, aux mêmes places ; des traits de 0,35 mm ;
16 repères codés de 16 mm (AprilTag 36h11, numéros 0 à 15, lus ligne par
ligne de gauche à droite et de haut en bas) ; son identifiant, en petit, sous
la grille.

## Imprimer et contrôler

1. Ouvrir le PDF, imprimer sur A3, échelle **100 % / taille réelle** (surtout
   pas « ajuster à la page »).
2. Mesurer à la règle le carré de la grille, du premier au dernier trait, dans
   les deux sens : **280 mm**, à 0,3 mm près. Un repère noir : **16 mm**.
3. Si la mesure est fausse, regarder les réglages de l'imprimante (marges,
   mise à l'échelle) avant de chercher ailleurs : le PDF lui-même est vérifié
   par les tests à 0,001 mm près.
4. Couper la feuille le long des deux pointillés : carré de 297 mm.

Les essais suivants (atelier, nuit, papier, répétabilité) sont décrits à la
fin de `docs/etudes/2.1-detection-mire.md`.

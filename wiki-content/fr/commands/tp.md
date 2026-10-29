## Description

`/tp` vous téléporte instantanément à des coordonnées précises. Idéal pour explorer, retrouver une construction ou aller voir de près le sommet d'une montagne repérée au loin.

## Syntaxe

`/tp <x> <y> <z>`. Chaque coordonnée peut être :
- un nombre (décimales acceptées) ;
- `~` : votre propre coordonnée actuelle ;
- `~` suivi d'un nombre : décalage par rapport à votre position (`~5`, `~-10`).

## Exemples

- `/tp 0 200 0` : direction le centre du monde, bien au-dessus du sol.
- `/tp ~ ~20 ~` : monter de vingt blocs sur place.
- `/tp ~100 ~ ~` : faire un saut de puce vers l'est.

## Comportement

Exécutée par le serveur, en **mode créatif** uniquement. Vous arrivez **en vol** et immobile : pas de chute surprise. La destination est ramenée à l'intérieur de la bordure du monde et de sa hauteur. La console confirme les coordonnées réellement atteintes.

## Conseils

Notez les coordonnées affichées par F3 avant de partir : vous pourrez revenir d'un seul `/tp`.

## Découverte en jeu

`/help tp` rappelle la syntaxe, et la touche Tab complète le nom de la commande. F3 vous montre votre position pour calculer vos destinations.

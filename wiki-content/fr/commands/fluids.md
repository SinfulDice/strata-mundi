## Description

`/fluids` fait couler ou fige l'eau et la lave du monde, et règle combien de blocs de liquide le serveur met à jour à chaque tick. Le réglage est enregistré avec le monde ; il se choisit aussi à la création du monde (bouton « Fluides »).

## Syntaxe

`/fluids [on|off|query|<budget>]`
- `on` : les liquides coulent.
- `off` : les liquides restent tels quels, là où ils sont.
- `query` (ou rien) : affiche le réglage actuel.
- `<budget>` : nombre de mises à jour de liquide par tick au plus (2048 par défaut) ; le reste attend le tick suivant.

## Exemples

- `/fluids off` : fige un lac pendant une construction.
- `/fluids 512` : écoulements plus lents mais moins coûteux, utile sur une petite machine.

## Comportement

Exécutée par le serveur, en **mode créatif** uniquement. Figer les liquides oublie les blocs en train de couler : ils repartent seulement quand un bloc voisin change après `/fluids on`.

## Conseils

Un budget plus petit ne change pas la forme finale d'un écoulement, seulement sa vitesse quand beaucoup d'eau bouge à la fois.

## Découverte en jeu

`/help fluids` rappelle la syntaxe ; Tab complète `on`, `off` et `query`.

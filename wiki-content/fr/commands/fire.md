## Description

`/fire` règle le comportement du feu dans ce monde. Le réglage est enregistré avec le monde ; il se choisit aussi à la création (boutons « Feu » et « Brûle »).

## Syntaxe

`/fire spread on|off`, `/fire destroy on|off`, `/fire query`
- `spread off` : le feu n'allume plus de nouveaux feux autour de lui ; il s'use et s'éteint là où il est.
- `destroy off` : ce qui brûle n'est plus détruit (le feu finit tout de même par s'éteindre).
- `query` (ou rien) : affiche les deux réglages.

## Exemples

- `/fire spread off` : une cheminée qui ne met jamais le feu à la maison.
- `/fire destroy off` puis `/fire spread on` : un incendie spectaculaire qui ne laisse aucune trace.

## Comportement

Exécutée par le serveur, en **mode créatif** uniquement. Les feux déjà allumés suivent aussitôt le nouveau réglage.

## Découverte en jeu

`/help fire` rappelle la syntaxe ; Tab complète `spread`, `destroy`, `query`, puis `on` et `off`.

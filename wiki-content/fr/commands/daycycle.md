## Description

`/daycycle` fait avancer l'heure du monde ou la fige. Le réglage est enregistré avec le monde ; il se choisit aussi à la création du monde (bouton « Cycle jour/nuit »).

## Syntaxe

`/daycycle [on|off|query]`
- `on` : l'heure avance, le soleil et la lune se déplacent.
- `off` : l'heure reste où elle est (pratique avec `/time set` pour construire toujours en plein jour).
- `query` (ou rien) : affiche le réglage actuel.

## Exemples

- `/time set noon` puis `/daycycle off` : midi pour toujours.
- `/daycycle on` : la journée reprend son cours.

## Comportement

Exécutée par le serveur, en **mode créatif** uniquement. L'heure est aussi figée quand le jeu est en pause.

## Conseils

`/time add` fonctionne même heure figée : on peut avancer de quelques heures sans relancer le cycle.

## Découverte en jeu

`/help daycycle` rappelle la syntaxe ; Tab complète `on`, `off` et `query`.

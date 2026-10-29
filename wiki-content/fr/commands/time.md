## Description

`/time` lit ou règle l'horloge du monde. Chaque monde garde son heure, qui avance toute seule pendant la partie et est enregistrée avec la sauvegarde.

## Syntaxe

`/time <set|add|query> [valeur]`
- `query` : affiche l'heure actuelle.
- `set <valeur>` : règle l'heure, avec un mot-clé (`day` pour le lever du jour, `noon` pour midi, `night` pour le coucher, `midnight` pour minuit) ou un nombre de ticks.
- `add <ticks>` : avance l'horloge d'autant de ticks.

## Exemples

- `/time set noon` : plein midi.
- `/time query` : quelle heure est-il ?

## Comportement

Exécutée par le serveur, en **mode créatif** uniquement. La réponse donne l'heure en ticks, rapportée à la durée d'une journée complète ; au-delà, l'horloge repart simplement de zéro. En v0.1, l'heure est comptée et sauvegardée, mais **le ciel ne change pas encore** : le cycle jour-nuit visible arrivera plus tard.

## Conseils

Les mots-clés sont espacés d'un quart de journée : pratique pour se repérer sans calculer.

## Découverte en jeu

`/help time` rappelle la syntaxe et les mots-clés ; Tab complète `set`, `add`, `query` et les heures.

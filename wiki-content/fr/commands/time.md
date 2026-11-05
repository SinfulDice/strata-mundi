## Description

`/time` lit ou règle l'horloge du monde. Chaque monde garde son heure, qui avance toute seule pendant la partie (sauf en pause ou avec `/daycycle off`) et est enregistrée avec la sauvegarde. Le ciel la suit : soleil, lune, étoiles et lumière du jour.

## Syntaxe

`/time <set|add|query> [valeur]`
- `query` (ou rien) : affiche l'heure actuelle et le numéro du jour.
- `set <valeur>` : avance jusqu'au prochain moment demandé, avec un mot-clé (`day` juste après le lever du soleil, `noon` pour midi, `night` juste après le coucher, `midnight` pour minuit) ou un nombre de ticks depuis le lever.
- `add <ticks>` : avance l'horloge d'autant de ticks.

## Exemples

- `/time set noon` : plein midi.
- `/time set night` : la nuit tombe, la lune se lève.
- `/time add 24000` : un jour plus tard, à la même heure.

## Comportement

Exécutée par le serveur, en **mode créatif** uniquement. Une journée de Terra dure 24 000 ticks (20 minutes) : la moitié de jour, la moitié de nuit. L'heure ne recule jamais : `set` va jusqu'au prochain moment demandé, le lendemain s'il est déjà passé. La lune change de phase chaque nuit, sur un cycle de huit nuits.

## Conseils

Les phases de la lune comptent les jours : de pleine lune à pleine lune, huit nuits passent.

## Découverte en jeu

`/help time` rappelle la syntaxe et les mots-clés ; Tab complète `set`, `add`, `query` et les heures.

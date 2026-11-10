# Guide des sons

Tous les sons du jeu sont **synthétisés** à partir de recettes écrites dans les packs : aucun fichier audio n'est livré. Un pack de ressources peut changer une recette ou la remplacer par un enregistrement `.ogg`, sans toucher au code.

## Où sont les sons

```
assets/packs/strata/
  sounds/
    step_stone.ron       # une recette par son
    break_glass.ron
    ...
  sound_sets/
    stone.ron            # ce qu'un matériau joue : pas, casse, pose…
    water.ron
```

Un bloc choisit son matériau avec `sound: "strata:stone"` (fichier `blocks/<bloc>.ron`). Le matériau (`sound_sets/<nom>.ron`) nomme ses sons, tous facultatifs :

```ron
SoundSet(
    step: "strata:step_stone",       // marcher dessus
    breaking: "strata:break_stone",  // le casser
    place: "strata:place_stone",     // le poser
    enter: "strata:water_splash",    // tomber dedans (liquides)
    swim: "strata:swim",             // nager dedans (liquides)
    ambient: "strata:lava_bubble",   // le bloc fait du bruit tout seul (lave, feu)
    ambient_chance: 0.05,            // chance quand le bloc est tiré au sort autour du joueur
)
```

Les blocs voisins du joueur sont tirés au sort quelques centaines de fois par seconde ; au plus 4 sons d'ambiance de blocs démarrent par seconde.

## Une recette

```ron
// Pas sur la pierre : un claquement sec sur un court frottement.
Sound(
    category: Blocks,                 // Blocks, Ambient ou Music : curseur de volume
    subtitle: "subtitle-footsteps",   // clé de traduction du sous-titre
    volume: 0.3,                      // crête du son, 0 à 1
    pitch: (0.9, 1.1),                // vitesse de lecture tirée à chaque lecture
    variants: 6,                      // versions différentes (1 à 16)
    duration: 0.14,                   // secondes (au plus 10)
    range: 16.0,                      // distance d'écoute en blocs (1 à 256)
    layers: [
        Noise(color: White, filter: BandPass((hz: (2400, 3400), q: 0.8)),
              envelope: (attack: 0.001, decay: (0.025, 0.045))),
        Modes(modes: [(hz: (90, 130), decay: (0.03, 0.05))], gain: 0.4),
    ],
)
```

Un couple `(a, b)` est une **plage** : chaque variante tire sa valeur entre les deux. Les variantes sont fabriquées la première fois qu'elles sont jouées, puis gardées en mémoire ; la même recette donne toujours les mêmes variantes. Une lecture ne rejoue jamais la variante précédente.

Les couches sont additionnées, puis le tout est ramené à la crête `volume` et adouci à la fin.

| Couche | Pour | Champs |
|---|---|---|
| `Noise` | frottements, souffles, impacts | `color` (`White`, `Pink`, `Brown`), `filter`, `envelope`, `gain`, `delay`, `wobble` |
| `Grains` | gravier, neige, feuilles, crépitement | `count`, `spread` (s), `length` (s), `color`, `filter`, `gain`, `fade` (volume des derniers grains), `delay` |
| `Modes` | bois, pierre, verre, métal frappés | `modes: [(hz, decay, gain)]`, `gain`, `delay` |
| `Bubbles` | eau, lave | `count`, `spread`, `hz`, `rise` (montée de la hauteur), `decay`, `gain`, `delay` |

- **Filtres** : `None`, `LowPass((hz, q, sweep))`, `HighPass(…)`, `BandPass(…)` ; `sweep` est le rapport entre la fréquence de fin et celle du début (0,5 : descend d'une octave).
- **Enveloppe** : `attack` (s), `hold` (plage, s), `decay` (plage : secondes pour baisser de 60 dB).
- **`wobble: (rate: 0.25, depth: 0.7)`** : rafales lentes du volume et du filtre (vent).

Les valeurs hors limites sont ramenées dans les limites et signalées dans le journal au chargement (fichier, champ, limites) ; le son reste utilisable.

## Sons en boucle

`looped: true` fait d'une recette un **fond sonore en boucle** (ambiances) : la dernière seconde est fondue dans le début, la boucle ne s'entend pas. Une telle recette n'a pas de sous-titre et n'utilise que sa première variante ; ses couches doivent durer tout le son (`envelope: (hold: (10, 10), decay: (1, 1))`).

## Ambiances

Une ambiance (`ambiences/<nom>.ron`) dit ce qu'on entend dans un groupe de biomes, ou sous terre :

```ron
// Plaines : vent qui forcit en altitude, une rafale de temps en temps.
Ambience(
    biomes: ["strata:plains", "strata:flower_meadow"],   // ou `underground: true`
    beds: [   // fonds en boucle (recettes `looped: true`), 8 au plus
        (sound: "strata:ambient_wind", volume: 0.5,
         altitude: (from: 130, to: 220, gain: (0.7, 1.4)),  // facultatif
         time: Any),                                        // Any, Day ou Night
    ],
    events: [ // sons ponctuels (sous-titrés), 8 au plus
        (sound: "strata:wind", every: (25.0, 60.0), distance: (8.0, 16.0), time: Any),
    ],
)
```

- Un biome a **une** ambiance ; un biome sans ambiance est silencieux. `underground: true` remplace celle du biome sous terre.
- Le jeu regarde 25 points autour de l'auditeur (tous les 8 blocs, jusqu'à 16) : chaque ambiance pèse la part des points dans ses biomes (on entend la mer depuis la plage). **Sous terre** : peu de lumière du ciel à la tête (moins de 12, tout à fait sous 6) et du sol plein au-dessus (à partir de 3 blocs, tout à fait à 8) ; une maison n'est donc pas « sous terre ».
- `volume` × part × moment de la journée (`Day` suit la lumière du jour, `Night` l'inverse) × `altitude` (gain selon la hauteur de l'auditeur, linéaire entre `from` et `to`) ; deux fonds identiques s'additionnent (2 au plus).
- Les fonds changent de volume d'au plus 1 en **3 secondes** : passer d'un biome à l'autre ou entrer dans une grotte fait un fondu enchaîné.
- `every` : secondes entre deux sons (compte à rebours ralenti si l'ambiance ne pèse qu'une part, ou selon `time`) ; le son part d'un point au hasard à `distance` blocs, un peu au-dessus ou au-dessous.

## Audio spatial

Réglage **Audio spatial** (onglet Son) : *avancé* ou *stéréo simple* (préréglages DS et très bas).

- **Réverbération** : sept rayons par image (26 directions, 48 blocs au plus) mesurent l'espace autour de la tête. Plein air (ciel à 15, ou rayons vers le haut qui s'échappent sous un peu de ciel), sinon distance moyenne aux parois : **grande caverne** au-delà de 9 blocs, **petite grotte** si l'on est sous terre, **intérieur** sinon. Chaque lieu a son écho (aucun en plein air).
- **Étouffement** : entre un son et l'auditeur, chaque bloc traversé compte selon sa matière (bloc plein 1, verre et blocs non pleins 0,6, eau et lave 0,4, feuilles 0,2, plantes et air 0) ; le son perd des aigus et du volume (sourd et au tiers du volume derrière 3 blocs pleins). Recalculé 10 fois par seconde pour les sons en cours.
- En stéréo simple, rien de tout cela n'est calculé.

## Écouter ses recettes

```
strata --render-sounds sons/
```

écrit chaque variante de chaque son en WAV (`sons/<namespace>/<nom>-<n>.wav`). Avec `--resource-pack <nom>`, les recettes du pack sont prises en compte. `ffmpeg -i son.wav -lavfi showspectrumpic=s=800x400 spectre.png` en montre le spectre.

## Remplacer un son par un enregistrement

Déposer `sounds/<namespace>/<nom>.ogg` dans un pack de ressources :

```
resourcepacks/
  mes-sons/
    pack.ron
    sounds/
      strata/
        step_stone.ogg     # remplace la recette strata:step_stone
```

- Format **Ogg Vorbis**, mono ou stéréo (mélangé en mono : le jeu place lui-même le son dans l'espace), n'importe quelle fréquence (convertie en 44,1 kHz). 60 secondes au plus sont gardées.
- La catégorie, le sous-titre, la plage de `pitch` et la distance restent ceux de la recette.
- Un `.ogg` sans recette du même nom est signalé et ignoré. Un fichier illisible est signalé et la recette est jouée à sa place.
- Le pack le plus prioritaire gagne : un pack placé au-dessus qui redonne une recette `.ron` reprend la main sur un `.ogg` plus bas.

## Réglages

Onglet **Son** des options : volume général, blocs, ambiance, musique (le curseur suit une courbe : 50 % sonne environ deux fois moins fort), sous-titres, audio spatial. Les préréglages ne touchent qu'à l'audio spatial (stéréo simple pour DS et très bas) ; le changer à la main passe en « personnalisé ». Sans carte son, le jeu tourne en silence (une ligne d'information dans le journal).

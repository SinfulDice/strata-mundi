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

Onglet **Son** des options : volume général, blocs, ambiance, musique (le curseur suit une courbe : 50 % sonne environ deux fois moins fort), sous-titres. Les préréglages graphiques n'y touchent pas. Sans carte son, le jeu tourne en silence (une ligne d'information dans le journal).

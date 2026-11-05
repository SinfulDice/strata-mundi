## Principe

Sur Terra, une journée dure **20 minutes** : environ 10 minutes de jour, 10 de nuit, avec un court crépuscule à chaque lever et coucher du soleil. Le **soleil** se lève à l'est, traverse le ciel en s'inclinant un peu vers le sud, puis laisse la place à la **lune**. La part du jour et la lumière qui reste la nuit viennent du pack de la dimension : un pack peut donc les changer.

## À voir

- **Le ciel** passe du bleu du jour à l'orangé du crépuscule, puis au bleu sombre de la nuit. Le brouillard et l'horizon suivent.
- **Le soleil et la lune** sont de gros pixels bien nets. La lune a **8 phases**, une par jour, de la pleine lune à la nouvelle lune, puis retour.
- **Les étoiles** scintillent la nuit et s'effacent près de l'horizon et à l'aube.
- **Les nuages** dérivent doucement, toujours dans le même sens, très haut : au-dessus des collines, sous les grands pics. Leur couleur suit l'heure (blancs le jour, rosés au crépuscule, gris sombre la nuit). Le vent ne les fait avancer que tant que le temps passe : en pause, ils s'arrêtent.
- **La lumière du jour** baisse la nuit sans jamais tomber à zéro : on y voit encore un peu à la lueur du ciel. Les lumières posées (torches, lanternes, pierre lumineuse) font la différence.

## Régler l'heure

- `/time` lit ou règle l'heure du monde ; `/daycycle` arrête ou relance son cours. Ces deux commandes se tapent en mode créatif.
- À la création d'un monde, le bouton **Cycle jour/nuit** choisit d'avoir un temps qui passe ou une heure figée. Le choix est enregistré avec le monde.
- L'heure est enregistrée avec la sauvegarde. Elle ne recule jamais : les phases de la lune continuent de compter les jours.
- Le temps est gelé quand le jeu est en pause.

## Réglages

Deux réglages graphiques concernent le ciel (onglet Graphismes) :
- **Étoiles** : activées ou non. Coût presque nul. Coupées dans le préréglage « Très bas ».
- **Nuages** : **désactivés**, **simples** (une couche plate, très légère) ou **complets** (avec du volume, un peu plus de travail pour la carte graphique). Les préréglages « Très bas » les coupent, « Bas » et « DS » les gardent simples, les autres les veulent complets.

Les changements s'appliquent tout de suite. Le pack de la dimension règle aussi la durée du jour, la quantité de nuages et leur vitesse.

## Conseils

- La nuit venue, un tour de `/time set day` (en créatif) évite d'attendre le matin.
- Pour construire sans que le soleil bouge : `/time set noon` puis `/daycycle off`.
- Une nuit de pleine lune est la plus claire à regarder ; comptez huit jours d'une pleine lune à l'autre.

## Découverte en jeu

Le ciel se découvre simplement en jouant : le soleil, la lune et les nuages sont là dès le premier jour. Les commandes sont expliquées par `/help time` et `/help daycycle`, et les réglages par leurs phrases d'aide dans le menu d'options. [LORE À ÉCRIRE] : ce que les gens de Terra disent du soleil et de la lune.

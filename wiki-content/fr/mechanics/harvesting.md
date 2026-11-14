## Principe

En créatif, un clic casse un bloc. En **survie**, il faut **maintenir** le bouton d'attaque (clic gauche, ou la gâchette droite à la manette) sur le bloc : des **fissures** apparaissent et grandissent en dix étapes, accompagnées de petits coups et d'éclats, jusqu'à ce que le bloc cède. Regarder ailleurs, lâcher le bouton ou changer d'objet en main arrête la casse et efface les fissures.

## Durée

Le temps dépend de la **dureté** du bloc et de l'**outil** tenu : chaque bloc a un type d'outil préféré (pioche, hache, pelle, cisailles). À mains nues, un bloc prend sa dureté × 1,5 seconde (la terre : 0,75 s) ; le bon outil va plus vite selon sa vitesse. La page de chaque bloc donne sa dureté, son outil et son temps à mains nues.

## Outil requis

Certains blocs (la pierre, les minerais, l'obsidienne…) **exigent** un outil : sans lui, ou avec un outil de **niveau** trop faible, la casse est très lente (dureté × 5 secondes) et **ne donne rien**. L'infobulle de l'objet le dit (« Outil requis : pioche de niveau 2 ou plus »). Niveaux : 0 bois, 1 pierre, 2 fer, 3 diamant.

## Butin

Un bloc cassé en survie laisse son butin au sol. Par défaut, c'est le bloc lui-même ; certains donnent autre chose, décrit par une **table de butin** du pack : l'herbe donne de la terre, les feuilles et les hautes herbes ne se gardent qu'avec des cisailles, les blocs de champignon donnent parfois des champignons, le verre et la glace se brisent. Les tirages sont fixés par la graine du monde.

## Pour les créateurs de packs

Champs de bloc `tool: Some(BlockTool(kind: "strata:pickaxe", level: 1, required: true))` et `loot: Some("strata:ma_table")` ; tables dans `loot_tables/` (réserves, entrées pondérées, quantités, conditions `Tool`, `Not`, `Chance`) ; champ d'objet `tool: Some(Tool(kinds: ["strata:pickaxe"], level: 2, speed: 6.0))`.

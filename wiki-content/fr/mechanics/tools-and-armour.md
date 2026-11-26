## Principe

Les outils accélèrent la casse des blocs de leur type et permettent de récolter ceux qui les exigent. Chaque outil appartient à un **tier** de matériau : **bois, pierre, fer, or, diamant** (d'autres viendront après le diamant). Le tier donne le **niveau de minage** (0 bois, 1 pierre, 2 fer et or, 3 diamant), la **vitesse**, le nombre d'**utilisations**, les **dégâts** ajoutés aux armes et les **emplacements d'amélioration**. Il faut une pioche en fer pour le diamant, une pioche en diamant pour l'obsidienne. L'or est très rapide mais s'use vite. La page de chaque objet donne ses chiffres.

## Outils

- **Pioche, hache, pelle** : pierre et minerais, bois, terre et sable.
- **Houe** : un clic droit sur de la terre ou de l'herbe à l'air libre la laboure en terre labourée.
- **Épée** : garde ses dégâts et sa cadence pour le combat, qui arrive plus tard.
- **Cisailles** : gardent les feuilles, les lianes et les hautes herbes.
- **Faucille** : couper une plante coupe aussi celles qui l'entourent, jusqu'à deux blocs, pour une seule utilisation.
- **Burin** : un clic droit sur des briques de pierre les sculpte ; chaque coup passe au motif suivant, puis revient aux briques unies.
- **Outil multi-usage** : pioche, hache et pelle à la fois, au niveau du fer, mais moins durable.

Les outils en bois et en pierre sont connus dès le départ.

## Armures

Quatre cases : tête, torse, jambes, pieds. Trois **poids** :

- **légère** (cuir, tissu) : protège peu, ne gêne pas ;
- **intermédiaire** (mailles, écailles) : protège mieux et retire un peu d'endurance (2,5 % par pièce) et de mana ;
- **lourde** (plates) : protège le plus, mais chaque pièce **ralentit la marche** de 2,5 % (10 % pour l'ensemble) et retire davantage d'endurance (5 % par pièce) et de mana.

Le fer et l'or existent en mailles et en plates, le diamant en écailles et en plates. Les mailles se font avec des anneaux (un lingot en donne deux), les écailles de diamant avec un diamant (deux écailles). La **protection** totale s'affiche dans l'inventaire.

## Durabilité

Chaque utilisation use l'outil ; la barre sous la case montre ce qui reste et l'infobulle le nombre exact. À la création du monde, l'option « Objets usés » choisit : l'objet **se casse** à zéro, ou il est **gardé jusqu'à réparation**, grisé et inutilisable (un outil cassé vaut une main nue, une armure cassée ne protège plus).

## Matériaux provisoires

Les hautes herbes et les fougères coupées sans cisailles laissent parfois une **fibre végétale**. Au **métier à tisser**, trois fibres font un **tissu** (et quatre, pour l'instant, une **laine**) ; à la **tannerie**, deux fibres et une boue font, pour l'instant, un **cuir**, en attendant les peaux des animaux.

Chaque matériau a sa station : les pièces de cuir à la tannerie, celles de tissu au métier à tisser, celles de métal et de diamant à la **forge** (voir [Stations de travail](stations.html)).

## Pour les créateurs de packs

Dossiers `tiers/` (`Tier(level, speed, durability, damage, enchantability, upgrade_slots, armor_durability)`), `upgrade_slots/` (`UpgradeSlot(max_per_item)`) et `armor_weights/` (`ArmorWeight(speed, stamina, mana)`, parts par pièce) ; champs d'objet `tier: "strata:iron"`, `tool: Some(Tool(kinds: [...], level, speed, sweep))` (le tier remplit ce qui manque), `weapon: Some(Weapon(damage, attack_speed))`, `armor: Some(Armor(protection, weight))`, `upgrade_slots`, `action: Some(Work)` ; champ de bloc `worked: {"strata:hoe": Worked(into: "strata:farmland", open_above: true)}`.

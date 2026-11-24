## Principe

Les **coffres** et les **tonneaux** existent en quatre tiers : bois (27 cases), fer (45), or (63) et diamant (81). À partir du fer, chaque tier ajoute des **emplacements d'amélioration** (1 à 3), visibles dans le panneau à droite de l'écran. Quand le rangement dépasse cinq rangées, la grille défile (boutons du panneau de droite, molette).

## Monter de tier sans vider

Un **kit de rangement** utilisé (clic droit) sur un coffre ou un tonneau le fait passer au tier suivant **sur place** : contenu et améliorations sont gardés. Un coffre double monte d'un coup ses deux moitiés : il faut **deux kits** en main, pour qu'il garde un seul tier. On peut aussi fabriquer directement le tier supérieur autour d'un coffre (huit lingots), mais il faut alors le vider.

## Améliorations

- **Tri automatique** : le rangement se trie par type quand son écran se ferme, ou quand il reçoit des objets tout seul.
- **Filtre** : neuf cases où poser un objet tenu ; en mode « accepte la liste », seuls ces objets entrent par Maj-clic, dépôt rapide et aimant ; en mode « refuse la liste », tout sauf eux. Un filtre vide laisse tout passer. À la main, on peut toujours tout poser.
- **Compression** : neuf pépites deviennent un lingot, neuf lingots un bloc, dès qu'il y a la place.
- **Destruction** : ce qui entre de soi-même (Maj-clic, dépôt rapide, aimant) et que son filtre désigne est détruit ; ce qui était déjà rangé ou posé à la main reste. Un filtre vide ne détruit rien.
- **Aimant** : attire les objets au sol à 5 blocs, deux fois par seconde (option de monde « Aimants » pour le couper).

Une même amélioration ne se pose qu'une fois par rangement ; dans un coffre double, celles des deux moitiés comptent pour tout le coffre. Les améliorations d'automatisation (entonnoir, tapis) arriveront avec les machines.

## Sacs à dos

Le **sac à dos** se porte sur le dos (case « Dos » de l'inventaire, à côté de la main secondaire) ou se garde dans l'inventaire. Il s'ouvre avec sa touche (B par défaut, croix haut à la manette) quand il est porté, ou d'un clic droit en main. Accroupi + clic droit, il **se pose** comme un bloc ; cassé, il redevient le sac avec **tout ce qu'il contient**, jamais éparpillé, et il ne brûle pas.

Quatre tiers : cuir (27 cases, 1 emplacement d'amélioration), fer (45, 2), or (63, 3), diamant (81, 4). Un **kit de sac à dos** monte un sac **posé** d'un tier, sans le vider. Aucune recette ne consomme un sac : son contenu ne se perd jamais.

Ses améliorations sont celles des coffres (tri, filtre, compression, destruction, aimant : il attire alors les objets vers vous) et les siennes :

- **Ramassage** : ce que vous ramassez entre d'abord dans le sac, s'il passe son filtre (et sa destruction s'applique).
- **Nourrissage** : vous nourrira de ce qu'il contient quand vous aurez faim (la faim arrive avec les jauges de survie).
- **Établi intégré** : une grille 3×3 dans l'écran du sac.
- **Réservoir** : garde jusqu'à 8 seaux d'un liquide ; avec un seau plein en main, le bouton « Seau tenu » le vide dedans, avec un seau vide il le remplit.

**Pas de sac dans un sac** : aucun sac ni bourse n'entre dans un sac, ni à la main, ni par Maj-clic, ni au ramassage.

## Bourses et sacoches

Petites (9 cases, sans amélioration), elles se remplissent **toutes seules au ramassage** des objets de leur sorte, avant l'inventaire : bourse à graines (fleurs et champignons, puis graines), sacoche à minerais (minerais, charbon, diamants, pépites), bourse à munitions et sacoche à potions (pour les munitions et potions à venir). Un clic droit en main les ouvre.

## Pour les créateurs de packs

Dossiers `storage_tiers/` (`StorageTier(rows, upgrade_slots, upgrade_family)`) et `compression/` (`Compression(from, to, count: 9)`) ; champ de bloc `container: Some(Container(joins, upgradable: Some(Upgradable(family, tier))))` ; kit `kit: Some(Kit(on: Storage, from, to))` ; améliorations `upgrades/` de l'emplacement `strata:storage` avec les briques `Sort`, `Filter`, `Compress`, `Void`, `Magnet(radius, period)`, portées par un objet `upgrade: Some(...)`. Sacs : champ d'objet `bag: Some(Bag(placed: Some(bloc)))` (cases et emplacements du bloc posé, famille `strata:backpack`) ou `bag: Some(Bag(slots: 9, accepts: Some(tag)))` (bourse) ; `equip: Some(Back)` pour le porter ; briques `Pickup`, `Feed`, `Crafting`, `Tank(buckets)`.

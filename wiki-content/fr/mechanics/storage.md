## Principe

Les **coffres** et les **tonneaux** existent en quatre tiers : bois (27 cases), fer (45), or (63) et diamant (81). À partir du fer, chaque tier ajoute des **emplacements d'amélioration** (1 à 3), visibles dans le panneau à droite de l'écran. Quand le rangement dépasse six rangées, la grille défile (boutons du panneau de droite, molette).

## Monter de tier sans vider

Un **kit de rangement** utilisé (clic droit) sur un coffre ou un tonneau le fait passer au tier suivant **sur place** : contenu et améliorations sont gardés. Un coffre double monte d'un coup ses deux moitiés : il faut **deux kits** en main, pour qu'il garde un seul tier. On peut aussi fabriquer directement le tier supérieur autour d'un coffre (huit lingots), mais il faut alors le vider.

## Améliorations

- **Tri automatique** : le rangement se trie par type quand son écran se ferme, ou quand il reçoit des objets tout seul.
- **Filtre** : neuf cases où poser un objet tenu ; en mode « accepte la liste », seuls ces objets entrent par Maj-clic, dépôt rapide et aimant ; en mode « refuse la liste », tout sauf eux. Un filtre vide laisse tout passer. À la main, on peut toujours tout poser.
- **Compression** : neuf pépites deviennent un lingot, neuf lingots un bloc, dès qu'il y a la place.
- **Destruction** : ce qui entre et que son filtre désigne est détruit. Un filtre vide ne détruit rien.
- **Aimant** : attire les objets au sol à 5 blocs, deux fois par seconde (option de monde « Aimants » pour le couper).

Une même amélioration ne se pose qu'une fois par rangement ; dans un coffre double, celles des deux moitiés comptent pour tout le coffre. Les améliorations d'automatisation (entonnoir, tapis) arriveront avec les machines.

## Pour les créateurs de packs

Dossiers `storage_tiers/` (`StorageTier(rows, upgrade_slots, upgrade_family)`) et `compression/` (`Compression(from, to, count: 9)`) ; champ de bloc `container: Some(Container(joins, upgradable: Some(Upgradable(family, tier))))` ; kit `kit: Some(Kit(on: Storage, from, to))` ; améliorations `upgrades/` de l'emplacement `strata:storage` avec les briques `Sort`, `Filter`, `Compress`, `Void`, `Magnet(radius, period)`, portées par un objet `upgrade: Some(...)`.

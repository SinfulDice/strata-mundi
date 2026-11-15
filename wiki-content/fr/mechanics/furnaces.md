## Principe

Un **four** cuit l'objet posé en haut grâce au combustible posé en bas, et le résultat apparaît à droite. La flamme montre le combustible qui reste, la flèche la cuisson en cours. Le four continue à travailler écran fermé, et sa façade s'illumine tant que le feu brûle. Le **haut fourneau** ne fond que les minerais et le **fumoir** que la nourriture, deux fois plus vite.

## Combustibles

Charbon et charbon de bois : 8 cuissons dans un four en pierre ; seau de lave : 100 (le seau revient) ; bois, planches, établi, coffre : une cuisson et demie ; bâton : une demi. Le feu ne prend que s'il y a quelque chose à cuire et de la place à la sortie.

## Fours améliorables

Un **kit** utilisé (clic droit) sur un four le fait passer au tier suivant **sur place** : pierre → fer → or → diamant. Contenu, combustible et cuisson en cours sont gardés. Chaque tier cuit plus vite (10, 8, 5 puis 3 s) et tire plus d'un charbon (8, 10, 12, 16 cuissons), et gagne des emplacements d'amélioration (0 à 3) pour des **modules** : vitesse, économie de combustible, double sortie. Deux modules identiques ne s'additionnent pas. Le même kit sert au fumoir et au haut fourneau.

## Tailleur de pierre

Posez un bloc dans sa case, choisissez une variante dans la liste (briques de pierre, pierre polie, grès taillé…) et prenez-la. Pas de combustible, pas de perte : un bloc donne un bloc.

## Pour les créateurs de packs

Dossiers `cooking/` (`Cooking(ingredient, result, time, experience, kind)`), `fuels/` (`Fuel(ingredient, burn_time)`), `stonecutting/` (`Cutting(ingredient, result)`), `furnace_tiers/` (`FurnaceTier(cook_time, fuel_value, upgrade_slots)`) ; champs de bloc `furnace: Some(Furnace(family, tier, kinds, speed))` et `placement: Facing(lit: Some(Lit(front, light_emission)))` ; champs d'objet `kit: Some(Kit(from, to))` et `module: Some(Furnace(cook_time, fuel_value, output))`.

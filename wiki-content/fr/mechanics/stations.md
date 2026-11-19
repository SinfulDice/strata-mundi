## Une station par matériau

Chaque matériau se travaille à sa **station de travail** : un bloc qu'on pose, puis qu'on ouvre d'un clic droit.

- **Établi** : la grille 3×3 de fabrication (outils, armes, blocs, stations).
- **Fours** (four, fumoir, haut fourneau) : la cuisson et la fonte des minerais en lingots, avec du combustible.
- **Tailleur de pierre** : les variantes de pierre, une à une.
- **Forge** : 4 minerais et 1 charbon y font 5 lingots, tout de suite ; les lingots deviennent anneaux de mailles, plates d'armure et bottes de mobilité ; chaque pièce de métal brûle un charbon. Le diamant s'y taille en écailles.
- **Tannerie** : le cuir et les pièces d'armure en cuir.
- **Métier à tisser** : le tissu, la laine, les vêtements de tissu et la cape de laine.
- **Joaillier** : anneaux, amulettes et ceintures.
- **Enclume** : améliorations et réparations.

## Les ateliers à liste

La forge, la tannerie, le métier à tisser et le joaillier marchent de la même façon : une **liste** de ce qu'ils savent faire, **quatre cases** et le **résultat**. Choisissez un objet : ses ingrédients passent de l'inventaire aux cases ; l'infobulle dit ce qu'il demande et s'il vous manque quelque chose. On peut aussi poser les ingrédients à la main. Chaque atelier a son écran : sa disposition, sa couleur et son aide. À la manette, la croix passe d'une case à l'autre ; les flèches « < » « > » font défiler une longue liste.

## Pour les créateurs de packs

Un bloc `station: Some(Bench("monpack:atelier"))`, des recettes `bench_recipes/<nom>.ron` (`BenchRecipe(bench, ingredients: [(Item(..), n)…], result)`, une à quatre sortes d'ingrédients) et, si l'on veut, l'allure de son écran dans `benches/<nom>.ron` : `Bench(layout: "forge", accent: "#7a3b1e")` (la disposition nomme un groupe `bench_layout_<nom>` de l'écran d'inventaire : `plain`, `jeweller`, `forge`, `tannery`, `loom` ; inconnue, l'établi prend `plain` et une erreur le signale). L'aide de l'écran est la traduction `bench-<espace>-<nom>`.

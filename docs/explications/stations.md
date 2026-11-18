# Stations de travail

## À quoi ça sert

Chaque matériau se travaille à **sa** station : le bois et les outils à l'établi, la pierre au tailleur, les minerais au four, le métal à la **forge**, le cuir à la **tannerie**, le tissu et la laine au **métier à tisser**, les bijoux au **joaillier**, les améliorations à l'**enclume**. Le joueur comprend où aller en regardant la station ; le monde se remplit d'ateliers qu'on a envie de construire.

| Station | Genre (`StationDef`) | Ce qu'on y fait |
|---|---|---|
| Établi | `Crafting(3)` | grille 3×3 (outils, armes, blocs, stations) |
| Fours | entité de bloc `furnace` | cuisson et fonte, avec combustible |
| Tailleur de pierre | `Stonecutter` | variantes de pierre |
| Enclume | `Anvil` | améliorations, réparations |
| Forge | `Bench("strata:forge")` | anneaux, écailles, armures de métal et de diamant, bottes de mobilité |
| Tannerie | `Bench("strata:tannery")` | cuir (provisoire), armure de cuir |
| Métier à tisser | `Bench("strata:loom")` | tissu, laine (provisoire), vêtements de tissu, cape de laine |
| Joaillier | `Bench("strata:jeweller")` | anneaux, amulettes, ceintures |

## Comment ça marche

### Un seul établi, quatre stations

La forge, la tannerie, le métier à tisser et le joaillier sont le **même** établi générique (D089) : quatre cases, une liste de recettes, un résultat. Seuls changent le bloc, l'identifiant et les recettes :

```ron
// blocks/forge.ron
Block(
    textures: TopBottomSides(top: "strata:forge_top", bottom: "strata:cobblestone", sides: "strata:forge_side"),
    station: Some(Bench("strata:forge")),
    …
)

// bench_recipes/iron_plate_chestplate.ron
BenchRecipe(
    bench: "strata:forge",
    ingredients: [(Item("strata:iron_ingot"), 8), (Tag("strata:coals"), 1)],
    result: Result(item: "strata:iron_plate_chestplate"),
)
```

Choisir une recette envoie `ChooseCut` au serveur, qui pose les ingrédients de l'inventaire dans les cases (`fill_bench`) ; prendre le résultat les consomme (`take_bench`). Le client ne décide rien.

### Le charbon de la forge

La forge « brûle » du charbon sans mécanique nouvelle : le charbon est **un ingrédient comme un autre** (le tag `strata:coals`, charbon ou charbon de bois), un par pièce de métal. Pas de jauge de chaleur ni d'entité de bloc : tout est dans les recettes, réglable par pack. Les écailles de diamant se taillent sans feu.

### Un écran par station

Chaque établi a son allure, décrite dans le pack :

```ron
// benches/forge.ron
Bench(layout: Forge, accent: "#7a3b1e")
```

- `layout` choisit une **disposition** parmi celles que l'écran d'inventaire connaît (`Plain`, `Jeweller`, `Forge`, `Tannery`, `Loom`). Dans `assets/ui/inventory.ron`, chacune est un groupe `bench_layout_<nom>` qui place autrement les mêmes éléments : cases (2×2, en ligne pour la tannerie comme des peaux sur un séchoir, en colonne pour le métier comme une chaîne de fils), liste, résultat, aide.
- `accent` colore les **bandes** (`Band`, un nouvel élément d'interface) : le bandeau du titre et la surface de travail sous les cases.
- L'aide sous la liste est la traduction `bench-<espace>-<nom>` (sinon l'aide générale).
- Sans fichier `benches/`, un établi prend la disposition `Plain` et les couleurs du thème : un pack peut ajouter un établi avec seulement un bloc et des recettes.

La liste montre 5 × 3 recettes dans toutes les dispositions ; au-delà (la forge en a 29), les boutons « < » « > » et la molette la font défiler par rangées. À la manette, la croix passe d'un élément à l'autre comme partout.

```mermaid
flowchart LR
    B[blocks/forge.ron : Bench id] --> S[écran d'inventaire]
    St[benches/forge.ron : layout, accent] --> S
    R[bench_recipes/ : bench = id] --> L[liste]
    S --> G[groupe bench_layout_forge]
    G --> L
    L -- ChooseCut --> Srv[serveur : fill_bench, take_bench]
```

### Les recettes des stations

À l'établi : forge = 3 lingots de fer sur un four entouré de 5 galets ; tannerie = bâtons en cadre autour d'une boue sur 3 planches ; métier à tisser = 3 bâtons sur 3 fibres sur 3 planches. La tannerie et le métier sont connus dès le départ (le cuir et le tissu sont les premières armures) ; la forge s'apprend.

Le cuir et la laine restent **provisoires** (fibres et boue, fibres seules) jusqu'aux animaux de la v0.4 : leurs recettes sont déjà à la bonne station, il suffira d'en changer les ingrédients pour des peaux et de la laine de mouton.

## Dans le code

| Quoi | Où |
|---|---|
| Recettes d'établi, liste, poser, prendre | `crates/core/src/crafting/bench.rs`, `bench_load.rs` |
| Allure d'un établi (`benches/`) | `crates/core/src/crafting/bench_style.rs` |
| Fenêtre côté serveur | `crates/server/src/game/bench.rs` |
| Écran : disposition, couleur, aide, défilement | `crates/client/src/screens/bench.rs`, `assets/ui/inventory.ron` |
| Élément `Band` | `crates/client/src/ui/def.rs`, `ui/widgets/mod.rs` |
| Tests | `crates/core/tests/stations.rs`, `crafting/bench_tests.rs`, `crates/client/src/screens/bench_tests.rs`, `crates/server/src/game/equipment_tests.rs` |
| Wiki (page « Stations de travail ») | `crates/wikigen/src/stations.rs` |
| Contenu | `assets/packs/strata/{blocks,benches,bench_recipes,recipes,textures}/` |

<details><summary>Côté Rust</summary>

- **Un `enum` pour les dispositions** : `BenchLayout` est lu par serde ; une valeur inconnue (`layout: Spinning`) devient une erreur de pack, au lieu d'un écran vide. `BenchLayout::ALL` permet au test de vérifier que l'écran a un groupe pour chacune.
- **`#[serde(default)]`** : `layout` et `accent` peuvent manquer ; `BenchStyle::default()` donne `Plain` et aucune couleur.
- **`Option::then` puis `flatten`** : `id.starts_with("bench_band").then(|| …accent).flatten()` ne cherche la couleur que pour les bandes.
- **`strip_prefix`** : `id.strip_prefix("bench_layout_")` dit à la fois si l'id est une disposition et laquelle.
- **`saturating_add_signed`** : faire défiler de −1 au début reste à 0 au lieu de déborder.

</details>

## Pour aller plus loin

- Décisions D091 et D092 dans `docs/decisions.md` ; recettes et coûts dans `docs/regles/equilibrage.md` (« Stations de travail »).
- [Craft](craft.md), [Fours et tailleur de pierre](fours.md), [Accessoires et joaillier](accessoires-et-sets.md), [Améliorations et enclume](ameliorations.md).

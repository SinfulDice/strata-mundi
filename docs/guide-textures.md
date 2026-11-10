# Guide des textures et des packs de ressources

Toutes les images du jeu peuvent être remplacées, sans toucher au code, par un **pack de ressources**.

## Comment le jeu fabrique ses textures

Chaque texture a un **identifiant** égal à celui du bloc ou de l'objet qui l'utilise, par exemple `strata:stone`. Le jeu la cherche dans les packs, du moins prioritaire au plus prioritaire ; la dernière trouvée gagne :

1. le contenu du jeu : `assets/packs/<namespace>/textures/` ;
2. les packs de ressources activés : `resourcepacks/<nom>/textures/`.

Une texture peut être :
- une **image PNG** `stone.png` de **16×16, 32×32 ou 64×64** pixels (carrée) ;
- une **recette** `stone.ron`, dessinée par le jeu à partir de la **palette globale** (`palette.ron`).

Si un même dossier contient les deux, le PNG gagne. Les tailles peuvent être mélangées : le jeu agrandit toutes les textures à la plus grande taille présente (pixels nets, sans flou).

## Arborescence d'un pack de ressources

```
resourcepacks/
  mon-pack/
    pack.ron                  # (namespace: "mon_pack", description: "Mes textures")
    palette.ron               # facultatif : recolore tout le jeu
    textures/
      strata/                 # remplace des textures du jeu de base (namespace strata)
        stone.png
        oak_planks.png
```

- `namespace` : nom court en minuscules, chiffres et `_`.
- Les fichiers posés directement dans `textures/` appartiennent au namespace du pack ; pour **remplacer** une texture du jeu, place-la dans `textures/strata/`.
- Noms de fichiers : l'identifiant sans le namespace (`strata:oak_planks` → `oak_planks.png`), en minuscules.

## Pas à pas : remplacer la texture de la pierre

1. Crée le dossier `resourcepacks/ma-pierre/` à côté du jeu.
2. Crée `resourcepacks/ma-pierre/pack.ron` :
   ```ron
   (namespace: "ma_pierre", description: "Ma pierre à moi")
   ```
3. Dessine une image carrée de 16×16 (ou 32×32, 64×64) pixels dans ton logiciel préféré (Aseprite, LibreSprite, GIMP, Piskel…) et enregistre-la en PNG sous `resourcepacks/ma-pierre/textures/strata/stone.png`.
4. Lance le jeu avec le pack activé :
   ```sh
   strata --resource-pack ma-pierre
   ```
   (le menu d'activation et d'ordre des packs arrive avec les menus du jeu, jalon 7).
5. Pour vérifier d'un coup d'œil : `strata --resource-pack ma-pierre --texture-sheet planche.png` produit une planche de toutes les textures.

Si l'image a une mauvaise taille ou n'est pas un PNG valide, le jeu l'indique dans ses messages (fichier et raison) et affiche à la place un damier magenta et noir.

## Recolorer tout le jeu avec une palette

Toutes les textures dessinées par le jeu prennent leurs couleurs dans des **rampes** nommées (du plus sombre au plus clair) de `palette.ron`. Un pack de ressources peut en remplacer quelques-unes :

```ron
// resourcepacks/automne/palette.ron
{
    "grass": ["#6b5a1e", "#8a7426", "#a88e30", "#c4a83c"],
    "oak_leaves": ["#7a3a12", "#9a4a18", "#b85e20", "#d4782c"],
}
```

Les rampes existantes sont listées dans `assets/packs/strata/palette.ron`. Couleurs en `#rrggbb` ou `#rrggbbaa` (transparence).

## Recettes de textures (pour aller plus loin)

Un fichier `textures/<nom>.ron` décrit comment dessiner la texture :

| Recette | Effet | Exemple |
|---|---|---|
| `Material(ramp, pattern)` | Une matière entière | `Material(ramp: "stone", pattern: Noise(cells: (4, 4), octaves: 3))` |
| `Overlay(base, ramp, pattern)` | Un motif peint sur une autre texture | côté du bloc d'herbe : `Overlay(base: "strata:dirt", ramp: "grass", pattern: TopFringe(depth: 4))` |
| `Ore(base, ramp, clusters)` | Petits amas de minerai éclairés en haut à gauche | `Ore(base: "strata:stone", ramp: "gold", clusters: 4)` |
| `Sprite(colors, rows)` | Pixel art écrit en texte : 16 lignes de 16 caractères, `.` = transparent | voir `textures/wooden_pickaxe.ron` |
| `Recolor(base, swap)` | Le même dessin avec d'autres rampes | `Recolor(base: "strata:wooden_pickaxe", swap: {"oak_wood": "iron"})` |

Motifs (`pattern`) : `Noise(cells: (x, y), octaves)` (grain ; `(8, 2)` donne des stries verticales), `Rings(border)` (cernes d'une bûche), `Planks(boards)` (planches), `Leaves(holes)` (feuillage ajouré), `Stripes(period)` (côtes du cactus), `TopFringe(depth)` (frange en haut), `Frame` (cadre de verre).

Règles de style : lumière unique venant d'**en haut à gauche** ; contour des objets et des mobs dans un ton **plus foncé de leur propre couleur** ; couleurs prises dans la palette globale pour garder un ensemble harmonieux.

## Couleurs des biomes

L'herbe, les feuilles (sauf bouleau et sapin), les plantes et l'eau prennent la couleur du biome : le bloc porte `tint: (kind: Grass)` (ou `Foliage`, `Water`), et chaque biome donne dans `colors` un **facteur** par canal (rouge, vert, bleu) : `(1.0, 1.0, 1.0)` garde la couleur peinte, `(1.4, 1.05, 0.6)` la jaunit (savane), 2 au plus. Peins donc ces textures dans la couleur des plaines.

Pour le côté du bloc d'herbe (`faces: TopFringe`), seule la frange prend la couleur : les pixels qui doivent garder leur couleur (la terre) ont une transparence de **254** au lieu de 255 (invisible à l'œil). La recette `Overlay` le fait d'elle-même ; dans un PNG, mets l'alpha de ces pixels à 254.

## Textures vivantes : animées, connectées, variantes

Une texture peut avoir **plusieurs images**. Le bloc la nomme comme d'habitude (`textures: All("strata:water")`) ; le jeu se charge de choisir l'image.

### Textures animées

Les images défilent en boucle, chacune pendant `frame_time` **ticks** (1 tick = 1/20 s ; 2 par défaut, 255 au plus). La carte graphique choisit l'image selon le temps : rien n'est recalculé.

```ron
// textures/fire.ron : la flamme dessinée dans fire_base.ron, qui ondule sur 8 images
Animated(frames: Sway(base: "strata:fire_base", count: 8, amplitude: 1), frame_time: 2)
// textures/portal.ron : un grain violet qui glisse en diagonale, 16 images
Animated(frames: Scroll(base: "strata:portal_base", count: 16, step: (1, 1)), frame_time: 2)
// des images dessinées une à une
Animated(frames: List(["mon_pack:braise_1", "mon_pack:braise_2", "mon_pack:braise_3"]), frame_time: 4)
```

Façons de fabriquer les images (`frames`), 64 images au plus :

| Forme | Effet |
|---|---|
| `List([...])` | une image par texture citée (PNG ou recette) |
| `Reseed(recipe, count)` | la même recette dessinée `count` fois avec un autre tirage (grain différent) |
| `Scroll(base, count, step)` | `base` décalée de `step` pixels (à l'échelle 16 px) par image, en boucle (textures raccordables) |
| `Sway(base, count, amplitude)` | les lignes de `base` balancées de gauche à droite (flammes, eau) |

**En PNG**, comme Minecraft : une **bande verticale** d'images carrées (16×48 = 3 images de 16×16). Une bande qui remplace une texture animée garde son rythme (`frame_time`) ; sinon 2 ticks par image. Attention : une bande posée sur une texture **à variantes** (`stone.png` de 16×48) en fait une **animation**, pas trois variantes ; pour des variantes dessinées, écris une recette `Random` avec `List` (voir plus bas).

`List` n'accepte que des textures d'une seule image : citer une texture animée, connectée ou à variantes est une erreur signalée au chargement.

Le réglage graphique « Textures animées » fige toutes les animations (première image).

### Textures connectées

Les bordures disparaissent entre deux blocs identiques voisins. Le jeu fabrique toutes les images à partir d'une texture de base en remplaçant la bordure de chaque côté raccordé par la bande juste à l'intérieur (`border` pixels à l'échelle 16 px, 1 par défaut, 4 au plus).

```ron
// textures/glass.ron : les 4 côtés de chaque face (16 images)
Connected(base: "strata:glass_base", connect: Edges)
// textures/bookshelf.ron : seulement en haut et en bas des faces de côté (4 images)
Connected(base: "strata:bookshelf_base", connect: Vertical)
```

**Pour redessiner du verre ou une bibliothèque**, remplace la **base** (`glass_base.png`, `bookshelf_base.png`) : le jeu en tire toutes les images raccordées. Un `glass.png` remplacerait la recette entière et le verre perdrait ses raccords.

Un voisin ne se raccorde pas si sa propre face est cachée par un bloc identique devant elle : l'arête intérieure d'un mur en L garde sa bordure. Les coins entre deux côtés raccordés s'ouvrent aussi (pas de petit coin isolé quand seule la diagonale manque, comme les textures connectées simples). La texture de base peut être un PNG.

### Variantes aléatoires

Chaque bloc prend une variante selon un **hachage de sa position** : toujours la même au même endroit, pour tout le monde, sans rien sauvegarder.

```ron
// textures/stone.ron : trois tirages du grain de la pierre, aussi fréquents
Random(variants: Reseed(recipe: Material(ramp: "stone", pattern: Noise(cells: (4, 4), octaves: 3)), count: 3))
// des dessins choisis, le premier deux fois plus fréquent
Random(variants: List(["mon_pack:herbe_a", "mon_pack:herbe_b"]), weights: [2, 1])
```

**Pour redessiner la pierre en gardant des variantes**, un simple `stone.png` ne suffit pas : il remplace la recette et la pierre n'a plus qu'une image. Dessine plusieurs PNG à la racine de `textures/` (`stone_a.png`, `stone_b.png`…) et un `textures/strata/stone.ron` (sans `stone.png` à côté, qui gagnerait) qui les liste : `Random(variants: List(["mon_pack:stone_a", "mon_pack:stone_b"]))`.

`weights` (facultatif) : une valeur de 1 à 64 par variante. Les recettes qui partent d'une texture à variantes (`Ore(base: "strata:stone")`) utilisent sa première image.

Le réglage graphique « Variantes de textures » désactive variantes et raccords (les variantes ne coûtent pas de faces en plus : la carte graphique les choisit pixel par pixel ; sans raccords, les vitres voisines fusionnent).

### Limites

- Le tableau de textures de la carte graphique a **au moins 256 couches** sur toute machine Vulkan (2 048 sur la plupart) ; chaque image compte pour une couche. Le jeu de base en utilise environ 170.
- Une texture animée, connectée ou à variantes ne peut pas en contenir une autre (pas d'animation de variantes).

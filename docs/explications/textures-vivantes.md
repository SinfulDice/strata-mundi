# Textures vivantes : animées, aléatoires, connectées

## À quoi ça sert

Une texture peut avoir **plusieurs images** : des étapes d'animation (feu, eau, portail), des variantes pour casser la répétition (pierre, terre, herbe) ou des raccords avec les voisins (verre, bibliothèque). Le **format des packs** (recettes `Animated`, `Random`, `Connected`, PNG en bande verticale, bornes) est décrit dans le [guide des textures](../guide-textures.md) : cette page explique seulement comment le moteur s'en sert.

## Comment ça marche

### Une couche par texture, les autres images derrière

Le tableau de textures du GPU a une couche par image. Chaque texture nommée garde **une couche à elle** (sa première image, son `TextureId` ne change jamais : blocs, icônes, wiki n'ont rien à savoir). Les images supplémentaires sont rangées **après toutes les premières**. Un petit descripteur par texture (`TextureLayout`) dit combien d'images elle a, où commencent les extra et à quoi elles servent.

```
couches : [ pierre | terre | feu | ... | pierre#2 pierre#3 | feu#2 feu#3 ... ]
           premières images               images supplémentaires
```

### Trois usages, trois endroits

```mermaid
graph TD
    T[Texture à plusieurs images] --> A[Animated]
    T --> R[Random]
    T --> C[Connected]
    A -->|choix à chaque image| S[Shader]
    R -->|choix à chaque pixel| S
    C -->|choix à la fabrication du maillage| M[Mailleur]
```

- **Animée** : le shader choisit l'image. Il lit une table (un entier par couche : où sont les autres images, combien, durée de chacune) et calcule `(ticks / durée) % nombre_d'images`. Aucun remaillage, rien à renvoyer au GPU à chaque image. Le défilement de l'eau et de la lave (qui montre le sens du courant) est conservé, et l'ondulation par images (`Sway`) s'y ajoute.
- **Aléatoire** : le shader de fragments choisit selon un **hachage de la position du bloc** dans le monde (pas de la section) : un même bloc a la même variante partout et à chaque lancement. Les poids rendent certaines variantes plus rares. Pour retrouver son bloc, chaque pixel part de sa position dans la section (interpolée entre les coins), recule d'un demi-bloc le long de la normale de la face (une face est posée sur la limite entre deux blocs) et ajoute l'origine de la section, transmise à chaque dessin. Une table envoyée une fois au GPU donne, par couche, où sont les autres images et les **cumuls** des poids.
- **Connectée** : le mailleur regarde les voisins. Pour chaque côté de la face (haut, droite, bas, gauche) où un bloc identique est présent, il prend l'image dont la bordure a été **retirée** de ce côté : deux vitres côte à côte forment une seule vitre. `Edges` donne 16 images (4 côtés = 16 combinaisons), `Vertical` 4 (haut et bas, faces latérales seulement : bibliothèques). Un voisin dont la face est elle-même couverte par un bloc identique ne compte pas, d'où le coin intérieur d'un « L » qui garde son cadre. Les bûches couchées ne se raccordent pas.

### Variantes et fusion gloutonne

La fusion gloutonne (voir [Chunks et maillage](chunks-et-maillage.md)) ne regroupe que des faces de même clé. Les variantes aléatoires étant choisies par le shader, la clé garde la couche de base : un sol de pierre à trois variantes reste **un seul rectangle**, chaque pixel prenant ensuite la variante de son bloc (section de surface de test : 1 732 sommets avant, 1 164 après, en lumière plate). Les raccords, eux, dépendent des voisins : le mailleur met l'image choisie dans la clé, et seules des vitres de même image fusionnent. Deux réglages :

- `animated_textures` « Textures animées » : coupe l'animation (coût nul : c'est une valeur du shader) ;
- `texture_variants` « Variantes de textures » : coupe les variantes (valeur du shader) et les raccords (remaille le monde).

Les deux sont coupés au préréglage le plus bas. Les blocs sans raccord ne coûtent rien au mailleur (`MeshBlock.varied`).

### Fabrication des images

Au chargement, la bibliothèque dessine toutes les images avec le processeur (déterministe). `Reseed` redessine la même recette avec une autre graine ; l'image 0 garde la graine d'origine, donc les minerais posés sur la pierre ne changent pas. `Scroll` décale l'image en bouclant ; `Sway` pousse les lignes de côté selon une vague (flammes, eau). La carte graphique doit accepter assez de couches : le jeu demande la limite de l'adaptateur (256 garanties en Vulkan) et le jeu de base tient dedans (test).

## Où c'est dans le code

| Quoi | Où |
|---|---|
| Recettes d'images multiples (`Frames`, `Connection`, `Layout`, `connected`, `scrolled`, `swayed`, `split_strip`) | `crates/core/src/texture/multi.rs` |
| Bibliothèque, couches, `TextureLayout` (`frame_at`, `random_index`), `position_hash` | `crates/core/src/content/textures.rs` |
| Raccords au maillage | `crates/client/src/mesher/variants.rs` (`TextureVariants`, `connection_mask`) |
| Tables d'animation et de variantes envoyées au GPU | `crates/client/src/render/texture_array.rs` (`animation_table`, `variant_table`) |
| Animation et variantes dans le shader | `crates/client/src/render/shaders/block.wgsl` (`animated_layer`, `variant_layer`, `position_hash`) |
| Tests | `crates/client/src/mesher/variants_tests.rs`, `crates/core/tests/texture_layouts.rs` |
| Scènes de contrôle | `assets/scenes/textures.ron`, `assets/scenes/fire.ron` |

<details><summary>Côté Rust</summary>

- **`enum` avec données** : `Layout::Animated { frame_time }`, `Random { weights }`, `Connected(Connection)` disent à la fois le genre et ses paramètres ; le `match` oblige à traiter chaque genre.
- **Tampon de stockage** (*storage buffer*) : un tableau de données que le shader lit. Ici un `u32` par couche, rangé par décalages de bits : `première_extra:16 | images:8 | durée:8`. Le shader redécode avec `>>` et `&`.
- **Miroir CPU/GPU** : `TextureLayout::frame_at` fait en Rust le même calcul que `animated_layer` en WGSL, pour pouvoir le tester sans carte graphique.
- **`Box<TextureRecipe>`** : une recette qui en contient une autre doit passer par un pointeur (`Box`), sinon le type aurait une taille infinie. La validation interdit tout de même l'imbrication de recettes à images multiples.
- **`impl Fn(u32, u32) -> (u32, u32)`** : `shifted` reçoit une petite fonction qui dit « pour ce pixel, va chercher celui-là » ; `scrolled` et `swayed` ne diffèrent que par cette fonction.
- **`rem_euclid`** : modulo qui reste positif, nécessaire pour boucler un décalage négatif.

</details>

# Génération du monde

## À quoi ça sert

Le monde n'est pas stocké : il est **calculé** à partir de la graine (*seed*) au fur et à mesure que le joueur avance. Une même graine donne toujours exactement le même monde, quel que soit l'ordre dans lequel les morceaux sont générés.

## Comment ça marche

### Les cartes de climat

Quatre grandes « cartes » de bruit (des valeurs qui varient doucement d'un endroit à l'autre) couvrent le monde :

| Carte | Rôle |
|---|---|
| Température | froid (toundra) ↔ chaud (désert) |
| Humidité | sec (plaines, désert) ↔ humide (forêt) |
| Continentalité | océan profond ↔ côte ↔ intérieur des terres |
| Érosion | terrain érodé (plat) ↔ terrain jeune (montagnes) |

**La forme du sol** ne dépend que du climat (continentalité, érosion, « pics et vallées ») et de bruits de détail, jamais du biome ; elle est continue partout. **Le biome** choisit seulement ce qui recouvre le sol. Résultat : pas de falaise brutale à la frontière entre deux biomes, les transitions sont douces. Un peu de bruit fin « mélange » les blocs de surface le long des frontières.

- Mer à **y = 120**, côtes, plaines, collines, **plateaux en terrasses**, **rivières**, et des **montagnes** jusqu'à ~400 avec falaises et surplombs là où l'érosion est faible.
- Depuis la v0.2, le relief vient de **courbes réglables dans le pack** et d'une **densité 3D** autour de la surface : tout est expliqué dans [Relief](relief.md).
- Chaque biome est un point dans l'espace « température, humidité, continentalité, érosion » ; on prend le plus proche parmi ceux dont les conditions sont remplies (`min_height` / `max_height`, lit de rivière).

### Les biomes

Il y a **18 biomes** : plaines, prairie fleurie, forêt, forêt de bouleaux, taïga, toundra enneigée, désert, savane, jungle, marais, badlands, montagnes, pics enneigés, océan, océan profond, plage, rivière et champs de champignons. La liste détaillée, les conditions de hauteur et les strates sont dans [Biomes et végétation](biomes-et-vegetation.md).

Sur les pentes raides, la surface devient de la roche nue (voir [Relief](relief.md)).

Tout est décrit dans le pack : `biomes/*.ron` (climat, blocs, éléments, couleur sur la carte), `features/*.ron` (arbres, cactus, minerais), `dimensions/terra.ron` (niveau de la mer, roches, minerais, taille des biomes, courbes du relief).

### Une colonne, étape par étape

1. **Couches**, parcourues de haut en bas pour suivre les surplombs ([Relief](relief.md)) : socle irrégulier en bas (y 0 à 3), roche profonde sous ~64, pierre, quelques blocs de sous-sol (terre, grès…), le bloc de surface (sous chaque ouverture vers l'air) ; près de la mer, sable ou gravier (plages). Eau statique jusqu'au niveau de la mer.
2. **Grottes** : tunnels « spaghetti », nouilles, grandes cavernes, ravins, nappes d'eau souterraines et lave en profondeur, sans jamais ouvrir une mer ou une rivière. Détails dans [Grottes](grottes.md).
3. **Minerais** en petits filons, chacun dans sa plage de hauteur (charbon partout, diamant tout en bas) ; dans la roche profonde, c'est la variante « profonde ».
   **Grands gisements** (`Deposit`, liste `deposits` de `terra.ron`) : le monde est découpé en cellules de 256 blocs ; un hachage de chaque cellule décide si elle contient une veine géante (fer ou cuivre), son centre, sa longueur (40-80 blocs), son épaisseur (6-10) et son orientation. La veine est une lentille autour d'un segment, gonflée par un bruit 3D, faite de roche de remplissage (roche ferrugineuse, roche vert-de-gris) avec ~35 % de minerai ; elle ne remplace que la pierre et la roche profonde. Au-dessus de son empreinte, 5 % des blocs de surface à l'air libre deviennent cette roche : le signe à repérer. Chaque colonne regarde les cellules voisines, donc le résultat ne dépend pas de l'ordre de génération.
4. **Arbres, cactus et plantes** (formes d'arbres, touffes d'herbes et de fleurs : [Biomes et végétation](biomes-et-vegetation.md)) : chaque colonne place les éléments enracinés chez elle **et chez ses 8 voisines**, en n'écrivant que la partie qui tombe chez elle. Ainsi un arbre à cheval sur deux colonnes est toujours entier, dans n'importe quel ordre de génération. La hauteur du sol d'une voisine est recalculée exactement (`surface_height`, voir [Relief](relief.md)), surplombs compris.

Tout le hasard vient de générateurs **initialisés par la graine et la position** de la colonne : rien ne dépend de l'ordre de génération ni du thread utilisé.

### Point de départ

Le serveur cherche, en spirale depuis l'origine, un endroit en plaines ou en forêt (`spawn: true`), au sec, avec de l'eau à moins de ~100 blocs. Le village garanti à moins de 1 000 blocs arrivera en v0.7.

Mesures : ~2 ms par colonne (`cargo bench -p strata-worldgen`).

## Où c'est dans le code

| Quoi | Où |
|---|---|
| Graine | `crates/worldgen/src/lib.rs` (`Seed`) |
| Lecture des packs | `crates/worldgen/src/defs.rs`, `content.rs` (`WorldgenContent`) |
| Climat, hauteur plate, biome | `crates/worldgen/src/climate.rs` |
| Relief (courbes, densité 3D, carte) | voir [Relief](relief.md) |
| Colonne complète | `crates/worldgen/src/generator.rs` (`Generator::generate`) |
| Grottes | `crates/worldgen/src/caves/` (voir [Grottes](grottes.md)) |
| Minerais, arbres, cactus | `crates/worldgen/src/features.rs`, `trees.rs` |
| Plantes en touffes | `crates/worldgen/src/plants.rs` |
| Strates et surface | `crates/worldgen/src/surface.rs` |
| Grands gisements | `crates/worldgen/src/deposits/` |
| Point de départ | `crates/worldgen/src/spawn.rs` |
| Tests (déterminisme, biomes, départ) | `crates/worldgen/tests/generation.rs` |

<details><summary>Côté Rust</summary>

- **`fastnoise-lite`** fournit le bruit OpenSimplex2 en « fractale » (plusieurs couches de détails superposées). On l'utilise en `f64` pour rester précis à des millions de blocs de l'origine.
- **Fonctions pures** : `Generator::generate(&self, pos)` ne modifie rien dans le générateur ; plusieurs threads peuvent donc l'appeler en même temps sur le même générateur partagé par `Arc`.
- **SplitMix64** (`rng.rs`) : un tout petit générateur de nombres pseudo-aléatoires, reproductible partout.

</details>

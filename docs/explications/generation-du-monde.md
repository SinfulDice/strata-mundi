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
- Chaque biome est un point dans l'espace « température, humidité, continentalité, érosion » ; on prend le plus proche parmi ceux dont les conditions sont remplies (plage de hauteur, lit de rivière).

### Les biomes

| Biome | Surface | Particularités |
|---|---|---|
| Plaines | herbe | quelques chênes, lieu de départ possible |
| Forêt | herbe | chênes denses, lieu de départ possible |
| Désert | sable, grès dessous | cactus |
| Océan | sable, gravier au fond | eau jusqu'à y = 120 |
| Montagnes | herbe, puis pierre (≥ 200) et neige (≥ 265) | sapins |
| Toundra enneigée | herbe enneigée | eau gelée en surface, sapins |
| Rivière | herbe, sable au fond | seulement sur le lit des rivières (v0.2) |

Sur les pentes raides, la surface devient de la roche nue (voir [Relief](relief.md)).

Tout est décrit dans le pack : `biomes/*.ron` (climat, blocs, éléments, couleur sur la carte), `features/*.ron` (arbres, cactus, minerais), `dimensions/terra.ron` (niveau de la mer, roches, minerais, taille des biomes, courbes du relief).

### Une colonne, étape par étape

1. **Couches**, parcourues de haut en bas pour suivre les surplombs ([Relief](relief.md)) : socle irrégulier en bas (y 0 à 3), roche profonde sous ~64, pierre, quelques blocs de sous-sol (terre, grès…), le bloc de surface (sous chaque ouverture vers l'air) ; près de la mer, sable ou gravier (plages). Eau statique jusqu'au niveau de la mer.
2. **Grottes** : des tunnels creusés là où deux bruits 3D valent presque zéro en même temps (« grottes spaghetti »). Le bruit est calculé sur une grille grossière (tous les 4 blocs) puis interpolé, 60 fois moins cher. Les grottes avancées arrivent en v0.2.
3. **Minerais** en petits filons, chacun dans sa plage de hauteur (charbon partout, diamant tout en bas) ; dans la roche profonde, c'est la variante « profonde ».
4. **Arbres et cactus** : chaque colonne place les éléments enracinés chez elle **et chez ses 8 voisines**, en n'écrivant que la partie qui tombe chez elle. Ainsi un arbre à cheval sur deux colonnes est toujours entier, dans n'importe quel ordre de génération. La hauteur du sol d'une voisine est recalculée exactement (`surface_height`, voir [Relief](relief.md)), surplombs compris.

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
| Grottes | `crates/worldgen/src/caves.rs` |
| Minerais, arbres | `crates/worldgen/src/features.rs` |
| Point de départ | `crates/worldgen/src/spawn.rs` |
| Tests (déterminisme, biomes, départ) | `crates/worldgen/tests/generation.rs` |

<details><summary>Côté Rust</summary>

- **`fastnoise-lite`** fournit le bruit OpenSimplex2 en « fractale » (plusieurs couches de détails superposées). On l'utilise en `f64` pour rester précis à des millions de blocs de l'origine.
- **Fonctions pures** : `Generator::generate(&self, pos)` ne modifie rien dans le générateur ; plusieurs threads peuvent donc l'appeler en même temps sur le même générateur partagé par `Arc`.
- **SplitMix64** (`rng.rs`) : un tout petit générateur de nombres pseudo-aléatoires, reproductible partout.

</details>

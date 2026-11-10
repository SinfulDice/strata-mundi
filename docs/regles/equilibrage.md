# Contenu chiffré et équilibrage

Cibles fixées avec l'utilisateur. Toutes les valeurs de jeu vivent dans les packs (RON) : les ajuster ne demande jamais de toucher au code.

## Contenu visé à la v0.13 (jeu solo complet)

| Élément | Cible | Répartition indicative |
|---|---|---|
| Blocs | Pas d'objectif chiffré | Ce que demande le contenu ; familles de blocs générées automatiquement |
| Mobs (hors boss) | **~100** | v0.4 ~20, v0.6 +15 (bestiaire mythologique), v0.9 +10, v0.10 ~40 (≈10 par dimension), v0.11 +5, v0.12 +10 |
| Sorts | **~80** | 10 par école : ~7 par école en v0.9, ~3 sorts divins par école en v0.12 |
| Boss principaux | Ce que demande le contenu | Boss de structure, boss magiques, boss de dimension, Ianitor, épreuves divines, Arbiter |
| **Boss secondaires** | En plus des principaux | **Errants** (créatures uniques et rares qui parcourent un biome), **d'événement** (lune de sang, éclipse, raid), **de quête** (contrats de taverne), **gardiens de gisements** (protègent un grand gisement ou un trésor) |

## Durée et rythme

- **50-80 h jusqu'à Ianitor**, dont **~20 h dans Terra**.
- Progression **rapide au début, plus lente ensuite** : fer en ~1 h, diamant en ~5 h (joueur moyen), puis paliers plus longs (Ignium, Nihilium, Siderium, Divinium).
- La **voie mécanique** prend environ **1,5× plus de temps** que la voie magique pour atteindre un même palier, mais une usine terminée produit sans limite.

## Difficulté

- **Boss : exigeants mais pas trop.** On apprend leurs attaques ; un joueur bien préparé gagne en quelques essais.
- **Épreuves divines : 3 à 6 essais** pour un joueur moyen ; Arbiter le plus dur.
- Mobs à force fixe par espèce, bonus dans leur environnement naturel, pas de difficulté qui monte avec le temps.

## Ressources

- **Finies comme Minecraft** : un minerai miné ne revient pas ; on explore plus loin.
- **Non farmables** (aucune machine, aucune ferme ne peut les produire) : butin de boss et uniques nommés, matériaux magiques, cornes et fragments de licorne, sceaux, pièces de bâton. Tout le reste peut s'automatiser par la mécanique.

## Feu (v0.2, `blocks/fire.ron`, champ `fire`)

Valeurs de Minecraft (difficulté normale), toutes réglables dans le pack ; bornes dans `FireDef::clamped`.

| Champ | Défaut | Bornes | Effet |
|---|---|---|---|
| `tick_rate` | 30 | 1-10 000 | Ticks minimum entre deux pas d'un feu |
| `tick_jitter` | 10 | 1-1 000 | Le pas dure `tick_rate` + 0 à `tick_jitter − 1` ticks |
| `max_age` | 15 | 1-15 | Âge maximal : le feu ne se propage plus, et peut mourir |
| `bare_age` | 3 | 0-15 | Sans combustible voisin, meurt au-delà de cet âge |
| `burn_out_odds` | 4 | 1-10 000 | À l'âge maximal, 1 chance sur N de mourir (sauf sur un combustible) |
| `side_odds`, `vertical_odds` | 300, 250 | 1-100 000 | Un voisin prend avec `burn` chances sur N (côtés, dessus-dessous) |
| `rekindle_chance`, `rekindle_base` | 5, 10 | 0-10 000, 1-10 000 | Le voisin consumé devient un feu avec 5 chances sur (âge + 10) |
| `spread_bonus`, `spread_age_base` | 54, 30 | 0-10 000, 1-10 000 | L'air voisin s'allume avec `(catch + 54) / (âge + 30)` chances… |
| `spread_odds` | 100 | 1-100 000 | … sur 100, et 100 de plus par bloc au-delà d'un au-dessus |
| `spread_reach`, `spread_below`, `spread_above` | 1, 1, 4 | ≤ 3, ≤ 3, ≤ 8 | Zone d'air allumable : 3 × 6 × 3 par défaut |

Inflammabilité `(catch, burn)` des blocs : bûches (5, 5), planches (5, 20), feuilles (30, 60), herbes, fougères, buissons, fleurs (60, 100), liane (15, 100).

## Outils d'équilibrage (créés en v0.3, enrichis à chaque version)

- **Tableur généré** : `cargo run -p balance -- export` produit des CSV dans `docs/equilibrage/` (armes, armures, outils, recettes et coûts, mobs, boss, sorts, temps de farm) ouvrables dans LibreOffice ou Excel, avec comparaisons par palier.
- **Simulations** : la même crate simule des combats (dégâts par seconde contre vie et résistances, temps pour vaincre un boss avec l'équipement d'un palier) et des temps de production ; les résultats sont comparés aux cibles ci-dessus et les écarts signalés dans le rapport de version.
- **Mondes de test** : `test-worlds/` contient une sauvegarde par palier (début, fer, diamant, chaque dimension, Caelum), régénérée à chaque version, pour tester un boss sans rejouer.
- **Commandes de test** : `/kit <palier>` équipe le joueur comme à un palier donné (voie magique, mécanique ou mixte), `/boss <id>` invoque un boss, `/know all` révèle tout.

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

## Recettes de base (v0.3, `recipes/` du pack `strata`)

Coûts de Minecraft, pour qu'un joueur habitué ne soit pas dérouté. « Départ » : connue dès le début (sinon apprise en la posant, ou toutes connues selon l'option de monde).

| Résultat | Ingrédients | Grille | Départ |
|---|---|---|---|
| 4 planches (par essence) | 1 bûche de la même essence | 2×2 | oui |
| 4 bâtons | 2 planches quelconques en colonne | 2×2 | oui |
| Établi | 4 planches quelconques en carré | 2×2 | oui |
| 4 torches | 1 charbon sur 1 bâton | 2×2 | oui |
| Coffre | 8 planches quelconques en anneau | 3×3 | oui |
| Tonneau | 6 planches + 2 bâtons (haut et bas) | 3×3 | non |
| Grès | 4 sables en carré | 2×2 | non |
| 4 boues | 4 terres + 1 seau d'eau (le seau revient vide) | 3×3, sans forme | non |
| Bateau (par essence) | 5 planches de l'essence en U | 3×3 | non |

| Four | 8 pierres taillées en anneau | 3×3 | oui |
| Fumoir | four + 4 bûches en croix | 3×3 | non |
| Haut fourneau | four + 5 lingots de fer + 3 pierres polies | 3×3 | non |
| Tailleur de pierre | 1 lingot de fer sur 3 pierres | 3×3 | non |
| Forge | 3 lingots de fer sur 1 four entouré de 5 galets | 3×3 | non |
| Tannerie | 4 bâtons en cadre autour d'1 boue, sur 3 planches | 3×3 | oui |
| Métier à tisser | 3 bâtons sur 3 fibres végétales sur 3 planches | 3×3 | oui |
| 4 briques de pierre / pierres polies / grès taillés | 4 pierres / briques / grès en carré | 2×2 | non |

Butin associé : la pierre donne de la pierre taillée, les minerais de charbon un charbon, les minerais de diamant un diamant (pioche requise). Les outils en bois et en pierre sont des recettes de départ (voir « Outils, armes et armures »).

## Cuisson et combustibles (v0.3, `cooking/`, `fuels/`)

Valeurs de Minecraft, en ticks d'un four en pierre (20 ticks = 1 s ; une cuisson = 200 ticks).

| Combustible | Durée | Cuissons (four en pierre) |
|---|---|---|
| Charbon, charbon de bois | 1 600 | 8 |
| Seau de lave (rend le seau) | 20 000 | 100 |
| Bateau | 1 200 | 6 |
| Bûche, planche, établi, coffre, tonneau, bibliothèque | 300 | 1,5 |
| Bâton | 100 | 0,5 |

| Recette | Type | XP |
|---|---|---|
| Minerai de fer / cuivre → lingot | minerai | 0,7 |
| Minerai d'or → lingot, minerai de diamant → diamant | minerai | 1 |
| Minerai de charbon → charbon | minerai | 0,1 |
| Sable → verre, pierre taillée → pierre | divers | 0,1 |
| Bûche → charbon de bois | divers | 0,15 |

L'expérience compte une fois par cuisson, même avec le module de double sortie (sinon il doublerait aussi l'XP). Le four cuit tous les types, le haut fourneau les minerais, le fumoir la nourriture (aucune avant le jalon 8), ces deux-là deux fois plus vite (et le combustible brûle d'autant plus vite).

## Casse et outils (v0.3, champs `hardness` et `tool` des blocs)

Formule de Minecraft : un bloc cède après **dureté × 30 / vitesse** ticks (dureté × 1,5 s à mains nues) avec le bon outil ou quand aucun n'est requis, **dureté × 100** ticks sans l'outil requis (et alors sans butin). Dureté négative : incassable. Le serveur accepte une casse jusqu'à `max(2 ticks, 10 %)` en avance (latence).

Niveaux de minage : 0 bois, 1 pierre, 2 fer et or, 3 diamant (Ignium, Nihilium, Siderium au-delà). Vitesses : main 1, bois 2, pierre 4, fer 6, diamant 8, or 12 (table des tiers ci-dessous).

| Blocs | Dureté | Outil | Requis (niveau) |
|---|---|---|---|
| Pierre, pierre profonde, grès, terres cuites, roches ferreuse et vert-de-gris, lanterne | 1,5 / 3 / 0,8 / 1,25 / 1,8 / 3,5 | pioche | oui (0) |
| Minerais de charbon (et profond) | 3 / 4,5 | pioche | oui (0) |
| Minerais de cuivre, de fer (et profonds) | 3 / 4,5 | pioche | oui (1) |
| Minerais d'or, de diamant (et profonds) | 3 / 4,5 | pioche | oui (2) |
| Obsidienne | 50 | pioche | oui (3) |
| Glace | 0,5 | pioche | non |
| Bûches, planches, coffre, tonneau, bibliothèque, blocs et pied de champignon | 2 / 2 / 2,5 / 2,5 / 1,5 / 0,2 | hache | non |
| Terre, herbe, podzol, mycélium, boue, sables, gravier | 0,5-0,6 | pelle | non |
| Bloc de neige | 0,2 | pelle | oui (0) |
| Feuilles, liane | 0,2 | cisailles | non (butin seulement aux cisailles) |
| Plantes, fleurs, torche | 0 (instantané) | — | — |
| Socle, liquides | incassable | — | — |

## Outils, armes et armures (v0.3, `tiers/`, `armor_weights/`, `items/`)

Valeurs de départ, à ajuster avec la crate `balance` (jalon 17).

| Tier | Niveau | Vitesse | Utilisations | Dégâts ajoutés | Enchantabilité | Emplacements d'amélioration |
|---|---|---|---|---|---|---|
| Bois | 0 | 2 | 59 | 0 | 15 | 0 |
| Pierre | 1 | 4 | 131 | 1 | 5 | 0 |
| Fer | 2 | 6 | 250 | 2 | 14 | 1 |
| Or | 2 | 12 | 48 | 1 | 22 | 2 |
| Diamant | 3 | 8 | 1 561 | 3 | 10 | 2 |
| Ignium, Nihilium, Siderium | 4, 5, 6 | à définir (v0.10-v0.11) | | | | |

| Outil | Recette (M = matériau, S = bâton) | Particularité |
|---|---|---|
| Pioche, hache, pelle, houe, épée | formes de Minecraft ; bois (toute planche) et pierre (pierre taillée) connus au départ | épée : 4 + dégâts du tier, 1,6 coup/s |
| Cisailles | 2 lingots de fer en diagonale | vitesse 5, 238 utilisations |
| Faucille | 3 lingots de fer en crochet + 1 bâton | tier fer, coupe à 2 blocs autour (une utilisation) |
| Burin | 1 lingot de fer sur 1 bâton | tier fer, 128 utilisations |
| Outil multi-usage | 5 lingots de fer + 2 bâtons | pioche + hache + pelle au niveau du fer, 150 utilisations (au lieu de 250) |

Armures : protection par pièce (tête / torse / jambes / pieds), utilisations = facteur × (11 / 16 / 15 / 13), quantités de Minecraft (5 / 8 / 7 / 4 du matériau) à la station du matériau (voir « Stations de travail »).

| Ensemble | Poids | Protection | Total | Facteur | Matériau |
|---|---|---|---|---|---|
| Cuir | légère | 1 / 3 / 2 / 1 | 7 | 5 | cuir |
| Tissu | légère | 1 / 2 / 2 / 1 | 6 | 4 | tissu |
| Mailles de fer | intermédiaire | 2 / 5 / 4 / 1 | 12 | 13 | anneaux de fer (1 lingot → 2) |
| Plates de fer | lourde | 2 / 6 / 5 / 2 | 15 | 15 | lingots de fer |
| Mailles d'or | intermédiaire | 1 / 4 / 3 / 1 | 9 | 6 | anneaux d'or |
| Plates d'or | lourde | 2 / 5 / 4 / 2 | 13 | 7 | lingots d'or |
| Écailles de diamant | intermédiaire | 3 / 6 / 5 / 2 | 16 | 28 | écailles de diamant (1 diamant → 2) |
| Plates de diamant | lourde | 3 / 8 / 6 / 3 | 20 | 33 | diamants |

| Poids (par pièce) | Vitesse | Endurance | Mana |
|---|---|---|---|
| Légère | 0 | 0 | 0 |
| Intermédiaire | 0 | 2,5 % | 2,5 % |
| Lourde | 2,5 % (−10 % pour l'ensemble) | 5 % | 5 % |

Matériaux : fibre végétale (hautes herbes et fougères sans cisailles, 1 chance sur 4), tissu = 3 fibres (métier à tisser), cuir provisoire = 2 fibres + 1 boue (tannerie, jusqu'aux peaux des animaux, v0.4), anneaux = 2 lingots + 1 charbon → 4, écailles de diamant = 1 diamant → 2 (forge).

## Améliorations, enclume et marteau de mineur (v0.3, `upgrades/`, `anvil.ron`)

Valeurs de départ, à ajuster avec la crate `balance` (jalon 17). Coûts en XP affichés mais offerts jusqu'au jalon 9. Chaque niveau **remplace** le précédent ; un même effet ne se cumule pas entre améliorations (le plus fort compte).

| Amélioration (famille) | Objets | Niveau 1 | Niveau 2 | Niveau 3 |
|---|---|---|---|---|
| Aiguisage (artisanal) | outils, armes | +10 % dégâts et vitesse de minage ; 2 lingots de cuivre, 2 XP | +20 % ; 2 lingots de fer, 4 XP | +30 % ; 1 diamant, 7 XP |
| Renfort (artisanal) | outils, armes, armures | +20 % utilisations, +1 protection ; 2 lingots de fer, 2 XP | +40 %, +2 ; 4 lingots de fer, 4 XP | +60 %, +3 ; 2 diamants, 7 XP |
| Allègement (artisanal) | armures | −25 % de poids ; 4 tissus, 2 XP | −50 % ; 2 lingots d'or, 4 XP | −75 % ; 2 diamants, 7 XP |

Renfort et allègement s'excluent. Emplacements artisanaux : fer 1, or 2, diamant 2, cuir et tissu 1, bois et pierre 0.

| Marteau de mineur | Tier | Vitesse | Utilisations | Recette |
|---|---|---|---|---|
| Pierre | pierre (niveau 1) | 2,4 | 262 | 5 pierres taillées + 2 bâtons |
| Fer | fer (2) | 3,6 | 500 | 5 lingots de fer + 2 bâtons |
| Or | or (2) | 7,2 | 96 | 5 lingots d'or + 2 bâtons |
| Diamant | diamant (3) | 4,8 | 3 122 | 5 diamants + 2 bâtons |

Vitesse = 60 % de la pioche du tier, utilisations = deux fois celles du tier. Emplacements : une tête (`strata:area`), plus les emplacements artisanaux du tier (fer 1, or et diamant 2 ; pierre 0).

| Tête élargie | Zone | Coût |
|---|---|---|
| Niveau 1 | 3×1 | 4 lingots de fer, 3 XP |
| Niveau 2 | 5×1 | 8 lingots de fer, 5 XP |
| Niveau 3 | 3×3 | 8 lingots d'or, 8 XP |
| Niveau 4 | 3×5 | 3 diamants, 12 XP |
| Niveau 5 | 5×5 | 6 diamants, 18 XP |

Temps de casse de la zone = celui du bloc le plus dur qu'elle emporte ; une utilisation par bloc cassé.

| Réparation (`anvil.ron`) | Valeur |
|---|---|
| Une unité du matériau de l'objet (lingot du tier, planches, pierre taillée, cuir, tissu) | rend 25 % des utilisations (arrondi au-dessus), autant d'unités que nécessaire |
| Deux objets identiques (le second sans amélioration) | utilisations additionnées + 12 % d'un neuf |
| Coût en XP | 1 la première fois, +1 par réparation précédente de l'objet, 10 au plus |
| Enclume | 7 lingots de fer ; ne s'use pas |

## Accessoires, ensembles et mobilité (v0.3, `items/`, `sets/`, `bench_recipes/`, `mobility.ron`)

Valeurs de départ, à ajuster avec la crate `balance` (jalon 17). Un même effet (même stat, même opération) ne se cumule pas : de toutes les pièces portées et des bonus d'ensemble atteints, seul le plus fort compte.

| Objet (case) | Effet porté | Recette (joaillier sauf mention) |
|---|---|---|
| Anneau de vitalité (anneau) | +4 vie max (appliqué au jalon 8) | 1 lingot d'or, 2 lingots de cuivre |
| Amulette de respiration courte (amulette) | +5 s de souffle (jalon 8) | 2 lingots de cuivre, 1 lingot de fer, 1 tissu |
| Ceinture à fioles (ceinture) | +3 emplacements de potion (potions à venir) | 3 cuirs, 1 lingot de fer |
| Cape de laine (cape) | +4 isolation contre le froid (jalon 8) | métier à tisser : 4 laines, 2 fibres végétales |
| Ceinture de course (ceinture) | esquive | 2 cuirs, 1 lingot d'or, 1 lingot de cuivre |
| Bottes de saut (pieds, légères, protection 1, 65 utilisations) | +1 saut en l'air | forge : 1 paire de bottes en cuir, 2 lingots d'or, 1 charbon |
| Bottes à crampons (pieds, légères, protection 1, 65 utilisations) | escalade | forge : 1 paire de bottes en cuir, 2 lingots de fer, 1 charbon |
| Joaillier | — | à l'établi : 2 lingots de cuivre, 1 lingot de fer, 3 planches |
| Laine (provisoire jusqu'aux moutons, v0.4) | — | métier à tisser : 4 fibres végétales |

| Ensemble (4 pièces) | 2 pièces | 3 pièces | 4 pièces |
|---|---|---|---|
| Cuir | +1 protection | — | +2 protection, +1 isolation |
| Tissu | +1 isolation | — | +3 isolation |
| Mailles de fer | +1 protection | — | +2 protection, −20 % de poids |
| Plates de fer | +1 protection | — | +3 protection |
| Mailles d'or | +1 protection | — | +2 protection, +2 vie max |
| Plates d'or | +1 protection | — | +2 protection, +2 vie max |
| Écailles de diamant | +1 protection | +2 protection | +3 protection, −20 % de poids |
| Plates de diamant | +1 protection | +2 protection | +4 protection |

Le bonus atteint remplace le précédent. Une pièce cassée (option « Objets usés ») ne compte pas.

| Mobilité (`mobility.ron`) | Valeur |
|---|---|
| Saut en l'air | 7,6 blocs/s vers le haut (saut au sol : 8,4), une nouvelle pression de Sauter ; rendu seulement en touchant le sol (pas en grimpant) ; 3 au plus (`max_air_jumps`, 0 à 5) |
| Esquive | 14 blocs/s pendant 4 ticks (≈ 2,8 blocs), recharge 30 ticks (1,5 s) |
| Escalade | 2,4 blocs/s en avançant contre un mur ; accroupi : on reste accroché |

## Stations de travail (v0.3, `bench_recipes/`, `benches/`)

Chaque matériau a sa station ; aucune armure ne se fait à l'établi. Valeurs de départ, à ajuster avec la crate `balance` (jalon 17).

| Station | Recettes | Coût particulier |
|---|---|---|
| Forge (32) | lingots de fer, d'or et de cuivre (4 minerais + 1 charbon → 5, instantané ; le four : 1 → 1 en 10 s, 8 par charbon) ; anneaux de fer et d'or, écailles de diamant ; mailles de fer et d'or, plates de fer, d'or et de diamant, écailles de diamant (4 pièces chacune) ; bottes de saut et à crampons | 1 charbon (ou charbon de bois) par pièce de métal ; pas pour les écailles |
| Tannerie (5) | cuir (provisoire : 2 fibres + 1 boue), casque, tunique, pantalon, bottes en cuir | — |
| Métier à tisser (7) | tissu (3 fibres), laine (provisoire : 4 fibres), capuche, tunique, pantalon, chaussures en tissu, cape de laine | — |
| Joaillier (4) | anneau de vitalité, amulette de respiration courte, ceinture à fioles, ceinture de course | — |

Ce que le charbon ajoute : un ensemble de plates (24 lingots) brûle 4 charbons ; un ensemble de mailles (24 anneaux = 12 lingots) 6 charbons d'anneaux + 4 de pièces.

## Coffres et fours améliorables (v0.3)

Valeurs de départ, à ajuster avec la crate `balance` (jalon 17).

| Tier | Coffre (cases) | Four : temps d'une cuisson | Four : cuissons par charbon | Emplacements d'amélioration |
|---|---|---|---|---|
| Bois / pierre | 27 | 10 s | 8 | 0 |
| Fer | 45 | 8 s | 10 | 1 |
| Or | 63 | 5 s | 12 | 2 |
| Diamant | 81 | 3 s | 16 | 3 |

Un coffre double a deux fois les cases **et les emplacements** de son tier (les améliorations des deux moitiés comptent pour tout le coffre) ; il monte de tier d'un coup avec **deux kits** (un par bloc), pour garder un seul tier. Le kit passe d'un tier au suivant seulement (pas de saut) ; il coûte moins que la fabrication directe du tier visé. Coffres et tonneaux (`storage_tiers/`) : 3 / 5 / 7 / 9 rangées, 0 / 1 / 2 / 3 emplacements (famille `strata:storage`).

Fours (`furnace_tiers/`) : `cook_time` 100 / 80 / 50 / 30 %, `fuel_value` 100 / 125 / 150 / 200 % (d'où 8 / 10 / 12 / 16 cuissons par charbon). Fumoir et haut fourneau : mêmes tiers, deux fois plus rapides.

| Fabrication | Coût |
|---|---|
| Tier fer / or direct | 8 lingots du métal autour du four du tier précédent |
| Tier diamant direct | 4 diamants + 4 verres autour du four en or |
| Kit fer | 5 lingots de fer + 1 pierre taillée |
| Kit or | 5 lingots d'or + 1 lingot de fer |
| Kit diamant | 3 diamants + 3 verres |

| Module (non cumulable avec lui-même) | Temps d'une cuisson | Cuissons par combustible | Résultats |
|---|---|---|---|
| Vitesse (4 fer, 4 charbon, 1 or) | 75 % | 80 % | ×1 |
| Économie (4 fer, 4 charbon, 1 seau de lave) | 100 % | 150 % | ×1 |
| Double sortie (4 fer, 4 or, 1 diamant) | 125 % | 50 % | ×2 |

Depuis le jalon 7, les modules sont des améliorations (`upgrades/furnace_*.ron`, briques `Stat` sur `strata:cook_time`, `strata:fuel_value`, `strata:output`). Non-cumul : de deux gains d'une même stat, seul le plus fort compte ; les coûts (briques négatives) s'additionnent toujours. Vitesse + double sortie : temps 75 % × 125 %, cuissons par combustible 80 % × 50 % = 40 %.

| Rangement | Coût | Valeurs |
|---|---|---|
| Coffre / tonneau fer ou or direct | 8 lingots du métal autour du tier précédent | — |
| Coffre / tonneau diamant direct | 4 diamants + 4 verres autour du tier or | — |
| Kit de rangement fer | 5 lingots de fer + 1 planche | bois → fer |
| Kit de rangement or | 5 lingots d'or + 1 lingot de fer | fer → or |
| Kit de rangement diamant | 3 diamants + 3 verres | or → diamant |
| Tri automatique | 4 planches, 4 fer, 1 lingot de cuivre | à la fermeture, et après une entrée automatique |
| Filtre | 4 planches, 4 fer, 1 tissu | 9 cases, liste acceptée ou refusée ; vide = tout passe |
| Compression | 4 planches, 4 fer, 1 lingot d'or | 9 pépites → 1 lingot, 9 lingots → 1 bloc (`compression/`) |
| Destruction | 4 planches, 4 fer, 1 obsidienne | détruit ce que son filtre désigne ; vide = rien |
| Aimant | 4 planches, 4 fer, 1 diamant | rayon 5 blocs, toutes les 10 ticks (0,5 s) ; option de monde « Aimants » |

Pépites et blocs : 1 lingot ↔ 9 pépites, 9 lingots ↔ 1 bloc (fer, or, cuivre ; diamant ↔ bloc de diamant), à la grille ou par compression.

Manque pour plus tard : les tiers Ignium, Nihilium et Siderium (v0.10-v0.11) n'ont ni objets ni fichiers de tier.

## Outils d'équilibrage (créés en v0.3, enrichis à chaque version)

- **Tableur généré** : `cargo run -p balance -- export` produit des CSV dans `docs/equilibrage/` (armes, armures, outils, recettes et coûts, mobs, boss, sorts, temps de farm) ouvrables dans LibreOffice ou Excel, avec comparaisons par palier.
- **Simulations** : la même crate simule des combats (dégâts par seconde contre vie et résistances, temps pour vaincre un boss avec l'équipement d'un palier) et des temps de production ; les résultats sont comparés aux cibles ci-dessus et les écarts signalés dans le rapport de version.
- **Mondes de test** : `test-worlds/` contient une sauvegarde par palier (début, fer, diamant, chaque dimension, Caelum), régénérée à chaque version, pour tester un boss sans rejouer.
- **Commandes de test** : `/kit <palier>` équipe le joueur comme à un palier donné (voie magique, mécanique ou mixte), `/boss <id>` invoque un boss, `/know all` révèle tout.

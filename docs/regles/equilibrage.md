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

## Outils d'équilibrage (créés en v0.3, enrichis à chaque version)

- **Tableur généré** : `cargo run -p balance -- export` produit des CSV dans `docs/equilibrage/` (armes, armures, outils, recettes et coûts, mobs, boss, sorts, temps de farm) ouvrables dans LibreOffice ou Excel, avec comparaisons par palier.
- **Simulations** : la même crate simule des combats (dégâts par seconde contre vie et résistances, temps pour vaincre un boss avec l'équipement d'un palier) et des temps de production ; les résultats sont comparés aux cibles ci-dessus et les écarts signalés dans le rapport de version.
- **Mondes de test** : `test-worlds/` contient une sauvegarde par palier (début, fer, diamant, chaque dimension, Caelum), régénérée à chaque version, pour tester un boss sans rejouer.
- **Commandes de test** : `/kit <palier>` équipe le joueur comme à un palier donné (voie magique, mécanique ou mixte), `/boss <id>` invoque un boss, `/know all` révèle tout.

# Roadmap de Strata Mundi

**Strata Mundi** (« les couches du monde », titre court Strata) est un jeu voxel sandbox inspiré de Minecraft, en Rust avec un moteur maison sur wgpu. Ce fichier donne le découpage en versions ; les décisions de design globales sont dans `docs/regles/vision.md`, le détail de chaque version dans `docs/versions/vX.Y.md`.

## Vision en une phrase

Un Minecraft-like en pixel art 16×16 où l'équipement classique (jusqu'au diamant) suffit pour Terra et ses boss, mais où les autres mondes exigent de se préparer **par la magie ou par la mécanique**, jusqu'à subir les épreuves des dieux eux-mêmes, et peut-être se rebeller contre eux. Qui se prépare mal ne survit pas.

## Versions

| Version | Nom | Contenu principal | Statut |
|---|---|---|---|
| [v0.1](versions/v0.1.md) | Fondations | Moteur, monde infini, créatif, sauvegarde, menus, réglages complets, manette, 3 langues, premier wiki | Prêt |
| [v0.2](versions/v0.2.md) | Monde vivant | Grottes façon Caves & Cliffs, relief par biome, fluides, lumière, jour/nuit, atmosphère, sons, bateaux | Prêt |
| [v0.3](versions/v0.3.md) | Survie | Jauges, inventaire, craft, outils jusqu'au diamant, améliorations, mort, progrès, déco | Prêt |
| [v0.4](versions/v0.4.md) | Créatures | Entités animées, IA comportementale, animaux, monstres, chevaux, 3e personne, skins | Prêt |
| [v0.5](versions/v0.5.md) | Saisons et terroir | Météo, saisons, agriculture, pêche, cuisine, apiculture, élevage, ombres, reflets | Prêt |
| [v0.6](versions/v0.6.md) | Aventure | Structures, boss de structure et secondaires, bestiaire mythologique, toutes les familles d'armes, armes à poudre noire, archéologie, exploration, serrures, journal | Prêt |
| [v0.7](versions/v0.7.md) | Villages | Villageois nommés, dialogues à choix, monnaie, quêtes, raids, gestion de village, tavernes et jeux de hasard | Prêt |
| [v0.8](versions/v0.8.md) | Mécanique | Énergie rotative, machines, logistique, circuits, transport, armes à feu mécaniques, explosifs, équipements de dimension | Prêt |
| [v0.9](versions/v0.9.md) | Arcanes | Baguettes, bâton de sorts modulaire, bâton d'invocation, boss magiques, montures magiques, potions, sorts de dimension | Prêt |
| [v0.10](versions/v0.10.md) | Dimensions | Portails, Infernum, Crepuscula, Abyssus (mer à y = 384, six paliers), Machina Caeli, peuples | Prêt |
| [v0.11](versions/v0.11.md) | Le Vacuum | Vacuum, dragon (faux boss final) et révélation, tour infinie, zone cachée, matériaux ultimes, boss revisités | Prêt |
| [v0.12](versions/v0.12.md) | Caelum | Huit épreuves divines, Arbiter, armes et pièces divines, rébellion et déicide | Prêt |
| [v0.13](versions/v0.13.md) | Finitions | Équilibrage, accessibilité, performance (dont DS) : jeu solo complet et stable | Prêt |
| [v1.0](versions/v1.0.md) | Multijoueur | Serveur dédié, coopération en ligne et en réseau local, gel du format de sauvegarde, sortie publique | À valider |
| [v1.1](versions/v1.1.md) | Modding | Dossier de mods, scripts Lua côté serveur, éditeur en jeu, mod d'exemple et guide | Prêt |
| [v1.2](versions/v1.2.md) | Pêche | Matériel par tiers, 120 espèces et plus, mini-jeu à barre de tension, élevage, poissons légendaires | À valider |
| [v1.3](versions/v1.3.md) | Machina | Peuples armés, discrétion et infiltration, armes à feu steampunk, véhicules | À valider |
| [v1.4](versions/v1.4.md) | Villes et arènes | Villes et lois locales, ville du joueur, arènes, capture et ménageries | À valider |
| [v1.5](versions/v1.5.md) | Sièges | Sièges par vagues façon tower defense | À valider |
| [v1.6](versions/v1.6.md) | Guildes | Guildes, rangs, guilde du joueur | À valider |
| [v1.7](versions/v1.7.md) | Mers et pirates | Navires bloc par bloc, combat naval, pirates, routes commerciales | À valider |
| [v1.8](versions/v1.8.md) | Catastrophes | Catastrophes naturelles par biome, saison et dimension | À valider |

Numérotation semver : 0.9 → 0.10 → … → 0.13 → 1.0 → 1.1 → … → 1.8.

Ordre décidé le 29/10/2026 : le multijoueur passe en v1.0, avant le modding et les contenus post-v1.0, pour que ceux-ci soient conçus d'emblée pour plusieurs joueurs. Les idées détaillées des versions v1.2 à v1.8 sont dans `docs/idees-post-v1/`.

Claude Code s'arrête à la fin de chaque version. Avant de le relancer, relis le fichier de la version suivante et modifie ce que tu veux. Le wiki (`wiki/index.html`) est régénéré à chaque fin de version.

## Après la v1.8 (idées, rien de planifié)
- Lore et histoire écrits, quêtes scénarisées.
- Trains et autres structures mobiles (les navires arrivent en v1.7).
- Interface de l'écran du bas du DS (quand la console sera construite).
- Nouvelles activités.

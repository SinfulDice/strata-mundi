# Règles techniques

Détail de la stack, de l'architecture et du socle de rendu. Résumé dans `CLAUDE.md`.

## Stack technique

- Rust stable, workspace Cargo.
- Fenêtre/entrées : `winit`. Manettes : `gilrs`. Rendu : `wgpu` **forcé sur le backend Vulkan, sur toutes les plateformes** (un seul chemin à tester ; minimum = GPU compatible Vulkan). **Pas de moteur de jeu** (ni Bevy, ni autre).
- Maths : `glam`. Bruit : `noise` ou `fastnoise-lite`. Parallélisme : `rayon` et/ou threads + `crossbeam-channel`.
- Sauvegardes : `serde` + `bincode`, compression `zstd`.
- Formats texte : **TOML** pour ce que l'humain édite à la main (options, touches, `pack.toml`/`mod.toml` éventuels), **RON** pour le contenu des packs. Pas de JSON (sauf l'index de recherche du wiki).
- Profilage : **Tracy** (`tracing-tracy` ou équivalent) derrière une feature Cargo `profiling`, connectable à distance pour profiler le DS depuis un PC.
- Toolchain : **dernière Rust stable**, fixée dans `rust-toolchain.toml` et mise à jour volontairement.
- Audio : `kira` ou `rodio` pour la lecture ; les sons sont **synthétisés par code**.
- Traduction : **Fluent** (fichiers `.ftl` par langue dans les packs). Langues : **français (`fr`), anglais (`en`), LOLcat (`lol`)**. Aucun texte affiché codé en dur.
- Erreurs : `thiserror` (bibliothèques), `anyhow` (binaires). Logs : `tracing`.
- `egui` est autorisé **uniquement** pour les outils de debug ; l'interface du jeu est faite maison, en pixel art.
- Une nouvelle crate est permise si elle est maintenue et justifiée dans `docs/decisions.md`.

## Architecture

```
crates/
  core/      types purs : blocs, items, chunks, coordonnées, registres, packs de contenu — sans GPU
  worldgen/  génération procédurale déterministe (seed) : relief, biomes, grottes, structures
  server/    simulation autoritaire : tick fixe 20 TPS, mondes/dimensions, entités, IA, sauvegarde
  protocol/  messages client <-> serveur ; en solo, canaux en mémoire dans le même processus
  client/    rendu wgpu, entrées, caméra, UI, audio, interpolation entre ticks
  app/       binaire : lance serveur intégré + client
  wikigen/   binaire : génère le wiki HTML à partir des packs et des textes rédigés
```

Règles qui ne bougent jamais :
- Le client n'écrit jamais dans le monde : il envoie des intentions, le serveur décide et diffuse.
- **Plusieurs dimensions** supportées dès la v0.1 (identifiant de dimension partout).
- **Entités** : système d'entités côté serveur dès la v0.1 (le joueur en est une). **ECS maison** (choix de l'utilisateur, pour que tout soit lisible) : entités = identifiants générationnels, composants en tableaux denses, systèmes = fonctions qui itèrent sur des requêtes. Le documenter en détail dans `docs/explications/ecs.md`.
- **Tout le contenu passe par des packs** : `assets/packs/<namespace>/{blocks,items,recipes,biomes,loot_tables,structures,lang,textures,...}`. Le contenu de base est lui-même le pack `strata`. Aucun bloc, objet, mob, sort ou recette défini en dur dans le code.
- **Préparer le modding de la v1.1** dès maintenant : les packs se chargent par ordre de priorité, et un pack de priorité supérieure peut **ajouter, modifier ou retirer** n'importe quelle entrée d'un autre (y compris du pack `strata`). Les comportements (IA, sorts, machines, boss) sont construits à partir de briques nommées et paramétrables, pour pouvoir plus tard être appelés depuis des scripts Lua.
- `core` et `worldgen` sont testables sans fenêtre ni GPU.
- Simulation à pas fixe **20 TPS**, rendu découplé et interpolé.
- **Bordure de monde** réglable à la création, **10 millions de blocs** par défaut ; coordonnées de blocs en `i32`, position des entités en `f64` côté serveur, **rendu relatif à la caméra** côté client pour éviter les tremblements loin de l'origine.
- **Compute shaders autorisés** uniquement quand une mesure montre que le CPU est le goulot (décision consignée dans `decisions.md`).
- **Rechargement à chaud** des packs et des shaders en développement (et `/reload` en créatif).

## Monde et rendu (socle commun)

- Sections 16×16×16, colonnes **y de 0 à 511** (512 blocs, 32 sections) : socle indestructible en bas, **surface moyenne ~128**, **mer à y = 120**, reliefs courants jusqu'à ~256, **grands sommets ~400**, espace libre au-dessus pour construire. Horizontalement infini (bordure réglable).
- **Distance verticale de chargement réglable** : seules les sections proches de la hauteur du joueur sont chargées et maillées en priorité (réglage serré en profil DS) ; les sections vides ne coûtent quasiment rien.
- Maillage : face culling + greedy meshing sur threads de travail, jamais sur le thread de rendu.
- Textures 16×16 en texture array, filtrage nearest, **générées par code** de façon déterministe ; un PNG dans le pack remplace la version générée.
- Ambient occlusion par sommet dès la v0.1.
- Frustum culling, distance de rendu réglable, chargement progressif.
- **Tout effet ou système coûteux est désactivable ou réglable** (voir `docs/regles/plateformes.md`).

## Audio

- **Sons synthétisés par code, rendu visé réaliste.** Si un son synthétisé n'est pas convaincant (cris d'animaux, voix de monstres…), propose à l'utilisateur un **repli sur des sons libres CC0** (source et licence notées dans `docs/decisions.md` et les crédits) ; ne les ajoute qu'avec son accord.
- **Musique procédurale pré-rendue** : chaque morceau est généré une fois (au premier lancement ou à la première écoute) et mis en cache dans un fichier, puis simplement lu. Économise le CPU et la batterie du DS.
- **Un thème musical par dimension** (instruments et gammes propres), **un thème par dieu** (repris discrètement dans le monde qu'il a créé) et **un thème par boss majeur** ; musique calme et rare en exploration, adaptative en combat.
- **Audio spatial avancé** : réverbération selon le lieu (grottes, intérieur, plein air), occlusion (sons étouffés derrière un mur), sous-titres directionnels. Désactivable en **stéréo simple** (préréglage DS).

## Éclairage et rendu avancé

- **Lumière voxel blanche** façon Minecraft (niveaux 0-15, ciel et blocs, propagation par blocs), lissée avec l'ambient occlusion.
- **Ombres dynamiques du soleil et de la lune** en option (v0.5).
- **Lumière dynamique** (objet lumineux tenu, sorts, projectiles enflammés) réglable : complète, limitée au joueur, ou désactivée (profil DS).
- **Feuillage** transparent ou opaque selon le réglage ; eau et verre triés correctement.
- **Végétation au vent** animée dans le shader, désactivable.

## Stockage et sauvegardes

- **Fichiers de région maison** (32×32 colonnes, chunks compressés zstd), comme Minecraft.
- **Écriture atomique** : chaque fichier est écrit sous un nom temporaire puis renommé d'un coup (avec `fsync`), pour ne jamais laisser de fichier à moitié écrit, même en cas de coupure de batterie du DS. Combinée aux copies de sécurité tournantes.
- **Palette par section** sur disque et en mémoire (liste des blocs présents + index courts).
- États de blocs et entités de bloc : choix libre (consigné dans `docs/decisions.md`), en gardant compactes les données des blocs simples et à part les données riches (inventaires, machines).

## Threads

- **Adaptation automatique** au nombre de cœurs (sur le N100 à 4 cœurs : rendu, serveur, et 2 travailleurs pour génération, maillage et lumière), avec un réglage manuel du nombre de threads de travail.

## Simulation à distance

- **Niveaux de détail d'IA** : mobs proches en IA complète, mobs lointains en IA simplifiée et mise à jour moins souvent (indispensable sur le DS).
- Hors distance de simulation, **le monde est gelé** comme Minecraft, **sauf** les zones maintenues par une **ancre de chargement** (bloc mécanique de la v0.8, qui consomme des ressources ou de l'énergie et a une portée limitée).
- **Rattrapage au retour** : quand une zone se recharge, le temps écoulé est appliqué d'un coup aux systèmes simples (cultures, arbres, fours et machines de cuisson, réapprovisionnement des villages), par calcul et non par simulation tick par tick.

## Réseau (préparation de la v2.0)

- **Tout échange client ↔ serveur est un message sérialisable** (types définis dans `protocol`, encodables en binaire), même en solo où ils restent en mémoire. Aucun raccourci où le client lirait directement l'état du serveur. Ainsi, la v2.0 consistera surtout à brancher un transport réseau.

## Interface

- **Bibliothèque de widgets maison, déclarative** : chaque écran (inventaire, coffre, four, menus…) est décrit dans un fichier RON (disposition, widgets, fonds, liaisons), le code Rust n'implémente que les comportements. Ces fichiers restent **internes jusqu'à la v1.1**, où ils deviendront modifiables par les mods ; d'ici là leur format peut évoluer librement.
- **Texte riche** par balises simples : couleurs, gras, icônes d'objets et de touches (qui s'adaptent au clavier ou à la manette) dans le texte.
- **Thème visuel unique, à décider plus tard sur maquettes** : en v0.1, utiliser un thème pixel art neutre et provisoire ; joindre au rapport de la v0.1 **2-3 maquettes de thème** (menu principal, inventaire, HUD) pour que l'utilisateur choisisse.

## Traductions

- **Fluent** (Mozilla, crate `fluent`) : fichiers `.ftl` par langue (fr, en, lol), gestion propre des pluriels, des genres et des variables. Aucun texte en dur.

## Cinématiques et narration

- **Cinématiques courtes dans le moteur du jeu** (caméra scriptée de quelques secondes, décrite en pack) : arrivée dans une dimension, entrée d'un boss, mort de Ianitor et révélation, Arbiter, choix de la rébellion. Toujours **passables**.
- **Narration en texte uniquement** (journal, livres, lore) ; les PNJ gardent leurs petits sons.


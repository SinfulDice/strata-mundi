# PROGRESS — Strata Mundi

## État

- **Version en cours** : v0.2.1 — Retours de test. Détail : `docs/versions/v0.2.1.md`. La v0.3 attend la fin de la v0.2.1.
- **Jalon en cours** : 1 — Ravins plus rares et plus propres
  - [x] J1 Ravins rares (rareté en pack), étroits, parois verticales en pierre, pas d'herbe sur les parois ni d'arbre au bord ; tests (fréquence bornée, aucune herbe sur paroi)
  - [ ] J2 Biomes 2-3× plus grands, continents massifs, règle de voisinage chaud/froid en pack, pentes ordinaires franchissables ; tests (aucun voisinage interdit, déterminisme)
  - [ ] J3 Eau façon Minecraft (recherche du bas dans un rayon de 4, portée 7, paramètres en pack) ; tests
  - [ ] J4 Pose de blocs fiable (aucun clic perdu, prédiction client) ; tests
  - [ ] J5 Torches murales, lanternes suspendues (variantes, soutien) ; tests
  - [ ] J6 Sons (variantes au chargement, lave audible, vent plus doux, curseur Ambiances) ; tests
  - [ ] J7 Curseurs d'un clic, glisser, molette, répétition à l'appui maintenu ; tests
  - [ ] J8 Clôture (captures avant/après, explications, rapport, tag `v0.2.1`)
- **Dernière session** : 11/11/2026

## Fait

### v0.2

- v0.2 terminée : détail dans `docs/versions/v0.2.md` (rapport de fin de version).

### v0.1

- **Jalon 10 — Clôture** : critères de fin vérifiés (rapport dans `docs/versions/v0.1.md`), montagnes refaites, brouillard dans les captures, anisotropie retirée des préréglages, `assets/` dans le `.msi`, release répétée avec succès (tag `v0.1.0` à pousser par l'utilisateur), thème d'interface **Pierre** choisi (D035).
- **Jalon 9 — Wiki** : binaire `wikigen` (site statique HTML/CSS/JS sans dépendance, fr/en/lol, ouvrable sans serveur) : accueil, pages par bloc (infobox calculée depuis les packs, lieux de génération, historique), par biome (vue rendue par le jeu, blocs, végétation, climat), commandes, mécaniques (génération, sauvegarde, modes), contrôles (touches par défaut clavier et manette), réglages (effet sur les performances, préréglages comparés), nouveautés et historique des versions ; recherche sur index par langue, mode sombre (système + bouton), bascule de langue vers la même page ; `--strict` (et un test) échouent si un texte manque ; `strata --render-icons` et `--render-biomes` ; `scripts/build-wiki.sh` (utilisé par la release) ; 47 textes rédigés par langue dans `wiki-content/` et `history.ron`.
- **Jalon 8 — Réglages, console, debug** : **fichier unique** `config/settings.toml` (graphismes, interface, commandes, jeu ; valeurs invalides → défaut), **préréglages** très bas → ultra et **DS** avec passage automatique en « personnalisé », **écran d'options** à 6 onglets (description de l'impact de chaque réglage), **application à chaud** (comparaison des réglages, seul le nécessaire est refait) ; nouveaux réglages : échelle de rendu et **filtres daltoniens** (passe finale, coût nul si inutilisée), limite d'images/s, VSync, FOV, brouillard, mipmaps, anisotropie, fils de maillage et de génération, sensibilités souris/stick, axe inversé, zone morte, **touches reconfigurables** (clavier et manette, conflits retirés), sauvegarde auto, nombre de copies, 5 images/s en arrière-plan, zoom au sprint (désactivable) ; **police lisible** DejaVu Sans rastérisée à la résolution de l'écran ; **packs de ressources** activables et ordonnables ; **F3** (images/s, position, colonnes, maillages en attente, bloc visé, mémoire, carte graphique) ; **F5** recharge textures, traductions, interface, polices et shaders (erreurs GPU rattrapées) ; **rapports de plantage** (`crash-reports/`, 60 dernières lignes du journal) et « Signaler ce plantage » (issue GitHub pré-remplie) ; **vérification de mise à jour** désactivable ; **console** (T ou `/`) : `/tp`, `/give`, `/gamemode`, `/time`, `/seed` exécutées par le serveur avec réponses traduisibles, `/help`, `/cinematic`, historique, complétion Tab ; **cinématiques** en pack (Catmull-Rom, passables, bandes noires, sous-titres) ; heure du monde sauvegardée ; `installation-ds.md` complété (dépendances système).
- **Jalon 7 — Sauvegarde et menus** : dossier par monde (`world.ron` versionné, `player.ron`, capture `icon.png`), **régions** 32×32 ne contenant que les colonnes modifiées (`postcard` + `zstd` par colonne, blocs par identifiant texte), écriture atomique (`fsync` + renommage), région abîmée mise de côté, **copies tournantes** (3) à chaque ouverture, autosave 5 min, sauvegarde en quittant ou en fermant la fenêtre, monde d'une autre version grisé avec message ; **bordure** réglable à la création ; **écran titre** sur panorama généré qui tourne, logo ; **liste des mondes** (capture, date, mode, temps de jeu), création (nom, graine, bordure, mode), suppression confirmée ; **menu pause** (le monde s'arrête), option « pause dans tous les menus », **options d'interface** (langue, échelle, pause) dans `config/interface.toml`, **éditeur de HUD** (sélection, flèches/croix/glisser, défaut) ; widgets déclaratifs : listes, curseurs, clavier à l'écran pour la manette, **texte riche** avec invites de touches (dernier périphérique, lettre réelle du clavier AZERTY/QWERTY) ; **clapet du DS** : pause + sauvegarde sur `PrepareForSleep` de logind (`zbus`), reprise au réveil ; `--data-dir`. Regard du joueur sauvegardé ; le chargement priorise ce qui est devant.
- **Jalon 6 — Joueur et interaction** : joueur = entité d'un **ECS maison** côté serveur (entités à génération, stockages par composant, systèmes) ; **physique partagée** client/serveur dans `core` (gravité, collisions AABB par sous-pas, saut, sprint, accroupi sans chute au bord, montée d'un demi-bloc et saut automatique réglables, vol créatif au double saut) ; **prédiction et réconciliation** (entrées numérotées, rejeu, rendu interpolé) ; raycast Amanatides-Woo, contour du bloc visé, casser/poser validés par le serveur (portée, place libre, pas dans le joueur), **bûches orientées** (variantes générées depuis `placement: Axis`, écorce tournée au maillage), prise de bloc au clic molette, nom du bloc visé façon Jade (désactivable) ; **interface maison déclarative** (écrans RON, thème, police pixel en pack avec accents, atlas unique, icônes isométriques, navigation souris/clavier/manette au focus) : HUD (viseur, barre 9 cases, nom du bloc), **inventaire créatif** (onglets par catégorie, recherche, grille). Fenêtre verticale glissante : le sol reste visible de haut en profil DS.
- **Jalon 5 — Génération de terrain** : crate `worldgen` (bruit `fastnoise-lite` en f64), 4 cartes de climat, relief continu (mer 120, collines, montagnes en crêtes jusqu'à ~400), 6 biomes en pack (plaines, forêt, désert, océan, montagnes, toundra) avec frontières bruitées, couches (socle irrégulier, roche profonde, sous-sol, surface, plages), eau statique et glace, grottes spaghetti (grille 4³ interpolée, plafond sous le sol voisin), filons de minerais par profondeur avec variantes profondes, chênes, sapins, cactus placés depuis les 9 colonnes voisines, point de départ accueillant (plaines/forêt, arbres, eau proche, deux blocs d'air libres). Tests : déterminisme quel que soit l'ordre, graines différentes, 6 biomes atteignables et traduits, hauteurs, socle et mer, départ, arbres entiers à cheval sur les colonnes. ~2 ms par colonne.
- **Jalon 4 — Monde infini** : serveur qui charge un disque de colonnes autour du joueur (proches et devant d'abord, ordre mis en cache), threads de génération à file bornée, envoi limité par tick, déchargement avec hystérésis, positions client validées et bornées ; client : colonnes maillées quand leurs 8 voisines sont là, frustum culling, déchargement en O(1), distance de rendu réglable à chaud (F7/F8), threads de maillage et de génération par préréglage, `--seed`, `--dimension`, captures sur monde généré. Vol de 2 min (scène `flyover`) sans plantage, 20 TPS tenus.
- **Jalon 3 — Chunks et maillage** : sections 16³ à palette compressée (indices empaquetés 0-16 bits), colonnes de 32 sections, mailleur (culling, fusion gloutonne, AO par sommet à 4 niveaux réglable, opaque/découpé + translucide trié), sommets de 8 octets, maillage sur threads de travail (`crossbeam-channel`, file bornée, versions), envoi GPU limité à 4 ms/image, distance verticale réglable (maillage et rendu), sections enfouies ignorées, réglages graphiques par préréglage (distance verticale, AO, threads). Bench : ~0,27 ms par section de surface, ~1,3 ms au pire.
- **Jalon 2 — Registres et packs** : packs RON par priorité avec ajout/remplacement/retrait (`PackSet`, `remove.ron`), erreurs localisées (fichier:ligne:colonne) sans plantage, registres à identifiants texte et numériques (`Registry`, `BlockId`, `ItemId`, `TextureId`), propriétés de bloc (solide, rendu, opacité lumineuse, dureté, textures par face, son, catégorie), objet automatique par bloc, textures procédurales depuis une **palette globale** (`Material`, `Overlay`, `Ore`, `Sprite`, `Recolor`), PNG 16/32/64 px avec mise à l'échelle, tableau de textures GPU depuis le registre, packs de ressources (`--resource-pack`), traductions Fluent fr/en/lol + test de clés manquantes + glossaire LOLcat, `--texture-sheet`, `docs/guide-textures.md`, scène d'aperçu en RON, **3 planches d'essai** dans `docs/maquettes/` ; **style classique choisi** (D017) et textures v0.1 complètes (33 blocs : pierre, roche profonde, socle, terre, herbe, herbe enneigée, sable, grès, gravier, neige, glace, eau, verre, cactus, chêne et sapin, minerais normaux et profonds). Revue appliquée : recettes bornées, minerais/franges composés sur des bases PNG haute résolution, retraits et doublons signalés, limite de couches GPU vérifiée, `max_stack`/`cull_same` en pack.
- **Jalon 1 — Fenêtre et rendu minimal** : fenêtre `winit`, `wgpu` Vulkan seul, serveur intégré sur son thread à 20 TPS (`TickClock`) et rendu découplé, messages `protocol` sur canaux en mémoire, caméra libre en `f64` (rendu relatif, reverse-Z, plan lointain infini), entrées unifiées (actions, liaisons physiques, manette `gilrs`, zone morte, dernier périphérique, tactile prévu), cube texturé procédural (`core::texture`), `--screenshot` headless, `--bench-scene` + rapport Markdown, reconfiguration de surface (redimensionnement, perte), logs `tracing`.
- **Jalon 0 — Bootstrap** : workspace (`core`, `worldgen`, `server`, `protocol`, `client`, `app`), binaire `strata` (`clap`, `tracing`), feature `profiling` (Tracy), toolchain 1.97.0 épinglée, lints (`unsafe` interdit, `unwrap`/`expect` hors tests), licences MIT + CC BY-NC 4.0, `CONTRIBUTING.md`, modèles d'issues, CI (fmt, clippy, tests, build Linux/Windows), benchmarks avec alerte > 10 % vs dernier tag, release sur tag (`.tar.gz`, AppImage, Flatpak, `.zip`, `.msi`, branche `wiki`), agents/skills/hook Claude Code, `docs/decisions.md`, `docs/explications/README.md`.

## Environnement

- Session cloud Claude Code : Ubuntu 24.04, 4 cœurs, 15 Go RAM, sans GPU. Rust 1.97.0.
- Paquets système (`scripts/install-linux-deps.sh`) : déjà présents, sauf `libxkbcommon-x11-0` (requis par `winit` sous X11), ajouté.
- **Rendu headless** : `--screenshot` fonctionne avec lavapipe (~0,3 s par capture). Fenêtre et `--bench-scene` testés sous **Xvfb** (`xvfb-run -a`, `XDG_RUNTIME_DIR` à définir).
- `vulkaninfo --summary` : **lavapipe OK** — `llvmpipe (LLVM 20.1.2, 256 bits)`, Vulkan 1.4.318, Mesa 25.2.8.
- Machine de jeu de l'utilisateur : PC portable Linux Mint 22.3, i5-1235U (12 fils), Intel Iris Xe intégré, 16 Go.
- **Console DS pas encore construite** (29/10/2026). En attendant : mesures sur le PC de l'utilisateur avec `taskset -c 0-3 strata --bench-scene flyover --preset ds` (4 cœurs comme le N100). Valables pour la **mémoire** (≤ 2 Go) et la **tendance** d'une version à l'autre, **pas** pour les FPS du DS (GPU du N100 bien plus faible) : les rapports `perf-reports/` le précisent. Ne redemande pas de mesure ni d'essai manette sur le DS avant que l'utilisateur annonce la console prête ; la cible 30 FPS DS reste « à mesurer » (simulation documentée en v0.13 si besoin).
- Manette : essais sur PC avec une manette quelconque (`gilrs`, même code que sur le DS) ; disposition des boutons du DS à vérifier plus tard.

## Mesures

| Date | Machine | Profil | FPS moyen | 1 % low | Mémoire | Source |
|---|---|---|---|---|---|---|
| 11/11/2026 | PC utilisateur (i5-1235U, Iris Xe intégré) | ultra | 61,6 | 30,6 | 291 Mio | `perf-reports/2026-11-11-sinfuldice-njx0pu-ultra-flyover.md` |
| 11/11/2026 | PC utilisateur, 4 cœurs (`taskset`) | DS | 343,5 | 48,9 | 179 Mio | `perf-reports/2026-11-11-sinfuldice-njx0pu-ds-flyover.md` (GPU ~3× plus puissant que le N100 : pas une mesure DS) |

Benchmarks CPU (cloud, `cargo bench --workspace --bench '*'`), dernière valeur de chaque opération ; historique dans les rapports `docs/versions/v0.X.md` :

| Date | Opération | Temps | Source |
|---|---|---|---|
| 11/11/2026 | Génération d'une colonne (plaine / montagne / caverne / gisement) | 1,71 / 2,65 / 1,39 / 2,59 ms | `generate_*column` |
| 11/11/2026 | Carte des biomes d'une colonne | 0,18 ms | `biome_map_column` |
| 11/11/2026 | Lumière d'une colonne générée / poser puis casser une torche | 0,78 / 0,44 ms | `light_*` |
| 11/11/2026 | Maillage : section de surface / damier / eau qui coule / frontière de biomes | 0,26 / 2,07 / 0,35 / 0,26 ms | `mesh_section_*` |
| 11/11/2026 | Teintes d'une colonne / voisinage | 8 / 42 µs | `column_tints_blend`, `neighborhood_build` |
| 11/11/2026 | Grand écoulement (lac par une brèche de 32 blocs) | 5,38 ms | `fluids/large_flow_settles` |
| 11/11/2026 | Incendie de plaine, 40 ticks | 33,7 ms | `fire/prairie_fire_40_ticks` |
| 11/11/2026 | Particules, une image à 2 000 | 0,38 ms | `particles/frame_2000` |
| 11/11/2026 | Mixage audio d'un bloc de 11,6 ms, avancé / stéréo (forte variance de la VM : le code du jalon 10 donne la même valeur) | 1,1 ms / 12-21 µs | `audio/mix_block_32_voices_*` |
| 10/11/2026 | Rendu d'un morceau de musique (release) ; fichier de cache | 0,14 à 0,32 s ; 8 à 15 Mo | `strata --render-music` (D055) |

Mémoire et temps (cloud, rendu logiciel lavapipe, indicatif) :

| Date | Profil | Mesure | Valeur | Source |
|---|---|---|---|---|
| 11/11/2026 | DS | Pic de mémoire (RSS), vol de 2 min | 241 Mio (cible ≤ 2 Go) | `perf-reports/2026-11-11-cloud-ds-flyover.md` |
| 11/11/2026 | DS | Lancement → carte graphique prête | 1,1 s (cible < 5 s) | journal |
| 11/11/2026 | DS | Chargement initial du monde (distance 6, disque demandé de 8) | ≈ 9 s d'après le journal (cible < 10 s) ; 10,9 s en durée totale du processus | journal |
| 10/11/2026 | Tous | Pic RSS de `--render-music` ; en jeu | 74 Mio ; < 1 Mio | `/proc/<pid>/status` (D055) |

## Limitations connues

- CI verte sur GitHub ; les benchmarks de la CI échouaient depuis le tag v0.1.0 (options de criterion passées aux tests unitaires), corrigé à la clôture de la v0.2 (`--bench '*'`, base de comparaison tolérante) : à confirmer au prochain passage. `release.yml` répété à blanc avec succès en v0.1.
- Clavier à l'écran et navigation manette testés par tests unitaires seulement (pas de manette en session cloud).
- « Progression » dans la liste des mondes : il n'y a pas encore de progrès (v0.2+) ; la liste montre date, mode et temps de jeu.
- Capture d'un monde prise seulement en le quittant (pas aux sauvegardes automatiques).
- Pas de conversion des sauvegardes avant la v1.0 (règle du projet) : la « copie avant migration » arrivera avec la première migration.
- Nom du bloc visé sans « outil requis » : les outils arrivent en v0.3.
- Bateaux : retirés seulement en créatif (clic gauche), sans objet rendu ; hors créatif un bateau posé n'est pas récupérable tant que la survie (v0.3) n'existe pas. Seaux réservés au créatif pour la même raison.
- Horodatage GPU indisponible sur lavapipe (« n/d » dans les rapports cloud).
- Réglage sans système : distance de simulation (pas encore d'entités simulées).
- Console : pas de saisie à la manette (règle du projet : « console exceptée ») ; `/gamemode survival` répond que la survie arrive en v0.3.
- Wiki : images en PNG (pas de WebP sans dépendance C)  ; le biome rivière n'a pas de vue (aucune rivière près de l'origine pour `--render-biomes`).
- Lumière : un bloc changé copie la colonne entière (`Arc::make_mut`, 1 fois par colonne et par image) pour le thread de lumière ; la courbe de lumière est calculée par fragment dans le shader (une table serait moins chère). Une colonne renvoyée laisse chez ses voisines la lumière de ses anciens blocs.
- Sons : verre, lave et vent synthétisés à faire écouter ; pas de cris d'oiseaux (repli CC0 possible avec l'accord de l'utilisateur).

## À surveiller

- DS (cloud, lavapipe) : chargement initial ≈ 9 s (cible < 10 s, de justesse) : le client demande sa distance + 2 (disque de rayon 8 au lieu de 6, ~1,8 fois plus de colonnes) ; en cloud le rendu logiciel prend du processeur à la génération. À remesurer sur PC ; piste si besoin : charger d'abord le disque dessiné.

## Blocages

- Envoi des tags refusé (HTTP 403) par la passerelle Git de la session cloud ; les branches passent. Contournement : l'utilisateur pousse le tag de fin de version.

## En attente de l'utilisateur

- Repli CC0 pour les sons : reporté à plus tard (l'utilisateur, 11/11/2026 : les sons conviennent pour l'instant).
- Retester la v0.2.1 en jeu à sa fin.


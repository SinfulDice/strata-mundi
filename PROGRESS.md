# PROGRESS — Strata Mundi

## État

- **Version en cours** : v0.1 — Fondations
- **Jalon en cours** : 9 — Wiki
- **Dernière session** : 23/10/2026

## Fait

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
- Machine de jeu de l'utilisateur : PC Linux (carte graphique à renseigner par l'utilisateur).

## Mesures

| Date | Machine | Profil | FPS moyen | 1 % low | Mémoire | Source |
|---|---|---|---|---|---|---|

Benchmarks CPU (cloud, `cargo bench`) :

| Date | Opération | Temps | Source |
|---|---|---|---|
| 23/10/2026 | Maillage d'une section de surface | 0,27 ms | `meshing/mesh_section_terrain` |
| 23/10/2026 | Maillage pire cas (damier) | 1,32 ms | `meshing/mesh_section_checkerboard` |
| 23/10/2026 | Génération d'une colonne | 2,08 ms | `generation/generate_column` |
| 23/10/2026 (jalon 8) | Maillage d'une section de surface | 0,18 ms | `meshing/mesh_section_terrain` |
| 23/10/2026 (jalon 8) | Maillage pire cas (damier) | 1,06 ms | `meshing/mesh_section_checkerboard` |
| 23/10/2026 (jalon 8) | Génération d'une colonne | 2,27 ms (+9 %, bruit de VM partagée, sous le seuil de 10 %) | `generation/generate_column` |

Mémoire et temps (cloud, rendu logiciel lavapipe, indicatif) :

| Date | Profil | Mesure | Valeur | Source |
|---|---|---|---|---|
| 23/10/2026 | DS | Pic de mémoire (RSS), vol de 2 min | 201 Mio (cible ≤ 2 Go) | `--bench-scene flyover --preset ds` |
| 23/10/2026 | DS | Lancement → carte graphique prête | 0,2 s (cible < 5 s) | journal |
| 23/10/2026 | DS | Chargement initial du monde (distance 6) | ≈ 4 s (cible < 10 s) | durée totale − préchauffage − mesure |

## Limitations connues

- CI et benchmarks verts sur GitHub ; `release.yml` (AppImage, Flatpak, `.msi`) ne tournera qu'au premier tag.
- L'installeur `.msi` n'embarque pas encore le dossier `assets/` (à faire avant la release v0.1.0).
- Clavier à l'écran et navigation manette testés par tests unitaires seulement (pas de manette en session cloud).
- « Progression » dans la liste des mondes : il n'y a pas encore de progrès (v0.2+) ; la liste montre date, mode et temps de jeu.
- Capture d'un monde prise seulement en le quittant (pas aux sauvegardes automatiques).
- Pas de conversion des sauvegardes avant la v1.0 (règle du projet) : la « copie avant migration » arrivera avec la première migration.
- Nom du bloc visé sans « outil requis » : les outils arrivent en v0.3.
- Horodatage GPU indisponible sur lavapipe (« n/d » dans les rapports cloud).
- Réglages sans système en v0.1, ajoutés avec leur système : nuages, particules, distance de simulation (pas encore d'entités simulées), son.
- Console : pas de saisie à la manette (règle du projet : « console exceptée ») ; `/gamemode survival` répond que la survie arrive en v0.3.
- Vérification de mise à jour : aucune release publiée avant le tag v0.1.0 (réponse 404, sans effet).

## Blocages

## En attente de l'utilisateur

- Maquettes de thème d'interface : en fin de v0.1.

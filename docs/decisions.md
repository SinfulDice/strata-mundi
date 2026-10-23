# Décisions techniques

Une entrée courte par décision : contexte, choix, alternatives écartées. Les plus récentes en bas.

## D001 — Workspace et noms des crates (v0.1, jalon 0)

- **Contexte** : architecture en crates imposée par `CLAUDE.md`.
- **Choix** : un workspace Cargo (`resolver = "3"`, édition 2024) dont les crates se nomment `strata-<dossier>` (`strata-core`…) pour éviter les collisions avec des crates existantes (`core` est réservé par Rust). Le binaire du jeu s'appelle `strata`. Dépendances et lints déclarés une seule fois dans le `Cargo.toml` racine.
- **Alternatives écartées** : un seul crate avec modules (compilation plus lente, frontières client/serveur non garanties par le compilateur).

## D002 — Toolchain fixée (v0.1, jalon 0)

- **Choix** : `rust-toolchain.toml` épingle **Rust 1.97.0** (dernière stable au 23/10/2026). Mise à jour volontaire, dans un commit dédié.
- **Alternative écartée** : `channel = "stable"` (la CI changerait de compilateur sans prévenir).

## D003 — Lints de sécurité (v0.1, jalon 0)

- **Choix** : `unsafe_code = "deny"` et `clippy::unwrap_used`/`expect_used` en avertissement (donc bloquants avec `-D warnings`), autorisés dans les tests (`clippy.toml`). Un `unsafe` exigera une entrée ici et un `#[allow]` local.

## D004 — Formats de fichiers (v0.1, jalon 0)

- **Choix** : **TOML** (`toml` 1.x) pour ce que l'humain édite (options, touches), **RON** (`ron` 0.12) pour le contenu des packs, **postcard + zstd** pour les sauvegardes (bincode prévu à l'origine, abandonné : D026), **Fluent** pour les traductions. Tout passe par `serde`.

## D005 — Ligne de commande : `clap` (v0.1, jalon 0)

- **Contexte** : le binaire aura beaucoup d'options (`--screenshot`, `--seed`, `--pos`, `--look`, `--bench-scene`, `--preset`, `--texture-sheet`, `--render-icons`…).
- **Choix** : `clap` 4 (dérivation), crate de référence très maintenue ; aide générée automatiquement.
- **Alternative écartée** : analyse manuelle de `std::env::args` (beaucoup de code répétitif, messages d'erreur pauvres).

## D006 — Logs et profilage (v0.1, jalon 0)

- **Choix** : `tracing` + `tracing-subscriber` (filtre `RUST_LOG`, `info` par défaut, sortie sur stderr). Feature Cargo `profiling` qui ajoute `tracing-tracy` 0.12 : les spans `tracing` deviennent des zones Tracy, sans code spécifique. Tracy n'est jamais compilé dans les builds normaux.
- **Note** : `tracing-tracy` 0.12 embarque le client Tracy ; utiliser la version du profileur indiquée dans la documentation de `tracy-client` correspondante.

## D007 — CI, benchmarks et release (v0.1, jalon 0)

- **CI** (`ci.yml`) : fmt, puis clippy + tests + build release sur Ubuntu 24.04 et Windows.
- **Benchmarks** (`bench.yml`, séparé pour ne pas rendre la CI principale rouge à cause du bruit des machines partagées) : `criterion` sur le dernier tag `v*` (baseline `release`), puis sur `main` ; `scripts/bench-compare.sh` échoue si une moyenne ralentit de plus de 10 %.
- **Release** (`release.yml`, sur tag `v*`) : Linux compilé sur Ubuntu 22.04 (glibc plus ancienne, binaire plus portable) → `.tar.gz` et AppImage (`appimagetool`) ; Flatpak (`flatpak-builder`, runtime freedesktop 24.08) à partir du même binaire ; Windows → `.zip` et `.msi` (`cargo-wix`, WiX 3). Puis publication de la release et régénération de la branche orpheline `wiki` (force-push d'un seul commit).
- **Identifiant d'application** : `io.github.sinfuldice.StrataMundi`.

## D008 — Fenêtre, rendu et maths : `winit`, `wgpu` (Vulkan seul), `glam` (v0.1, jalon 1)

- **Choix** : `winit` 0.30 (fenêtre, clavier, souris ; référence de l'écosystème), `wgpu` 30 compilé **uniquement avec la feature `vulkan`** (plus `wgsl`), `glam` 0.34 (module `camera` pour les matrices). `pollster` pour attendre les quelques appels `async` d'initialisation, `bytemuck` pour envoyer les sommets sans `unsafe`.
- **Alternatives écartées** : backends GL/DX12/Metal (plusieurs chemins à tester), `nalgebra` (plus lourd), `cgmath` (peu maintenu).

## D009 — Rendu relatif à la caméra, reverse-Z, immediates (v0.1, jalon 1)

- **Caméra** en `f64` ; chaque maillage est dessiné avec son origine relative à la caméra, envoyée en **immediates** (ex-*push constants*, 16 octets par draw, feature `IMMEDIATES` requise : tous les GPU Vulkan la supportent). Pas de tremblement à 10 millions de blocs.
- **Reverse-Z + plan lointain infini** (`Depth32Float`, test `Greater`, effacement à 0) : précision de profondeur quasi uniforme, pas de z-fighting lointain.
- **Convention** : lacet 0 = nord (−Z), 90 = est (+X) ; tangage positif vers le haut ; Y vers le haut, repère main droite.
- **Alternative écartée** : décalage dans le uniform de caméra (une écriture de buffer par objet, inutilisable avec plusieurs objets dans une même passe).

## D010 — Manettes : `gilrs` (v0.1, jalon 1)

- **Choix** : `gilrs` 0.11 (Linux evdev, Windows XInput/WGI), boutons nommés par position (South/East…) donc indépendants de la marque. Facultatif : sans manette ou sans droits, le jeu tourne au clavier.
- **Couche d'actions** maison (`Action`, `Bindings`, `InputState`) : clavier en **codes physiques** (`KeyCode`), plusieurs liaisons par action, zone morte radiale de 15 %, dernier périphérique mémorisé pour les invites, tactile déjà prévu comme périphérique.

## D011 — Boucle de jeu et transport local (v0.1, jalon 1)

- **Serveur** sur son propre thread, horloge à pas fixe (`TickClock`, 20 TPS, rattrapage plafonné à 10 ticks). **Client** sur le thread principal (exigé par `winit`), rendu aussi vite que permis (VSync par défaut).
- **Transport solo** : canaux `std::sync::mpsc` typés (`ClientMessage`/`ServerMessage`, tous `Serialize`). `crossbeam-channel` n'est pas nécessaire tant qu'il n'y a qu'un producteur par sens.

## D012 — Captures et bench (v0.1, jalon 1)

- **Captures** : rendu hors écran (`Rgba8UnormSrgb`) puis relecture et encodage avec la crate `png` (légère, pure Rust ; `image` serait bien plus lourd).
- **Bench** : scènes en RON (`assets/bench-scenes/`), horodatage GPU par requêtes de timestamps lues en différé (4 emplacements tournants), mémoire et description de la machine via `sysinfo` 0.39 (multi-plateforme, maintenu), date via `chrono` (sans fuseau horaire embarqué). Rapport `perf-reports/<date>-<machine>-<profil>-<scène>.md`.
- **Headless** : en cloud, Mesa **lavapipe** suffit pour `--screenshot` ; la fenêtre et le bench se testent sous **Xvfb** (`xvfb-run`). Sous X11, `winit` charge `libxkbcommon-x11.so.0` à l'exécution (paquet `libxkbcommon-x11-0`).

## D013 — Format des packs (v0.1, jalon 2)

- **Choix** : un fichier RON par entrée (`blocks/<nom>.ron`), le nom du fichier donne l'identifiant ; un sous-dossier portant un namespace sert à **remplacer** l'entrée d'un autre pack ; `remove.ron` retire des entrées. Ordre de chargement : `priority` puis namespace. Erreurs collectées avec fichier, ligne et colonne ; jamais de plantage.
- **Remplacement complet** (pas de fusion champ par champ) : plus simple à comprendre, et les champs par défaut gardent les fichiers courts. Une fusion partielle pourra s'ajouter en v1.1 si le modding le demande.
- **Air** : intégré au moteur (`strata:air`, `BlockId` 0), non définissable par un pack.
- **Numéros de registre** attribués dans l'ordre alphabétique des identifiants, valables seulement en mémoire ; les sauvegardes stockeront les identifiants texte.

## D014 — Textures : palette globale + recettes (v0.1, jalon 2)

- **Choix** : toutes les couleurs viennent de **rampes nommées** d'une palette globale (`palette.ron`, fusionnée entre packs). Les textures sont des **recettes** RON (`Material`, `Overlay`, `Ore`, `Sprite`, `Recolor`) ou des PNG (16/32/64 px, agrandis au plus grand format présent, plus proche voisin). Un style complet = une palette ; c'est ainsi que sont produites les planches d'essai.
- **Bruit** maison (hachage entier + bruit de valeur périodique) : textures qui se répètent sans couture, identiques sur toutes les plateformes, sans dépendance.
- **Alternatives écartées** : PNG dessinés à la main uniquement (contraire à « généré par code ») ; couleurs écrites dans chaque recette (impossible de restyler le jeu d'un coup).

## D015 — Traductions : `fluent-bundle` (v0.1, jalon 2)

- **Choix** : `fluent-bundle` 0.16 (version `concurrent`, partageable entre threads) + `fluent-syntax` (liste des clés d'un fichier) + `unic-langid`. Le LOLcat utilise les règles de grammaire anglaises. Isolation Unicode désactivée (les marques invisibles s'afficheraient en carrés dans une police bitmap).
- **Clés** dérivées des identifiants : `block-strata-stone`, `item-…`, `category-…`.

## D016 — PNG dans `core` (v0.1, jalon 2)

- **Choix** : la crate `png` (déjà utilisée par le client) est aussi utilisée par `core` pour lire les textures des packs : `core` reste sans GPU et testable partout.

## D017 — Style des textures : « classique » (v0.1, jalon 2)

- **Contexte** : trois planches d'essai (classique, chaleureux, pastel) produites avec `--texture-sheet` sur le même échantillon (`docs/maquettes/`).
- **Choix de l'utilisateur (23/10/2026)** : style **1, classique** — tons naturels proches de Minecraft, contrastes moyens. Sa palette est celle de `assets/packs/strata/palette.ron`.
- Les deux autres palettes restent dans `docs/maquettes/styles/` comme exemples de packs de ressources.

## D018 — Sections à palette et maillage (v0.1, jalon 3)

- **Stockage** : palette par section + indices empaquetés dans des `u64` (0/1/2/4/8/16 bits, jamais à cheval sur deux mots, plus simple et aussi rapide). Section vide = aucun indice alloué. Colonnes de 32 sections.
- **Maillage** : face culling + fusion gloutonne + AO par sommet (4 niveaux, fusion seulement entre faces de même AO), diagonale choisie selon l'AO. Opaque/découpé et translucide séparés ; translucide trié par section, sans écriture de profondeur.
- **Sommet de 8 octets** (position 5+5+5 bits, face, AO, répétition UV, couche) décodé dans le shader ; origine de section en immediates. Alternatives écartées : sommets `f32` (28 octets), texture atlas (bavures aux bords, pas de répétition).
- **Threads** : `crossbeam-channel` 0.5 (file bornée multi-consommateurs, très maintenue) ; versions par section pour jeter les résultats obsolètes ; budget d'envoi GPU de 4 ms par image.
- **Benchmarks** : `criterion` sur une section de surface et sur le pire cas (damier).
- **Limites connues** : le translucide est trié par section, pas à l'intérieur d'une section (artefacts possibles entre deux surfaces d'eau d'une même section, à revoir avec les fluides en v0.2). Les sections enfouies (une seule sorte de bloc opaque, entourées de sections identiques) ne sont pas maillées. AO désactivée = coins tous éclairés, ce qui permet aussi plus de fusions.

## D019 — Génération : `fastnoise-lite`, climat à 4 cartes (v0.1, jalon 5)

- **Bruit** : `fastnoise-lite` 1.1 (portage officiel de FastNoiseLite, MIT, sans dépendance lourde) avec la feature `f64` pour la précision loin de l'origine. Alternative écartée : `noise` (plus lent, API plus lourde).
- **Modèle** : 4 cartes (température, humidité, continentalité, érosion) façon Minecraft 1.18. La hauteur ne dépend pas du biome (transitions sans falaise) ; biome = plus proche voisin dans l'espace climatique (continentalité pondérée ×3 pour garder les océans entiers), frontière légèrement bruitée. Montagnes = érosion faible × bruit en crêtes.
- **Contenu en packs** : `dimensions/`, `biomes/`, `features/` (arbres, piliers, minerais). `remove.ron` accepte ces catégories.
- **Déterminisme** : générateurs pseudo-aléatoires par colonne (SplitMix64 initialisé par graine + position + usage) ; les éléments de surface sont placés depuis les 9 colonnes voisines, dans un ordre absolu, pour ne pas dépendre de l'ordre de génération. Testé.
- **Grottes** : bruit 3D double sur grille 4×4×4 interpolée.

## D020 — Chargement du monde (v0.1, jalon 4)

- **Serveur** : disque de colonnes autour du joueur, priorité distance × orientation, file de génération bornée (4 travaux par thread), envoi limité à 48 colonnes/tick, déchargement avec 2 colonnes d'hystérésis. Les colonnes voyagent en `ServerMessage::Column(Box<Column>)` (structures `serde`).
- **Client** : maillage d'une colonne quand ses 8 voisines sont présentes (jamais de remaillage pour une voisine arrivée plus tard) ; frustum culling (plans latéraux + proche) ; le bench commence quand ~80 % du disque est chargé et maillé.
- **Distance verticale** : relevée (DS 4 sections, ultra 12) après essais, une valeur plus faible coupait les reliefs vus d'en haut.

## D021 — ECS maison (v0.1, jalon 6)

- **Choix** : entités à génération + un `Storage<T>` (sparse set) par composant + systèmes en fonctions libres, dans `server::ecs` (~200 lignes, tests compris).
- **Alternatives écartées** : `bevy_ecs` (lourd, API changeante, ordonnanceur inutile à 20 TPS), `hecs` (bien, mais le besoin v0.1 est un seul joueur ; on garde la main sur la sérialisation des sauvegardes). À réévaluer avec les mobs (v0.4) si les requêtes multi-composants deviennent nombreuses.

## D022 — Physique partagée, prédiction et réconciliation (v0.1, jalon 6)

- **Choix** : la physique du joueur (`core::physics::step`) est déterministe, à pas fixe (50 ms), et ne lit le monde qu'à travers une fonction « solide ? ». Client et serveur exécutent le même code.
- Le client applique chaque entrée tout de suite, l'envoie numérotée (`Input { sequence }`), puis repart de l'état officiel (`PlayerState`) en rejouant les entrées non confirmées. Rendu interpolé entre les deux derniers pas.
- **Anti-triche minimal** : entrées appliquées au rythme de l'horloge (réserve de rattrapage de 3), file bornée à 40, entrées non finies ignorées, chute automatique après 0,5 s sans entrée ; casser/poser : position validée avant tout calcul, portée (6 blocs serveur, 5 client), 10 actions/s au plus ; téléportation réservée au créatif. Solidité du monde (dessous, bordure, colonnes absentes) partagée : `physics::solid_at`.
- **Alternative écartée** : le client envoie sa position (jalons 1-5) — incompatible avec un serveur autoritaire.

## D023 — Interface maison déclarative (v0.1, jalon 6)

- **Choix** : écrans en RON (`assets/ui/`), nœuds ancrés + décalage en pixels d'interface, échelle entière automatique, comportement dans des `ScreenModel` Rust. Un seul atlas GPU (police, icônes, pixel blanc), un seul appel de dessin.
- **Police** : bitmap décrite en RON dans le pack (glyphes en `#`/`.`, accents composés automatiquement), donc remplaçable par pack de ressources. **Icônes** isométriques calculées sur CPU depuis les textures.
- **Alternatives écartées** : `egui`/`iced` (style difficile à rendre pixel art, navigation manette incomplète, dépendances lourdes) ; police TTF (`fontdue`) gardée pour la police d'accessibilité du jalon 8.
- Format interne jusqu'à la v1.1 (mods).

## D024 — Blocs orientés par variantes générées (v0.1, jalon 6)

- **Choix** : `placement: Axis` dans le pack ; au chargement, trois blocs du registre (`x`, debout, `z`) se partagent nom et objet, faces permutées. Le serveur choisit la variante d'après la face cliquée ; le mailleur tourne d'un quart la texture des faces longues (aucun octet de plus par sommet).
- **Alternative écartée** : états de bloc (propriétés par bloc stockées dans les chunks) — plus général mais coûteux en mémoire et en complexité ; les variantes « aplaties » sont ce que fait Minecraft depuis 1.13 et suffisent aux escaliers et dalles à venir.

## D025 — Fenêtre verticale glissante (v0.1, jalon 6)

- **Constat** : avec une distance verticale de 4 sections (DS), un joueur à 230 de haut ne voyait plus rien.
- **Choix** : la fenêtre verticale est centrée sur `min(section de la caméra, plus haute section non vide de la colonne)`. Sous terre rien ne change ; vu du ciel, chaque colonne garde ses sections de surface.

## D026 — Format des sauvegardes : `postcard` + `zstd` (v0.1, jalon 7)

- **Contexte** : `technique.md` prévoyait `bincode`, mais `bincode` 3.0 est publié comme crate abandonnée (plus maintenue) ; la règle « crate maintenue » l'emporte.
- **Choix** : `postcard` 1.1 (serde, format binaire compact et stable, très maintenu) pour les colonnes ; `zstd` 0.14 (niveau 3) colonne par colonne ; RON lisible pour `world.ron` et `player.ron`.
- **Régions** de 32×32 colonnes : un fichier `r.<x>.<z>.region` (en-tête `SMRG` + postcard) ne contient **que les colonnes modifiées**, compressées une à une ; les autres sont régénérées depuis la graine. Les blocs sont stockés par **identifiant texte** (palette par colonne), donc un monde survit à l'ajout ou au retrait de packs (bloc disparu → air).
- **Écriture atomique** (fichier temporaire, `fsync`, renommage, `fsync` du dossier) ; région illisible mise de côté en `.damaged` au lieu d'être écrasée ; colonnes relues validées (`Column::remapped`), jamais de panique sur un fichier abîmé.
- **Versions** : `FORMAT_VERSION` dans `world.ron` ; autre version → statut « trop ancien / trop récent » dans la liste, pas d'ouverture. Pas de migration en 0.x (règle du projet), donc pas encore de « copie avant migration » : elle viendra avec la première migration (v1.0). Les copies tournantes (3 par défaut) sont faites à chaque ouverture d'un monde.
- **Dossiers** : `dirs` 7 (emplacements standard Linux/Windows), surchargeable par `--data-dir`.

## D027 — Menus : pile de menus, modèles et lanceur (v0.1, jalon 7)

- **Pile** de menus (`Menu`), chaque menu = une disposition RON + un modèle Rust ; les modèles ne font qu'émettre des **effets** (ouvrir, fermer, action) que `Screens` et la session appliquent. Les actions qui touchent aux mondes passent par le trait **`Launcher`** fourni par `app` : le client ne lit ni n'écrit aucun fichier de sauvegarde.
- **Étapes** de la session : écran titre (panorama généré hors serveur, graine fixe) ou monde en cours (connexion + fil du serveur, rejoint en quittant pour que tout soit écrit avant de revenir à la liste).
- **Captures des mondes** : rendu hors écran 96×54 au format de la fenêtre, envoyé au serveur dans `Save { icon }` ; affichées depuis des emplacements réservés de l'atlas d'interface (mis à jour par `write_texture`).
- **Options d'interface** dans `config/interface.toml` (langue, échelle, pause dans tous les menus, disposition du HUD) ; le fichier de réglages complet du jalon 8 les reprendra.
- **Boutons de manette** nommés comme sur la New 3DS (A droite, B bas) ; les invites suivent le dernier périphérique et, au clavier, la lettre réellement imprimée (apprise à la frappe).

## D028 — Veille du DS : `zbus` (v0.1, jalon 7)

- **Choix** : `zbus` 5 (D-Bus en Rust pur, très maintenu, API bloquante), Linux uniquement (`cfg(target_os = "linux")`), sur un fil dédié : verrou de retardement logind + signal `PrepareForSleep`. Sans logind, simple message dans le journal.
- **Alternatives écartées** : lancer `dbus-monitor`/`gdbus` (fragile, dépend d'outils absents d'un Linux minimal), `dbus` (lie `libdbus` en C).

## D029 — Fichier de réglages unique (v0.1, jalon 8)

- **Choix** : `config/settings.toml` remplace `interface.toml` ; quatre sections (`graphics`, `interface`, `controls`, `game`) et un `preset` (absent = « personnalisé »). Toute valeur manquante ou invalide reprend sa valeur par défaut, le reste du fichier est gardé. Les touches sont écrites par action (`MoveForward = [{ Key = "KeyW" }, …]`) pour rester modifiables à la main.
- **Application à chaud** : l'écran d'options modifie une copie ; chaque image, la session la compare à la copie en vigueur et ne refait que ce qui a changé (re-maillage, échantillonneur, VSync…).
- **Passe finale** (`render/post.rs`) uniquement si l'échelle de rendu ≠ 100 % ou si un filtre daltonien est actif : coût nul sinon. Filtres : méthode « daltonize » de Fidaner, Lin et Ozguven (simulation LMS, puis report de l’écart vers des couleurs distinguables).
- **Option `--preset`** : appliquée au lancement sans être écrite, sauf si le joueur modifie ensuite un réglage.

## D030 — Mises à jour et plantages : `ureq` et `open` (v0.1, jalon 8)

- **Vérification de mise à jour** : `ureq` 3 (client HTTP bloquant, petit, maintenu, rustls) sur un fil à part, une requête à l'API GitHub des releases, délai 8 s, certificats du système (`platform-verifier`). Désactivable ; rien n'est envoyé hormis la requête elle-même.
- **Rapports de plantage** : crochet de panique qui écrit `crash-reports/crash-<date>.txt` (version, système, message, pile d'appels, 60 dernières lignes du journal gardées en mémoire). Au lancement suivant, l'écran titre propose « Signaler ce plantage » : `open` 5 ouvre le navigateur sur une issue GitHub pré-remplie (adresse limitée à ~6 000 caractères). Jamais d'envoi automatique.
- **Alternatives écartées** : `reqwest` (tire `tokio`), `webbrowser` (équivalent, moins utilisé).

## D031 — Police lisible : `fontdue` (v0.1, jalon 8)

- **Choix** : `fontdue` 0.9 (rastériseur TrueType en Rust pur, sans dépendance C, très rapide) ; la police (DejaVu Sans, licence libre propre) est rastérisée au chargement dans le même atlas que la police pixel, à la **densité de l'écran** (échelle d'interface ×1 à ×3, moins si l'atlas de 512 px ne suffit pas). Le dessin du texte accepte des positions fractionnaires en pixels d'interface.
- **Alternatives écartées** : `ab_glyph` (équivalent, un peu plus lent), `cosmic-text` (mise en page complète, beaucoup plus lourd, inutile pour du texte d'interface court).

## D032 — Console : commandes exécutées par le serveur, réponses traduisibles (v0.1, jalon 8)

- **Choix** : la ligne tapée part telle quelle (`ClientMessage::Command`) ; le serveur l'analyse et répond par `Feedback { key, args, error }` (clé Fluent + arguments), jamais par du texte. La table des commandes (`protocol::commands`) est partagée pour l'aide et la complétion côté client. `/help` et `/cinematic` restent côté client (rien à décider pour le monde).
- **Heure du monde** : `day_time` dans `world.ron` (`#[serde(default)]`, compatible avec les mondes existants), journée de 24 000 ticks (20 min) ; le ciel la suivra en v0.2.
- **Alternative écartée** : analyser côté client et envoyer des messages typés par commande — chaque nouvelle commande toucherait le protocole, et le serveur doit de toute façon tout revérifier.

## D033 — Cinématiques en packs (v0.1, jalon 8)

- **Choix** : `cinematics/<nom>.ron` (points clés temps/position relative/angles, sous-titre optionnel en clé de traduction, `skippable`, `letterbox`), trajectoire Catmull-Rom, angles par le plus court chemin, validés au chargement (au moins deux points, temps croissants, valeurs finies). Lecture côté client, joueur immobile pendant le plan.
- **Plus tard** : déclenchement par le serveur (boss, découvertes) avec un message dédié ; le format ne changera pas.

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

- **Choix** : **TOML** (`toml` 1.x) pour ce que l'humain édite (options, touches), **RON** (`ron` 0.12) pour le contenu des packs, **bincode + zstd** pour les sauvegardes, **Fluent** pour les traductions. Tout passe par `serde`.

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

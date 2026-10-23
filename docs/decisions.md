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

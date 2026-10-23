# Strata Mundi

*« Les couches du monde »* — un jeu voxel sandbox inspiré de Minecraft, en Rust, sur un moteur maison (`wgpu`, Vulkan).

> État : **v0.1 en développement** (fondations). Voir `PROGRESS.md` et `docs/roadmap.md`.

## Prérequis

- **Rust** : installé avec [rustup](https://rustup.rs). La version exacte est fixée dans `rust-toolchain.toml` et téléchargée automatiquement.
- **Une carte graphique compatible Vulkan** (le jeu n'utilise que Vulkan).
- **Linux** : quelques paquets système (Debian/Ubuntu) :

  ```sh
  ./scripts/install-linux-deps.sh
  ```

  (installe `mesa-vulkan-drivers vulkan-tools libvulkan-dev pkg-config libudev-dev libasound2-dev libxkbcommon-dev libxkbcommon-x11-0 libwayland-dev libx11-dev`).
- **Windows** : un pilote graphique à jour suffit.

## Compiler et lancer

```sh
cargo run --release            # lance le jeu
cargo test --workspace         # lance les tests
cargo bench --workspace        # lance les benchmarks
```

Le binaire s'appelle `strata` (`target/release/strata`). `strata --help` liste les options de ligne de commande, notamment :

```sh
strata --screenshot capture.png --pos=-12,15.5,14 --look 41,-28   # image fixe sans fenêtre
strata --bench-scene flyover --preset ds                           # mesure de performance (voir perf-reports/)
strata --seed 42                                                   # monde temporaire, sans passer par les menus
strata --data-dir ./donnees                                        # sauvegardes et réglages ailleurs
```

Le jeu s'ouvre sur l'écran titre : Jouer, puis Nouveau pour créer un monde. Contrôles : ZQSD/WASD, Espace pour sauter (deux fois pour voler), Maj pour s'accroupir, Ctrl pour sprinter, clic gauche/droit pour casser/poser, clic molette pour prendre le bloc visé, E pour l'inventaire, 1-9 ou molette pour la barre, Échap pour le menu pause (Sauvegarder et quitter). Détails et manette : `docs/explications/entrees.md`.

### Profilage avec Tracy

```sh
cargo run --release --features profiling
```

puis connecter le profileur [Tracy](https://github.com/wolfpld/tracy) (version compatible avec `tracing-tracy` 0.12, voir `docs/decisions.md`). La connexion peut se faire à distance, par exemple pour profiler la console DS depuis un PC.

## Organisation

| Dossier | Contenu |
|---|---|
| `crates/core` | Types purs : blocs, objets, chunks, registres, packs (sans GPU) |
| `crates/worldgen` | Génération procédurale déterministe |
| `crates/server` | Simulation autoritaire à 20 TPS |
| `crates/protocol` | Messages client ↔ serveur |
| `crates/client` | Rendu, entrées, interface |
| `crates/app` | Le binaire du jeu |
| `docs/` | Conception, règles, versions, explications |

Le manuel technique, écrit pour les débutants, est dans [`docs/explications/`](docs/explications/README.md).

## Licences

- **Code** : licence MIT (`LICENSE`).
- **Contenu** (textures, sons, musiques, textes, lore, traductions, textes du wiki) : **CC BY-NC 4.0** (`LICENSE-CONTENT`) — partage et modification libres avec attribution, sans usage commercial.

## Contribuer

Les issues sont bienvenues ; les pull requests extérieures ne sont pas acceptées pour l'instant. Voir `CONTRIBUTING.md`.

Aucune télémétrie.

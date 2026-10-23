# PROGRESS — Strata Mundi

## État

- **Version en cours** : v0.1 — Fondations
- **Jalon en cours** : 1 — Fenêtre et rendu minimal
- **Dernière session** : 23/10/2026

## Fait

- **Jalon 0 — Bootstrap** : workspace (`core`, `worldgen`, `server`, `protocol`, `client`, `app`), binaire `strata` (`clap`, `tracing`), feature `profiling` (Tracy), toolchain 1.97.0 épinglée, lints (`unsafe` interdit, `unwrap`/`expect` hors tests), licences MIT + CC BY-NC 4.0, `CONTRIBUTING.md`, modèles d'issues, CI (fmt, clippy, tests, build Linux/Windows), benchmarks avec alerte > 10 % vs dernier tag, release sur tag (`.tar.gz`, AppImage, Flatpak, `.zip`, `.msi`, branche `wiki`), agents/skills/hook Claude Code, `docs/decisions.md`, `docs/explications/README.md`.

## Environnement

- Session cloud Claude Code : Ubuntu 24.04, 4 cœurs, 15 Go RAM, sans GPU. Rust 1.97.0.
- Paquets système (`scripts/install-linux-deps.sh`) : déjà présents.
- `vulkaninfo --summary` : **lavapipe OK** — `llvmpipe (LLVM 20.1.2, 256 bits)`, Vulkan 1.4.318, Mesa 25.2.8.
- Machine de jeu de l'utilisateur : PC Linux (carte graphique à renseigner par l'utilisateur).

## Mesures

| Date | Machine | Profil | FPS moyen | 1 % low | Mémoire | Source |
|---|---|---|---|---|---|---|

## Limitations connues

- Workflows CI/release écrits mais pas encore exécutés sur GitHub : le premier tag les validera (`release.yml` surtout : AppImage, Flatpak, `.msi`).
- L'installeur `.msi` n'embarque pas encore le dossier `assets/` (à ajouter quand il existera, jalon 2).
- `app --bench-scene` arrive avec le rendu (jalon 1+).

## Blocages

## En attente de l'utilisateur

*(décisions ou validations demandées : planches de textures au jalon 2, maquettes de thème en fin de v0.1…)*

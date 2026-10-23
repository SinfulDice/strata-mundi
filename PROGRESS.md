# PROGRESS — Strata Mundi

## État

- **Version en cours** : v0.1 — Fondations
- **Jalon en cours** : 2 — Registres et packs
- **Dernière session** : 23/10/2026

## Fait

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

## Limitations connues

- Workflows CI/release écrits mais pas encore exécutés sur GitHub : le premier tag les validera (`release.yml` surtout : AppImage, Flatpak, `.msi`).
- L'installeur `.msi` n'embarque pas encore le dossier `assets/` (à ajouter quand il existera, jalon 2).
- Les préréglages (`--preset`) ne changent encore rien au rendu : réglages au jalon 8 (le rapport de bench le signale).
- Invites à l'écran selon le dernier périphérique : le périphérique est suivi, l'affichage viendra avec l'interface (jalon 7).
- Titre de fenêtre « Strata Mundi » en dur : passera par Fluent au jalon 2.
- Vitesse de vol, sensibilités souris/stick en dur dans `FlyController` : réglages au jalon 8.
- Actions `Use`/`Attack`/prise de bloc : ajoutées avec l'interaction (jalon 6).
- Horodatage GPU indisponible sur lavapipe (« n/d » dans les rapports cloud).

## Blocages

## En attente de l'utilisateur

*(décisions ou validations demandées : planches de textures au jalon 2, maquettes de thème en fin de v0.1…)*

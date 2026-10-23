# PROGRESS — Strata Mundi

## État

- **Version en cours** : v0.1 — Fondations
- **Jalon en cours** : 4 — Monde infini
- **Dernière session** : 23/10/2026

## Fait

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

## Limitations connues

- Workflows CI/release écrits mais pas encore exécutés sur GitHub : le premier tag les validera (`release.yml` surtout : AppImage, Flatpak, `.msi`).
- L'installeur `.msi` n'embarque pas encore le dossier `assets/` (à faire avant la release v0.1.0).
- Menu d'activation et d'ordre des packs de ressources : avec les menus (jalon 7) ; d'ici là `--resource-pack <nom>`.
- Eau dessinée opaque et sans passe triée, et seuil `discard` appliqué à tous les blocs : passe translucide séparée au jalon 3.
- Les préréglages (`--preset`) ne changent encore rien au rendu : réglages au jalon 8 (le rapport de bench le signale).
- Invites à l'écran selon le dernier périphérique : le périphérique est suivi, l'affichage viendra avec l'interface (jalon 7).
- Vitesse de vol, sensibilités souris/stick en dur dans `FlyController` : réglages au jalon 8.
- Actions `Use`/`Attack`/prise de bloc : ajoutées avec l'interaction (jalon 6).
- Horodatage GPU indisponible sur lavapipe (« n/d » dans les rapports cloud).

## Blocages

## En attente de l'utilisateur

- Maquettes de thème d'interface : en fin de v0.1.

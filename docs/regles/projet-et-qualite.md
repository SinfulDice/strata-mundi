# Projet, distribution, robustesse et tests

## Projet, distribution et robustesse

- **Dépôt public** sur GitHub. **Licences** : code sous **MIT**, contenu (textures, sons, musiques, lore, textes) sous **CC BY-NC 4.0** ; fichiers `LICENSE` et `LICENSE-CONTENT`, et section claire dans le README.
- **Issues ouvertes** (modèles « bug », « plantage », « idée »), PR extérieures non acceptées pour l'instant (le dire dans `CONTRIBUTING.md`).
- **Tout se fait sur `main`** : commits au fil des jalons, tags de version.
- **Distribution** à chaque release : archives `.zip` (Windows) et `.tar.gz` (Linux), **installeur Windows** (`.msi`), **AppImage** et **Flatpak** pour Linux. Générés par la CI.
- **Vérification de mise à jour** au lancement (requête légère sur les releases GitHub, message sur l'écran titre, aucun téléchargement, silencieuse hors ligne, désactivable).
- **Rapports de plantage** : un rapport lisible (version, système, GPU, pile d'appels, derniers logs) dans `crash-reports/` ; au relancement, bouton « Signaler ce plantage » qui ouvre une issue GitHub pré-remplie dans le navigateur. Rien n'est envoyé automatiquement.
- **Aucune télémétrie.**
- **Sauvegardes de sécurité** : copies tournantes automatiques des mondes (les 3 dernières par défaut, réglable) + copie systématique avant toute migration de format.
- **Compatibilité des sauvegardes** : aucune garantie entre versions 0.x (le format est versionné et un monde trop ancien affiche un message clair au lieu de planter) ; **migrations garanties à partir de la v1.0**.
- **Objectifs DS** : **2 Go de RAM maximum** en profil DS ; lancement **< 5 s**, chargement d'un monde **< 10 s** depuis le SSD.

## Tests et benchmarks

- **Tests ciblés** : ceux listés dans chaque version, sur ce qui casserait en silence (déterminisme, sauvegardes, recettes, calculs de jeu).
- **Benchmarks** (`criterion`) des opérations clés : génération d'un chunk, maillage, propagation de lumière, pathfinding, tick d'une usine de référence. Lancés en CI ; **la CI signale toute régression de plus de 10 %** par rapport à la dernière version taguée.
- Pas d'objectif de couverture chiffré.

## Environnement de développement

- Claude Code tourne en **session cloud** (Ubuntu 24.04, sans GPU) : build, tests, benchmarks CPU, captures en Vulkan logiciel (lavapipe).
- L'utilisateur joue et mesure les FPS sur son **PC Linux** (et plus tard sur le DS) avec `app --bench-scene`, et committe les rapports dans `perf-reports/`.
- **Windows n'est testé qu'en CI** (build, tests, benchmarks sur un runner Windows) : ne jamais introduire de code spécifique à Windows sans test CI qui le couvre ; utiliser des abstractions portables (chemins, fichiers, fenêtres).
- Au jalon 0 (et dans le script d'environnement cloud) : toolchain Rust stable via `rust-toolchain.toml`, paquets système `mesa-vulkan-drivers vulkan-tools libvulkan-dev pkg-config libudev-dev libasound2-dev libxkbcommon-dev libwayland-dev libx11-dev` ; vérifier `vulkaninfo --summary` (lavapipe) et noter le résultat dans `PROGRESS.md`. Tracy : seule la feature Cargo est mise en place ; le profilage se fait sur la machine de l'utilisateur.
- Le dépôt GitHub **existe déjà** (créé par l'utilisateur) : ne pas le recréer ; vérifier le remote et pousser sur `main`.

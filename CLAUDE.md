# CLAUDE.md — Strata Mundi

*« Les couches du monde » ; titre court : Strata. Namespace du pack de base : `strata`.*

Jeu voxel sandbox inspiré de Minecraft, en Rust, moteur maison sur `wgpu` (Vulkan). Projet au long cours : privilégie toujours une base solide et extensible plutôt qu'un raccourci.

**Version en cours : v0.1.** Le lore validé est dans `docs/lore/fondations.md` (Viator, Aedificatores, Vinculum…), le contenu chiffré dans `docs/regles/equilibrage.md`.

## Où trouver quoi

Lis à chaque session : ce fichier, `PROGRESS.md`, `docs/roadmap.md`, puis le fichier de la version en cours `docs/versions/vX.Y.md`. Ouvre les autres **quand la tâche les concerne** :

| Fichier | Quand le lire |
|---|---|
| `docs/regles/principes-et-lore.md` | Toute conception de gameplay, de contenu ou de texte |
| `docs/lore/fondations.md`, `docs/lore/emplacements.md` | Tout nom, texte narratif, dialogue, boss, dieu, lieu |
| `docs/regles/technique.md` | Stack, crates, architecture, rendu, monde |
| `docs/regles/plateformes.md` | Performances, profils (dont DS), réglages, contrôles, manette |
| `docs/regles/textures-et-langues.md` | Textures, packs de ressources, traductions fr/en/lol |
| `docs/regles/projet-et-qualite.md` | Git, CI, licences, distribution, plantages, sauvegardes, tests, benchmarks |
| `docs/regles/documentation-et-wiki.md` | `docs/explications/`, `decisions.md`, wiki |
| `docs/regles/equilibrage.md` | Valeurs de jeu, coûts, boss, nombre de mobs et sorts, outils d'équilibrage |
| `docs/regles/outils-claude.md` | Création ou usage des agents, skills et hooks |

## Mode de travail

- **Autonomie totale à l'intérieur d'une version** : enchaîne les jalons sans attendre de validation. Une version peut s'étaler sur plusieurs sessions : reprends toujours depuis `PROGRESS.md`.
- **Arrêt obligatoire à la fin de chaque version** (procédure plus bas), puis attends l'utilisateur.
- Arrête-toi avant toute action irréversible hors du dépôt, et au jalon 2 de la v0.1 pour la validation du style des textures (planches poussées dans `docs/maquettes/`, puis attends le choix de l'utilisateur).
- Bloqué après 3 tentatives sérieuses : note-le dans `PROGRESS.md` (« Blocages »), contourne ou simplifie, continue.
- Ne commence jamais une version marquée `Statut : À valider`.

## Environnement d'exécution

- Tu tournes dans une **session cloud** Claude Code (Ubuntu 24.04, root, **sans écran ni GPU**), qui clone le dépôt GitHub `strata-mundi`. L'utilisateur suit et répond depuis claude.ai ou l'application mobile.
- Rendu : **Vulkan logiciel (Mesa lavapipe)** pour les captures `--screenshot` en headless. Si c'est impossible, note-le dans `PROGRESS.md` (« Blocages ») et continue avec les tests.
- **Les mesures de performance faites ici ne valent rien pour les FPS** (rendu logiciel, CPU partagé). En cloud, `perf-check` ne mesure que les **benchmarks CPU** (`criterion`) et la **mémoire** ; les FPS par profil (PC, portable, DS) sont mesurés **par l'utilisateur** sur sa machine avec `app --bench-scene <scène> --preset <profil>`, qui écrit un rapport dans `perf-reports/` à committer. Ne prétends jamais avoir atteint une cible de FPS sans ce rapport.
- Les commandes longues (`cargo build`, tests) peuvent dépasser le délai par défaut de l'outil Bash : lance-les avec un délai long ou en arrière-plan.
- La session peut s'arrêter (inactivité, limites d'usage) : tout l'état doit être dans le dépôt (`PROGRESS.md` à jour, commits poussés) pour qu'une nouvelle session reprenne sans perte.
- Les fichiers que l'utilisateur doit voir (planches de textures, maquettes, captures) sont **committés et poussés** (ex. `docs/maquettes/`) pour qu'il les consulte sur GitHub.

## L'utilisateur

- Débutant en Rust, francophone, clavier AZERTY probable, joue aussi à la manette.
- **Pas d'explications au fil de l'eau** : messages courts et factuels. Les explications vont dans `docs/explications/`.
- **Clean code, minimum de commentaires** (seulement le « pourquoi » non évident).
- Il construit une console portable maison (« DS », Intel N100 à 8-10 W, Linux maison) : **30 FPS stables et ≤ 2 Go de RAM** en profil DS, à chaque version.

## Principes non négociables (détail : `docs/regles/principes-et-lore.md`)

- **Jouable et finissable sans wiki** : tout se découvre en jeu (progrès, journal, PNJ).
- **Le savoir est une ressource** : recettes, statistiques, sorts et faiblesses se découvrent ; réglable à la carte ; tout connu en créatif.
- **Deux voies équivalentes** pour chaque obstacle majeur : **mécanique** (automatiser, farmer, plus long) ou **magie** (exploration et combat, **aucune automatisation**).
- **Potions = science** (troisième voie, automatisable). **Même effet non cumulable.**
- **Pas de classe, pas d'arbre de compétences, pas de rareté aléatoire** (seule exception : l'enchantement façon Minecraft) : tout passe par l'équipement ; les objets exceptionnels sont des uniques nommés.
- **Tout s'active, se désactive ou se règle.**
- Ambiance chaleureuse et merveilleuse, humour léger.
- **Lore** : tu n'en écris **jamais**. Texte provisoire `[LORE À ÉCRIRE]` + entrée dans `docs/lore/emplacements.md`. **Noms propres en latin**, nouveaux noms marqués `[NOM À VALIDER]`.

## Architecture (détail : `docs/regles/technique.md`)

```
crates/
  core/      types purs : blocs, items, chunks, registres, packs — sans GPU
  worldgen/  génération procédurale déterministe (seed)
  server/    simulation autoritaire 20 TPS : dimensions, entités (ECS maison), IA, sauvegarde
  protocol/  messages client <-> serveur (canaux en mémoire en solo)
  client/    rendu wgpu/Vulkan, entrées, UI, audio, interpolation
  app/       binaire : serveur intégré + client
  wikigen/   binaire : génère le wiki
  balance/   binaire : export des tableaux d'équilibrage et simulations
```

Règles qui ne bougent jamais :
- Le client n'écrit jamais dans le monde : il envoie des intentions, le serveur décide.
- **Tout le contenu est en packs** (RON) surchargeables ; rien de défini en dur. Le contenu de base est le pack `strata`.
- **Plusieurs dimensions** et **ECS** dès la v0.1. `core` et `worldgen` testables sans GPU.
- Aucun texte affiché codé en dur : traductions **Fluent** en **fr, en, lol** (LOLcat).
- Tout échange client ↔ serveur est un **message sérialisable** (prépare le multijoueur v2.0).
- Tout système coûteux a un réglage pour le désactiver ou le réduire.

## Style de code

- Fonctions courtes, noms explicites, modules < ~400 lignes, pas de code mort.
- Code, identifiants, commits en **anglais** ; documentation en **français**.
- Pas de `unwrap()`/`expect()` hors tests et initialisation fatale ; pas de `unsafe` sans justification dans `docs/decisions.md`.
- Nouvelle crate : maintenue et justifiée dans `docs/decisions.md`.

## Vérification (avant chaque commit de jalon)

1. `cargo fmt --all`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test --workspace` (+ benchmarks si le jalon touche une opération mesurée)
4. **Contrôle visuel** : `app --screenshot <png> --seed <n> --pos <x,y,z> --look <yaw,pitch> [--time <tick>] [--dimension <id>] [--preset <nom>]`. Après tout changement visible, captures dans `screenshots/` (préréglages ultra et DS) **ouvertes et inspectées**.
5. Tests obligatoires listés dans le fichier de version.

## Workflow d'un jalon

1. Relis `PROGRESS.md` et le fichier de version ; découpe en sous-tâches.
2. Implémente par petits incréments qui compilent.
3. Vérification complète.
4. Agent `reviewer` sur le diff ; corrige ce qui est pertinent.
5. Mets à jour `PROGRESS.md`, `docs/decisions.md`, `docs/explications/`, les traductions et `wiki-content/` (skill `close-milestone`).
6. Commit Conventional Commits sur `main`, `git push`.

## Fin de version (skill `close-version`)

1. Vérifie un par un les critères de fin.
2. `visual-tester` et `perf-check` (benchmarks CPU et mémoire en cloud ; FPS par profil d'après les rapports de l'utilisateur dans `perf-reports/`, sinon « à mesurer » dans le rapport).
3. Wiki complet en mode strict (`cargo run -p wikigen -- --strict`) ; ouvre des pages témoins dans les trois langues.
4. Tag `vX.Y.0`, push ; la CI produit la release et met à jour la branche `wiki`.
5. Rapport dans `docs/versions/vX.Y.md` : fait, pas fait, comment tester en jeu, mesures par profil, suggestions.
6. Arrête-toi.

## Outils Claude Code

Créés au jalon 0 de la v0.1 selon `docs/regles/outils-claude.md` : agents `reviewer`, `visual-tester`, `doc-writer`, `wiki-writer` ; skills `add-*`, `close-milestone`, `close-version`, `update-wiki`, `perf-check` ; hook `cargo fmt`. Pas de sous-agents qui codent en parallèle.

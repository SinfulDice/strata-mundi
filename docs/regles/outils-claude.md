# Outils Claude Code

## Outils Claude Code (créés au jalon 0 de la v0.1)

Crée-les au format actuel de Claude Code (vérifie la documentation si besoin).

**Agents (`.claude/agents/`)**
- `reviewer` — lecture seule. Relit le diff : clean code, frontière client/serveur, rien en dur (tout en packs, surchargeable), perf de la boucle chaude, réglage de désactivation pour tout système coûteux, `unwrap`/`unsafe`, tests manquants. Liste courte classée par gravité.
- `visual-tester` — lance les captures de référence (`docs/screenshots.ron`), en préréglages ultra et DS, inspecte les PNG, compare aux précédentes, signale les anomalies.
- `doc-writer` — met à jour `docs/explications/` à partir du diff d'un jalon.
- `wiki-writer` — rédige ou complète `wiki-content/` (fr, en, lol) et `history.ron` ; vérifie que les textes ne contredisent pas les packs.

**Skills (`.claude/skills/`)**
- `add-block`, `add-item`, `add-weapon`, `add-biome`, `add-mob`, `add-recipe`, `add-spell`, `add-boss` — procédure pas à pas (fichiers RON, texture/son procédural, traductions fr/en/lol, texte du wiki, historique, test).
- `close-milestone` — fmt, clippy, tests, captures, `reviewer`, `doc-writer`, `wiki-writer`, PROGRESS.md, commit, push.
- `update-wiki` — régénère images et wiki, vérifie le mode strict, ouvre des pages témoins.
- `close-version` — la procédure « Fin de version ».
- `perf-check` — en cloud : benchmarks `criterion` et mémoire, comparés à la mesure précédente ; FPS par profil lus dans les rapports de l'utilisateur (`perf-reports/`), jamais estimés.

**Hook (`.claude/settings.json`)** : après chaque `Edit`/`Write` sur un `.rs`, `cargo fmt --all`.

Pas de sous-agents qui codent en parallèle : les jalons dépendent les uns des autres et les conflits coûtent plus qu'ils ne rapportent.

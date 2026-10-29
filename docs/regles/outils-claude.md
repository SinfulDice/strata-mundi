# Outils Claude Code

## Outils Claude Code (créés au jalon 0 de la v0.1, orchestration ajoutée le 29/10/2026)

Crée-les au format actuel de Claude Code (vérifie la documentation si besoin).

**Agents (`.claude/agents/`)**, avec le modèle de chacun :
- `implementer` (Opus) — implémente **une** sous-tâche de jalon dans un contexte vierge : code, tests, `scripts/verify.sh`, commit, push, case cochée dans `PROGRESS.md`, résumé de 20 lignes au plus.
- `verify` (Haiku) — lance `scripts/verify.sh` et rend seulement le résultat et les erreurs.
- `reviewer` (Opus) — lecture seule. Relit le diff : clean code, frontière client/serveur, rien en dur (tout en packs, surchargeable), perf de la boucle chaude, réglage de désactivation pour tout système coûteux, `unwrap`/`unsafe`, tests manquants. Liste courte classée par gravité.
- `visual-tester` (Sonnet) — lance les captures de référence (`docs/screenshots.ron`), en préréglages ultra et DS, inspecte les PNG, compare aux précédentes, signale les anomalies.
- `doc-writer` (Sonnet) — met à jour `docs/explications/` à partir du diff d'un jalon.
- `wiki-writer` (Sonnet) — rédige ou complète `wiki-content/` (fr, en, lol) et `history.ron` ; vérifie que les textes ne contredisent pas les packs.

**Skills (`.claude/skills/`)**
- `add-block`, `add-item`, `add-weapon`, `add-biome`, `add-mob`, `add-recipe`, `add-spell`, `add-boss` — procédure pas à pas (fichiers RON, texture/son procédural, traductions fr/en/lol, texte du wiki, historique, test).
- `close-milestone` — `verify`, captures, `reviewer`, `doc-writer`, `wiki-writer`, PROGRESS.md, commit, push.
- `update-wiki` — régénère images et wiki, vérifie le mode strict, ouvre des pages témoins.
- `close-version` — la procédure « Fin de version », dont l'allègement de `PROGRESS.md`.
- `perf-check` — en cloud : benchmarks `criterion` et mémoire, comparés à la mesure précédente ; FPS par profil lus dans les rapports de l'utilisateur (`perf-reports/`), jamais estimés.

**Script** : `scripts/verify.sh [--quick]` — fmt, clippy (debug, et release sans `--quick`), tests ; n'affiche qu'un résumé et les erreurs, journaux complets dans un dossier temporaire.

**Hooks (`.claude/settings.json`)**
- Après chaque `Edit`/`Write` sur un `.rs` : `cargo fmt --all`.
- Au redémarrage après une compaction automatique (`SessionStart`, `compact`) : `.claude/hooks/session-context.sh` réinjecte l'état, les blocages et les attentes de `PROGRESS.md`.

## Économie de tokens

Chaque action de Claude renvoie tout le contexte de la session : une session longue coûte de plus en plus cher à chaque appel d'outil. Claude ne peut pas taper `/clear` lui-même ; l'orchestration en tient lieu.

- **Orchestration** : la session principale garde un contexte léger. Elle découpe le jalon, lance un `implementer` par sous-tâche (contexte vierge, jeté à la fin, seul le résumé revient) et enchaîne revue, documentation et commit. Corrections sur la même sous-tâche : `SendMessage` au même agent, qui garde son contexte.
- **Une sous-tâche à la fois** : pas de sous-agents qui codent en parallèle. Les jalons dépendent les uns des autres et les conflits coûtent plus qu'ils ne rapportent.
- **Pas d'agent pour les petites tâches** (quelques fichiers, correction d'une ligne) : chaque agent part de zéro et relit `CLAUDE.md` et ses fichiers, ce qui coûte plus que la tâche.
- **Sorties bruyantes hors du contexte** : `scripts/verify.sh` au lieu des commandes `cargo` brutes ; captures inspectées par `visual-tester`.
- **Modèles** : Opus pour le code et la revue, Sonnet pour la rédaction et l'inspection des captures, Haiku pour la vérification.
- **Lecture de démarrage courte** : `docs/roadmap.md` ne contient que les versions (la vision est dans `docs/regles/vision.md`, lue à la demande) ; `close-version` allège `PROGRESS.md`.
- Les sous-agents ne peuvent pas lancer d'autres agents : `implementer` vérifie avec le script, pas avec `verify`.

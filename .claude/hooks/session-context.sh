#!/usr/bin/env bash
# SessionStart hook after an automatic compaction: re-injects the project state
# so the session keeps its bearings without re-reading every file.
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
[ -f PROGRESS.md ] || exit 0
echo "Contexte compacté. État actuel (extrait de PROGRESS.md) :"
awk '/^## État/ { p = 1 } /^## Fait/ { p = 0 } p' PROGRESS.md
awk '/^## Blocages/ { p = 1 } p' PROGRESS.md | head -n 40
echo "Rappel : orchestre le jalon selon « Workflow d'un jalon » de CLAUDE.md (agent implementer, scripts/verify.sh)."
exit 0

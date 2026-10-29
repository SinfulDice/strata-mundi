---
name: close-milestone
description: Clôturer un jalon de Strata Mundi — vérification complète, captures, revue, documentation, wiki, PROGRESS.md, commit Conventional Commits sur main et push.
---

# Clôturer un jalon

1. **Vérification** (tout doit passer) : agent `verify`, qui lance `scripts/verify.sh` (fmt, clippy debug et release car la CI compile aussi en release, tests) et ne rend que les erreurs. `cargo bench` en plus si le jalon touche une opération mesurée (skill `perf-check`).
2. **Captures** (si changement visible) : agent `visual-tester`, préréglages ultra et DS, PNG dans `screenshots/` **ouverts et inspectés**. Rendu impossible : noter dans `PROGRESS.md` (« Blocages »).
3. **Revue** : agent `reviewer` sur le diff du jalon ; fais corriger ce qui est pertinent par l'agent `implementer` (`SendMessage` au dernier agent si son contexte sert encore, sinon un nouveau), relance `verify`.
4. **Documentation** : agent `doc-writer` (`docs/explications/`) ; entrées dans `docs/decisions.md` pour chaque choix technique ou nouvelle crate.
5. **Traductions** fr/en/lol complètes ; **wiki** : agent `wiki-writer` si du contenu a été ajouté (à partir du jalon 9 de la v0.1).
6. **`PROGRESS.md`** : jalon terminé dans « Fait », jalon suivant dans « État », mesures, limitations, blocages, date de session.
7. **Commit** Conventional Commits en anglais sur `main` (`feat(worldgen): …`, `chore: …`), puis `git push origin main` (réessaie avec attente exponentielle en cas d'erreur réseau).

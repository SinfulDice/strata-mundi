---
name: close-version
description: Procédure de fin de version de Strata Mundi — critères de fin, visual-tester, perf-check, wiki strict, tag, rapport, puis arrêt.
---

# Fin de version

1. Ouvre `docs/versions/vX.Y.md` et **vérifie un par un les critères de fin** ; coche ceux qui sont atteints, documente les écarts.
2. Agent `visual-tester` (ultra et DS) et skill `perf-check`. FPS par profil **uniquement** d'après `perf-reports/` ; sinon « à mesurer ».
3. Wiki complet : `cargo run -p wikigen -- --strict` (skill `update-wiki`) ; ouvre des pages témoins dans les trois langues.
4. Vérification complète (fmt, clippy, tests), `PROGRESS.md` à jour, commit sur `main`.
5. Rédige le rapport dans `docs/versions/vX.Y.md` (section « Rapport de fin de version ») : fait, pas fait, comment tester en jeu, mesures par profil, maquettes ou choix en attente, suggestions pour la suite.
6. Tag : `git tag -a vX.Y.0 -m "vX.Y.0"` puis `git push origin main vX.Y.0`. La CI produit la release et met à jour la branche `wiki`.
7. **Arrête-toi** et attends l'utilisateur. Ne commence pas la version suivante.
